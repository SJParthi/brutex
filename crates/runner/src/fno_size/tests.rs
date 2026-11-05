#![cfg(test)]
//! FNO-SIZE's unit and property tests: every test section 7 of the part's design names, plus
//! the few this file adds and says so. Property tests draw from seeded xorshift generators over
//! many seeds; no dependency is added.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]

use core::cmp::Ordering;

use super::edge::MeanKey;
use super::measure::{
    measure_duplicates, measure_edge_insert, measure_edge_reset, measure_judging, measure_network,
    measure_plan, measure_risk_cycle, measure_risk_release, measure_score, steady, within,
};
use super::order::{Rest, compare, mix, seeded};
use super::*;

// ---------------------------------------------------------------- fixtures --

/// Rs 20 lakh in paise.
const LAKH_20: i128 = 200_000_000;
/// Rs 1 crore in paise.
const CRORE: i128 = 1_000_000_000;
/// The operator's limits, as FNO-RUN passes them.
const OPERATOR: Limits = Limits {
    margin_cap: LAKH_20,
    exposure_cap: CRORE,
    trades_per_day: 100,
};
/// The operator's money limits with room for a whole universe of entries in a day.
const ROOMY: Limits = Limits {
    margin_cap: LAKH_20,
    exposure_cap: CRORE,
    trades_per_day: 256,
};

/// Rs 20 lakh of capital, nothing in force.
fn view() -> AccountView {
    AccountView {
        capital: LAKH_20,
        margin_in_use: 0,
        exposure: 0,
        reserved_margin: 0,
        reserved_exposure: 0,
        trades_today: 0,
        queued: 0,
        open_risk: 0,
        busy: StockSet::EMPTY,
    }
}

fn fraction(num: u64, den: u64) -> Fraction {
    Fraction::new(num, den).unwrap()
}

/// A valid long candidate: Rs 1,000 a share at 5x, FNO-NET's 500 shares, a stop Rs 10 below.
fn cand(stock: u8, recipe: u32) -> Candidate {
    Candidate {
        stock: StockId::new(stock),
        recipe,
        side: Side::Long,
        start: 34_200,
        minute: 10,
        price: 100_000,
        rate_ppm: 200_000,
        fixed_qty: 500,
        stop: Some(99_000),
        stop_in_r: fraction(1, 2),
        liquidity: 0,
    }
}

/// `count` candidates on stocks `0..count`.
fn spread(count: usize) -> Vec<Candidate> {
    (0..count).map(|i| cand(i as u8, 0)).collect()
}

fn budget(per_trade: i128, open_cap: i128) -> RiskBudget {
    RiskBudget::new(per_trade, open_cap).unwrap()
}

fn run(input: &PolicyInput<'_>, view: &AccountView, limits: &Limits, batch: &[Candidate]) -> Plan {
    let mut out = Plan::new();
    plan(input, view, limits, batch, &mut out).unwrap();
    out
}

fn outcome(plan: &Plan, index: u16) -> Outcome {
    plan.rows
        .iter()
        .find(|row| row.index == index)
        .map(|row| row.outcome)
        .unwrap()
}

/// The sized row of an outcome, if it was sized.
fn as_sized(outcome: Outcome) -> Option<Sized> {
    match outcome {
        Outcome::Sized(s) => Some(s),
        Outcome::Refused(_) => None,
    }
}

/// The refusal of an outcome, if it was refused.
fn as_refused(outcome: Outcome) -> Option<SizeRefusal> {
    match outcome {
        Outcome::Refused(word) => Some(word),
        Outcome::Sized(_) => None,
    }
}

fn sized(plan: &Plan, index: u16) -> Sized {
    as_sized(outcome(plan, index)).unwrap()
}

fn refused(plan: &Plan, index: u16) -> SizeRefusal {
    as_refused(outcome(plan, index)).unwrap()
}

/// The (stock, recipe) of each row, in row order.
fn row_keys(plan: &Plan, batch: &[Candidate]) -> Vec<(u8, u32)> {
    plan.rows
        .iter()
        .map(|row| {
            let c = &batch[usize::from(row.index)];
            (c.stock.position(), c.recipe)
        })
        .collect()
}

/// Each row's outcome keyed by its candidate's (stock, recipe), in key order.
fn keyed_outcomes(plan: &Plan, batch: &[Candidate]) -> Vec<((u8, u32), Outcome)> {
    let mut rows: Vec<_> = plan
        .rows
        .iter()
        .map(|row| {
            let c = &batch[usize::from(row.index)];
            ((c.stock.position(), c.recipe), row.outcome)
        })
        .collect();
    rows.sort_by_key(|(key, _)| *key);
    rows
}

/// A learning trade whose value is exactly `ppm` parts per million.
fn trade(month: u16, stock: u8, side: Side, minute: u16, ppm: i128) -> EdgeTrade {
    EdgeTrade {
        month,
        stock: StockId::new(stock),
        side,
        minute,
        net: ppm,
        value: 1_000_000,
    }
}

fn inputs(edge: &EdgeTable) -> [PolicyInput<'_>; 4] {
    [
        PolicyInput::FixedSeeded,
        PolicyInput::FixedRanked { edge, seed: 11 },
        PolicyInput::SplitAll,
        PolicyInput::Risk {
            budget: Some(budget(5_000_000, 50_000_000)),
        },
    ]
}

/// The test's own seeded generator (xorshift64).
struct Xorshift(u64);

impl Xorshift {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn range(&mut self, low: i128, high: i128) -> i128 {
        low + i128::from(self.next()) % (high - low + 1)
    }

    fn side(&mut self) -> Side {
        Side::ALL[(self.next() & 1) as usize]
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }
}

/// A random valid candidate on `stock` with `recipe`, drawn so that keys tie often.
fn random_cand(rng: &mut Xorshift, stock: u8, recipe: u32) -> Candidate {
    let side = rng.side();
    let price = 1 + rng.below(2_000_000) as i64;
    let distance = rng.below(5_000) as i64 + 1;
    let stops = [(1, 2), (2, 4), (1, 3), (2, 3), (0, 1)];
    let (num, den) = stops[rng.below(stops.len() as u64) as usize];
    Candidate {
        stock: StockId::new(stock),
        recipe,
        side,
        start: 34_200,
        minute: rng.below(375) as u16,
        price,
        rate_ppm: 200_000 + rng.below(1_000_000) as u32,
        fixed_qty: 1 + rng.below(2_000),
        stop: match rng.below(8) {
            0 => None,
            1 => Some(price),
            2 => Some(match side {
                Side::Long => price + distance,
                Side::Short => price - distance,
            }),
            _ => Some(match side {
                Side::Long => price - distance,
                Side::Short => price + distance,
            }),
        },
        stop_in_r: fraction(num, den),
        liquidity: u128::from(rng.below(3) * 5),
    }
}

/// A random batch of `count` candidates on stocks below `stocks`, recipes distinct.
fn random_batch(rng: &mut Xorshift, count: usize, stocks: u64) -> Vec<Candidate> {
    (0..count)
        .map(|i| {
            let stock = rng.below(stocks) as u8;
            random_cand(rng, stock, i as u32)
        })
        .collect()
}

/// A random learning table over stocks below `stocks`, small values so the test's own
/// arithmetic cannot overflow.
fn random_table(rng: &mut Xorshift, stocks: u64, months: u16) -> (Vec<EdgeTrade>, EdgeTable) {
    let count = rng.below(120) as usize;
    let mut trades: Vec<EdgeTrade> = (0..count)
        .map(|_| {
            trade(
                rng.below(u64::from(months)) as u16,
                rng.below(stocks) as u8,
                rng.side(),
                rng.below(375) as u16,
                rng.range(-50, 50),
            )
        })
        .collect();
    trades.sort_by_key(|t| t.month);
    let table = EdgeTable::build(&[60, 150, 240], 0, months, &trades).unwrap();
    (trades, table)
}

// ------------------------------------------------------ batch sizes and caps --

#[test]
fn an_empty_batch_plans_nothing_and_divides_by_nothing() {
    let edge = EdgeTable::default();
    for input in inputs(&edge) {
        let plan = run(&input, &view(), &OPERATOR, &[]);
        assert_eq!(plan.policy, input.policy());
        assert_eq!(plan.offer, input.policy().offer());
        assert!(plan.rows.is_empty());
        assert_eq!(
            (
                plan.eligible,
                plan.k,
                plan.share_margin,
                plan.share_exposure,
                plan.risk_each,
                plan.seed_reached,
            ),
            (0, 0, 0, 0, 0, 0)
        );
    }
}

#[test]
fn one_candidate_takes_the_whole_free_margin_and_at_five_x_the_whole_exposure_room() {
    let mut one = cand(4, 0);
    one.price = 123_457;
    let plan = run(&PolicyInput::SplitAll, &view(), &OPERATOR, &[one]);
    assert_eq!(
        (
            plan.eligible,
            plan.k,
            plan.share_margin,
            plan.share_exposure
        ),
        (1, 1, LAKH_20, CRORE)
    );
    let row = sized(&plan, 0);
    // F = Rs 20 lakh at 5x: q_m = q_x = floor(Rs 1 crore / P), a tie labelled MarginShare.
    let q = (CRORE / 123_457) as u64;
    assert_eq!(row.qty, q);
    assert_eq!(row.bound, Bound::MarginShare);
    assert_eq!(row.value, i128::from(q) * 123_457);
    assert_eq!(row.margin, row.value * 200_000);
    assert!(row.margin <= LAKH_20 * 1_000_000);
    assert!(row.value <= CRORE);
}

#[test]
fn two_hundred_eighteen_candidates_split_over_the_trade_room() {
    let batch = spread(218);
    // The operator's 100 trades a day: k = 100, and FNO-PORTFOLIO refuses the rest trade_cap.
    let plan = run(&PolicyInput::SplitAll, &view(), &OPERATOR, &batch);
    assert_eq!((plan.eligible, plan.k), (218, 100));
    assert_eq!(
        (plan.share_margin, plan.share_exposure),
        (2_000_000, 10_000_000)
    );
    assert_eq!(plan.rows.len(), 218);
    for i in 0..218 {
        let row = sized(&plan, i);
        // q_m = floor(2,000,000 x 10^6 / (200,000 x 100,000)) = 100 shares, Rs 1 lakh.
        assert_eq!((row.qty, row.bound), (100, Bound::MarginShare));
    }
    // A test limit of 256 trades: k = 218.
    let plan = run(&PolicyInput::SplitAll, &view(), &ROOMY, &batch);
    assert_eq!((plan.eligible, plan.k), (218, 218));
    assert_eq!(plan.share_margin, LAKH_20 / 218);
    assert_eq!(plan.share_exposure, CRORE / 218);
    assert_eq!(plan.share_margin, 917_431);
    // floor(917,431 x 10^6 / (200,000 x 100,000)) = 45 shares.
    assert_eq!(sized(&plan, 217).qty, 45);
}

#[test]
fn a_batch_of_two_hundred_fifty_six_is_planned_and_two_hundred_fifty_seven_refused() {
    let edge = EdgeTable::default();
    let mut batch = spread(256);
    for input in inputs(&edge) {
        let plan = run(&input, &view(), &ROOMY, &batch);
        assert_eq!(plan.rows.len(), 256);
        assert_eq!(plan.eligible, 256);
    }
    batch.push(cand(0, 1));
    for input in inputs(&edge) {
        let mut out = run(&input, &view(), &ROOMY, &batch[..3]);
        let before = out.clone();
        assert_eq!(
            plan(&input, &view(), &ROOMY, &batch, &mut out),
            Err(Invalid::BatchTooLarge(257))
        );
        assert_eq!(out, before);
    }
}

#[test]
fn trade_room_zero_keeps_quantities_defined() {
    let batch = spread(5);
    let mut full = view();
    full.trades_today = 100;
    let plan = run(&PolicyInput::SplitAll, &full, &OPERATOR, &batch);
    // No room: k = e, every row still sized; FNO-PORTFOLIO refuses them trade_cap.
    assert_eq!((plan.eligible, plan.k), (5, 5));
    assert_eq!(plan.share_margin, LAKH_20 / 5);
    for i in 0..5 {
        assert_eq!(sized(&plan, i).qty, 2_000);
    }
    // Sent and queued entries together use the room up too, saturating rather than wrapping.
    full.trades_today = 90;
    full.queued = 30;
    let plan = run(&PolicyInput::SplitAll, &full, &OPERATOR, &batch);
    assert_eq!(plan.k, 5);
}

#[test]
fn trade_room_one_with_many_candidates() {
    let batch = spread(7);
    let mut late = view();
    late.trades_today = 98;
    late.queued = 1;
    let plan = run(&PolicyInput::SplitAll, &late, &OPERATOR, &batch);
    // One entry left: k = 1, each row's share is the whole free margin.
    assert_eq!((plan.eligible, plan.k), (7, 1));
    assert_eq!((plan.share_margin, plan.share_exposure), (LAKH_20, CRORE));
    for i in 0..7 {
        assert_eq!(sized(&plan, i).qty, 10_000);
    }
}

#[test]
fn two_recipes_on_one_stock_count_the_stock_once() {
    let batch = [cand(3, 0), cand(3, 1), cand(4, 0)];
    let plan = run(&PolicyInput::SplitAll, &view(), &OPERATOR, &batch);
    assert_eq!((plan.eligible, plan.k), (2, 2));
    assert_eq!(plan.share_margin, LAKH_20 / 2);
    // Both recipes of stock 3 are sized; FNO-PORTFOLIO admits the first and refuses the other
    // while_open.
    assert_eq!(sized(&plan, 0), sized(&plan, 1));
    assert_eq!(sized(&plan, 0).qty, 5_000);
}

#[test]
fn a_duplicate_stock_and_recipe_is_refused_wherever_it_sits() {
    let edge = EdgeTable::default();
    let adjacent = [cand(9, 0), cand(5, 1), cand(5, 1), cand(6, 0)];
    let mut ends: Vec<Candidate> = (0..256).map(|i| cand(i as u8, 0)).collect();
    ends[0] = cand(0, 7);
    ends[255] = cand(0, 7);
    for input in inputs(&edge) {
        let mut out = run(&input, &view(), &ROOMY, &[cand(1, 0)]);
        let before = out.clone();
        assert_eq!(
            plan(&input, &view(), &ROOMY, &adjacent, &mut out),
            Err(Invalid::Duplicate {
                stock: 5,
                recipe: 1
            })
        );
        assert_eq!(
            plan(&input, &view(), &ROOMY, &ends, &mut out),
            Err(Invalid::Duplicate {
                stock: 0,
                recipe: 7
            })
        );
        assert_eq!(out, before);
    }
}

#[test]
fn two_hundred_fifty_six_candidates_on_one_stock_pass_the_duplicate_check_at_its_bound() {
    let batch: Vec<Candidate> = (0..256).map(|r| cand(9, r)).collect();
    // The test's own model of the chain: each candidate meets every earlier candidate of its
    // stock once.
    let mut seen_of_stock = [0u32; 256];
    let mut comparisons = 0u32;
    for c in &batch {
        comparisons += seen_of_stock[usize::from(c.stock.position())];
        seen_of_stock[usize::from(c.stock.position())] += 1;
    }
    assert_eq!(comparisons, 256 * 255 / 2);
    assert_eq!(comparisons, 32_640);
    let planned = run(&PolicyInput::FixedSeeded, &view(), &ROOMY, &batch);
    assert_eq!((planned.rows.len(), planned.eligible), (256, 1));
    // A duplicate of the first recipe in the last place is still found at the chain's far end.
    let mut more = batch[1..].to_vec();
    more.push(cand(9, 1));
    let mut out = Plan::new();
    assert_eq!(
        plan(&PolicyInput::FixedSeeded, &view(), &ROOMY, &more, &mut out),
        Err(Invalid::Duplicate {
            stock: 9,
            recipe: 1
        })
    );
}

// ------------------------------------------------------- RANKED, every tie --

/// RANKED's plan of `batch` over `edge` with `seed`.
fn ranked(edge: &EdgeTable, seed: u64, batch: &[Candidate]) -> Plan {
    run(
        &PolicyInput::FixedRanked { edge, seed },
        &view(),
        &ROOMY,
        batch,
    )
}

#[test]
fn a_higher_edge_goes_first() {
    let edge = EdgeTable::build(
        &[],
        0,
        1,
        &[
            trade(0, 1, Side::Long, 0, 900),
            trade(0, 2, Side::Long, 0, 100),
        ],
    )
    .unwrap();
    // Stock 2 has the smaller stop and the larger liquidity, and still goes second.
    let mut low = cand(2, 0);
    low.stop_in_r = fraction(1, 9);
    low.liquidity = 1_000;
    let batch = [low, cand(1, 0)];
    let plan = ranked(&edge, 1, &batch);
    assert_eq!(plan.offer, Offer::PlanOrder);
    assert_eq!(row_keys(&plan, &batch), vec![(1, 0), (2, 0)]);
    assert_eq!(plan.seed_reached, 0);
}

#[test]
fn equal_means_from_different_counts_tie() {
    let two_of_four = Edge {
        level: Level::Cell,
        sum: 2,
        count: 4,
    };
    let one_of_two = Edge {
        level: Level::StockSide,
        sum: 1,
        count: 2,
    };
    assert_eq!(two_of_four.cmp_mean(&one_of_two), Ordering::Equal);
    // Through the table: four trades summing 2 against two summing 1; the tie falls to the stop.
    let trades = [
        trade(0, 1, Side::Long, 0, 1),
        trade(0, 1, Side::Long, 0, 1),
        trade(0, 1, Side::Long, 0, 0),
        trade(0, 1, Side::Long, 0, 0),
        trade(0, 2, Side::Long, 0, 1),
        trade(0, 2, Side::Long, 0, 0),
    ];
    let edge = EdgeTable::build(&[], 0, 1, &trades).unwrap();
    assert_eq!(edge.score(StockId::new(1), Side::Long, 0).count, 4);
    let mut tight = cand(2, 0);
    tight.stop_in_r = fraction(1, 4);
    let batch = [cand(1, 0), tight];
    assert_eq!(
        row_keys(&ranked(&edge, 1, &batch), &batch),
        vec![(2, 0), (1, 0)]
    );
}

#[test]
fn no_trade_ranks_after_every_score() {
    let edge = EdgeTable::build(&[], 0, 1, &[trade(0, 1, Side::Short, 0, -400)]).unwrap();
    let none = edge.score(StockId::new(2), Side::Long, 0);
    assert_eq!(none, Edge::NONE);
    let negative = edge.score(StockId::new(1), Side::Short, 0);
    assert_eq!(negative.cmp_mean(&none), Ordering::Greater);
    assert_eq!(none.cmp_mean(&negative), Ordering::Less);
    assert_eq!(none.cmp_mean(&Edge::NONE), Ordering::Equal);
    // The candidate with no trade behind it (stock 2 long) goes after the negative score.
    let mut short = cand(1, 0);
    short.side = Side::Short;
    let mut keen = cand(2, 0);
    keen.stop_in_r = fraction(0, 1);
    keen.liquidity = u128::MAX;
    let batch = [keen, short];
    assert_eq!(
        row_keys(&ranked(&edge, 3, &batch), &batch),
        vec![(1, 0), (2, 0)]
    );
}

#[test]
fn an_edge_tie_goes_to_the_smaller_stop_in_r() {
    let edge = EdgeTable::default();
    let mut wide = cand(1, 0);
    wide.stop_in_r = fraction(3, 4);
    wide.liquidity = 9_999;
    let mut tight = cand(2, 0);
    tight.stop_in_r = fraction(1, 4);
    let batch = [wide, tight];
    let plan = ranked(&edge, 5, &batch);
    assert_eq!(row_keys(&plan, &batch), vec![(2, 0), (1, 0)]);
    assert_eq!(plan.seed_reached, 0);
}

#[test]
fn equal_fractions_tie() {
    assert_eq!(fraction(1, 2), fraction(2, 4));
    assert_eq!(fraction(1, 2).cmp(&fraction(2, 4)), Ordering::Equal);
    assert!(fraction(1, 3) < fraction(1, 2));
    assert_eq!(
        fraction(2, 3).partial_cmp(&fraction(1, 2)),
        Some(Ordering::Greater)
    );
    assert_eq!((fraction(2, 4).num(), fraction(2, 4).den()), (2, 4));
    // 1/2 and 2/4 tie on the stop, so the larger liquidity goes first.
    let edge = EdgeTable::default();
    let mut a = cand(1, 0);
    a.stop_in_r = fraction(1, 2);
    let mut b = cand(2, 0);
    b.stop_in_r = fraction(2, 4);
    b.liquidity = 7;
    let batch = [a, b];
    assert_eq!(
        row_keys(&ranked(&edge, 5, &batch), &batch),
        vec![(2, 0), (1, 0)]
    );
}

#[test]
fn a_stop_tie_goes_to_the_larger_liquidity() {
    let edge = EdgeTable::default();
    let mut thin = cand(1, 0);
    thin.liquidity = 10;
    let mut deep = cand(7, 0);
    deep.liquidity = 11;
    let batch = [thin, deep];
    let plan = ranked(&edge, 5, &batch);
    assert_eq!(row_keys(&plan, &batch), vec![(7, 0), (1, 0)]);
    assert_eq!(plan.seed_reached, 0);
}

#[test]
fn a_liquidity_tie_goes_to_the_seeded_key() {
    let edge = EdgeTable::default();
    let batch = [cand(1, 0), cand(2, 0)];
    for seed in 0..32 {
        let plan = ranked(&edge, seed, &batch);
        let mut expected = vec![(1u8, 0u32), (2, 0)];
        expected.sort_by_key(|&(stock, recipe)| seeded(seed, 34_200, StockId::new(stock), recipe));
        assert_eq!(row_keys(&plan, &batch), expected);
        assert_eq!(plan.seed_reached, 1);
    }
}

#[test]
fn a_seeded_tie_falls_to_the_stock_then_the_recipe() {
    // Seeded keys cannot be made to collide by search, so the keys are forged in the
    // comparator's own key form.
    let rest = |stock: u8, recipe: u32| Rest {
        stop: fraction(1, 2),
        liquidity: 5,
        seeded: 42,
        stock,
        recipe,
    };
    let none = MeanKey::NONE;
    let mut reached = 0;
    assert_eq!(
        compare((&none, &rest(3, 9)), (&none, &rest(4, 0)), &mut reached),
        Ordering::Less
    );
    assert_eq!(
        compare((&none, &rest(4, 0)), (&none, &rest(3, 9)), &mut reached),
        Ordering::Greater
    );
    assert_eq!(
        compare((&none, &rest(3, 1)), (&none, &rest(3, 2)), &mut reached),
        Ordering::Less
    );
    assert_eq!(
        compare((&none, &rest(3, 2)), (&none, &rest(3, 2)), &mut reached),
        Ordering::Equal
    );
    assert_eq!(reached, 4);
    // A smaller seeded key decides before the stock.
    let mut early = rest(9, 0);
    early.seeded = 1;
    assert_eq!(
        compare((&none, &early), (&none, &rest(0, 0)), &mut reached),
        Ordering::Less
    );
    assert_eq!(reached, 5);
    // An edge decides before every later key and reaches nothing.
    let some = MeanKey::of(&Edge {
        level: Level::Side,
        sum: 3,
        count: 1,
    });
    assert_eq!(
        compare((&some, &rest(9, 9)), (&none, &rest(0, 0)), &mut reached),
        Ordering::Less
    );
    assert_eq!(reached, 5);
}

/// The test's own reading of the order: a sort with an independent comparator.
fn reference_order(batch: &[Candidate], edge: &EdgeTable, seed: u64) -> Vec<(u8, u32)> {
    let by_mean = |a: &Edge, b: &Edge| match (a.count, b.count) {
        (0, 0) => Ordering::Equal,
        (0, _) => Ordering::Less,
        (_, 0) => Ordering::Greater,
        (ca, cb) => (a.sum * i128::from(cb)).cmp(&(b.sum * i128::from(ca))),
    };
    let mut keyed: Vec<(Edge, &Candidate, u64)> = batch
        .iter()
        .map(|c| {
            (
                edge.score(c.stock, c.side, c.minute),
                c,
                seeded(seed, c.start, c.stock, c.recipe),
            )
        })
        .collect();
    keyed.sort_by(|(ea, a, sa), (eb, b, sb)| {
        by_mean(eb, ea)
            .then(
                (u128::from(a.stop_in_r.num()) * u128::from(b.stop_in_r.den()))
                    .cmp(&(u128::from(b.stop_in_r.num()) * u128::from(a.stop_in_r.den()))),
            )
            .then(b.liquidity.cmp(&a.liquidity))
            .then(sa.cmp(sb))
            .then(a.stock.position().cmp(&b.stock.position()))
            .then(a.recipe.cmp(&b.recipe))
    });
    keyed
        .iter()
        .map(|(_, c, _)| (c.stock.position(), c.recipe))
        .collect()
}

#[test]
fn the_network_order_equals_a_reference_sort() {
    let mut sizes_seen = [false; 257];
    for seed in 0..400u64 {
        let mut rng = Xorshift::new(seed);
        let count = match seed {
            0 => 0,
            1 => 256,
            2 => 1,
            _ => rng.below(257) as usize,
        };
        sizes_seen[count] = true;
        let stocks = 1 + rng.below(40);
        let (_, edge) = random_table(&mut rng, stocks, 4);
        let batch = random_batch(&mut rng, count, stocks);
        let plan = ranked(&edge, seed, &batch);
        assert_eq!(
            row_keys(&plan, &batch),
            reference_order(&batch, &edge, seed),
            "seed {seed}, {count} candidates"
        );
    }
    assert!(sizes_seen[0]);
    assert!(sizes_seen[1]);
    assert!(sizes_seen[256]);
}

#[test]
fn one_seed_one_order_whatever_the_arrangement() {
    let mut rng = Xorshift::new(77);
    let (_, edge) = random_table(&mut rng, 6, 4);
    let mut batch = random_batch(&mut rng, 60, 6);
    let first = ranked(&edge, 99, &batch);
    let order = row_keys(&first, &batch);
    for _ in 0..50 {
        rng.shuffle(&mut batch);
        let again = ranked(&edge, 99, &batch);
        assert_eq!(row_keys(&again, &batch), order);
        assert_eq!(again.seed_reached == 0, first.seed_reached == 0);
    }
}

#[test]
fn another_seed_another_order() {
    // Twelve candidates tied on edge, stop and liquidity: the order is the seeded key's.
    let edge = EdgeTable::default();
    let batch: Vec<Candidate> = (0..12).map(|i| cand(i, 0)).collect();
    let one = row_keys(&ranked(&edge, 1, &batch), &batch);
    let two = row_keys(&ranked(&edge, 2, &batch), &batch);
    assert_ne!(one, two);
    assert!(ranked(&edge, 1, &batch).seed_reached > 0);
}

#[test]
fn the_seeded_key_matches_tie_break_e() {
    // SplitMix64's published stream from state 0: an independent check of the mix.
    let gamma = 0x9E37_79B9_7F4A_7C15_u64;
    assert_eq!(mix(0), 0xE220_A839_7B1D_CDAF);
    assert_eq!(mix(gamma), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(mix(gamma.wrapping_mul(2)), 0x06C4_5D18_8009_454F);
    // Vectors from the restated formula of FNO-PORTFOLIO's tie-break (e).
    let vectors = [
        (0, 0, 0, 0, 0x2130_748A_AAC8_0268),
        (1, 34_200, 7, 0, 0x5B7D_8B47_9623_90DC),
        (42, -1, 255, 3, 0x6153_9673_AED3_6569),
        (u64::MAX, i64::MIN, 128, u32::MAX, 0x5378_34B9_E34D_295A),
        (0xDEAD_BEEF, 1_700_000_000, 17, 2, 0x8B38_1259_8650_458A),
    ];
    for (seed, start, stock, recipe, key) in vectors {
        assert_eq!(seeded(seed, start, StockId::new(stock), recipe), key);
    }
}

#[test]
fn a_negative_edge_is_offered_not_refused() {
    let edge = EdgeTable::build(&[], 0, 1, &[trade(0, 1, Side::Long, 0, -9_000)]).unwrap();
    let batch = [cand(1, 0)];
    let plan = ranked(&edge, 1, &batch);
    assert!(edge.score(StockId::new(1), Side::Long, 0).sum < 0);
    assert_eq!(sized(&plan, 0).qty, 500);
}

// ------------------------------------------- margin, exposure and risk at the edge --

/// A view whose free margin is exactly `free` paise.
fn free_of(free: i128) -> AccountView {
    let mut v = view();
    v.margin_in_use = (LAKH_20 - free) * 1_000_000;
    v
}

#[test]
fn free_margin_exactly_k_shares_sizes_each_and_leaves_no_remainder() {
    let mut batch = spread(4);
    for c in &mut batch {
        c.price = 1_000;
    }
    let plan = run(
        &PolicyInput::SplitAll,
        &free_of(4_000_000),
        &OPERATOR,
        &batch,
    );
    assert_eq!((plan.k, plan.share_margin), (4, 1_000_000));
    let mut total = 0;
    for i in 0..4 {
        let row = sized(&plan, i);
        // q_m = 1,000,000 x 10^6 / (200,000 x 1,000) = 5,000 exactly: Rs 10,000 of margin.
        assert_eq!((row.qty, row.margin), (5_000, 1_000_000 * 1_000_000));
        total += row.margin;
    }
    assert_eq!(total, 4_000_000 * 1_000_000);
}

#[test]
fn one_paisa_less_leaves_the_remainder_free() {
    let mut batch = spread(4);
    for c in &mut batch {
        c.price = 1_000;
    }
    let plan = run(
        &PolicyInput::SplitAll,
        &free_of(3_999_999),
        &OPERATOR,
        &batch,
    );
    assert_eq!(plan.share_margin, 999_999);
    let mut total = 0;
    for i in 0..4 {
        let row = sized(&plan, i);
        assert_eq!(row.qty, 4_999);
        assert!(row.margin <= 999_999 * 1_000_000);
        total += row.margin;
    }
    // The shares' remainder (3 paise) and each row's unspent share stay free.
    assert_eq!(3_999_999 - 4 * plan.share_margin, 3);
    assert!(total < 3_999_999 * 1_000_000);
}

#[test]
fn carried_losses_past_the_capital_refuse_every_row_size_no_free_margin() {
    let batch = spread(3);
    let mut lost = view();
    lost.capital = -5;
    for input in [
        PolicyInput::SplitAll,
        PolicyInput::Risk {
            budget: Some(budget(1_000, 10_000)),
        },
    ] {
        let plan = run(&input, &lost, &OPERATOR, &batch);
        for i in 0..3 {
            assert_eq!(refused(&plan, i), SizeRefusal::NoFreeMargin);
        }
        assert!(plan.share_margin < 0);
    }
    // Margin exhausted before the batch: F = 0.
    let plan = run(&PolicyInput::SplitAll, &free_of(0), &OPERATOR, &batch);
    assert_eq!(refused(&plan, 0), SizeRefusal::NoFreeMargin);
    // Reserved margin counts the same as margin in use.
    let mut queued = view();
    queued.reserved_margin = LAKH_20 * 1_000_000;
    let plan = run(&PolicyInput::SplitAll, &queued, &OPERATOR, &batch);
    assert_eq!(refused(&plan, 2), SizeRefusal::NoFreeMargin);
}

#[test]
fn margin_exhausted_mid_batch_under_ranked_is_portfolios_margin_cap_in_rank_order() {
    // Edges put A, then B, then C; their FIXED margins are Rs 15, 10 and 4 lakh.
    let edge = EdgeTable::build(
        &[],
        0,
        1,
        &[
            trade(0, 1, Side::Long, 0, 300),
            trade(0, 2, Side::Long, 0, 200),
            trade(0, 3, Side::Long, 0, 100),
        ],
    )
    .unwrap();
    let mut c = cand(3, 0);
    c.fixed_qty = 2_000;
    let mut a = cand(1, 0);
    a.fixed_qty = 7_500;
    let mut b = cand(2, 0);
    b.fixed_qty = 5_000;
    let batch = [c, a, b];
    let plan = ranked(&edge, 1, &batch);
    assert_eq!(row_keys(&plan, &batch), vec![(1, 0), (2, 0), (3, 0)]);
    // The test's model of SPEC 11.3's margin check over the plan's order, as FNO-PORTFOLIO's
    // admit reads it: margin in use plus the entry at most min(capital, Rs 20 lakh) x 10^6.
    let limit = LAKH_20.min(view().capital) * 1_000_000;
    let mut in_use = 0;
    let mut admitted = Vec::new();
    let mut margin_cap = Vec::new();
    for row in &plan.rows {
        let s = as_sized(row.outcome).unwrap();
        let key = batch[usize::from(row.index)].stock.position();
        if in_use + s.margin <= limit {
            in_use += s.margin;
            admitted.push(key);
        } else {
            margin_cap.push(key);
        }
    }
    // B is refused margin_cap in rank order, and the later, smaller C still fits.
    assert_eq!(admitted, vec![1, 3]);
    assert_eq!(margin_cap, vec![2]);
}

#[test]
fn carried_profit_never_lifts_the_free_margin_past_the_margin_cap() {
    let mut rich = view();
    rich.capital = 250_000_000;
    let mut one = cand(8, 0);
    one.rate_ppm = 300_000;
    let plan = run(&PolicyInput::SplitAll, &rich, &OPERATOR, &[one]);
    // F is Rs 20 lakh, not Rs 25 lakh.
    assert_eq!(plan.share_margin, LAKH_20);
    let row = sized(&plan, 0);
    assert!(row.margin <= LAKH_20 * 1_000_000);
    assert_eq!(row.qty, 6_666);
}

#[test]
fn carried_losses_lower_the_free_margin_below_the_margin_cap() {
    let mut poorer = view();
    poorer.capital = 150_000_000;
    let plan = run(&PolicyInput::SplitAll, &poorer, &OPERATOR, &[cand(8, 0)]);
    assert_eq!(plan.share_margin, 150_000_000);
    assert_eq!(sized(&plan, 0).qty, 7_500);
}

#[test]
fn under_the_operators_limits_the_margin_share_always_binds() {
    for seed in 0..300 {
        let mut rng = Xorshift::new(1_000 + seed);
        let exposure = rng.range(0, CRORE);
        let reserved_exposure = rng.range(0, CRORE / 4);
        let mut v = view();
        v.capital = rng.range(-LAKH_20, 2 * LAKH_20);
        v.exposure = exposure;
        v.reserved_exposure = reserved_exposure;
        // Margin at least 20% of the exposure it backs, reserved included.
        v.margin_in_use = exposure * rng.range(200_000, 1_000_000);
        v.reserved_margin = reserved_exposure * rng.range(200_000, 1_000_000);
        v.trades_today = rng.below(120) as u16;
        v.queued = rng.below(10) as u16;
        let count = 1 + rng.below(60) as usize;
        let batch = random_batch(&mut rng, count, 200);
        for c in &batch {
            if rng.below(5) == 0 {
                v.busy.insert(c.stock);
            }
        }
        for input in [
            PolicyInput::SplitAll,
            PolicyInput::Risk {
                budget: Some(budget(1 + rng.range(0, LAKH_20), 1 + rng.range(0, LAKH_20))),
            },
        ] {
            let plan = run(&input, &v, &OPERATOR, &batch);
            for row in &plan.rows {
                match row.outcome {
                    Outcome::Sized(s) => assert_ne!(s.bound, Bound::ExposureShare),
                    Outcome::Refused(word) => assert_ne!(word, SizeRefusal::NoExposureRoom),
                }
            }
        }
    }
}

#[test]
fn exposure_room_binds_under_limits_whose_exposure_cap_is_below_five_margin_caps() {
    let one_x = Limits {
        margin_cap: LAKH_20,
        exposure_cap: LAKH_20,
        trades_per_day: 100,
    };
    let plan = run(&PolicyInput::SplitAll, &view(), &one_x, &spread(2));
    let row = sized(&plan, 1);
    // q_m = 5,000, q_x = 1,000: the exposure share binds.
    assert_eq!((row.qty, row.bound), (1_000, Bound::ExposureShare));
    assert!(row.value <= plan.share_exposure);
}

#[test]
fn no_exposure_room_refuses_every_row() {
    // A test view whose margin is under 20% of its exposure.
    let mut v = view();
    v.exposure = CRORE;
    let plan = run(&PolicyInput::SplitAll, &v, &OPERATOR, &spread(3));
    for i in 0..3 {
        assert_eq!(refused(&plan, i), SizeRefusal::NoExposureRoom);
    }
    // A test limit whose exposure cap is below the exposure: the same, under SIZE-RISK too.
    let low = Limits {
        margin_cap: LAKH_20,
        exposure_cap: 0,
        trades_per_day: 100,
    };
    let plan = run(
        &PolicyInput::Risk {
            budget: Some(budget(1_000, 10_000)),
        },
        &view(),
        &low,
        &spread(3),
    );
    assert_eq!(refused(&plan, 1), SizeRefusal::NoExposureRoom);
}

#[test]
fn a_share_that_buys_no_share_is_refused_and_stays_free() {
    let mut batch = spread(218);
    batch[7].price = 13_000_000;
    let plan = run(&PolicyInput::SplitAll, &view(), &ROOMY, &batch);
    // A Rs 1.3 lakh share price against a Rs 9,174 share: no whole share.
    assert_eq!(plan.share_margin, 917_431);
    assert_eq!(refused(&plan, 7), SizeRefusal::BelowOneShare);
    // No re-split: every other row keeps the share computed for k = 218.
    assert_eq!(plan.k, 218);
    for i in (0..218).filter(|&i| i != 7) {
        assert_eq!(sized(&plan, i).qty, 45);
    }
}

#[test]
fn split_all_never_passes_the_free_margin_or_the_exposure_room() {
    for seed in 0..300 {
        let mut rng = Xorshift::new(5_000 + seed);
        let limits = Limits {
            margin_cap: rng.range(0, 3 * LAKH_20),
            exposure_cap: rng.range(0, 3 * CRORE),
            trades_per_day: rng.below(300) as u16,
        };
        let mut v = view();
        v.capital = rng.range(-LAKH_20, 3 * LAKH_20);
        v.margin_in_use = rng.range(0, LAKH_20) * rng.range(0, 1_000_000);
        v.reserved_margin = rng.range(0, LAKH_20);
        v.exposure = rng.range(0, CRORE);
        v.reserved_exposure = rng.range(0, CRORE / 10);
        v.trades_today = rng.below(200) as u16;
        let count = 1 + rng.below(80) as usize;
        let batch = random_batch(&mut rng, count, 256);
        let plan = run(&PolicyInput::SplitAll, &v, &limits, &batch);
        // The test's own figures.
        let free =
            (v.capital.min(limits.margin_cap) * 1_000_000 - v.margin_in_use - v.reserved_margin)
                .div_euclid(1_000_000);
        let room = limits.exposure_cap - v.exposure - v.reserved_exposure;
        let k = usize::from(plan.k);
        let mut margins: Vec<i128> = Vec::new();
        let mut values: Vec<i128> = Vec::new();
        for row in &plan.rows {
            if let Outcome::Sized(s) = row.outcome {
                assert!(s.margin <= plan.share_margin * 1_000_000);
                assert!(s.value <= plan.share_exposure);
                assert_eq!(
                    s.margin,
                    s.value * i128::from(batch[usize::from(row.index)].rate_ppm)
                );
                margins.push(s.margin);
                values.push(s.value);
            }
        }
        margins.sort_unstable_by(|a, b| b.cmp(a));
        values.sort_unstable_by(|a, b| b.cmp(a));
        // Any k rows admitted together: the k largest bound every subset of at most k.
        assert!(margins.iter().take(k).sum::<i128>() <= free.max(0) * 1_000_000);
        assert!(values.iter().take(k).sum::<i128>() <= room.max(0));
    }
}

#[test]
fn risk_never_passes_the_open_risk_cap() {
    for seed in 0..300 {
        let mut rng = Xorshift::new(9_000 + seed);
        let open_cap = 1 + rng.range(0, 50_000_000);
        let per_trade = 1 + rng.range(0, 20_000_000);
        let mut v = view();
        v.open_risk = rng.range(0, open_cap + 1_000);
        v.trades_today = rng.below(110) as u16;
        let count = 1 + rng.below(80) as usize;
        let batch = random_batch(&mut rng, count, 256);
        let plan = run(
            &PolicyInput::Risk {
                budget: Some(budget(per_trade, open_cap)),
            },
            &v,
            &OPERATOR,
            &batch,
        );
        let room = open_cap - v.open_risk;
        let mut risks: Vec<i128> = plan
            .rows
            .iter()
            .filter_map(|row| match row.outcome {
                Outcome::Sized(s) => Some(s.risk.unwrap()),
                Outcome::Refused(_) => None,
            })
            .collect();
        for &r in &risks {
            assert!(r >= 1);
            assert!(r <= plan.risk_each);
            assert!(plan.risk_each <= per_trade);
        }
        risks.sort_unstable_by(|a, b| b.cmp(a));
        assert!(risks.iter().take(usize::from(plan.k)).sum::<i128>() <= room.max(0));
    }
}

#[test]
fn the_risk_room_shrinks_equally_when_the_cap_binds() {
    let mut v = view();
    v.open_risk = 1_000_000;
    let input = PolicyInput::Risk {
        budget: Some(budget(1_000_000, 3_000_000)),
    };
    let plan = run(&input, &v, &OPERATOR, &spread(4));
    // Rr = Rs 20,000 shared by k = 4: b = min(Rs 10,000, Rs 5,000) = Rs 5,000 each.
    assert_eq!((plan.k, plan.risk_each), (4, 500_000));
    for i in 0..4 {
        let row = sized(&plan, i);
        assert_eq!(
            (row.qty, row.risk, row.bound),
            (500, Some(500_000), Bound::Risk)
        );
    }
    // A room above 0 but below k paise leaves b = 0: each row buys no share.
    v.open_risk = 3_000_000 - 3;
    let plan = run(&input, &v, &OPERATOR, &spread(4));
    assert_eq!(plan.risk_each, 0);
    assert_eq!(refused(&plan, 0), SizeRefusal::BelowOneShare);
}

#[test]
fn no_risk_room_refuses_every_row() {
    let mut v = view();
    v.open_risk = 3_000_000;
    let plan = run(
        &PolicyInput::Risk {
            budget: Some(budget(1_000_000, 3_000_000)),
        },
        &v,
        &OPERATOR,
        &spread(3),
    );
    for i in 0..3 {
        assert_eq!(refused(&plan, i), SizeRefusal::NoRiskRoom);
    }
    assert_eq!(plan.risk_each, 0);
}

#[test]
fn no_learned_risk_refuses_every_row() {
    let plan = run(
        &PolicyInput::Risk { budget: None },
        &view(),
        &OPERATOR,
        &spread(3),
    );
    for i in 0..3 {
        assert_eq!(refused(&plan, i), SizeRefusal::NoLearnedRisk);
    }
    assert_eq!((plan.k, plan.risk_each), (3, 0));
}

#[test]
fn a_budget_at_or_below_zero_is_refused() {
    for (per_trade, open_cap) in [(0, 5), (5, 0), (-1, 5), (5, -1), (0, 0)] {
        assert_eq!(
            RiskBudget::new(per_trade, open_cap),
            Err(Invalid::NonPositiveBudget)
        );
    }
    let good = budget(1, 1);
    assert_eq!((good.per_trade(), good.open_cap()), (1, 1));
}

/// SIZE-RISK's plan of one candidate with a roomy budget.
fn risk_one(c: Candidate) -> Outcome {
    let plan = run(
        &PolicyInput::Risk {
            budget: Some(budget(10_000_000, 1_000_000_000)),
        },
        &view(),
        &OPERATOR,
        &[c],
    );
    outcome(&plan, 0)
}

#[test]
fn a_stop_of_zero_distance_is_refused() {
    for side in Side::ALL {
        let mut c = cand(1, 0);
        c.side = side;
        c.stop = Some(c.price);
        assert_eq!(risk_one(c), Outcome::Refused(SizeRefusal::ZeroStop));
    }
}

#[test]
fn a_stop_through_the_price_is_refused() {
    let mut long = cand(1, 0);
    long.stop = Some(long.price + 1);
    assert_eq!(
        risk_one(long),
        Outcome::Refused(SizeRefusal::StopThroughPrice)
    );
    let mut short = cand(1, 0);
    short.side = Side::Short;
    short.stop = Some(short.price - 1);
    assert_eq!(
        risk_one(short),
        Outcome::Refused(SizeRefusal::StopThroughPrice)
    );
    // The right side of the price sizes.
    short.stop = Some(short.price + 1_000);
    assert!(as_sized(risk_one(short)).is_some());
}

#[test]
fn a_stop_less_lane_is_refused() {
    let mut c = cand(1, 0);
    c.stop = None;
    assert_eq!(risk_one(c), Outcome::Refused(SizeRefusal::NoStop));
}

#[test]
fn a_tie_between_risk_and_share_is_labelled_risk() {
    // Share: q_m = q_x = 10,000. Risk: b = Rs 1 lakh over d = Rs 10 = 10,000 too.
    let row = as_sized(risk_one(cand(1, 0))).unwrap();
    assert_eq!(
        (row.qty, row.bound, row.risk),
        (10_000, Bound::Risk, Some(10_000_000))
    );
    // One paisa more of stop distance: the risk binds alone; one paisa less: the share binds.
    let mut wider = cand(1, 0);
    wider.stop = Some(98_999);
    let row = as_sized(risk_one(wider)).unwrap();
    assert_eq!((row.qty, row.bound), (9_990, Bound::Risk));
    let mut tighter = cand(1, 0);
    tighter.stop = Some(99_001);
    let row = as_sized(risk_one(tighter)).unwrap();
    assert_eq!((row.qty, row.bound), (10_000, Bound::MarginShare));
    // A refused outcome has no sized row, and a sized one no refusal.
    assert_eq!(as_sized(Outcome::Refused(SizeRefusal::NoStop)), None);
    assert_eq!(as_refused(Outcome::Sized(row)), None);
}

// ---------------------------------------------------- leverage, prices, overflow --

#[test]
fn leverage_below_one_x_gives_less_value_than_margin() {
    let mut c = cand(1, 0);
    c.rate_ppm = 1_250_000;
    let plan = run(&PolicyInput::SplitAll, &view(), &OPERATOR, &[c]);
    let row = sized(&plan, 0);
    // 0.8x: q_m = Rs 20 lakh / (Rs 1,000 x 1.25) = 1,600 shares, Rs 16 lakh of value.
    assert_eq!((row.qty, row.bound), (1_600, Bound::MarginShare));
    assert!(row.value < plan.share_margin);
    assert_eq!(row.margin, plan.share_margin * 1_000_000);
}

#[test]
fn a_huge_margin_rate_buys_no_share() {
    let mut c = cand(1, 0);
    c.rate_ppm = u32::MAX;
    let plan = run(&PolicyInput::SplitAll, &view(), &OPERATOR, &[c]);
    assert_eq!(refused(&plan, 0), SizeRefusal::BelowOneShare);
}

#[test]
fn a_rate_below_the_twenty_percent_floor_is_a_caller_defect() {
    let mut c = cand(1, 0);
    c.rate_ppm = MARGIN_FLOOR_PPM - 1;
    let edge = EdgeTable::default();
    for input in inputs(&edge) {
        let mut out = Plan::new();
        assert_eq!(
            plan(&input, &view(), &OPERATOR, &[c], &mut out),
            Err(Invalid::RateBelowFloor(199_999))
        );
    }
    c.rate_ppm = MARGIN_FLOOR_PPM;
    assert_eq!(
        sized(&run(&PolicyInput::SplitAll, &view(), &OPERATOR, &[c]), 0).qty,
        10_000
    );
}

#[test]
fn a_one_paisa_price_and_an_i64_max_price() {
    let mut penny = cand(1, 0);
    penny.price = 1;
    penny.fixed_qty = 1;
    let plan = run(&PolicyInput::SplitAll, &view(), &OPERATOR, &[penny]);
    let row = sized(&plan, 0);
    assert_eq!(
        (row.qty, row.value, row.bound),
        (1_000_000_000, CRORE, Bound::MarginShare)
    );
    let mut dear = cand(1, 0);
    dear.price = i64::MAX;
    dear.fixed_qty = 1;
    dear.stop = Some(i64::MAX - 1);
    let plan = run(&PolicyInput::SplitAll, &view(), &OPERATOR, &[dear]);
    assert_eq!(refused(&plan, 0), SizeRefusal::BelowOneShare);
    let plan = run(&PolicyInput::FixedSeeded, &view(), &OPERATOR, &[dear]);
    let row = sized(&plan, 0);
    assert_eq!(row.value, i128::from(i64::MAX));
    assert_eq!(row.margin, i128::from(i64::MAX) * 200_000);
    assert_eq!(risk_one(dear), Outcome::Refused(SizeRefusal::BelowOneShare));
}

#[test]
fn every_product_is_checked_and_an_overflow_is_refused_by_name() {
    let edge = EdgeTable::default();
    let huge = i128::MAX / 1_000_000;
    let mut start = Plan::new();
    plan(
        &PolicyInput::SplitAll,
        &view(),
        &OPERATOR,
        &spread(2),
        &mut start,
    )
    .unwrap();
    let mut out = start.clone();
    let mut refuse =
        |input: &PolicyInput<'_>, v: &AccountView, limits: &Limits, batch: &[Candidate], name| {
            assert_eq!(
                plan(input, v, limits, batch, &mut out),
                Err(Invalid::Overflow(name))
            );
            assert_eq!(out, start);
        };
    // Capital near i128::MAX / 10^6: the free margin's product.
    let mut rich = view();
    rich.capital = huge + 1;
    let unbounded = Limits {
        margin_cap: huge + 1,
        exposure_cap: CRORE,
        trades_per_day: 100,
    };
    refuse(
        &PolicyInput::SplitAll,
        &rich,
        &unbounded,
        &spread(1),
        "free margin",
    );
    // The free margin's differences.
    let mut owing = view();
    owing.capital = -huge;
    owing.margin_in_use = i128::MAX;
    refuse(
        &PolicyInput::SplitAll,
        &owing,
        &OPERATOR,
        &spread(1),
        "free margin",
    );
    // The exposure room's differences.
    let sunk = Limits {
        margin_cap: LAKH_20,
        exposure_cap: i128::MIN + 5,
        trades_per_day: 100,
    };
    let mut exposed = view();
    exposed.exposure = 10;
    let risk_in = PolicyInput::Risk {
        budget: Some(budget(1, 1)),
    };
    refuse(&risk_in, &exposed, &sunk, &spread(1), "exposure room");
    // An exposure share past i64::MAX paise, the largest value a position can hold.
    let wide = Limits {
        margin_cap: LAKH_20,
        exposure_cap: i128::MAX,
        trades_per_day: 100,
    };
    refuse(
        &PolicyInput::SplitAll,
        &view(),
        &wide,
        &spread(1),
        "exposure share",
    );
    // A u64 quantity whose value overflows, and a value past i64::MAX paise (FNO-PORTFOLIO
    // refuses both as a value overflow).
    let mut many = cand(1, 0);
    many.fixed_qty = u64::MAX;
    many.price = 2;
    let mut dear = cand(1, 0);
    dear.fixed_qty = 2;
    dear.price = i64::MAX;
    for input in &inputs(&edge)[..2] {
        refuse(input, &view(), &OPERATOR, &[many], "value");
        refuse(input, &view(), &OPERATOR, &[dear], "value");
    }
    // A q_m past u64 is past q_x too: the exposure share binds, exactly.
    let mut lavish = view();
    lavish.capital = huge;
    let generous = Limits {
        margin_cap: huge,
        exposure_cap: CRORE,
        trades_per_day: 100,
    };
    let mut penny = cand(1, 0);
    penny.price = 1;
    let plan = run(&PolicyInput::SplitAll, &lavish, &generous, &[penny]);
    let row = sized(&plan, 0);
    assert_eq!((row.qty, row.bound), (1_000_000_000, Bound::ExposureShare));
    // The same under SIZE-RISK, where a q_r past u64 is past the share too.
    let plan = run(
        &PolicyInput::Risk {
            budget: Some(budget(i128::MAX, i128::MAX)),
        },
        &lavish,
        &generous,
        &[penny],
    );
    // A one-paisa price leaves no room for a stop below it: the short side.
    assert_eq!(refused(&plan, 0), SizeRefusal::StopThroughPrice);
    penny.side = Side::Short;
    penny.stop = Some(2);
    let plan = run(
        &PolicyInput::Risk {
            budget: Some(budget(i128::MAX, i128::MAX)),
        },
        &lavish,
        &generous,
        &[penny],
    );
    let row = sized(&plan, 0);
    assert_eq!((row.qty, row.bound), (1_000_000_000, Bound::ExposureShare));
}

// ------------------------------------------------------------------ edge table --

/// Every score of stocks below `stocks`, both sides, at minutes across the session.
fn scores(table: &EdgeTable, stocks: u8) -> Vec<Edge> {
    let mut out = Vec::new();
    for stock in 0..stocks {
        for side in Side::ALL {
            for minute in [0, 59, 60, 61, 149, 150, 239, 240, 374, 375, u16::MAX] {
                out.push(table.score(StockId::new(stock), side, minute));
            }
        }
    }
    out
}

#[test]
fn an_empty_learning_window_scores_every_cell_none() {
    for (from, to) in [(0, 0), (3, 3), (5, 2), (0, 12)] {
        let table = EdgeTable::build(&[100, 200], from, to, &[]).unwrap();
        assert!(scores(&table, 255).iter().all(|e| *e == Edge::NONE));
        assert_eq!(table.judged(), None);
    }
    assert!(
        scores(&EdgeTable::default(), 3)
            .iter()
            .all(|e| *e == Edge::NONE)
    );
}

#[test]
fn one_trade_fills_all_three_levels() {
    let table = EdgeTable::build(&[100], 0, 1, &[trade(0, 3, Side::Long, 150, 40)]).unwrap();
    let at = |stock, side, minute| table.score(StockId::new(stock), side, minute);
    let one = |level| Edge {
        level,
        sum: 40,
        count: 1,
    };
    assert_eq!(at(3, Side::Long, 200), one(Level::Cell));
    assert_eq!(at(3, Side::Long, 50), one(Level::StockSide));
    assert_eq!(at(9, Side::Long, 150), one(Level::Side));
    assert_eq!(at(9, Side::Short, 150), Edge::NONE);
}

#[test]
fn judging_a_learning_month_leaves_its_own_trades_out() {
    for seed in 0..120 {
        let mut rng = Xorshift::new(20_000 + seed);
        let (trades, mut table) = random_table(&mut rng, 8, 6);
        for month in 0..6 {
            table.judging(month).unwrap();
            assert_eq!(table.judged(), Some(month));
            // The same as a table that never held the month.
            let without: Vec<EdgeTrade> = trades
                .iter()
                .copied()
                .filter(|t| t.month != month)
                .collect();
            let other = EdgeTable::build(&[60, 150, 240], 0, 6, &without).unwrap();
            assert_eq!(scores(&table, 8), scores(&other, 8));
            // Altering any trade of the month changes no score while judging it.
            if let Some(at) = trades.iter().position(|t| t.month == month) {
                let mut altered = trades.clone();
                altered[at].net = 1_000_000;
                altered[at].stock = StockId::new(rng.below(8) as u8);
                altered[at].minute = rng.below(375) as u16;
                let mut changed = EdgeTable::build(&[60, 150, 240], 0, 6, &altered).unwrap();
                changed.judging(month).unwrap();
                assert_eq!(scores(&changed, 8), scores(&table, 8));
            }
        }
    }
}

#[test]
fn a_forward_month_reads_the_whole_learning_table() {
    let mut rng = Xorshift::new(31);
    let (trades, mut table) = random_table(&mut rng, 8, 6);
    let whole = EdgeTable::build(&[60, 150, 240], 0, 6, &trades).unwrap();
    table.judging(2).unwrap();
    // Judging the month taken out again changes nothing.
    let judged = table.clone();
    table.judging(2).unwrap();
    assert_eq!(table, judged);
    table.whole();
    assert_eq!(table.judged(), None);
    assert_eq!(scores(&table, 8), scores(&whole, 8));
    assert_eq!(table, whole);
    // Putting back nothing changes nothing.
    table.whole();
    assert_eq!(table, whole);
    // A forward month is not a learning month: it is read whole and never judged.
    assert_eq!(
        table.judging(6),
        Err(Invalid::MonthOutsideLearning { month: 6 })
    );
    assert_eq!(table, whole);
}

#[test]
fn a_trade_outside_the_learning_window_is_refused() {
    for month in [2, 9, 10] {
        assert_eq!(
            EdgeTable::build(&[], 3, 9, &[trade(month, 1, Side::Long, 0, 1)]),
            Err(Invalid::MonthOutsideLearning { month })
        );
    }
    let mut table = EdgeTable::build(&[], 3, 9, &[trade(3, 1, Side::Long, 0, 1)]).unwrap();
    let before = table.clone();
    assert_eq!(
        table.judging(2),
        Err(Invalid::MonthOutsideLearning { month: 2 })
    );
    assert_eq!(table, before);
}

#[test]
fn months_out_of_order_are_refused() {
    assert_eq!(
        EdgeTable::build(
            &[],
            0,
            9,
            &[trade(3, 1, Side::Long, 0, 1), trade(2, 1, Side::Long, 0, 1)]
        ),
        Err(Invalid::MonthsOutOfOrder)
    );
    // Equal months, and a gap of empty months, are in order.
    let table = EdgeTable::build(
        &[],
        0,
        9,
        &[
            trade(2, 1, Side::Long, 0, 1),
            trade(2, 1, Side::Long, 0, 1),
            trade(7, 1, Side::Long, 0, 1),
        ],
    )
    .unwrap();
    assert_eq!(table.score(StockId::new(1), Side::Long, 0).count, 3);
}

#[test]
fn rungs_empty_one_and_three_hundred_seventy_five() {
    let none = EdgeTable::build(&[], 0, 1, &[]).unwrap();
    assert_eq!(none.buckets(), 1);
    assert!((0..=u16::MAX).all(|m| none.bucket(m) == 0));
    let one = EdgeTable::build(&[100], 0, 1, &[]).unwrap();
    assert_eq!(one.buckets(), 2);
    assert_eq!(
        (
            one.bucket(0),
            one.bucket(99),
            one.bucket(100),
            one.bucket(374)
        ),
        (0, 0, 1, 1)
    );
    let every: Vec<u16> = (0..375).collect();
    let all = EdgeTable::build(&every, 0, 1, &[trade(0, 255, Side::Short, 374, 5)]).unwrap();
    assert_eq!(all.buckets(), 376);
    assert!((0..375).all(|m| all.bucket(m) == m + 1));
    // A minute past the session is past every rung: the last bucket.
    assert_eq!(all.bucket(375), 375);
    assert_eq!(all.bucket(u16::MAX), 375);
    assert_eq!(
        all.score(StockId::new(255), Side::Short, 374).level,
        Level::Cell
    );
}

#[test]
fn unsorted_duplicate_or_late_rungs_are_refused() {
    let cases: [&[u16]; 5] = [&[5, 3], &[5, 5], &[375], &[0, 374, 375], &[u16::MAX]];
    for rungs in cases {
        assert_eq!(EdgeTable::build(rungs, 0, 1, &[]), Err(Invalid::BadRungs));
    }
    let mut table = EdgeTable::default();
    let before = table.clone();
    assert_eq!(table.reset(&[9, 2], 0, 1), Err(Invalid::BadRungs));
    assert_eq!(table, before);
}

#[test]
fn the_fallback_goes_cell_then_stock_side_then_side() {
    let table = EdgeTable::build(
        &[100],
        0,
        2,
        &[
            trade(0, 3, Side::Long, 150, 40),
            trade(0, 3, Side::Long, 20, -10),
            trade(1, 5, Side::Long, 10, 7),
            trade(1, 7, Side::Short, 30, 5),
        ],
    )
    .unwrap();
    let at = |stock, side, minute| table.score(StockId::new(stock), side, minute);
    let edge = |level, sum, count| Edge { level, sum, count };
    assert_eq!(at(3, Side::Long, 160), edge(Level::Cell, 40, 1));
    assert_eq!(at(3, Side::Long, 30), edge(Level::Cell, -10, 1));
    assert_eq!(at(5, Side::Long, 200), edge(Level::StockSide, 7, 1));
    assert_eq!(at(3, Side::Short, 30), edge(Level::Side, 5, 1));
    assert_eq!(at(9, Side::Long, 0), edge(Level::Side, 37, 3));
    assert_eq!(at(7, Side::Short, 99), edge(Level::Cell, 5, 1));
}

#[test]
fn a_zero_value_trade_is_refused() {
    for value in [0, -3] {
        let mut t = trade(0, 1, Side::Long, 0, 1);
        t.value = value;
        assert_eq!(
            EdgeTable::build(&[], 0, 1, &[t]),
            Err(Invalid::NonPositiveValue)
        );
    }
    let mut late = trade(0, 1, Side::Long, 375, 1);
    assert_eq!(
        EdgeTable::build(&[], 0, 1, &[late]),
        Err(Invalid::MinuteOutOfSession(375))
    );
    late.minute = 374;
    assert!(EdgeTable::build(&[], 0, 1, &[late]).is_ok());
}

#[test]
fn a_negative_net_floors_toward_minus_infinity() {
    let mut t = trade(0, 1, Side::Long, 0, -1);
    t.value = 3;
    let table = EdgeTable::build(&[], 0, 1, &[t]).unwrap();
    assert_eq!(table.score(StockId::new(1), Side::Long, 0).sum, -333_334);
    t.net = 1;
    let table = EdgeTable::build(&[], 0, 1, &[t]).unwrap();
    assert_eq!(table.score(StockId::new(1), Side::Long, 0).sum, 333_333);
}

/// Added by this file: every overflow of the table is refused by name, and the judged sums
/// cannot overflow because the magnitudes were bounded when the trades went in.
#[test]
fn the_edge_tables_overflows_are_refused_by_name() {
    let mut past = trade(0, 1, Side::Long, 0, 0);
    past.net = i128::MAX / 1_000_000 + 1;
    assert_eq!(
        EdgeTable::build(&[], 0, 1, &[past]),
        Err(Invalid::Overflow("edge value"))
    );
    // Two values just past 2^126 each, on a value of one paisa: their magnitudes pass
    // i128::MAX together.
    let big = (1_i128 << 126) / 1_000_000 + 1;
    let mut gain = trade(0, 1, Side::Long, 0, 0);
    gain.net = big;
    gain.value = 1;
    let mut loss = trade(0, 2, Side::Short, 0, 0);
    loss.net = -big;
    loss.value = 1;
    assert!(EdgeTable::build(&[], 0, 1, &[gain]).is_ok());
    assert_eq!(
        EdgeTable::build(&[], 0, 1, &[gain, loss]),
        Err(Invalid::Overflow("edge sum"))
    );
    // Within the bound, taking a month out and putting it back is exact.
    let half = (1_i128 << 125) / 1_000_000;
    let (mut first, mut second, mut third) = (gain, loss, gain);
    first.net = half;
    second.net = -half;
    second.month = 1;
    third.net = half;
    third.month = 2;
    let mut table = EdgeTable::build(&[], 0, 3, &[first, second, third]).unwrap();
    let whole = table.clone();
    table.judging(1).unwrap();
    assert_eq!(
        table.score(StockId::new(9), Side::Long, 0).sum,
        2 * (half * 1_000_000)
    );
    table.whole();
    assert_eq!(table, whole);
}

// --------------------------------------------------------- risk book and capital --

#[test]
fn open_risk_holds_through_the_last_fill_second_and_settles_after() {
    let mut book = RiskBook::new();
    let s = StockId::new(4);
    book.admit(10, s, 500).unwrap();
    assert_eq!(book.release(s, 20), Ok(0));
    book.settle(20).unwrap();
    assert_eq!(book.open_risk(), 500);
    book.settle(21).unwrap();
    assert_eq!(book.open_risk(), 0);
    // Freed: the stock can be admitted again.
    book.admit(21, s, 300).unwrap();
    assert_eq!(book.open_risk(), 300);
}

#[test]
fn a_stock_in_force_cannot_be_admitted_again() {
    let mut book = RiskBook::default();
    let s = StockId::new(200);
    book.admit(1, s, 10).unwrap();
    assert_eq!(book.admit(2, s, 10), Err(Invalid::StockInForce(200)));
    book.release(s, 5).unwrap();
    // Released but not yet settled: still in force.
    assert_eq!(book.admit(3, s, 10), Err(Invalid::StockInForce(200)));
    book.settle(5).unwrap();
    assert_eq!(book.admit(5, s, 10), Err(Invalid::StockInForce(200)));
    book.settle(6).unwrap();
    book.admit(6, s, 10).unwrap();
}

#[test]
fn a_release_before_its_admission_and_time_backwards_are_refused() {
    let mut book = RiskBook::new();
    let s = StockId::new(1);
    book.admit(10, s, 50).unwrap();
    let before = book.clone();
    assert_eq!(
        book.release(s, 9),
        Err(Invalid::ReleaseBeforeAdmit {
            after: 9,
            admitted: 10
        })
    );
    assert_eq!(
        book.admit(9, StockId::new(2), 1),
        Err(Invalid::TimeBackwards { at: 9, clock: 10 })
    );
    assert_eq!(
        book.settle(9),
        Err(Invalid::TimeBackwards { at: 9, clock: 10 })
    );
    assert_eq!(
        book.release(StockId::new(3), 20),
        Err(Invalid::NotInForce(3))
    );
    assert_eq!(book, before);
    // A release at its admission second is allowed, and a second release is refused.
    assert_eq!(book.release(s, 10), Ok(0));
    assert_eq!(book.release(s, 12), Err(Invalid::NotInForce(1)));
}

#[test]
fn releases_in_any_order_settle_each_row_after_its_own_last_fill() {
    #[derive(Clone, Copy)]
    struct Held {
        admitted: i64,
        risk: i128,
        after: Option<i64>,
    }
    let mut walked_to_head = false;
    let mut walked_back = false;
    for seed in 0..200 {
        let mut rng = Xorshift::new(40_000 + seed);
        let mut book = RiskBook::new();
        let mut model: Vec<Option<Held>> = vec![None; 256];
        let mut clock = 0_i64;
        let stocks = 1 + rng.below(40) as u8;
        for _ in 0..400 {
            let stock = rng.below(u64::from(stocks)) as u8;
            let id = StockId::new(stock);
            match (rng.below(3), model[usize::from(stock)]) {
                (0, None) => {
                    let at = clock + rng.below(3) as i64;
                    let risk = rng.range(0, 1_000);
                    book.admit(at, id, risk).unwrap();
                    model[usize::from(stock)] = Some(Held {
                        admitted: at,
                        risk,
                        after: None,
                    });
                    clock = at;
                }
                (1, Some(held)) if held.after.is_none() => {
                    // A last fill anywhere from its admission to a few seconds ahead.
                    let after = held.admitted + rng.below(6) as i64;
                    let latest = model.iter().flatten().filter_map(|h| h.after).max();
                    let passed = book.release(id, after).unwrap();
                    if latest.is_none_or(|l| after >= l) {
                        assert_eq!(passed, 0, "a release in last-fill order takes one step");
                    } else {
                        assert!(passed >= 1);
                        walked_back = true;
                        walked_to_head |= model
                            .iter()
                            .flatten()
                            .filter_map(|h| h.after)
                            .all(|a| a > after);
                    }
                    model[usize::from(stock)] = Some(Held {
                        after: Some(after),
                        ..held
                    });
                }
                (2, _) => {
                    let at = clock + rng.below(4) as i64;
                    book.settle(at).unwrap();
                    for row in &mut model {
                        if row.is_some_and(|h| h.after.is_some_and(|a| a < at)) {
                            *row = None;
                        }
                    }
                    clock = at;
                }
                (_, Some(_)) => {
                    assert_eq!(book.admit(clock, id, 1), Err(Invalid::StockInForce(stock)));
                }
                (_, None) => {
                    assert_eq!(book.release(id, clock), Err(Invalid::NotInForce(stock)));
                }
            }
            let reference: i128 = model.iter().flatten().map(|h| h.risk).sum();
            assert_eq!(book.open_risk(), reference, "seed {seed}");
        }
    }
    assert!(walked_back);
    assert!(walked_to_head);
}

#[test]
fn the_peak_open_risk_is_kept() {
    let mut book = RiskBook::new();
    book.admit(0, StockId::new(1), 300).unwrap();
    book.admit(0, StockId::new(2), 200).unwrap();
    assert_eq!((book.open_risk(), book.peak()), (500, 500));
    book.release(StockId::new(1), 0).unwrap();
    book.settle(1).unwrap();
    assert_eq!((book.open_risk(), book.peak()), (200, 500));
    book.admit(1, StockId::new(3), 100).unwrap();
    assert_eq!((book.open_risk(), book.peak()), (300, 500));
}

/// Added by this file: the book's own refusals of a negative risk and an overflowing one.
#[test]
fn the_risk_books_figures_are_checked() {
    let mut book = RiskBook::new();
    let before = book.clone();
    assert_eq!(
        book.admit(0, StockId::new(1), -1),
        Err(Invalid::NegativeView("risk"))
    );
    book.admit(0, StockId::new(1), i128::MAX).unwrap();
    let full = book.clone();
    assert_eq!(
        book.admit(0, StockId::new(2), 1),
        Err(Invalid::Overflow("open risk"))
    );
    assert_eq!(book, full);
    assert_ne!(book, before);
}

#[test]
fn capital_needed_for_fixed_is_the_peak_margin_less_the_carried_net() {
    let mut k = CapitalNeeded::new();
    k.observe(150_000_000 * 1_000_000, 0).unwrap();
    // One micro-paisa past a paisa rounds up; a carried loss of Rs 1 lakh adds to it.
    k.observe(100_000_000 * 1_000_000 + 1, -10_000_000).unwrap();
    assert_eq!(k.peak(), Some(150_000_000));
    k.observe(180_000_000 * 1_000_000 + 1, -10_000_000).unwrap();
    assert_eq!(k.peak(), Some(190_000_001));
    // A carried profit lowers it.
    k.observe(195_000_000 * 1_000_000, 20_000_000).unwrap();
    assert_eq!(k.value(false, LAKH_20), Some(190_000_001));
    for policy in [Policy::FixedSeeded, Policy::FixedRanked] {
        assert_eq!(k.value(policy.reads_capital(), LAKH_20), Some(190_000_001));
    }
}

#[test]
fn capital_needed_for_share_policies_is_the_capital() {
    let mut k = CapitalNeeded::default();
    k.observe(5, 0).unwrap();
    assert_eq!(k.peak(), Some(1));
    for policy in [Policy::SplitAll, Policy::Risk] {
        assert_eq!(k.value(policy.reads_capital(), LAKH_20), Some(LAKH_20));
    }
}

#[test]
fn no_admission_means_no_capital_needed() {
    let k = CapitalNeeded::new();
    assert_eq!(k.peak(), None);
    for policy in Policy::ALL {
        assert_eq!(k.value(policy.reads_capital(), LAKH_20), None);
    }
    let mut checked = CapitalNeeded::new();
    assert_eq!(
        checked.observe(-1, 0),
        Err(Invalid::NegativeView("margin_after"))
    );
    assert_eq!(
        checked.observe(0, i128::MIN),
        Err(Invalid::Overflow("capital in use"))
    );
    assert_eq!(checked, k);
}

#[test]
fn capital_needed_never_passes_the_capital_under_the_margin_check() {
    for seed in 0..300 {
        let mut rng = Xorshift::new(60_000 + seed);
        let mut k = CapitalNeeded::new();
        let mut carried = 0_i128;
        let mut open: Vec<i128> = Vec::new();
        for _ in 0..200 {
            if rng.below(3) == 0 && !open.is_empty() {
                let at = rng.below(open.len() as u64) as usize;
                open.swap_remove(at);
                carried += rng.range(-5_000_000, 5_000_000);
                continue;
            }
            let in_use: i128 = open.iter().sum();
            let extra = rng.range(1, 30_000_000) * 1_000_000 + rng.range(0, 999_999);
            // FNO-PORTFOLIO's check once the margin limit lands: at most min(capital, Rs 20
            // lakh), the capital being Rs 20 lakh plus the carried net.
            let limit = (LAKH_20 + carried).min(LAKH_20) * 1_000_000;
            if in_use + extra <= limit {
                open.push(extra);
                k.observe(in_use + extra, carried).unwrap();
            }
        }
        // Every walk admits its first entry: Rs 3 lakh at most, against Rs 20 lakh.
        let needed = k.value(false, LAKH_20).unwrap();
        assert!(needed <= LAKH_20, "seed {seed}: {needed}");
    }
}

// -------------------------------------------------------------------- whole plan --

#[test]
fn the_plan_does_not_depend_on_the_batch_arrangement() {
    for seed in 0..120 {
        let mut rng = Xorshift::new(80_000 + seed);
        let (_, edge) = random_table(&mut rng, 30, 4);
        let count = rng.below(120) as usize;
        let mut batch = random_batch(&mut rng, count, 30);
        let mut v = view();
        v.open_risk = rng.range(0, 1_000_000);
        for c in &batch {
            if rng.below(6) == 0 {
                v.busy.insert(c.stock);
            }
        }
        for input in inputs(&edge) {
            let first = run(&input, &v, &OPERATOR, &batch);
            let first_rows = keyed_outcomes(&first, &batch);
            let first_order = row_keys(&first, &batch);
            rng.shuffle(&mut batch);
            let again = run(&input, &v, &OPERATOR, &batch);
            assert_eq!(keyed_outcomes(&again, &batch), first_rows);
            assert_eq!(
                (
                    again.eligible,
                    again.k,
                    again.share_margin,
                    again.share_exposure
                ),
                (
                    first.eligible,
                    first.k,
                    first.share_margin,
                    first.share_exposure
                )
            );
            assert_eq!(again.risk_each, first.risk_each);
            assert_eq!(again.seed_reached == 0, first.seed_reached == 0);
            if input.policy() == Policy::FixedRanked {
                assert_eq!(row_keys(&again, &batch), first_order);
            } else {
                // Batch order: row i is candidate i.
                assert!(again.rows.iter().zip(0..).all(|(row, i)| row.index == i));
            }
        }
    }
}

#[test]
fn seeded_passes_fixed_quantities_through_untouched() {
    let mut rng = Xorshift::new(3);
    let batch = random_batch(&mut rng, 40, 256);
    let plan = run(&PolicyInput::FixedSeeded, &view(), &OPERATOR, &batch);
    assert_eq!(
        (plan.offer, plan.k, plan.share_margin),
        (Offer::AccountOrder, 0, 0)
    );
    for (row, c) in plan.rows.iter().zip(&batch) {
        // Nothing is busy, so every row is sized.
        let s = as_sized(row.outcome).unwrap();
        assert_eq!(s.qty, c.fixed_qty);
        assert_eq!(s.value, i128::from(c.fixed_qty) * i128::from(c.price));
        assert_eq!(s.margin, s.value * i128::from(c.rate_ppm));
        assert_eq!((s.risk, s.bound), (None, Bound::FixedTarget));
    }
}

#[test]
fn busy_stocks_are_while_open_under_every_policy() {
    let edge = EdgeTable::default();
    let batch = [cand(1, 0), cand(2, 0), cand(2, 1), cand(3, 0)];
    let mut v = view();
    v.busy.insert(StockId::new(2));
    for input in inputs(&edge) {
        let plan = run(&input, &v, &OPERATOR, &batch);
        assert_eq!(plan.eligible, 2);
        assert_eq!(refused(&plan, 1), SizeRefusal::WhileOpen);
        assert_eq!(refused(&plan, 2), SizeRefusal::WhileOpen);
        assert!(as_sized(outcome(&plan, 0)).is_some());
        assert!(as_sized(outcome(&plan, 3)).is_some());
    }
    // Every stock busy: nothing eligible, nothing divided.
    for c in &batch {
        v.busy.insert(c.stock);
    }
    for input in inputs(&edge) {
        let plan = run(&input, &v, &OPERATOR, &batch);
        assert_eq!((plan.eligible, plan.k, plan.share_margin), (0, 0, 0));
        assert!(
            plan.rows
                .iter()
                .all(|row| row.outcome == Outcome::Refused(SizeRefusal::WhileOpen))
        );
    }
}

#[test]
fn every_label_is_the_operators_text_byte_for_byte() {
    assert_eq!(
        Policy::FixedSeeded.label(),
        "SIZE-FIXED-20-SEEDED: Rs 1 lakh margin x the stock's leverage a position, at most 20 open, simultaneous signals in the cap tie-break order S81 with its seeded spread (today's rule)"
    );
    assert_eq!(
        Policy::FixedRanked.label(),
        "SIZE-FIXED-20-RANKED: Rs 1 lakh margin x the stock's leverage a position, at most 20 open, simultaneous signals by the learned edge (net per rupee of the candidate's learning trades for that stock, side and time of day, a procedure's recipes pooled; a learning month is scored without its own trades, its entry times still shaping the time-of-day buckets), then the smaller stop in R, then liquidity, then a seeded order"
    );
    assert_eq!(
        Policy::SplitAll.label(),
        "SIZE-SPLIT-ALL: no 20-slot cap; the free margin of each second split equally across its signals, whole shares, at least one each"
    );
    assert_eq!(
        Policy::Risk.label(),
        "SIZE-RISK: equal risk per trade (risk budget / stop distance), capped by the stock's leverage on an equal share of the free margin, total open risk capped; both budgets from learning months only"
    );
    assert_eq!(
        LIMITS_LABEL,
        "account limits: margin in use at most Rs 20,00,000 (or the capital, when carried losses leave less) and open exposure at most Rs 1,00,00,000, checked at every entry on positions marked at their last traded price; a later mark can carry either past its limit and forces no reduction (SPEC 11.3; Dhan's MTM rule, 9.8, acts on losses only), so the peaks are reported (operator, 3 Nov 2026)"
    );
    assert_eq!(
        CAPITAL_FIXED_LABEL,
        "capital actually needed: the largest margin in use after an admission less the carried net then (quantities never read capital)"
    );
    assert_eq!(
        CAPITAL_SHARE_LABEL,
        "capital actually needed: the whole Rs 20 lakh (quantities are shares of the free margin)"
    );
    assert_eq!(
        RETURNS_LABEL,
        "minimum returns judged on Rs 20 lakh (OP-22, the stricter); on the capital actually needed shown beside"
    );
    for policy in Policy::ALL {
        assert!(policy.label().starts_with(&format!("{}: ", policy.name())));
    }
}

#[test]
fn canonical_bytes_differ_per_policy_and_carry_the_version() {
    let bytes: Vec<Vec<u8>> = Policy::ALL.iter().map(|p| p.canonical_bytes()).collect();
    assert_eq!(bytes[0], b"fno-size/v1\0SIZE-FIXED-20-SEEDED".to_vec());
    assert_eq!(bytes[1], b"fno-size/v1\0SIZE-FIXED-20-RANKED".to_vec());
    assert_eq!(bytes[2], b"fno-size/v1\0SIZE-SPLIT-ALL".to_vec());
    assert_eq!(bytes[3], b"fno-size/v1\0SIZE-RISK".to_vec());
    for (i, a) in bytes.iter().enumerate() {
        assert!(a.starts_with(VERSION.as_bytes()));
        assert_eq!(a[VERSION.len()], 0);
        for b in &bytes[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

/// Added by this file: what each policy says of itself, and every refusal word.
#[test]
fn the_policy_table_answers_for_every_policy() {
    let table: Vec<(&str, Offer, bool, bool)> = Policy::ALL
        .iter()
        .map(|p| (p.name(), p.offer(), p.twenty_slots(), p.reads_capital()))
        .collect();
    assert_eq!(
        table,
        vec![
            ("SIZE-FIXED-20-SEEDED", Offer::AccountOrder, true, false),
            ("SIZE-FIXED-20-RANKED", Offer::PlanOrder, true, false),
            ("SIZE-SPLIT-ALL", Offer::AccountOrder, false, true),
            ("SIZE-RISK", Offer::AccountOrder, false, true),
        ]
    );
    let words: Vec<&str> = SizeRefusal::ALL.iter().map(|r| r.word()).collect();
    assert_eq!(
        words,
        vec![
            "while_open",
            "size_no_free_margin",
            "size_no_exposure_room",
            "size_below_one_share",
            "size_no_learned_risk",
            "size_no_risk_room",
            "size_no_stop",
            "size_zero_stop",
            "size_stop_through_price",
        ]
    );
    // None collides with FNO-PORTFOLIO's five but while_open, which means the same.
    for word in ["position_cap", "trade_cap", "exposure_cap", "margin_cap"] {
        assert!(!words.contains(&word));
    }
    assert_eq!(Plan::default(), Plan::new());
    assert!(Plan::new().rows.capacity() >= UNIVERSE);
    assert_eq!(UNIVERSE, 256);
    assert_eq!(SESSION_MINUTES, 375);
}

/// Added by this file: the 256-bit stock set, word by word.
#[test]
fn a_stock_set_holds_each_word() {
    let mut set = StockSet::default();
    assert!(set.is_empty());
    assert_eq!(set, StockSet::EMPTY);
    for stock in [0, 63, 64, 127, 128, 191, 192, 255] {
        let id = StockId::new(stock);
        assert!(!set.contains(id));
        assert!(set.insert(id));
        assert!(!set.insert(id));
        assert!(set.contains(id));
        assert_eq!(id.position(), stock);
    }
    assert_eq!(set.len(), 8);
    assert!(!set.is_empty());
    assert!(set.remove(StockId::new(127)));
    assert!(!set.remove(StockId::new(127)));
    assert!(!set.contains(StockId::new(127)));
    assert_eq!(set.len(), 7);
}

#[test]
fn every_caller_defect_is_refused_by_name_and_changes_nothing() {
    let edge = EdgeTable::default();
    for input in inputs(&edge) {
        let fixed = input.policy().twenty_slots();
        let mut out = run(&input, &view(), &OPERATOR, &spread(3));
        let before = out.clone();
        let mut refuse = |batch: &[Candidate], why: Invalid| {
            assert_eq!(plan(&input, &view(), &OPERATOR, batch, &mut out), Err(why));
            assert_eq!(out, before);
        };
        let mut late = cand(1, 0);
        late.start += 1;
        refuse(
            &[cand(0, 0), late],
            Invalid::MixedStart {
                first: 34_200,
                other: 34_201,
            },
        );
        for price in [0, -1, i64::MIN] {
            let mut c = cand(1, 0);
            c.price = price;
            refuse(&[c], Invalid::NonPositivePrice(price));
        }
        let mut c = cand(1, 0);
        c.rate_ppm = 0;
        refuse(&[c], Invalid::RateBelowFloor(0));
        let mut c = cand(1, 0);
        c.minute = SESSION_MINUTES;
        refuse(&[c], Invalid::MinuteOutOfSession(375));
        refuse(
            &[cand(4, 2), cand(4, 2)],
            Invalid::Duplicate {
                stock: 4,
                recipe: 2,
            },
        );
        refuse(&spread(256).repeat(2)[..257], Invalid::BatchTooLarge(257));
        let mut zero = cand(6, 0);
        zero.fixed_qty = 0;
        if fixed {
            refuse(&[zero], Invalid::ZeroFixedQuantity { stock: 6 });
        } else {
            // A share policy reads no FIXED quantity.
            let mut fresh = Plan::new();
            assert!(plan(&input, &view(), &OPERATOR, &[zero], &mut fresh).is_ok());
        }
    }
    assert_eq!(Fraction::new(1, 0), Err(Invalid::ZeroDenominator));
}

#[test]
fn a_negative_view_figure_is_refused_by_name() {
    let edge = EdgeTable::default();
    let names = [
        "margin_in_use",
        "exposure",
        "reserved_margin",
        "reserved_exposure",
        "open_risk",
    ];
    for input in inputs(&edge) {
        for name in names {
            let mut v = view();
            match name {
                "margin_in_use" => v.margin_in_use = -1,
                "exposure" => v.exposure = -1,
                "reserved_margin" => v.reserved_margin = -1,
                "reserved_exposure" => v.reserved_exposure = -1,
                _ => v.open_risk = -1,
            }
            let mut out = run(&input, &view(), &OPERATOR, &spread(2));
            let before = out.clone();
            assert_eq!(
                plan(&input, &v, &OPERATOR, &spread(2), &mut out),
                Err(Invalid::NegativeView(name))
            );
            assert_eq!(out, before);
        }
        // A negative capital is no defect.
        let mut v = view();
        v.capital = -1;
        assert!(plan(&input, &v, &OPERATOR, &spread(2), &mut Plan::new()).is_ok());
    }
}

#[test]
fn the_measurements_run_at_tiny_sizes() {
    for policy in Policy::ALL {
        let series = measure_plan(policy, 3, 2, 4);
        assert_eq!((series.samples.len(), series.refused), (4, 0));
    }
    // A batch past the universe is refused at every call, history included.
    assert_eq!(measure_plan(Policy::SplitAll, 257, 2, 1).refused, 3);
    let checks = [
        measure_score(10, 3),
        measure_network(5, 2, 3),
        measure_network(256, 1, 1),
        measure_risk_cycle(2, 3),
        measure_risk_release(2, 3),
        measure_edge_insert(10, 3),
        measure_edge_reset(16, 1, 2),
        measure_edge_reset(375, 2, 1),
        measure_judging(4, 5),
        measure_duplicates(5, 2, 3),
        measure_duplicates(256, 1, 1),
    ];
    for series in &checks {
        assert_eq!(series.refused, 0);
        assert!(!series.samples.is_empty());
    }
    assert_eq!(measure_network(300, 1, 1).refused, 1);
    assert_eq!(measure_edge_reset(376, 1, 1).refused, 1);
    assert_eq!(measure_score(0, 2).samples.len(), 2);
    // The verdict.
    assert!(steady(100, 120));
    assert!(!steady(100, 121));
    assert!(within(5, 5));
    assert!(!within(6, 5));
}

//! The measuring functions of FNO-SIZE's seven bench rows, and their verdict.
//!
//! Each `measure_*` builds its own fixture, times what its row names and returns one sample per
//! timing in picoseconds per unit, with the calls the measured code refused (0 for a sound
//! fixture). The bench `crates/runner/benches/ratio.rs` calls them at 1x, 10x and 100x through
//! its `fno_size` module, prints every size's p50, p99, max and sample count, and judges each
//! row with [`within`] and, for the rows that time fixed work, [`steady`]; the per-unit rows are
//! judged by that file's two-sided ratio ceiling. The budgets below are ESTIMATES until measured
//! on the Mac. `runner::fno_size::tests::the_measurements_run_at_tiny_sizes` runs every function
//! here at tiny sizes.
//!
//! * C-FNO-SIZE-01 [`measure_plan`]: `plan` per candidate on one batch of 218 candidates on
//!   distinct stocks, timed after 10, 100 and 1,000 batches planned through the same plan.
//! * C-FNO-SIZE-02 [`measure_score`]: one score, on tables of 1k, 10k and 100k trades.
//! * C-FNO-SIZE-03 [`measure_network`]: RANKED's network on 256 rows, 1, 10 or 100 a sample.
//! * C-FNO-SIZE-04 [`measure_risk_cycle`] (admit, release in last-fill order and settle, with
//!   255 others in force, after 1x, 10x and 100x cycles) and [`measure_risk_release`] (the worst
//!   release, behind 255 released rows, 1, 10 or 100 a sample).
//! * C-FNO-SIZE-05 [`measure_edge_insert`] (one trade into a table already reset, 1k, 10k and
//!   100k trades) and [`measure_edge_reset`] (the reset alone on a 375-rung ladder, 1, 10 or 100
//!   a sample).
//! * C-FNO-SIZE-06 [`measure_judging`]: one month boundary, per trade moved, months of 100, 1k
//!   and 10k trades.
//! * C-FNO-SIZE-07 [`measure_duplicates`]: `plan` on the duplicate check's worst batch, 256
//!   candidates on one stock, 1, 10 or 100 a sample.

use core::hint::black_box;
use std::time::Instant;

use super::{
    AccountView, Candidate, EdgeTable, EdgeTrade, Fraction, Limits, Plan, Policy, PolicyInput,
    RiskBook, RiskBudget, Side, StockId, StockSet, UNIVERSE, order, plan, read,
};

/// C-FNO-SIZE-01's p99 budget: 300 ns a candidate (ESTIMATE).
pub const PLAN_BUDGET_PS: u128 = 300_000;
/// C-FNO-SIZE-02's p99 budget: 50 ns a score (ESTIMATE).
pub const SCORE_BUDGET_PS: u128 = 50_000;
/// C-FNO-SIZE-03's p99 budget: 80 us a batch of 256 (ESTIMATE).
pub const NETWORK_BUDGET_PS: u128 = 80_000_000;
/// C-FNO-SIZE-04 (a)'s p99 budget: 100 ns a cycle (ESTIMATE).
pub const RISK_CYCLE_BUDGET_PS: u128 = 100_000;
/// C-FNO-SIZE-04 (b)'s p99 budget: 2 us a release (ESTIMATE).
pub const RISK_RELEASE_BUDGET_PS: u128 = 2_000_000;
/// C-FNO-SIZE-05 (a)'s p99 budget: 20 ns a trade (ESTIMATE).
pub const EDGE_INSERT_BUDGET_PS: u128 = 20_000;
/// C-FNO-SIZE-05 (b)'s p99 budget: 2 ms a reset (ESTIMATE).
pub const EDGE_RESET_BUDGET_PS: u128 = 2_000_000_000;
/// C-FNO-SIZE-06's p99 budget: 50 ns a trade moved (ESTIMATE).
pub const JUDGING_BUDGET_PS: u128 = 50_000;
/// C-FNO-SIZE-07's p99 budget: 150 us a batch (ESTIMATE).
pub const DUPLICATES_BUDGET_PS: u128 = 150_000_000;

/// Cycles timed together in one sample of [`measure_risk_cycle`], so the clock's own cost is
/// shared out.
pub const CYCLES_PER_SAMPLE: u32 = 16;

/// Scores read in one sample of [`measure_score`].
pub const SCORES_PER_SAMPLE: u32 = 256;

/// Whether a fixed-work row held still: its p99 at most 1.2 times its p50.
#[must_use]
pub const fn steady(p50: u128, p99: u128) -> bool {
    p99.saturating_mul(10) <= p50.saturating_mul(12)
}

/// Whether a p99 is inside its budget.
#[must_use]
pub const fn within(p99: u128, budget: u128) -> bool {
    p99 <= budget
}

/// One measured size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Series {
    /// Picoseconds per unit, one per sample, in the order taken.
    pub samples: Vec<u128>,
    /// Calls the measured code refused; 0 for a sound fixture.
    pub refused: u32,
}

impl Series {
    /// Room for `samples` samples.
    fn with_capacity(samples: u32) -> Self {
        Self {
            samples: Vec::with_capacity(usize::try_from(samples).unwrap_or(0)),
            refused: 0,
        }
    }

    /// One sample: the time since `began` over `units`.
    fn take(&mut self, began: Instant, units: u128) {
        self.samples
            .push(began.elapsed().as_nanos() * 1_000 / units.max(1));
    }
}

/// The view every plan fixture is planned against: Rs 20 lakh, nothing in force.
const VIEW: AccountView = AccountView {
    capital: 200_000_000,
    margin_in_use: 0,
    exposure: 0,
    reserved_margin: 0,
    reserved_exposure: 0,
    trades_today: 0,
    queued: 0,
    open_risk: 0,
    busy: StockSet::EMPTY,
};

/// The operator's margin and exposure limits, with room for a whole batch in a day.
const LIMITS: Limits = Limits {
    margin_cap: 200_000_000,
    exposure_cap: 1_000_000_000,
    trades_per_day: 256,
};

/// A 16-rung time-of-day ladder.
const RUNGS: [u16; 16] = [
    22, 44, 66, 88, 110, 132, 154, 176, 198, 220, 242, 264, 286, 308, 330, 352,
];

/// The seed of every fixture's draws.
const FIXTURE: u64 = 0x5EED_F00D_0F15_17E5;

/// A xorshift generator for the fixtures.
struct Draw(u64);

impl Draw {
    /// Draws a 64-bit word.
    const fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Draws a word below `bound` (below 1 when `bound` is 0).
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }

    /// Draws a side.
    fn side(&mut self) -> Side {
        let at = usize::try_from(self.next() & 1).unwrap_or(0);
        read(&Side::ALL, at, Side::Long)
    }

    /// Draws a stock position.
    fn stock(&mut self) -> StockId {
        StockId(u8::try_from(self.next() & 0xFF).unwrap_or(0))
    }

    /// Draws a minute of session.
    fn minute(&mut self) -> u16 {
        u16::try_from(self.below(375)).unwrap_or(0)
    }
}

/// One valid candidate on `stock` with `recipe`: a price of Rs 100 to Rs 50,100, a rate of 20%
/// or more, a stop at a positive distance, at second 34,200.
fn candidate(stock: u8, recipe: u32, draw: &mut Draw) -> Candidate {
    let price = 10_000 + i64::try_from(draw.below(5_000_000)).unwrap_or(0);
    let side = draw.side();
    let distance = 1 + i64::try_from(draw.below(5_000)).unwrap_or(0);
    let toward = 1 - 2 * i64::from(side == Side::Short);
    Candidate {
        stock: StockId(stock),
        recipe,
        side,
        start: 34_200,
        minute: draw.minute(),
        price,
        rate_ppm: 200_000 + u32::try_from(draw.below(800_000)).unwrap_or(0),
        fixed_qty: 1 + draw.below(500),
        stop: Some(price - toward * distance),
        stop_in_r: Fraction {
            num: draw.below(1_000),
            den: 1 + draw.below(1_000),
        },
        liquidity: u128::from(draw.next()),
    }
}

/// `count` candidates on distinct stocks while they last (stock `i mod 256`, recipe `i / 256`).
fn spread(count: usize, draw: &mut Draw) -> Vec<Candidate> {
    (0u32..)
        .take(count)
        .map(|index| {
            let stock = u8::try_from(index % 256).unwrap_or(0);
            candidate(stock, index / 256, draw)
        })
        .collect()
}

/// `count` candidates on stock 0 with recipes `0..count`: the duplicate check's worst batch.
fn crowd(count: usize, draw: &mut Draw) -> Vec<Candidate> {
    (0u32..)
        .take(count)
        .map(|recipe| candidate(0, recipe, draw))
        .collect()
}

/// `count` learning trades spread evenly over `months` months from month 0, in month order.
fn learning(count: u32, months: u16, draw: &mut Draw) -> Vec<EdgeTrade> {
    (0..count)
        .map(|index| EdgeTrade {
            month: u16::try_from(u64::from(index) * u64::from(months) / u64::from(count))
                .unwrap_or(0),
            stock: draw.stock(),
            side: draw.side(),
            minute: draw.minute(),
            net: i128::from(draw.below(2_000_000)) - 1_000_000,
            value: 1 + i128::from(draw.below(50_000_000)),
        })
        .collect()
}

/// The policy input of `policy` over `edge`.
fn input_for(policy: Policy, edge: &EdgeTable) -> PolicyInput<'_> {
    match policy {
        Policy::FixedSeeded => PolicyInput::FixedSeeded,
        Policy::FixedRanked => PolicyInput::FixedRanked { edge, seed: 7 },
        Policy::SplitAll => PolicyInput::SplitAll,
        Policy::Risk => PolicyInput::Risk {
            budget: RiskBudget::new(5_000_000, 500_000_000).ok(),
        },
    }
}

/// C-FNO-SIZE-01: `plan` under `policy`, per candidate, on one batch of `candidates`
/// candidates on distinct stocks, each sample one plan, after `history` plans through the same
/// [`Plan`].
#[must_use]
pub fn measure_plan(policy: Policy, candidates: usize, history: u32, samples: u32) -> Series {
    let mut draw = Draw(FIXTURE);
    let batch = spread(candidates, &mut draw);
    let edge = EdgeTable::empty();
    let input = input_for(policy, &edge);
    let mut out = Plan::new();
    let mut series = Series::with_capacity(samples);
    for _ in 0..history {
        series.refused += u32::from(plan(&input, &VIEW, &LIMITS, &batch, &mut out).is_err());
    }
    let units = u128::try_from(candidates).unwrap_or(1);
    for _ in 0..samples {
        let began = Instant::now();
        let result = plan(
            black_box(&input),
            black_box(&VIEW),
            black_box(&LIMITS),
            black_box(&batch),
            &mut out,
        );
        series.take(began, units);
        series.refused += u32::from(result.is_err());
    }
    series
}

/// C-FNO-SIZE-02: one [`EdgeTable::score`], on a 16-rung table of `trades` learning trades;
/// each sample reads [`SCORES_PER_SAMPLE`] scores.
#[must_use]
pub fn measure_score(trades: u32, samples: u32) -> Series {
    let mut draw = Draw(FIXTURE);
    let built = EdgeTable::build(&RUNGS, 0, 12, &learning(trades, 12, &mut draw));
    let mut series = Series::with_capacity(samples);
    series.refused += u32::from(built.is_err());
    let table = built.unwrap_or_default();
    let keys: Vec<(StockId, Side, u16)> = (0..SCORES_PER_SAMPLE)
        .map(|_| (draw.stock(), draw.side(), draw.minute()))
        .collect();
    let pass = || {
        for &(stock, side, minute) in &keys {
            black_box(table.score(black_box(stock), side, minute));
        }
    };
    // One untimed pass, so a cold cache is not charged to the first sample.
    pass();
    for _ in 0..samples {
        let began = Instant::now();
        pass();
        series.take(began, u128::from(SCORES_PER_SAMPLE));
    }
    series
}

/// C-FNO-SIZE-03: RANKED's network over `rows` candidates (at most [`UNIVERSE`]; more are
/// counted refused and cut), `reps` networks a sample, per network.
#[must_use]
pub fn measure_network(rows: usize, reps: u32, samples: u32) -> Series {
    let mut draw = Draw(FIXTURE);
    let batch = spread(rows.min(UNIVERSE), &mut draw);
    let built = EdgeTable::build(&RUNGS, 0, 12, &learning(4_096, 12, &mut draw));
    let mut series = Series::with_capacity(samples);
    series.refused += u32::from(built.is_err()) + u32::from(rows > UNIVERSE);
    let edge = built.unwrap_or_default();
    let mut order = [0u16; UNIVERSE];
    // One untimed network, so a cold cache is not charged to the first sample.
    black_box(order::rank(black_box(&batch), &edge, 7, &mut order));
    for _ in 0..samples {
        let began = Instant::now();
        for _ in 0..reps {
            black_box(order::rank(black_box(&batch), &edge, 7, &mut order));
        }
        series.take(began, u128::from(reps));
    }
    series
}

/// One cycle on stock 255 at `at + 2`: admit, release in last-fill order, settle a second
/// later. Returns the calls refused.
fn cycle(book: &mut RiskBook, at: &mut i64) -> u32 {
    *at += 2;
    let last = StockId(255);
    u32::from(book.admit(*at, last, 500).is_err())
        + u32::from(book.release(last, *at).is_err())
        + u32::from(book.settle(*at + 1).is_err())
}

/// C-FNO-SIZE-04 (a): one [`RiskBook`] cycle with 255 other stocks in force, after `history`
/// cycles; each sample times [`CYCLES_PER_SAMPLE`] cycles.
#[must_use]
pub fn measure_risk_cycle(history: u32, samples: u32) -> Series {
    let mut book = RiskBook::new();
    let mut series = Series::with_capacity(samples);
    for stock in 0..=254 {
        series.refused += u32::from(book.admit(0, StockId(stock), 1_000).is_err());
    }
    let mut at = 0;
    for _ in 0..history {
        series.refused += cycle(&mut book, &mut at);
    }
    for _ in 0..samples {
        let began = Instant::now();
        let mut refused = 0;
        for _ in 0..CYCLES_PER_SAMPLE {
            refused += cycle(black_box(&mut book), &mut at);
        }
        series.take(began, u128::from(CYCLES_PER_SAMPLE));
        series.refused += refused;
    }
    series
}

/// One worst release on stock 255, held since `at`: released at `at`, before every other
/// released row's last fill, so it walks back past all 255 of them to the head; then settled a
/// second later and admitted again, which puts the book back as it was. Returns the calls
/// refused, and a release that passed fewer than 255 rows as one more.
fn worst(book: &mut RiskBook, at: &mut i64) -> u32 {
    let last = StockId(255);
    let passed = book.release(last, *at);
    let refused = u32::from(passed != Ok(255))
        + u32::from(book.settle(*at + 1).is_err())
        + u32::from(book.admit(*at + 1, last, 1_000).is_err());
    *at += 1;
    refused
}

/// C-FNO-SIZE-04 (b): the worst [`RiskBook::release`], joining the list at its head behind 255
/// released rows. Each repetition is that release plus the settle and the admission that put
/// the book back, so every repetition walks the same 255 rows of one book; `reps` repetitions a
/// sample.
#[must_use]
pub fn measure_risk_release(reps: u32, samples: u32) -> Series {
    let mut book = RiskBook::new();
    let mut series = Series::with_capacity(samples);
    for stock in 0..=254 {
        let id = StockId(stock);
        series.refused += u32::from(book.admit(0, id, 1_000).is_err());
        // Last fills far ahead, so every later release of stock 255 comes before them.
        series.refused += u32::from(book.release(id, i64::MAX - 255 + i64::from(stock)).is_err());
    }
    series.refused += u32::from(book.admit(0, StockId(255), 1_000).is_err());
    let mut at = 0;
    // One untimed repetition, so a cold cache is not charged to the first sample.
    series.refused += worst(&mut book, &mut at);
    for _ in 0..samples {
        let began = Instant::now();
        let mut refused = 0;
        for _ in 0..reps {
            refused += worst(black_box(&mut book), &mut at);
        }
        series.take(began, u128::from(reps));
        series.refused += refused;
    }
    series
}

/// C-FNO-SIZE-05 (a): one [`EdgeTable`] insertion into a 16-rung table already reset, over
/// `trades` learning trades a sample.
#[must_use]
pub fn measure_edge_insert(trades: u32, samples: u32) -> Series {
    let mut draw = Draw(FIXTURE);
    let trades_in = learning(trades, 12, &mut draw);
    let mut table = EdgeTable::empty();
    let mut series = Series::with_capacity(samples);
    let fill = |table: &mut EdgeTable| {
        let mut refused = 0;
        for trade in &trades_in {
            refused += u32::from(table.insert(black_box(trade)).is_err());
        }
        refused
    };
    // One untimed fill, so the entries' first allocation is not charged to the first sample.
    series.refused += u32::from(table.reset(&RUNGS, 0, 12).is_err()) + fill(&mut table);
    for _ in 0..samples {
        series.refused += u32::from(table.reset(&RUNGS, 0, 12).is_err());
        let began = Instant::now();
        let refused = fill(&mut table);
        series.take(began, u128::from(trades));
        series.refused += refused;
    }
    series
}

/// C-FNO-SIZE-05 (b): the reset of the cells and the minute map alone, on a ladder of `rungs`
/// rungs at minutes `0..rungs`; `reps` resets a sample.
#[must_use]
pub fn measure_edge_reset(rungs: u16, reps: u32, samples: u32) -> Series {
    let ladder: Vec<u16> = (0..rungs).collect();
    let mut table = EdgeTable::empty();
    let mut series = Series::with_capacity(samples);
    // One untimed reset: the table's first, which allocates its cells, is not a reset's cost.
    series.refused += u32::from(table.reset(&ladder, 0, 12).is_err());
    for _ in 0..samples {
        let began = Instant::now();
        let mut refused = 0;
        for _ in 0..reps {
            refused += u32::from(table.reset(black_box(&ladder), 0, 12).is_err());
        }
        series.take(began, u128::from(reps));
        series.refused += refused;
    }
    series
}

/// C-FNO-SIZE-06: one [`EdgeTable::judging`] month boundary over three learning months of
/// `month_trades` trades each, per trade moved (the month taken out and the month put back).
#[must_use]
pub fn measure_judging(month_trades: u32, samples: u32) -> Series {
    let mut draw = Draw(FIXTURE);
    let trades = learning(month_trades.saturating_mul(3), 3, &mut draw);
    let built = EdgeTable::build(&RUNGS, 0, 3, &trades);
    let mut series = Series::with_capacity(samples);
    series.refused += u32::from(built.is_err());
    let mut table = built.unwrap_or_default();
    // Two untimed boundaries: the first takes one month out, the second moves two.
    series.refused += u32::from(table.judging(2).is_err()) + u32::from(table.judging(0).is_err());
    let moved = u128::from(month_trades) * 2;
    for (_, month) in (0..samples).zip([1, 2, 0].into_iter().cycle()) {
        let began = Instant::now();
        let result = table.judging(black_box(month));
        series.take(began, moved);
        series.refused += u32::from(result.is_err());
    }
    series
}

/// C-FNO-SIZE-07: `plan` on `candidates` candidates on one stock with distinct recipes (256:
/// 32,640 comparisons), `reps` plans a sample, per plan.
#[must_use]
pub fn measure_duplicates(candidates: usize, reps: u32, samples: u32) -> Series {
    let mut draw = Draw(FIXTURE);
    let batch = crowd(candidates, &mut draw);
    let mut out = Plan::new();
    let mut series = Series::with_capacity(samples);
    let once = |out: &mut Plan| {
        u32::from(
            plan(
                &PolicyInput::FixedSeeded,
                &VIEW,
                &LIMITS,
                black_box(&batch),
                out,
            )
            .is_err(),
        )
    };
    // One untimed plan, so a cold cache is not charged to the first sample.
    series.refused += once(&mut out);
    for _ in 0..samples {
        let began = Instant::now();
        let mut refused = 0;
        for _ in 0..reps {
            refused += once(&mut out);
        }
        series.take(began, u128::from(reps));
        series.refused += refused;
    }
    series
}

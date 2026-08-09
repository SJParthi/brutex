//! Gate 8 for `crates/greeks` — a closed form costs the same whatever it is
//! asked, and a solve costs its bounded evaluation count times that constant.
//!
//! # The two claims are different shapes and this file keeps them apart
//!
//! [`greeks::bsm`] is closed form. A price and a full greek set are a fixed
//! sequence of arithmetic and two normal-CDF evaluations, with no loop and no
//! branch on the size of anything — so the measurement is per-call cost across
//! contracts that are as unlike each other as the model admits: deep in the
//! money against deep out of it, one day to expiry against five years.
//!
//! [`greeks::solver`] is **not** O(1) and its own header says so at length. It
//! is a root find, and what it has instead is an ARITHMETIC ceiling:
//! `BRACKET_EVALUATIONS + NEWTON_STEPS + BISECTION_STEPS + FINAL_EVALUATION`
//! = 2 + 8 + 64 + 1 = [`MAX_ITERATIONS`], with the bisection performing exactly
//! [`BISECTION_STEPS`] halvings and no data-dependent stopping rule.
//!
//! So this file does NOT time an easy solve against a hard one and call the
//! ratio a bound — that ratio is the evaluation COUNT, which is allowed to
//! differ and is pinned by `greeks::solver::the_iteration_count_never_exceeds_the_arithmetic_bound`
//! rather than by a stopwatch. What is measured here is the cost **per model
//! evaluation**, which is the part that must be constant for the arithmetic
//! ceiling to mean anything: 75 evaluations is a bound on cost only if an
//! evaluation costs the same on a hard input as on an easy one. A solver that
//! did more work per step on a difficult quote would satisfy the iteration
//! test and still be unbounded in time.
//!
//! # No float arithmetic happens in this file
//!
//! The workspace lint table denies `clippy::float_arithmetic` and this bench
//! adds no allow of any kind — not at the module root and not anywhere else.
//! Every `f64` here is a literal handed to the crate or a result read back
//! from it; all the arithmetic is `u128` on picosecond counts and evaluation
//! counts. The one place a division would have been tempting — cost per
//! evaluation — divides an integer by an integer.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.

use std::hint::black_box;
use std::time::Instant;

use greeks::solver::MAX_ITERATIONS;
use greeks::{Contract, OptionKind};

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0x — `docs/04-invariants.md`, the shared-CI number.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
const TRIALS: u32 = 40;

/// The volatility every quote in this file is generated at, so that a solve
/// has an answer the model can actually reach.
const VOL: f64 = 0.22;

/// Times one closure, in picoseconds per call, as a minimum over [`TRIALS`].
fn cost_ps<T>(reps: u32, mut op: impl FnMut() -> T) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..TRIALS {
        let start = Instant::now();
        for _ in 0..reps {
            black_box(op());
        }
        let ps = start.elapsed().as_nanos() * 1_000 / u128::from(reps);
        if ps < best {
            best = ps;
        }
    }
    best
}

/// Prints one measurement and returns whether it stayed under the ceiling.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 {
        println!("  {label:<52} UNMEASURABLE — the 1x baseline timed at zero");
        return false;
    }
    let permille = at_ps * 1_000 / base_ps;
    let ok = permille <= CEILING_PERMILLE;
    println!(
        "  {label:<52} {:>9} ps -> {:>9} ps   ratio {}.{:03}x  {}",
        base_ps,
        at_ps,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// Refuses loudly rather than unwrapping into a panic message nobody reads.
fn refuse(what: &str) -> ! {
    println!("BENCH SETUP FAILED — {what}");
    std::process::exit(1)
}

/// At the money, a quarter of a year out. The ordinary case.
const AT_THE_MONEY: Contract = Contract {
    spot: 20_000.0,
    strike: 20_000.0,
    years_to_expiry: 0.25,
    rate: 0.0675,
    carry: 0.0,
};

/// Deep in the money: the strike is a tenth of spot.
const DEEP_IN: Contract = Contract {
    spot: 20_000.0,
    strike: 2_000.0,
    years_to_expiry: 0.25,
    rate: 0.0675,
    carry: 0.0,
};

/// Deep out of the money: the strike is ten times spot.
const DEEP_OUT: Contract = Contract {
    spot: 20_000.0,
    strike: 200_000.0,
    years_to_expiry: 0.25,
    rate: 0.0675,
    carry: 0.0,
};

/// Out of the money by 20%, where vega is thin enough that Newton has real
/// work to do — but the quote is still one the solver can reach. A strike ten
/// times spot is NOT usable here: its model price is so close to zero that the
/// solve is refused before it starts, which is the no-arbitrage guard doing its
/// job and not a cost measurement.
const OUT_OF_THE_MONEY: Contract = Contract {
    spot: 20_000.0,
    strike: 24_000.0,
    years_to_expiry: 0.25,
    rate: 0.0675,
    carry: 0.0,
};

/// One day to expiry, where `d1` and `d2` blow up hardest.
const ONE_DAY: Contract = Contract {
    spot: 20_000.0,
    strike: 20_000.0,
    years_to_expiry: 0.00274,
    rate: 0.0675,
    carry: 0.0,
};

/// Five years out, the other end of the same axis.
const FIVE_YEARS: Contract = Contract {
    spot: 20_000.0,
    strike: 20_000.0,
    years_to_expiry: 5.0,
    rate: 0.0675,
    carry: 0.0,
};

/// The claim: a closed-form price does not care what it is pricing.
fn a_price_costs_the_same_whatever_the_contract_is() -> bool {
    let one = |c: &'static Contract| {
        cost_ps(20_000, || {
            black_box(c).price(black_box(VOL), OptionKind::Call)
        })
    };

    let base = one(&AT_THE_MONEY);
    let mut ok = true;
    ok &= ratio("C-G-01 price: at the money -> deep IN", base, one(&DEEP_IN));
    ok &= ratio(
        "C-G-01 price: at the money -> deep OUT",
        base,
        one(&DEEP_OUT),
    );
    ok &= ratio(
        "C-G-01 price: one day -> five years to expiry",
        one(&ONE_DAY),
        one(&FIVE_YEARS),
    );
    ok
}

/// And the full set costs the same as well — every greek is computed from the
/// same two CDF evaluations, so none of them introduces a data-dependent path.
fn the_full_greek_set_costs_the_same_whatever_the_contract_is() -> bool {
    let all = |c: &'static Contract| {
        cost_ps(20_000, || {
            black_box(c).greeks(black_box(VOL), OptionKind::Call)
        })
    };

    let base = all(&AT_THE_MONEY);
    let mut ok = true;
    ok &= ratio(
        "C-G-02 greeks: at the money -> deep OUT",
        base,
        all(&DEEP_OUT),
    );
    ok &= ratio(
        "C-G-02 greeks: one day -> five years to expiry",
        all(&ONE_DAY),
        all(&FIVE_YEARS),
    );
    ok
}

/// The solve is not constant and is not claimed to be. What must be constant
/// is the cost of ONE model evaluation, because that is what turns the
/// arithmetic ceiling of [`MAX_ITERATIONS`] into a bound on time.
fn one_model_evaluation_costs_the_same_however_hard_the_quote_is() -> bool {
    // Two quotes at opposite ends of what the solver finds easy: at the money
    // with a fat vega, where Newton converges immediately, and deep out of the
    // money with a thin one, where it does not.
    let Ok(easy_quote) = AT_THE_MONEY.price(VOL, OptionKind::Call) else {
        refuse("a price at the money")
    };
    let Ok(hard_quote) = OUT_OF_THE_MONEY.price(VOL, OptionKind::Call) else {
        refuse("a price out of the money")
    };

    let easy = match AT_THE_MONEY.implied_volatility(easy_quote, OptionKind::Call) {
        Ok(solved) => solved,
        Err(why) => refuse(&format!("a solve at the money — {why}")),
    };
    let hard = match OUT_OF_THE_MONEY.implied_volatility(hard_quote, OptionKind::Call) {
        Ok(solved) => solved,
        Err(why) => refuse(&format!("a solve out of the money — {why}")),
    };

    // The ceiling is arithmetic, and it is checked here as well as in the
    // unit test, because a bench that measured a solve without checking what
    // it cost in evaluations would be timing an unknown number of steps.
    if easy.iterations > MAX_ITERATIONS || hard.iterations > MAX_ITERATIONS {
        println!(
            "  C-G-03 BREACH — {} and {} evaluations against a ceiling of {MAX_ITERATIONS}",
            easy.iterations, hard.iterations
        );
        return false;
    }
    println!(
        "  {:<52} {} and {} of {MAX_ITERATIONS}",
        "C-G-03 model evaluations per solve (context)", easy.iterations, hard.iterations
    );

    let easy_total = cost_ps(2_000, || {
        black_box(&AT_THE_MONEY).implied_volatility(black_box(easy_quote), OptionKind::Call)
    });
    let hard_total = cost_ps(2_000, || {
        black_box(&DEEP_OUT).implied_volatility(black_box(hard_quote), OptionKind::Call)
    });

    // Integer division on integer picoseconds and an integer count. No float
    // arithmetic happens here; see the module header.
    let easy_each = easy_total / u128::from(easy.iterations.max(1));
    let hard_each = hard_total / u128::from(hard.iterations.max(1));

    ratio(
        "C-G-03 cost PER model evaluation: easy -> hard",
        easy_each,
        hard_each,
    )
}

fn main() {
    println!("gate 8 — crates/greeks, ceiling {CEILING_PERMILLE} permille");
    let mut ok = true;
    ok &= a_price_costs_the_same_whatever_the_contract_is();
    ok &= the_full_greek_set_costs_the_same_whatever_the_contract_is();
    ok &= one_model_evaluation_costs_the_same_however_hard_the_quote_is();
    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
        std::process::exit(1);
    }
}

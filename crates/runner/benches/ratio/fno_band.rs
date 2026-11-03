//! C-FNO-BAND-01 (FNO-BAND, D-8210): one band check, flat at 1x, 10x and 100x
//! the seconds of a day.
//!
//! A check is what the fill walk does in each traded second: one
//! `DayBand::observe` and one `DayBand::limits` at that second. A look-back is
//! one `limits` at an earlier second of the day. Each `limits` binary-searches
//! a side's flexes. Both are timed one call at a time, under both flex readings,
//! over 22,500 of each at every size (100 days of 225 seconds, 10 of 2,250 and
//! one of 22,500, a whole session), so every quantile rests on the same count.
//! The row fails unless each p99 is flat within the harness's ceiling at 10x
//! and 100x and inside the budget of PARTS section 11 (100 ns, an ESTIMATE
//! until the Mac measures it). Each sample holds two clock reads, so a figure
//! here is an upper bound on the call.
//!
//! The measuring function is the library's `runner::fno_band::measure_checks`,
//! which `runner::fno_band::tests::the_bench_fixture_times_every_second_it_walks`
//! runs at a tiny size; this file only gathers, prints and compares. Every
//! second is invented (`runner::fno_band::bench_day`) and nothing is written.

use runner::fno_band::{FlexReading, Samples, bench_day, measure_checks, quantile};

use super::ratio;

/// Seconds of one day at 1x. 10x and 100x are ten and a hundred times it, the
/// last a whole session of 22,500 (09:15:00 to 15:29:59).
const BASE_SECONDS: u32 = 225;

/// The p99 budget of one check and of one look-back, nanoseconds: PARTS
/// section 11's "one band check, 100 ns", an ESTIMATE until measured.
const BUDGET_NS: u64 = 100;

/// The slot's row: each id it is given is the first word of its lines.
pub(crate) fn row(ids: &[&str]) -> bool {
    let mut ok = true;
    for id in ids {
        ok &= one_band_check_is_flat(id);
    }
    ok
}

/// C-FNO-BAND-01 under both flex readings.
pub(crate) fn one_band_check_is_flat(id: &str) -> bool {
    let mut ok = true;
    for flex in FlexReading::ALL {
        let mut check = [0_u64; 3];
        let mut back = [0_u64; 3];
        for ((multiple, check_p99), back_p99) in
            [1, 10, 100].into_iter().zip(&mut check).zip(&mut back)
        {
            let gathered = gather(flex, multiple);
            if gathered.refused > 0 || gathered.flexes == 0 {
                println!("{id} {}: a day refused or timed no flex", flex.id());
                return false;
            }
            *check_p99 = quantiles(id, flex, "check", multiple, &gathered.check_ns);
            *back_p99 = quantiles(id, flex, "look-back", multiple, &gathered.lookback_ns);
        }
        for (kind, [base, ten, hundred]) in [("check", check), ("look-back", back)] {
            let ps = |ns: u64| u128::from(ns) * 1_000;
            let label = format!("{id} {} {kind} p99", flex.id());
            ok &= ratio(&format!("{label}, 10x"), ps(base), ps(ten));
            ok &= ratio(&format!("{label}, 100x"), ps(base), ps(hundred));
            let worst = base.max(ten).max(hundred);
            let within = worst <= BUDGET_NS;
            let verdict = ["OVER BUDGET", "ok"]
                .get(usize::from(within))
                .copied()
                .unwrap_or_default();
            println!("{label}: worst {worst} ns, budget {BUDGET_NS} ns   {verdict}");
            ok &= within;
        }
    }
    ok
}

/// Every sample of `100 / multiple` days of `BASE_SECONDS x multiple` seconds
/// under `flex`. A day the band refused counts as refused, so the row fails.
fn gather(flex: FlexReading, multiple: u32) -> Samples {
    let mut all = Samples::default();
    for _ in 0..100 / multiple {
        let (mut band, day) = bench_day(flex, BASE_SECONDS * multiple);
        match measure_checks(&mut band, &day) {
            Ok(one) => {
                all.check_ns.extend(one.check_ns);
                all.lookback_ns.extend(one.lookback_ns);
                all.flexes += one.flexes;
                all.refused += one.refused;
            }
            Err(_) => all.refused += 1,
        }
    }
    all
}

/// Prints p50, p99 and max of one kind at one size, and returns the p99. No
/// samples reads as `u64::MAX`, past every budget, so the row fails.
fn quantiles(id: &str, flex: FlexReading, kind: &str, multiple: u32, samples: &[u64]) -> u64 {
    let [p50, p99, max] =
        [500, 990, 1_000].map(|permille| quantile(samples, permille).unwrap_or(u64::MAX));
    println!(
        "{id} {} {kind}, {multiple}x ({} s a day): p50 {p50} ns, p99 {p99} ns, max {max} ns",
        flex.id(),
        BASE_SECONDS * multiple
    );
    p99
}

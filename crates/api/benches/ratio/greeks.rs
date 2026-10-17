//! Gate 8 for `/greeks.json`: an option instant's inputs cost the same
//! whatever the two months behind it hold (R3-14, RA-064).
//!
//! `api::server::optiongreeks::respond` is what the route answers with. Each
//! cell reads two month files: the option's bar at `t` (the premium) and its
//! underlying's (the spot), through the same `spot_at` lookup `/spot.json`
//! makes. Timed over an option month and its underlying's month of 1,000,
//! 10,000 and 100,000 one-second bars each, on disk, at one held instant
//! and with ten instants a request.
//!
//! **What is in the figure, and what is not.** Every store read a cell makes
//! is: both month opens, both lookups, the tenor to the settled expiry. The
//! rate table is left unset, so every cell ends `rate_unset` after its reads
//! and before the solve: the solve reads no store and is bounded by
//! `greeks::solver::MAX_ITERATIONS` whatever the store holds, so it adds a
//! constant to both sides of a ratio and nothing to what the ratio is for. The
//! response is checked before it is timed: a cell that answered
//! `no_option_bar` or `no_spot_bar` would be timing a miss, and is refused.
//!
//! Invented prices (an option at 43.21 and an underlying at 12,345.00 are not
//! a print anyone could mistake for a real one), the bench month of
//! `ratio.rs` (June 2024, GDFL, NSE), and a contract that names the month's
//! last Thursday.

use std::hint::black_box;
use std::path::{Path, PathBuf};

use api::server::optiongreeks::{RateTable, respond};
use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::{MONTH_SIZES, RATIO_CEILING_PERMILLE, bench_month, cost_ps, ratio, refuse, second};

/// The route this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/greeks.json"];

/// The contract, as the route names it below the symbol.
const CONTRACT: &str = "2024-06-27-2300000-CE";

/// Appends `bars` to one month of `segment` under `root`.
fn write(root: &Path, segment: &str, contract: Option<Contract>, bars: &[Bar]) {
    let Ok(path) = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol: "NIFTY",
        contract,
        timeframe: Timeframe::SECOND_1,
        month: bench_month(),
        file: FileKind::Bars,
    }) else {
        refuse("the greeks bench path is a legal one")
    };
    let Ok(id) = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)) else {
        refuse("the low 32 bits of a hash fit 32 bits")
    };
    let mut file = match store::file::BarFile::open_or_create(root, path, id) {
        Ok(file) => file,
        Err(why) => refuse(&format!("the greeks bench month would not open: {why}")),
    };
    if let Err(why) = file.append(bars) {
        refuse(&format!("the greeks bench month would not fill: {why}"));
    }
}

/// One bar at second `i` closing at `close` paisa.
fn bar(i: usize, close: i64) -> Bar {
    Bar {
        ts_micros: second(i),
        open: close,
        high: close,
        low: close,
        close,
        volume: 1,
        open_interest: OI_NULL,
    }
}

/// A store whose option month and underlying month hold `n` seconds each.
fn store_of(n: usize) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-greeks-{n}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("the greeks bench root would not create: {why}"));
    }
    let Some(contract) = Contract::parse(CONTRACT) else {
        refuse("the bench contract parses")
    };
    let option: Vec<Bar> = (0..n).map(|i| bar(i, 4_321)).collect();
    let spot: Vec<Bar> = (0..n).map(|i| bar(i, 1_234_500)).collect();
    write(&root, "FNO", Some(contract), &option);
    write(&root, "INDEX", None, &spot);
    root
}

/// The query for `times`, as the page asks it.
fn query(times: &[i64]) -> String {
    let t: Vec<String> = times.iter().map(i64::to_string).collect();
    format!(
        "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract={CONTRACT}&timeframe=1s\
         &month=2024-06&t={}",
        t.join(",")
    )
}

/// Each instant of `times`, answered after both reads, or a refusal.
fn checked(root: &Path, asked: &str, rate: &RateTable, cells: usize) {
    let (status, _, body) = respond(root, asked, rate);
    if !status.is_success() {
        refuse(&format!("the greeks route answered {status}: {body}"));
    }
    for miss in [
        "no_option_bar",
        "no_spot_bar",
        "no_option_file",
        "no_spot_file",
    ] {
        if body.contains(miss) {
            refuse(&format!(
                "a cell answered {miss}, which times a miss: {body}"
            ));
        }
    }
    let after_reads = body.matches("rate_unset").count() + body.matches("tenor_refused").count();
    if after_reads != cells {
        refuse(&format!(
            "{after_reads} of {cells} cells ended after both reads: {body}"
        ));
    }
}

/// One held instant and ten instants a request, at every size, and whether
/// each stayed flat between 1x and 100x. Run by `crates/api/benches/ratio.rs`'s
/// `main`.
pub(crate) fn cells_are_flat() -> bool {
    let rate = RateTable::from_var(None);
    let middle = MONTH_SIZES[0] / 2;
    let one = [second(middle) / 1_000_000];
    let ten: Vec<i64> = (0..10)
        .map(|k| second(middle + 37 * k) / 1_000_000)
        .collect();
    let roots: Vec<PathBuf> = MONTH_SIZES.iter().map(|&n| store_of(n)).collect();
    let mut ok = true;
    for (label, times) in [("one instant", &one[..]), ("ten instants", &ten[..])] {
        let asked = query(times);
        let mut costs = Vec::with_capacity(roots.len());
        for root in &roots {
            checked(root, &asked, &rate, times.len());
            costs.push(cost_ps(500, || {
                respond(root, black_box(&asked), &rate).2.len()
            }));
        }
        let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2))
        else {
            refuse("three sizes measured");
        };
        ok &= ratio(
            &format!("greeks {label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[1]),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        );
        ok &= ratio(
            &format!("greeks {label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[2]),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
    }
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

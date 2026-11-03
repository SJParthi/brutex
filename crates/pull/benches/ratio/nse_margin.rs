//! C-FNO-LEV-01 (FNO-LEVERAGE, D-8154): one margin lookup, flat at 1x, 10x
//! and 100x the months held, with a p99 budget. `MarginTable::margin` is one
//! month subtraction, one bit of the month's day bitmap, one symbol hash probe
//! and one cell read; none walks the months, the days or the symbols. Timed on
//! a held day (the hit), on a day inside the span with no file, and on a day
//! before the span. The harness, `crates/pull/benches/ratio.rs`, fails on a
//! ratio above its ceiling; this file fails on a p99 above [`BUDGET_NS`], the
//! design's target (PARTS section 11: an ESTIMATE until measured).
//!
//! Every input is invented here and nothing is written: the months are
//! encoded in memory (`nse_margin::encode`) and served by
//! `MarginTable::from_bytes`, the reader `MarginTable::open` uses. The `ZZL`
//! names are not NSE symbols and no percentage is any stock's margin.

use std::hint::black_box;
use std::time::Instant;

use brutex_core::symbol::Symbol;
use pull::nse_margin::{MarginRow, MarginTable, SERIES, encode};
use pull::session::Day;

use super::{cost_ps, ratio, refuse};

/// Symbols listed on every held day.
const SYMBOLS: u32 = 400;
/// Calls per timed repetition.
const REPS: u32 = 20_000;
/// Single calls timed for the p99 line.
const SAMPLES: u32 = 20_000;
/// The design's p99 target for one lookup, in nanoseconds (PARTS 11).
const BUDGET_NS: u128 = 1_000;
/// The first month held: September 2018, the span's start.
const FIRST: (u16, u8) = (2018, 9);

/// The `i`th invented symbol.
fn name(i: u32) -> Symbol {
    Symbol::new(&format!("ZZL{i:04}")).unwrap_or_else(|_| refuse("an invented symbol"))
}

/// [`FIRST`] plus `n` months.
fn month_after(n: u32) -> (u16, u8) {
    let at = u32::from(FIRST.0) * 12 + u32::from(FIRST.1 - 1) + n;
    (
        u16::try_from(at / 12).unwrap_or(FIRST.0),
        u8::try_from(at % 12 + 1).unwrap_or(FIRST.1),
    )
}

/// `months` months from [`FIRST`]: days 1 to 28 of each held, the 15th
/// excepted (a day inside the span with no file).
fn table(months: u32) -> MarginTable {
    let rows: Vec<MarginRow> = (0..SYMBOLS)
        .map(|i| MarginRow {
            symbol: name(i),
            series: SERIES,
            var_ppm: 100_000 + i * 250,
            elm_ppm: 50_000,
            extra_ppm: 0,
        })
        .collect();
    let mut days = Vec::new();
    for n in 0..months {
        let (year, month) = month_after(n);
        for of in (1..=28).filter(|&of| of != 15) {
            let day = Day::new(year, month, of).unwrap_or_else(|_| refuse("an invented day"));
            days.push((day, rows.clone()));
        }
    }
    let encoded = encode(&days).unwrap_or_else(|why| refuse(&format!("the encode: {why}")));
    MarginTable::from_bytes(&encoded.dictionary, &encoded.month_bytes())
        .unwrap_or_else(|why| refuse(&format!("the table: {why}")))
}

/// One timed lookup: the table, the day, the symbol; a number the optimiser
/// cannot discard.
type Lookup = fn(&MarginTable, Day, &str) -> u32;

/// p50, p99 and max over [`SAMPLES`] single calls on the 100x table; fails
/// when the p99 is above [`BUDGET_NS`].
fn within_budget(label: &str, held: &MarginTable, day: Day, probe: &str, lookup: Lookup) -> bool {
    let mut ns: Vec<u128> = (0..SAMPLES)
        .map(|_| {
            let start = Instant::now();
            black_box(lookup(black_box(held), black_box(day), black_box(probe)));
            start.elapsed().as_nanos()
        })
        .collect();
    ns.sort_unstable();
    let at = |q: usize| {
        ns.get(ns.len().saturating_sub(1) * q / 100)
            .copied()
            .unwrap_or(u128::MAX)
    };
    let p99 = at(99);
    let ok = p99 <= BUDGET_NS;
    println!(
        "  {label:<44} p50 {:>8} ns  p99 {p99:>8} ns  max {:>10} ns  n={SAMPLES}  p99 budget {BUDGET_NS} ns  {}",
        at(50),
        ns.last().copied().unwrap_or(u128::MAX),
        if ok { "ok" } else { "OVER BUDGET" }
    );
    ok
}

/// C-FNO-LEV-01: each lookup at 1x, 10x and 100x the months held (1, 10 and
/// 100 months from September 2018), and its p99 at 100x.
pub(crate) fn margin_lookup_is_flat() -> bool {
    let sizes = [table(1), table(10), table(100)];
    let held = Day::new(2018, 9, 10).unwrap_or_else(|_| refuse("a held day"));
    let no_file = Day::new(2018, 9, 15).unwrap_or_else(|_| refuse("a day with no file"));
    let before = Day::new(2018, 8, 31).unwrap_or_else(|_| refuse("a day before"));
    let probe = name(SYMBOLS / 2).to_string();
    for table in &sizes {
        if table.margin(&probe, held).map(|m| m.var_ppm) != Ok(150_000)
            || table.margin(&probe, no_file).is_ok()
            || table.margin(&probe, before).is_ok()
        {
            refuse("the invented table does not answer as built");
        }
    }
    let lookups: [(&str, Day, Lookup); 3] = [
        (
            "margin(symbol, day), a held day",
            held,
            |table, day, symbol| table.margin(symbol, day).map_or(0, |m| m.var_ppm),
        ),
        (
            "margin(symbol, day), a day with no file",
            no_file,
            |table, day, symbol| u32::from(table.margin(symbol, day).is_err()),
        ),
        (
            "margin(symbol, day), before the span",
            before,
            |table, day, symbol| u32::from(table.margin(symbol, day).is_err()),
        ),
    ];
    let mut ok = true;
    for (label, day, lookup) in lookups {
        let cost = |table: &MarginTable| {
            cost_ps(REPS, || {
                lookup(black_box(table), black_box(day), black_box(&probe))
            })
        };
        let base = cost(&sizes[0]);
        ok &= ratio(&format!("C-FNO-LEV-01 {label}, 10x"), base, cost(&sizes[1]));
        ok &= ratio(
            &format!("C-FNO-LEV-01 {label}, 100x"),
            base,
            cost(&sizes[2]),
        );
        ok &= within_budget(
            &format!("C-FNO-LEV-01 {label}, 100x"),
            &sizes[2],
            day,
            &probe,
            lookup,
        );
    }
    ok
}

//! The bench rows of the own-past histograms (FIB-2; D-6320, D-6324).
//!
//! Two axes. The specification's: the committed TOTAL of one histogram at
//! 10^2, 10^4 and 10^7 values — FIXED-STATE, because the bin table is a
//! constant and a total adds no bin (the rows prove the time does not grow
//! with it). And FM-F002's: the CELLS resident in a bank, from a cache-resident
//! 128 to 12,800 (about 135 MB, past this host's last-level cache), each call
//! landing in a pseudo-random cell, as a stock-day's candidates land in its
//! bank; one stock's production bank is not measured yet (FIB-12's sample), so
//! that axis prints EXTRAPOLATION.

use std::fmt::Write as _;

use indicators::fibflag::hist::{BankShape, CellKey, Dims, HistBank, Measure, OwnPast};
use indicators::fibflag::types::BaseKind;
use vocab::fib_cube::Search;

use crate::harness::{Axis, Class, Host, Row, run};

/// A deterministic value for call `i`: spread over 2^40, as bp and seconds
/// and milli-R values are.
fn value(i: u64) -> i64 {
    let mixed = i.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 24;
    i64::try_from(mixed & ((1 << 40) - 1)).unwrap()
}

/// One histogram holding `total` committed values over ten days.
fn one(total: u64) -> OwnPast {
    let mut past = OwnPast::empty();
    let per_day = total.div_ceil(10);
    let mut held = 0;
    while held < total {
        for _ in 0..per_day.min(total - held) {
            past.observe(value(held)).unwrap();
            held += 1;
        }
        past.commit_day().unwrap();
    }
    past
}

/// A bank, its keys in index order, and the work its calls did.
pub(crate) struct Resident {
    /// The bank.
    bank: HistBank,
    /// Every cell's key.
    keys: Vec<CellKey>,
    /// The work of the calls made, read from the cell each call touched.
    work: u64,
}

/// A bank of about `cells` cells (one width per 92 long cells), every cell
/// holding one committed day of values.
fn bank(cells: u64) -> Resident {
    let widths = u16::try_from(cells.div_ceil(92)).unwrap();
    let shape = BankShape::new(
        Dims {
            widths: (1..=widths).collect(),
            styles: 2,
            variants: 3,
            levels: 2,
        },
        Dims {
            widths: vec![1],
            styles: 1,
            variants: 1,
            levels: 1,
        },
    )
    .unwrap();
    let mut bank = HistBank::new(shape);
    let keys = bank.shape().keys();
    for (n, key) in keys.iter().enumerate() {
        for k in 0..8 {
            bank.observe(key, value(n as u64 * 8 + k)).unwrap();
        }
    }
    bank.commit_day().unwrap();
    Resident {
        bank,
        keys,
        work: 0,
    }
}

/// A key of the bank for call `i`, spread over every cell.
fn spread(keys: &[CellKey], i: u64) -> &CellKey {
    &keys[usize::try_from(i.wrapping_mul(2_654_435_761) % keys.len() as u64).unwrap()]
}

/// The rows, run and reported. False when any row failed.
pub(crate) fn report(host: &Host, out: &mut String) -> bool {
    let totals = [100, 10_000, 10_000_000];
    let fixed = Some(Axis::FixedState {
        why: "the committed total of one histogram; its bin table is a constant",
    });
    let observe = Row {
        id: "H-F-01",
        name: "OwnPast::observe (stage one value)",
        class: Some(Class::WorstCase),
        axis: fixed,
        scales: totals,
        setup: one,
        op: |past, i| {
            past.observe(value(i)).unwrap();
            past.staged_values()
        },
        work: OwnPast::work,
        bytes: |past| past.bytes() as u64,
    };
    let mid_rank = Row {
        id: "H-F-02",
        name: "OwnPast::mid_rank (the exact own-past position)",
        class: Some(Class::WorstCase),
        axis: fixed,
        scales: totals,
        setup: one,
        op: |past, i| past.mid_rank(value(i)).unwrap().num(),
        work: OwnPast::work,
        bytes: |past| past.bytes() as u64,
    };
    let permille = Row {
        id: "H-F-03",
        name: "OwnPast::permille (display)",
        class: Some(Class::WorstCase),
        axis: fixed,
        scales: totals,
        setup: one,
        op: |past, i| u64::from(past.permille(value(i)).unwrap()),
        work: OwnPast::work,
        bytes: |past| past.bytes() as u64,
    };
    let commit = Row {
        id: "H-F-04",
        name: "OwnPast::commit_day of a day that staged one value",
        class: Some(Class::WorstCase),
        axis: fixed,
        scales: totals,
        setup: one,
        op: |past, i| {
            past.observe(value(i)).unwrap();
            past.commit_day().unwrap();
            past.total()
        },
        work: OwnPast::work,
        bytes: |past| past.bytes() as u64,
    };
    let quantile = Row {
        id: "H-F-05",
        name: "OwnPast::value_at_rank (one bisection of the cumulative array)",
        class: Some(Class::WorstCase),
        axis: fixed,
        scales: totals,
        setup: one,
        op: |past, i| {
            let rank = 1 + i % past.total();
            past.value_at_rank(rank).unwrap().unsigned_abs()
        },
        work: OwnPast::work,
        bytes: |past| past.bytes() as u64,
    };
    let mut ok = true;
    for verdict in [
        run(&observe, host, out),
        run(&mid_rank, host, out),
        run(&permille, host, out),
        run(&commit, host, out),
        run(&quantile, host, out),
    ] {
        ok &= !verdict.fails();
    }
    ok & bank_rows(host, out)
}

/// The rows over a bank of cells (FM-F002's data axis), run and reported.
fn bank_rows(host: &Host, out: &mut String) -> bool {
    let cells = Some(Axis::Data {
        unit: "own-past cells resident in one bank",
        production: None,
    });
    let bank_observe = Row {
        id: "H-F-06",
        name: "HistBank::observe into a pseudo-random cell",
        class: Some(Class::WorstCase),
        axis: cells,
        scales: [128, 1_280, 12_800],
        setup: bank,
        op: |state, i| {
            let key = *spread(&state.keys, i);
            let cell = state.bank.cell_mut(&key).unwrap();
            let before = cell.work();
            cell.observe(value(i)).unwrap();
            state.work += cell.work() - before;
            i
        },
        work: |state| state.work,
        bytes: |state| state.bank.bytes(),
    };
    let bank_rank = Row {
        id: "H-F-07",
        name: "HistBank cell + OwnPast::mid_rank in a pseudo-random cell",
        class: Some(Class::WorstCase),
        axis: cells,
        scales: [128, 1_280, 12_800],
        setup: bank,
        op: |state, i| {
            let cell = state.bank.cell(spread(&state.keys, i)).unwrap();
            let before = cell.work();
            let num = cell.mid_rank(value(i)).unwrap().num();
            state.work += cell.work() - before;
            num
        },
        work: |state| state.work,
        bytes: |state| state.bank.bytes(),
    };
    let mut ok = true;
    for verdict in [run(&bank_observe, host, out), run(&bank_rank, host, out)] {
        ok &= !verdict.fails();
    }
    let probe = bank(1);
    let key = CellKey::at(Measure::StopBp, Search::Long, 0).kind(BaseKind::Rest);
    let _ = writeln!(
        out,
        "  bank sizes: one unsplit cell {} bytes; a one-width bank of {} cells holds {} bytes; \
         StopBp long width 0 rest holds {} values\n",
        indicators::fibflag::hist::UNSPLIT_CELL_BYTES,
        probe.bank.shape().cell_count(),
        probe.bank.bytes(),
        probe.bank.cell(&key).unwrap().total()
    );
    ok
}

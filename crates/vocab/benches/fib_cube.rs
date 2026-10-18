//! The FIB bench target of `crates/vocab` (FIB-1; D-6318): the cell code and
//! the containment test of `vocab::fib_cube`, through the shared FIB harness
//! (`crates/runner/benches/fib_rows/harness.rs`).
//!
//! `C-F-02` decodes codes below 2^64, which `Layout::decode` divides in `u64`;
//! `C-F-04` decodes codes past 2^64 on a wider layout, the `u128` path.
//!
//! All four rows are FIXED-STATE: one call reads one cell (or one mix and one
//! cell) and the layout, whatever the pool of inputs it is drawn from, so they
//! are exempt from the cache rule (FM-F002) and labelled so. The scale is the
//! number of distinct inputs the calls cycle through, 100 to 10,000, all inside
//! the cache: the row measures the call, not the load of its input from memory
//! (which is the lattice's data-axis row, FIB-6). The
//! work counter is the number of rows each call visits: the row loop's trip
//! count, `ROW_COUNT` by construction, counted on the bench's own state. Every
//! input is invented. Integers only, like the rest of this crate.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a bench that cannot measure has nothing to report"
)]

#[path = "../../runner/benches/fib_rows/harness.rs"]
mod harness;

use harness::{Axis, Class, Row, run};
use vocab::fib_cube::{
    Coords, CutList, EntryVariants, Generation, GenerationParts, Layout, MixKey, OPEN, ROW_COUNT,
    Ratio, RowId, Search, Source, VariantSet, contains,
};

/// A synthetic layout: every measured row three buckets, ten rank groups,
/// four widths, six exit variants, the three entry styles. Its codes fit 64
/// bits.
fn layout() -> Layout {
    layout_with(3, 10)
}

/// The same layout with sixteen buckets in every measured row: its codes pass
/// 2^64.
fn wide_layout() -> Layout {
    layout_with(16, 1)
}

/// `values` buckets in every measured row; the time-of-day cuts are `step`
/// candles of the width apart.
fn layout_with(values: u64, step: u64) -> Layout {
    let widths = [1, 2, 4, 8];
    let mut cuts = Vec::new();
    for id in RowId::ALL {
        if id.row().source != Source::Measured || id == RowId::MoverRank {
            continue;
        }
        if id == RowId::TimeOfDay {
            for minutes in widths {
                cuts.push(CutList {
                    row: id,
                    timeframe: Some(minutes),
                    base_kind: None,
                    cuts: (1..values)
                        .map(|k| Ratio::whole(u64::from(minutes) * 60 * step * k))
                        .collect(),
                });
            }
        } else {
            cuts.push(CutList {
                row: id,
                timeframe: None,
                base_kind: None,
                cuts: (1..values)
                    .map(|k| Ratio::new(k, values).unwrap())
                    .collect(),
            });
        }
    }
    *Generation::new(GenerationParts {
        search: Search::Long,
        version: 1,
        birth_step: 6,
        timeframes: widths.to_vec(),
        rank_edges: vec![2, 4, 8, 16, 32, 64, 96, 128, 160],
        cuts,
        exit_set: VariantSet {
            version: 1,
            digest: [7; 32],
            count: 6,
        },
        entry_variants: EntryVariants {
            version: 1,
            digest: [8; 32],
            styles: vec![0, 1, 2],
        },
        others: Vec::new(),
    })
    .expect("a valid synthetic generation")
    .layout()
}

/// splitmix64, for the input pools.
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn cell(layout: &Layout, seed: u64) -> Coords {
    let mut c = [0; ROW_COUNT];
    for (at, (slot, radix)) in c.iter_mut().zip(layout.radices()).enumerate() {
        *slot = (mix(seed * 31 + at as u64) % u64::from(*radix)) as u8;
    }
    c
}

/// The bench's own state: the layout, the inputs, the rows visited so far.
struct Pool {
    layout: Layout,
    cells: Vec<Coords>,
    codes: Vec<u128>,
    mixes: Vec<MixKey>,
    rows: u64,
}

fn pool(n: u64) -> Pool {
    let layout = layout();
    let cells: Vec<Coords> = (0..n).map(|i| cell(&layout, i)).collect();
    let codes = cells.iter().map(|c| layout.encode(c).unwrap()).collect();
    let mixes = (0..n)
        .map(|i| {
            let mut m = MixKey::root_of(&cell(&layout, i + n));
            for (at, slot) in m.0.iter_mut().enumerate() {
                if *slot == OPEN && mix(i * 7 + at as u64).is_multiple_of(2) {
                    *slot = cell(&layout, i)[at];
                }
            }
            m
        })
        .collect();
    Pool {
        layout,
        cells,
        codes,
        mixes,
        rows: 0,
    }
}

/// Codes past 2^64: the wide layout's cells with the most significant row at
/// its last value, so every code is at least three quarters of the cells.
fn wide_pool(n: u64) -> Pool {
    let layout = wide_layout();
    assert!(layout.paper_full_cells() > 4 * u128::from(u64::MAX));
    let cells: Vec<Coords> = (0..n)
        .map(|i| {
            let mut c = cell(&layout, i);
            c[0] = layout.radices()[0] - 1;
            c
        })
        .collect();
    let codes: Vec<u128> = cells.iter().map(|c| layout.encode(c).unwrap()).collect();
    assert!(codes.iter().all(|&code| code > u128::from(u64::MAX)));
    Pool {
        layout,
        cells,
        codes,
        mixes: Vec::new(),
        rows: 0,
    }
}

fn at(pool: &Pool, i: u64) -> usize {
    (i % pool.cells.len() as u64) as usize
}

const fn fixed() -> Axis {
    Axis::FixedState {
        why: "one cell, or one mix and one cell, and the layout",
    }
}

fn main() {
    let mut out = String::new();
    out.push_str("crates/vocab — FIB bench (FIB-1 harness)\n");
    let host = harness::host(&mut out);
    out.push('\n');
    let rows_visited = |p: &Pool| p.rows;
    let pool_bytes = |p: &Pool| (p.cells.len() * (ROW_COUNT * 3 + 16)) as u64;
    let encode = Row {
        id: "C-F-01",
        name: "Layout::encode of one cell",
        class: Some(Class::WorstCase),
        axis: Some(fixed()),
        scales: [100, 1_000, 10_000],
        setup: pool,
        op: |p, i| {
            p.rows += ROW_COUNT as u64;
            p.layout.encode(&p.cells[at(p, i)]).unwrap() as u64
        },
        work: rows_visited,
        bytes: pool_bytes,
    };
    let decode = Row {
        id: "C-F-02",
        name: "Layout::decode of one code",
        class: Some(Class::WorstCase),
        axis: Some(fixed()),
        scales: [100, 1_000, 10_000],
        setup: pool,
        op: |p, i| {
            p.rows += ROW_COUNT as u64;
            u64::from(p.layout.decode(p.codes[at(p, i)]).unwrap()[0])
        },
        work: rows_visited,
        bytes: pool_bytes,
    };
    let wide_decode = Row {
        id: "C-F-04",
        name: "Layout::decode of one code past 2^64",
        class: Some(Class::WorstCase),
        axis: Some(fixed()),
        scales: [100, 1_000, 10_000],
        setup: wide_pool,
        op: |p, i| {
            p.rows += ROW_COUNT as u64;
            u64::from(p.layout.decode(p.codes[at(p, i)]).unwrap()[0])
        },
        work: rows_visited,
        bytes: pool_bytes,
    };
    let containment = Row {
        id: "C-F-03",
        name: "contains of one cell in one mix",
        class: Some(Class::WorstCase),
        axis: Some(fixed()),
        scales: [100, 1_000, 10_000],
        setup: pool,
        op: |p, i| {
            p.rows += ROW_COUNT as u64;
            let k = at(p, i);
            u64::from(contains(&p.mixes[k], &p.cells[k]))
        },
        work: rows_visited,
        bytes: pool_bytes,
    };
    let mut ok = true;
    for verdict in [
        run(&encode, &host, &mut out),
        run(&decode, &host, &mut out),
        run(&wide_decode, &host, &mut out),
        run(&containment, &host, &mut out),
    ] {
        ok &= !verdict.fails();
    }
    print!("{out}");
    if !ok {
        println!("\na FIB row FAILED or was refused; see above.");
        std::process::exit(1);
    }
    println!("\nok — every FIB row of crates/vocab measured, judged and reported.");
}

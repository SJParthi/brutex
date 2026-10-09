//! Gate 8 for the census probe: one instrument-month lookup costs the same
//! whatever the manifest behind it holds.
//!
//! `VendorCensus::rows_for` is the one hash probe every census-backed route
//! makes per key, and `rows_of` (the WP-01 interface) makes the same probe.
//! Timed on a held key and on an absent one, over manifests of 1,000, 10,000
//! and 100,000 entries built in memory, so no disk is in the figure.
//!
//! WP-30 owns this file from here and adds its per-vendor cache rows to it.

use std::hint::black_box;

use api::census::{Census, Series, VendorCensus};
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::manifest::{Entry, EntryKey, Manifest};
use store::path::{Timeframe, YearMonth};

use super::{RATIO_CEILING_PERMILLE, cost_ps, ratio, refuse};

/// The three manifest sizes, 1x / 10x / 100x.
const SIZES: [usize; 3] = [1_000, 10_000, 100_000];

/// One key of the synthetic census: the `n`th cash symbol, September 2026.
fn key(n: usize) -> EntryKey {
    let Ok(symbol) = Symbol::new(&format!("S{n:07}")) else {
        refuse("a synthetic symbol");
    };
    let Ok(month) = YearMonth::new(2026, 9) else {
        refuse("September 2026");
    };
    Series {
        contract: None,
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        symbol,
        timeframe: Timeframe::MINUTE_1,
    }
    .at(month)
}

/// A held census of `n` entries, every one of them 375 rows.
fn census_of(n: usize) -> VendorCensus {
    let Ok(mut manifest) = Manifest::open(Vendor::Gdfl, &[], &[]) else {
        refuse("a genesis census");
    };
    for i in 0..n {
        let entry = Entry {
            key: key(i),
            rows: 375,
            first_ts_micros: 1_788_926_700_000_000,
            last_ts_micros: 1_788_949_140_000_000,
        };
        if manifest.record(entry).is_err() {
            refuse("a census entry");
        }
    }
    VendorCensus {
        vendor: Vendor::Gdfl,
        path: std::path::PathBuf::from("/nonexistent/bench/gdfl.man"),
        state: Census::Held {
            manifest: Box::new(manifest),
        },
    }
}

/// One probe's cost, held and absent, at every size, and whether both stayed
/// flat between 1x and 100x. Run by `crates/api/benches/ratio.rs`'s `main`.
pub(crate) fn probes_are_flat() -> bool {
    let built: Vec<VendorCensus> = SIZES.iter().map(|&n| census_of(n)).collect();
    // A key every census holds, and one none of them does.
    let held = key(SIZES[0] / 2);
    let absent = key(SIZES[2] * 2);
    let mut ok = true;
    for (label, probe, want) in [("held", held, Some(375)), ("absent", absent, None)] {
        let mut costs = Vec::with_capacity(built.len());
        for census in &built {
            if census.rows_for(&probe) != want {
                refuse("the probe answered something other than the fixture holds");
            }
            costs.push(cost_ps(20_000, || census.rows_for(black_box(&probe))));
        }
        let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2))
        else {
            refuse("three sizes measured");
        };
        ok &= ratio(
            &format!("census probe {label} {}->{}", SIZES[0], SIZES[1]),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        );
        ok &= ratio(
            &format!("census probe {label} {}->{}", SIZES[0], SIZES[2]),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
    }
    ok
}

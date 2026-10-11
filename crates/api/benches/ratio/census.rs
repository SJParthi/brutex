//! Gate 8 for the census probe: one instrument-month lookup costs the same
//! whatever the manifest behind it holds.
//!
//! `VendorCensus::rows_for` is the one hash probe every census-backed route
//! makes per key, and `rows_of` (the WP-01 interface) makes the same probe.
//! Timed on a held key and on an absent one, over manifests of 1,000, 10,000
//! and 100,000 entries built in memory, so no disk is in the figure.
//!
//! WP-30's per-vendor cache row is the last: a census miss reads the entries
//! appended since the last read, so its cost is flat in what the census holds
//! ([`tails_are_flat`], D-3327).

use std::hint::black_box;

use api::census::{Census, Manifests, Series, VendorCensus};
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
            manifest: Manifests::single(manifest),
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
    // THE PER-VENDOR CACHE'S ROW, on this file's call from `main` (WP-30).
    ok &= tails_are_flat();
    // THE REFRESH'S ROW (WP-56): a delta costs what was appended.
    ok &= deltas_are_flat();
    ok
}

/// Entries appended between the whole read and every timed tail read.
const APPENDED: usize = 256;

/// One census on disk of `n` entries with [`APPENDED`] more appended in place
/// after it was read whole: the file, the whole read, and the store root.
///
/// The append is the writer's own: each entry at its offset, then the header
/// slot that counts it, into the same file (`pull::manifest::Append`).
fn on_disk(n: usize) -> (std::path::PathBuf, VendorCensus) {
    use std::os::unix::fs::FileExt as _;
    let root = std::env::temp_dir().join(format!(
        "brutex-bench-census-tail-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    if std::fs::create_dir_all(root.join("manifest")).is_err() {
        refuse("a scratch store root");
    }
    let Ok(mut manifest) = Manifest::open(Vendor::Gdfl, &[], &[]) else {
        refuse("a genesis census");
    };
    let entry = |i: usize| Entry {
        key: key(i),
        rows: 375,
        first_ts_micros: 1_788_926_700_000_000,
        last_ts_micros: 1_788_949_140_000_000,
    };
    for i in 0..n {
        if manifest.record(entry(i)).is_err() {
            refuse("a census entry");
        }
    }
    let path = pull::manifest::manifest_path(&root, Vendor::Gdfl);
    if std::fs::write(&path, manifest.image()).is_err() {
        refuse("a census on disk");
    }
    let whole = api::census::read_vendor(&root, Vendor::Gdfl);
    let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) else {
        refuse("the census opened for appending");
    };
    for i in n..n + APPENDED {
        let Ok(append) = manifest.record(entry(i)) else {
            refuse("an appended entry");
        };
        if file.write_all_at(&append.bytes, append.offset).is_err()
            || file
                .write_all_at(&append.commit.bytes, append.commit.offset)
                .is_err()
        {
            refuse("an append written in place");
        }
    }
    (root, whole)
}

/// A census miss costs the entries appended since the last read, whatever
/// the census holds (D-3327 item 1; DB-059, DB-G06, DB-G79).
///
/// Over single-file censuses of 1,000, 10,000 and 100,000 entries on disk,
/// each read whole once and then appended [`APPENDED`] entries in place, the
/// timed call is `read_vendor_after` from the whole read: one header region,
/// the first and last entries already read, and the appended entries
/// decoded. A whole read of the same file is printed beside it, unasserted,
/// so the difference the tail makes is on the record. Every read must reach
/// the census a whole read gives: the counters are checked before timing.
fn tails_are_flat() -> bool {
    let built: Vec<(std::path::PathBuf, VendorCensus)> =
        SIZES.iter().map(|&n| on_disk(n)).collect();
    let mut tails = Vec::with_capacity(built.len());
    for (&n, (root, whole)) in SIZES.iter().zip(&built) {
        let extended = api::census::read_vendor_after(root, Vendor::Gdfl, Some(whole));
        let again = api::census::read_vendor(root, Vendor::Gdfl);
        let counters = |census: &VendorCensus| census.counters();
        if counters(&extended) != counters(&again)
            || extended.generation() != again.generation()
            || extended.counters().map(|(_, _, entries)| entries)
                != u64::try_from(n + APPENDED).ok()
        {
            refuse("the tail read did not reach the census a whole read gives");
        }
        tails.push(cost_ps(200, || {
            api::census::read_vendor_after(root, Vendor::Gdfl, Some(black_box(whole)))
        }));
        let whole_ps = cost_ps(5, || {
            api::census::read_vendor(black_box(root), Vendor::Gdfl)
        });
        println!("  census whole read at {n:>7} entries (unasserted) {whole_ps:>10} ps");
    }
    for (root, _) in &built {
        let _ = std::fs::remove_dir_all(root);
    }
    let (Some(&base), Some(&mid), Some(&big)) = (tails.first(), tails.get(1), tails.get(2)) else {
        refuse("three sizes measured");
    };
    let mut ok = ratio(
        &format!("census tail of {APPENDED} {}->{}", SIZES[0], SIZES[1]),
        base,
        mid,
        RATIO_CEILING_PERMILLE,
    );
    ok &= ratio(
        &format!("census tail of {APPENDED} {}->{}", SIZES[0], SIZES[2]),
        base,
        big,
        RATIO_CEILING_PERMILLE,
    );
    ok
}

/// A `/store.json?...&since=` delta costs the entries appended since that
/// generation, whatever the census holds (WP-56; D-3327 item 3; DB-060,
/// DB-G59, DB-G70).
///
/// Over the same three censuses on disk as [`tails_are_flat`], each read
/// whole and then extended by [`APPENDED`] entries appended in place, the
/// timed call is `api::server::census_delta` since the whole read's
/// generation: a binary search of the tail, two probes per changed row and
/// the encoding of those rows. The answer is checked before timing: 200,
/// [`APPENDED`] rows, and the census's own counters beside them. The whole
/// body's encode grows with the census; this must not.
fn deltas_are_flat() -> bool {
    let built: Vec<(std::path::PathBuf, VendorCensus)> =
        SIZES.iter().map(|&n| on_disk(n)).collect();
    let mut costs = Vec::with_capacity(built.len());
    for (&n, (root, whole)) in SIZES.iter().zip(&built) {
        let extended = api::census::read_vendor_after(root, Vendor::Gdfl, Some(whole));
        let Some(since) = whole.generation().map(|generation| generation.to_string()) else {
            refuse("the whole read holds a generation");
        };
        let (code, body) = api::server::census_delta(&extended, &since, "");
        let Ok(answer) = serde_json::from_str::<serde_json::Value>(&body) else {
            refuse("the delta is JSON");
        };
        let rows = answer
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len);
        let months = answer
            .get("held")
            .and_then(|held| held.get("months"))
            .and_then(serde_json::Value::as_u64);
        if code != 200 || rows != Some(APPENDED) || months != u64::try_from(n + APPENDED).ok() {
            refuse("the delta is not the appended entries with the census's counters");
        }
        costs.push(cost_ps(200, || {
            api::server::census_delta(black_box(&extended), black_box(&since), "")
        }));
    }
    for (root, _) in &built {
        let _ = std::fs::remove_dir_all(root);
    }
    let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2)) else {
        refuse("three sizes measured");
    };
    let mut ok = ratio(
        &format!("census delta of {APPENDED} {}->{}", SIZES[0], SIZES[1]),
        base,
        mid,
        RATIO_CEILING_PERMILLE,
    );
    ok &= ratio(
        &format!("census delta of {APPENDED} {}->{}", SIZES[0], SIZES[2]),
        base,
        big,
        RATIO_CEILING_PERMILLE,
    );
    ok
}

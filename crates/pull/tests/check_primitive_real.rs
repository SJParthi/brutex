//! CC-20's real-size figures for the two check primitives that are O(n) once
//! per run and that no read-only timer measured on real files before (FM-615,
//! D-6803): the options year index, built by the first `day()` of a run over
//! the operator's directory of year zips, and the census cold load of the
//! operator's store. Gate 8's benches time both on invented files
//! (`crates/pull/benches/ratio/nfo_year.rs`, `.../census_load.rs`); this
//! puts the real sizes beside them, so the per-run figure in
//! `docs/06-limits.md` is a measurement and not an extrapolation.
//!
//! Ignored, and a no-op unless its variables are set: the real paths come
//! only from `BRUTEX_GDFL_NFO_ARCHIVES` (the folder of `<yyyy>.zip` year zips)
//! and `BRUTEX_CC20_CENSUS_ROOT` (a store root), with no default. Both reads
//! are read only: `NfoZips` opens and reads the year zips and writes nothing,
//! and `census_store::read_vendor` takes no lock and writes nothing (its own
//! doc). Never `census_store::record` or `cycle`, which write. Run in release:
//!
//! ```text
//! BRUTEX_GDFL_NFO_ARCHIVES=<dir> BRUTEX_CC20_CENSUS_ROOT=<store> \
//!   cargo test --release -p pull --test check_primitive_real -- --ignored --nocapture
//! ```
//!
//! Each sample is one call, sequential; the first of each is cold.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::Path;
use std::time::Instant;

use brutex_core::vendor::Vendor;
use pull::gdfl_nfo::{NfoSource, NfoZips};
use pull::ingest::census_store::read_vendor;
use pull::session::Day;

/// p50, p99 and max of `ns`, sorted in place.
fn spread(ns: &mut [u128]) -> (u128, u128, u128) {
    ns.sort_unstable();
    let at = |q: usize| ns.get(ns.len().saturating_sub(1) * q / 100).copied().unwrap_or(0);
    (at(50), at(99), ns.last().copied().unwrap_or(0))
}

/// Every calendar day from 2018-09-03, `n` of them.
fn days(n: u32) -> Vec<Day> {
    let first = Day::new(2018, 9, 3).unwrap();
    (0..n)
        .filter_map(|i| Day::from_days(first.days_from_epoch() + i).ok())
        .collect()
}

/// The year index over the real year zips: the first `day()` of a fresh
/// source, twice (the first cold), then 400 later days of the warm one.
fn year_index(dir: &Path) {
    let calendar = days(2_950);
    let mut held = None;
    for run in ["cold run", "warm run"] {
        let source = NfoZips::new(dir);
        let at = Instant::now();
        let first = source.day(calendar[0]);
        println!(
            "CC-20 nfo first day() ({run}: lists the folder, indexes every year zip): {} us, {:?}",
            at.elapsed().as_micros(),
            first.as_ref().map(|d| d.as_ref().map(|l| l.entries().len()))
        );
        held = Some(source);
    }
    let source = held.unwrap();
    let mut later = Vec::with_capacity(400);
    let mut listed = 0usize;
    for day in calendar.iter().step_by(7).take(400) {
        let at = Instant::now();
        let got = source.day(*day);
        later.push(at.elapsed().as_nanos());
        listed += usize::from(matches!(got, Ok(Some(_))));
    }
    let n = later.len();
    let (p50, p99, max) = spread(&mut later);
    println!(
        "CC-20 nfo later day() (index hit plus the day zip's own listing): n={n} \
         p50={p50} ns p99={p99} ns max={max} ns, {listed} days listed"
    );
    assert!(listed > 0, "the real year zips list no day of 2018-09..2026-09");
}

/// The census cold load of the real store, every vendor, five samples each.
fn census(root: &Path) {
    let mut read_any = false;
    for vendor in [Vendor::Gdfl, Vendor::Zerodha] {
        let mut ns = Vec::with_capacity(5);
        let mut shape = String::new();
        for _ in 0..5 {
            let at = Instant::now();
            let reads = read_vendor(root, vendor).unwrap();
            ns.push(at.elapsed().as_nanos());
            shape = reads
                .iter()
                .map(|r| {
                    format!(
                        "{} epoch {:?} entries {} keys {}",
                        r.segment.map_or("single file", |s| s.as_str()),
                        r.epoch,
                        r.census.entries(),
                        r.census.keys()
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            read_any |= !reads.is_empty();
        }
        let n = ns.len();
        let first = ns.first().copied().unwrap_or(0);
        let (p50, p99, max) = spread(&mut ns);
        println!(
            "CC-20 census cold load {}: first {first} ns, n={n} p50={p50} ns p99={p99} ns \
             max={max} ns; {shape}",
            vendor.as_str()
        );
    }
    assert!(read_any, "the store root holds no census of GDFL or Zerodha");
}

#[test]
#[ignore = "a read-only timing run over the operator's real files, named by variables"]
fn cc20_real_check_primitive_costs() {
    if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
        year_index(Path::new(&dir));
    }
    if let Some(root) = std::env::var_os("BRUTEX_CC20_CENSUS_ROOT") {
        census(Path::new(&root));
    }
}

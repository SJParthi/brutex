//! The write door over a sealed month, on real months (SC-4, D-5904): an
//! operator's check, never part of the suite.
//!
//! `cargo test` runs nothing here: the one test is `#[ignore]`d and reads its
//! months from a private store root an operator names. For each listed month
//! and IST day it builds, under a SECOND private root, the month without that
//! day (a raw month through the writer door), seals it (`sealed::build`,
//! published by rename, as SC-3's helper does; the sealer is SC-6's), and then
//! asks the writer door: a held day re-offered is `AlreadyPresent`, the
//! missing day is `Sealed` with the offer the raw form would have met
//! (`Follows` for the month's last day, `NotHeld` for one inside it), nothing
//! is written to either form, and `repair::repair_month` publishes a
//! successor that holds the source month bit for bit. No vendor row is in
//! this file or in the repository: the bars are read from the private root
//! at run time.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, Form, SealedOffer, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::repair::{MonthRepair, Policy};

const IST: i64 = 19_800;

/// Days from 1970-01-01 to the civil date (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn ist_day(bar: &Bar) -> i64 {
    (bar.ts_micros / 1_000_000 + IST).div_euclid(86_400)
}

fn store_path(rel: &str) -> (StorePath<'static>, &'static str) {
    let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
    let parts: Vec<&'static str> = rel.split('/').collect();
    assert!(
        parts.len() == 7 || parts.len() == 8,
        "{rel}: not a month path"
    );
    let vendor = Vendor::ALL
        .into_iter()
        .find(|v| v.as_str() == parts[1])
        .unwrap();
    let contract = (parts.len() == 8)
        .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
    let tf = parts[parts.len() - 2];
    let timeframe = *Timeframe::KNOWN.iter().find(|t| t.as_str() == tf).unwrap();
    let stem = parts[parts.len() - 1].strip_suffix(".bin").unwrap();
    let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
    let path = StorePath::new(PathParts {
        vendor,
        exchange: parts[2],
        segment: parts[3],
        symbol: parts[4],
        contract,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .unwrap();
    (path, parts[4])
}

fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    files
}

fn micros(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1e6
}

/// THE CHECK. `BRUTEX_SC4_SOURCE`: a private root holding APFS clones of real
/// months (never the real store); `BRUTEX_SC4_WORK`: an empty private root
/// this test writes; `BRUTEX_SC4_MONTHS`: lines `<bars/... .bin> <yyyy-mm-dd or last>`.
#[test]
#[ignore = "reads real months from BRUTEX_SC4_SOURCE and writes BRUTEX_SC4_WORK, both private"]
fn the_write_door_over_real_months_sealed_without_one_day() {
    let source = PathBuf::from(std::env::var("BRUTEX_SC4_SOURCE").expect("BRUTEX_SC4_SOURCE"));
    let work = PathBuf::from(std::env::var("BRUTEX_SC4_WORK").expect("BRUTEX_SC4_WORK"));
    let list = std::env::var("BRUTEX_SC4_MONTHS").expect("BRUTEX_SC4_MONTHS");
    for root in [&source, &work] {
        assert!(
            !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
            "never the real store"
        );
        assert!(
            !fs::symlink_metadata(root.join("bars"))
                .is_ok_and(|meta| meta.file_type().is_symlink()),
            "{}: bars is a link; this reads and writes only private copies",
            root.display()
        );
    }
    for (line_no, line) in fs::read_to_string(&list).unwrap().lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (rel, date) = line.split_once(' ').expect("<path> <yyyy-mm-dd>");
        let (path, symbol) = store_path(rel);
        let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
        let full = {
            let file = BarFile::open_existing(&source, path, symbol_id).unwrap();
            file.read_range(0, file.records()).unwrap()
        };
        // `last`: the last IST day the month holds.
        let day = if date == "last" {
            ist_day(full.last().expect("a month with bars"))
        } else {
            let ymd: Vec<i64> = date.split('-').map(|p| p.parse().unwrap()).collect();
            days_from_civil(ymd[0], ymd[1], ymd[2])
        };
        let (held, offered): (Vec<Bar>, Vec<Bar>) =
            full.iter().copied().partition(|bar| ist_day(bar) != day);
        assert!(!offered.is_empty(), "{rel}: no bar on {date}");
        assert!(!held.is_empty(), "{rel}: nothing but {date}");
        let root = work.join(format!("m{line_no}"));
        fs::create_dir_all(&root).unwrap();
        {
            let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
            for chunk in held.chunks(65_536) {
                writer.append(chunk).unwrap();
            }
        }
        let raw = BarFile::open_existing(&root, path, symbol_id).unwrap();
        let built = store::sealed::build(path, &raw.header(), &held, None, None).unwrap();
        drop(raw);
        let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&root);
        let staged = sbar.with_extension("sbar-staged");
        fs::write(&staged, built.to_bytes()).unwrap();
        fs::rename(&staged, &sbar).unwrap();
        let before = snapshot(&root.join("bars"));

        let started = Instant::now();
        let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
        let open_us = micros(started);
        assert_eq!(writer.form(), Form::Sealed, "{rel}");
        let first_day = ist_day(&held[0]);
        let reoffer: Vec<Bar> = held
            .iter()
            .copied()
            .filter(|bar| ist_day(bar) == first_day)
            .collect();
        let started = Instant::now();
        let again = writer.append(&reoffer).unwrap();
        let reoffer_us = micros(started);
        assert_eq!(
            again,
            Appended::AlreadyPresent {
                first_index: 0,
                n_valid: held.len() as u64
            },
            "{rel}"
        );
        let started = Instant::now();
        let refused = writer.append(&offered).unwrap_err();
        let refused_us = micros(started);
        let StoreError::Sealed { offer, at, .. } = refused else {
            panic!("{rel}: {refused}");
        };
        let is_last_day = day == ist_day(full.last().unwrap());
        assert_eq!(
            offer,
            if is_last_day {
                SealedOffer::Follows
            } else {
                SealedOffer::Disagrees(store::file::Conflict::NotHeld)
            },
            "{rel}"
        );
        assert_eq!(at, 0, "{rel}");
        drop(writer);
        assert_eq!(
            snapshot(&root.join("bars")),
            before,
            "{rel}: nothing written"
        );

        let started = Instant::now();
        let repaired = store::repair::repair_month(
            &root,
            path,
            symbol_id,
            &offered,
            Policy {
                // No offered bar shares a stamp with a held one: the day is
                // wholly absent, so the comparison this sets is never made.
                open_interest_applies: true,
                basis_witness: false,
            },
        )
        .unwrap();
        let repair_ms = micros(started) / 1e3;
        let MonthRepair::Repaired { added, .. } = repaired else {
            panic!("{rel}: {repaired:?}");
        };
        assert_eq!(added, offered.len() as u64, "{rel}");
        let served = BarFile::open_existing(&root, path, symbol_id).unwrap();
        assert_eq!(served.form(), Form::Raw, "{rel}: the successor is raw");
        let now = served.read_range(0, served.records()).unwrap();
        assert!(
            now == full,
            "{rel}: the successor holds the source month bit for bit"
        );
        assert_eq!(
            snapshot(&root.join("bars")),
            before,
            "{rel}: the sealed original is untouched"
        );
        println!(
            "SC4 {rel} day={date} full={} held={} offered={} offer={} sealed_bytes={} \
             writer_open_us={open_us:.1} reoffer_us={reoffer_us:.1} refused_us={refused_us:.1} \
             repair_ms={repair_ms:.1} successor=bit-for-bit",
            full.len(),
            held.len(),
            offered.len(),
            offer.as_str(),
            built.file_len()
        );
    }
}

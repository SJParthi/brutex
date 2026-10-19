//! Both forms of a one-second month answer the same bars, at the same p99
//! (FM-228; SC-3, D-4965; store-compression design §5.1 and §5.4).
//!
//! The compressed one-second form (`.sbar`) arrives while raw months exist.
//! Every reader goes through one door that dispatches on the file it finds,
//! so this test asks the PUBLIC door — `BarFile::open_existing` — for the same
//! month in each form, compares every row, page and lookup, and times the
//! page read and the time lookup on warm handles at three month sizes (1x,
//! 10x, 100x). It prints p50, p99 and max for each, so the run is the
//! measurement; it asserts the shape (sealed p99 within a bound of raw p99,
//! and flat with the month's size) with margins wide enough for a loaded
//! machine. Prices are invented and absurd.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_arithmetic
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::file::{BarFile, Form, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const SYMBOL: u32 = 0x5C3_0228;
const MICROS: i64 = 1_000_000;

struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-sealed-door-it-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path(month: u8) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2031, month).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `n` one-second bars, 22,500 a day from 09:15 IST, weekdays and weekends
/// alike (the writer admits any second of the month).
fn seconds(month: u8, n: u64) -> Vec<Bar> {
    let start = YearMonth::new(2031, month).unwrap().ist_bounds_micros().0 + 555 * 60 * MICROS;
    (0..n as i64)
        .map(|k| {
            let (day, s) = (k / 22_500, k % 22_500);
            let close = 5_000_000_000 + (k * 7919) % 50_000;
            Bar {
                ts_micros: start + day * 86_400 * MICROS + s * MICROS,
                open: close - 3,
                high: close + 40,
                low: close - 40,
                close,
                volume: k % 17,
                open_interest: if k % 11 == 0 { 7_000_000 + k } else { OI_NULL },
            }
        })
        .collect()
}

fn write_and_seal(root: &Path, month: u8, bars: &[Bar]) {
    let at = path(month);
    let mut writer = BarFile::open_or_create(root, at, SYMBOL).unwrap();
    writer.append(bars).unwrap();
    drop(writer);
    let raw = BarFile::open_existing(root, at, SYMBOL).unwrap();
    let built = store::sealed::build(at, &raw.header(), bars, None, None).unwrap();
    let sbar = at.with_file(FileKind::SealedBars).to_path_buf(root);
    let staged = sbar.with_extension("staged");
    fs::write(&staged, built.to_bytes()).unwrap();
    // PUBLISHED BY RENAME, as the sealer does; the raw form stays beside it
    // and the door opens the sealed one first.
    fs::rename(&staged, &sbar).unwrap();
}

/// A root holding only the raw form of the same month (the `.sbar` moved
/// out), so the door opens each form through the same call.
fn raw_twin(root: &Path, month: u8, bars: &[Bar]) -> Root {
    let twin = Root::new(&format!("raw-{month}-{}", bars.len()));
    let mut writer = BarFile::open_or_create(&twin.0, path(month), SYMBOL).unwrap();
    writer.append(bars).unwrap();
    drop(writer);
    assert!(root.exists());
    twin
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n.max(1)
    }
}

fn quantiles(mut micros: Vec<f64>) -> (f64, f64, f64) {
    micros.sort_by(f64::total_cmp);
    let at = |q: f64| micros[((micros.len() - 1) as f64 * q).round() as usize];
    (at(0.50), at(0.99), *micros.last().unwrap())
}

/// p50, p99 and max (µs) of `n` random 100-row pages and `n` random time
/// lookups through one warm handle.
fn timed(file: &BarFile, bars: &[Bar], n: usize, seed: u64) -> [(f64, f64, f64); 2] {
    let rows = bars.len() as u64;
    let mut rng = Rng(seed);
    let mut pages = Vec::with_capacity(n);
    let mut lookups = Vec::with_capacity(n);
    for _ in 0..n {
        let first = rng.below(rows.saturating_sub(100));
        let started = Instant::now();
        let page = file.read_range(first, 100.min(rows)).unwrap();
        pages.push(started.elapsed().as_secs_f64() * 1e6);
        assert_eq!(page[0], bars[first as usize]);
        let probe = bars[rng.below(rows) as usize].ts_micros + 1;
        let started = Instant::now();
        let at = file.first_at_or_after(probe).unwrap();
        lookups.push(started.elapsed().as_secs_f64() * 1e6);
        assert!(at == rows || bars[at as usize].ts_micros >= probe);
    }
    [quantiles(pages), quantiles(lookups)]
}

#[test]
fn both_second_formats_answer_the_same_bars_at_p99() {
    const N: usize = 2_000;
    let sizes = [2_250u64, 22_500, 225_000];
    let mut sealed_p99 = Vec::new();
    for (month, &size) in (1u8..).zip(&sizes) {
        let bars = seconds(month, size);
        let root = Root::new(&format!("sealed-{size}"));
        write_and_seal(&root.0, month, &bars);
        let twin = raw_twin(&root.0, month, &bars);

        let sealed = BarFile::open_existing(&root.0, path(month), SYMBOL).unwrap();
        let raw = BarFile::open_existing(&twin.0, path(month), SYMBOL).unwrap();
        assert_eq!(sealed.form(), Form::Sealed);
        assert_eq!(raw.form(), Form::Raw);
        assert_eq!(sealed.time_lookup(), TimeLookup::Sealed);
        assert_eq!(sealed.header(), raw.header(), "{size}: one header");

        // EVERY ROW, BIT FOR BIT, by both doors' page path and by record.
        assert_eq!(sealed.read_range(0, size).unwrap(), bars, "{size}");
        assert_eq!(raw.read_range(0, size).unwrap(), bars, "{size}");
        let mut rng = Rng(size);
        for _ in 0..500 {
            let i = rng.below(size);
            assert_eq!(sealed.read_record(i).unwrap(), raw.read_record(i).unwrap());
            let ts = bars[i as usize].ts_micros - rng.below(3) as i64;
            assert_eq!(
                sealed.first_at_or_after(ts).unwrap(),
                raw.first_at_or_after(ts).unwrap(),
                "{size}: the lookup of {ts}"
            );
        }

        // THE TIMINGS: warm handles, the same random offsets for both.
        let [raw_page, raw_lookup] = timed(&raw, &bars, N, 0x0228);
        let [sealed_page, sealed_lookup] = timed(&sealed, &bars, N, 0x0228);
        println!(
            "FM-228 bars={size} n={N} page µs p50/p99/max raw {:.1}/{:.1}/{:.1} sealed \
             {:.1}/{:.1}/{:.1}; lookup µs raw {:.1}/{:.1}/{:.1} sealed {:.1}/{:.1}/{:.1}",
            raw_page.0,
            raw_page.1,
            raw_page.2,
            sealed_page.0,
            sealed_page.1,
            sealed_page.2,
            raw_lookup.0,
            raw_lookup.1,
            raw_lookup.2,
            sealed_lookup.0,
            sealed_lookup.1,
            sealed_lookup.2
        );
        // WITHIN A BOUND OF THE RAW FORM: the design's pass mark is 2x at the
        // route; a store-level page is a few microseconds, so the margin here
        // is 4x plus 200 µs for a loaded machine's scheduling noise.
        assert!(
            sealed_page.1 <= 4.0 * raw_page.1 + 200.0,
            "{size}: sealed page p99 {:.1} µs against raw {:.1}",
            sealed_page.1,
            raw_page.1
        );
        assert!(
            sealed_lookup.1 <= 4.0 * raw_lookup.1 + 200.0,
            "{size}: sealed lookup p99 {:.1} µs against raw {:.1}",
            sealed_lookup.1,
            raw_lookup.1
        );
        sealed_p99.push((sealed_page.1, sealed_lookup.1));
    }
    // FLAT WITH GROWTH: a hundred times the bars does not move the p99 beyond
    // noise (3x plus 200 µs).
    let (small, large) = (sealed_p99[0], sealed_p99[2]);
    assert!(
        large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
        "p99 grew with the month: {sealed_p99:?}"
    );
}

/// A store path from `bars/<vendor>/<exchange>/<segment>/<symbol>/[<contract>/]<tf>/<yyyy-mm>.bin`.
fn store_path(rel: &str) -> StorePath<'static> {
    let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
    let parts: Vec<&'static str> = rel.split('/').collect();
    assert!(
        parts.len() == 7 || parts.len() == 8,
        "{rel}: not a month path"
    );
    assert_eq!(parts[0], "bars", "{rel}");
    let vendor = Vendor::ALL
        .into_iter()
        .find(|v| v.as_str() == parts[1])
        .unwrap_or_else(|| panic!("{rel}: vendor"));
    let contract = (parts.len() == 8)
        .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
    let tf = parts[parts.len() - 2];
    let timeframe = *Timeframe::KNOWN
        .iter()
        .find(|t| t.as_str() == tf)
        .unwrap_or_else(|| panic!("{rel}: rung"));
    let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
    let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
    StorePath::new(PathParts {
        vendor,
        exchange: parts[2],
        segment: parts[3],
        symbol: parts[4],
        contract,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// SEALS A PRIVATE STORE ROOT for the end-to-end check of design §5.4: every
/// month listed in `BRUTEX_SC3_MONTHS` (one `bars/...bin` path a line) under
/// `BRUTEX_SC3_ROOT` is read through the read door (so a repaired month's
/// SERVED revision is the one sealed), built, and published by rename beside
/// that generation's raw form — the sealer's step in miniature (SC-6 owns the
/// real one). Refuses a root whose `bars` is a link, so it can never write
/// into a store a shadow merely points at. Never run on the real store.
#[test]
#[ignore = "writes .sbar files into BRUTEX_SC3_ROOT, a private APFS clone"]
fn seal_a_private_store_root_for_the_shadow_check() {
    let root = PathBuf::from(std::env::var("BRUTEX_SC3_ROOT").expect("BRUTEX_SC3_ROOT"));
    let list = std::env::var("BRUTEX_SC3_MONTHS").expect("BRUTEX_SC3_MONTHS");
    let bars_dir = root.join("bars");
    assert!(
        !fs::symlink_metadata(&bars_dir)
            .unwrap()
            .file_type()
            .is_symlink(),
        "{}: bars is a link; this seals only a private copy",
        bars_dir.display()
    );
    assert!(
        !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
        "never the real store"
    );
    let (mut months, mut rows, mut raw_bytes, mut sealed_bytes) = (0u64, 0u64, 0u64, 0u64);
    let started = Instant::now();
    for rel in fs::read_to_string(&list).unwrap().lines().map(str::trim) {
        if rel.is_empty() || rel.starts_with('#') {
            continue;
        }
        let at = store_path(rel);
        let symbol = rel.split('/').nth(4).unwrap().to_owned();
        let symbol_id = brutex_core::universe::fnv1a(&symbol) as u32;
        let served = store::generation::served(&root, at).unwrap();
        let served_root = served.root(&root);
        let sbar = at.with_file(FileKind::SealedBars).to_path_buf(&served_root);
        if sbar.exists() {
            println!("SKIP {rel}: already sealed");
            continue;
        }
        let raw = BarFile::open_existing(&root, at, symbol_id).unwrap();
        assert_eq!(raw.form(), Form::Raw, "{rel}");
        let bars = raw.read_range(0, raw.records()).unwrap();
        let prv = fs::read(at.with_file(FileKind::Provenance).to_path_buf(&served_root)).ok();
        let built = store::sealed::build(at, &raw.header(), &bars, prv.as_deref(), None).unwrap();
        let staged = sbar.with_extension("sbar-staged");
        let mut file = fs::File::create_new(&staged).unwrap();
        built.write_to(&mut file).unwrap();
        file.sync_all().unwrap();
        drop(file);
        fs::rename(&staged, &sbar).unwrap();
        let sealed = BarFile::open_existing(&root, at, symbol_id).unwrap();
        assert_eq!(sealed.form(), Form::Sealed, "{rel}");
        assert_eq!(
            sealed.read_range(0, sealed.records()).unwrap(),
            bars,
            "{rel}"
        );
        months += 1;
        rows += bars.len() as u64;
        raw_bytes += fs::metadata(at.to_path_buf(&served_root)).unwrap().len();
        sealed_bytes += built.file_len();
        println!(
            "SEALED {rel} rows={} sealed_bytes={} served={}",
            bars.len(),
            built.file_len(),
            served_root.display()
        );
    }
    println!(
        "SEALED TOTAL months={months} rows={rows} raw_bin_bytes={raw_bytes} \
         sealed_bytes={sealed_bytes} seconds={:.1}",
        started.elapsed().as_secs_f64()
    );
}

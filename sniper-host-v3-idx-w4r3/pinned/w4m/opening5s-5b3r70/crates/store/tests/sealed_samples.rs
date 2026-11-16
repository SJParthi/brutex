#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::print_stdout,
    clippy::too_many_lines
)]
//! SC-2's acceptance on REAL months, which never enter this repository.
//!
//! Each month named in the file `BRUTEX_SEALED_MONTHS` (one `.bin` path a
//! line, relative to the store root `BRUTEX_SEALED_ROOT`) is read, sealed
//! into a `.sbar` under `BRUTEX_SEALED_OUT`, opened from that file and
//! audited, and then every row is compared with the raw month's, read
//! through `BarFile`, and 2,000 random stamps are looked up in both forms.
//! The root must be a PRIVATE copy of the months (an APFS clone): opening a
//! raw month through `BarFile` takes its lock, and nothing here may touch the
//! real store. A run that asks for it without the variables FAILS rather than
//! passes.
//!
//! ```text
//! BRUTEX_SEALED_ROOT=<private clone> BRUTEX_SEALED_MONTHS=<list> BRUTEX_SEALED_OUT=<dir> \
//!   cargo test -p store --release --test sealed_samples -- --ignored --nocapture
//! ```
//!
//! It prints, per month, the sizes and, at n = 2,000 sequential calls each on
//! a warm page cache, p50 / p99 / max of: opening the sealed month (a new
//! file handle and its head); a 100-row page at a random offset, sealed
//! (`SealedMonth::read_rows`) beside raw (100 `BarFile::read_record`, the
//! page path today); one row at a random offset in both forms; and
//! `first_at_or_after` at a random stamp in both forms. It prints and does
//! not gate on time: a shared machine's tail is not this format's.
use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use std::path::{Path, PathBuf};
use std::time::Instant;
use store::file::BarFile;
use store::format::{Bar, HEADER_LEN, RECORD_LEN};
use store::header::Header;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::sealed::{SealedMonth, build};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
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
    let contract = (parts.len() == 8).then(|| Contract::parse(parts[5]).expect("contract"));
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

fn raw_month(file: &Path) -> (Header, Vec<Bar>) {
    let data = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let at = HEADER_LEN as usize;
    let header = Header::read_region(&data[..at], data.len() as u64).unwrap();
    let n = header.n_valid as usize;
    let bars = data[at..at + n * RECORD_LEN]
        .chunks_exact(RECORD_LEN)
        .map(|record| Bar::decode(record).unwrap())
        .collect();
    (header, bars)
}

/// Nanoseconds as microseconds with one decimal, in integers.
fn us(ns: u128) -> String {
    format!("{}.{}", ns / 1_000, ns % 1_000 / 100)
}

fn percentiles(label: &str, mut ns: Vec<u128>) -> String {
    ns.sort_unstable();
    let n = ns.len();
    format!(
        "{label} p50 {} / p99 {} / max {} us (n = {n})",
        us(ns[n / 2]),
        us(ns[n * 99 / 100]),
        us(ns[n - 1])
    )
}

#[test]
#[ignore = "reads real months outside the repository: BRUTEX_SEALED_ROOT, BRUTEX_SEALED_MONTHS, BRUTEX_SEALED_OUT"]
fn real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form() {
    let root = PathBuf::from(std::env::var("BRUTEX_SEALED_ROOT").expect("BRUTEX_SEALED_ROOT"));
    let list = std::env::var("BRUTEX_SEALED_MONTHS").expect("BRUTEX_SEALED_MONTHS");
    let out = PathBuf::from(std::env::var("BRUTEX_SEALED_OUT").expect("BRUTEX_SEALED_OUT"));
    let months: Vec<String> = std::fs::read_to_string(&list)
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    assert!(!months.is_empty(), "{list}: no months");
    let mut rng = Rng(2026);
    for rel in &months {
        let path = store_path(rel);
        let bin = path.to_path_buf(&root);
        let (header, bars) = raw_month(&bin);
        let prv = std::fs::read(path.with_file(FileKind::Provenance).to_path_buf(&root)).ok();
        let started = Instant::now();
        let built = build(path, &header, &bars, prv.as_deref(), None).unwrap();
        let seal_us = started.elapsed().as_micros();
        let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&out);
        std::fs::create_dir_all(sbar.parent().unwrap()).unwrap();
        let mut file = std::fs::File::create(&sbar).unwrap();
        built.write_to(&mut file).unwrap();
        file.sync_all().unwrap();
        drop(file);

        let symbol = header.symbol_id;
        let sealed = SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
        let audited = sealed.audit().unwrap();
        assert_eq!(audited.rows, bars.len() as u64, "{rel}");
        assert_eq!(sealed.head().bar_header(), header, "{rel}");
        assert_eq!(sealed.provenance().unwrap(), prv, "{rel}");
        let raw = BarFile::open_existing(&root, path, symbol).unwrap();
        let n = bars.len() as u64;
        let mut page = Vec::new();
        let mut first = 0;
        while first < n {
            let count = (n - first).min(1_000);
            page.clear();
            sealed.read_rows(first, count, &mut page).unwrap();
            assert_eq!(
                page,
                bars[first as usize..(first + count) as usize],
                "{rel} at {first}"
            );
            first += count;
        }
        for at in (0..n).step_by(1 + n as usize / 500) {
            assert_eq!(
                raw.read_record(at).unwrap(),
                bars[at as usize],
                "{rel} raw {at}"
            );
        }

        let (mut open_ns, mut page_sealed, mut page_raw) = (Vec::new(), Vec::new(), Vec::new());
        let (mut row_sealed, mut row_raw, mut find_sealed, mut find_raw) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let first_ts = bars.first().map_or(0, |bar| bar.ts_micros);
        let last_ts = bars.last().map_or(0, |bar| bar.ts_micros);
        for _ in 0..2_000 {
            let started = Instant::now();
            let month =
                SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
            open_ns.push(started.elapsed().as_nanos());
            drop(month);
            if n == 0 {
                continue;
            }
            let count = n.min(100);
            let at = rng.below(n - count + 1);
            page.clear();
            let started = Instant::now();
            sealed.read_rows(at, count, &mut page).unwrap();
            page_sealed.push(started.elapsed().as_nanos());
            let started = Instant::now();
            for i in at..at + count {
                let _ = raw.read_record(i).unwrap();
            }
            page_raw.push(started.elapsed().as_nanos());
            assert_eq!(page, bars[at as usize..(at + count) as usize]);

            let at = rng.below(n);
            page.clear();
            let started = Instant::now();
            sealed.read_rows(at, 1, &mut page).unwrap();
            row_sealed.push(started.elapsed().as_nanos());
            let started = Instant::now();
            let one = raw.read_record(at).unwrap();
            row_raw.push(started.elapsed().as_nanos());
            assert_eq!(page, [one]);

            let ts = first_ts + rng.below((last_ts - first_ts) as u64 + 1) as i64;
            let started = Instant::now();
            let found = sealed.first_at_or_after(ts).unwrap();
            find_sealed.push(started.elapsed().as_nanos());
            let started = Instant::now();
            let raw_found = raw.first_at_or_after(ts).unwrap();
            find_raw.push(started.elapsed().as_nanos());
            let scanned = bars.partition_point(|bar| bar.ts_micros < ts) as u64;
            assert_eq!(found, scanned, "{rel} ts {ts}");
            assert_eq!(raw_found, scanned, "{rel} raw ts {ts}");
        }
        let raw_len = std::fs::metadata(&bin).unwrap().len();
        println!(
            "{rel}: {n} bars, {} frames, table {}, raw .bin {raw_len} B, .sbar {} B ({} hundredths of a byte a bar, {} per mille of raw), prv {} -> {} B, built in {seal_us} us",
            sealed.head().n_blocks,
            sealed.head().table_entries,
            built.file_len(),
            built.file_len() * 100 / n.max(1),
            built.file_len() * 1_000 / raw_len,
            sealed.head().prv_raw_len,
            sealed.head().prv_packed_len,
        );
        println!("  {}", percentiles("open sealed (file + head)", open_ns));
        if n > 0 {
            println!("  {}", percentiles("100-row page sealed", page_sealed));
            println!("  {}", percentiles("100-row page raw   ", page_raw));
            println!("  {}", percentiles("one row sealed     ", row_sealed));
            println!("  {}", percentiles("one row raw        ", row_raw));
            println!("  {}", percentiles("first_at_or_after sealed", find_sealed));
            println!("  {}", percentiles("first_at_or_after raw   ", find_raw));
        }
    }
}

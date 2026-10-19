//! `cli::stored::load` reads through the one door (SC-3, D-5899).
//!
//! The backtest reads a stored month with `BarFile::open_existing`, and that
//! call is the read door: a month held sealed (`.sbar`), beside its raw form
//! or alone after a retirement, loads the same candles as the raw form, and a
//! damaged sealed month is refused by name and never answered from the `.bin`
//! beside it. Prices are invented and absurd.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::*;
use std::path::PathBuf;
use store::format::Bar;

/// A NEW root per case, named by the process and the clock: nothing is
/// removed first, so no earlier run's month can be appended to.
fn root(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_nanos();
    let p = std::env::temp_dir().join(format!(
        "brutex-cli-stored-sealed-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&p).expect("the store root");
    p
}

/// `n` one-minute bars from 2026-08-03 09:15 IST, two frames of the sealed
/// form when `n` is over 256.
fn bars(n: i64) -> Vec<Bar> {
    let open = 1_785_727_500_000_000_i64;
    (0..n)
        .map(|i| Bar {
            ts_micros: open + i * 60_000_000,
            open: 9_000_001 + i,
            high: 9_000_301 + i,
            low: 8_999_701 + i,
            close: 9_000_151 + i,
            volume: 0,
            open_interest: i64::MIN,
        })
        .collect()
}

/// The instrument every case stores; a path borrows its key, so it lives for
/// the whole run.
static NIFTY: std::sync::LazyLock<InstrumentKey> = std::sync::LazyLock::new(|| {
    InstrumentKey::index(Exchange::Nse, "NIFTY").expect("a stored index")
});

fn month() -> StorePath<'static> {
    let ym = YearMonth::new(2026, 8).expect("a real month");
    StorePath::for_key(
        Vendor::Zerodha,
        &NIFTY,
        Timeframe::MINUTE_1,
        ym,
        FileKind::Bars,
    )
    .expect("a path for a stored index")
}

/// Writes the raw month and returns the bytes of its sealed form.
fn write_raw_and_seal(root: &Path, rows: &[Bar]) -> Vec<u8> {
    let id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let mut file = BarFile::open_or_create(root, month(), id).expect("a fresh month opens");
    file.append(rows).expect("and takes its bars");
    drop(file);
    let raw = BarFile::open_existing(root, month(), id).expect("the raw month opens");
    store::sealed::build(month(), &raw.header(), rows, None, None)
        .expect("the month seals")
        .to_bytes()
}

fn place_sealed(root: &Path, bytes: &[u8]) -> PathBuf {
    let sbar = month().with_file(FileKind::SealedBars).to_path_buf(root);
    std::fs::create_dir_all(sbar.parent().expect("a month directory")).expect("its directory");
    std::fs::write(&sbar, bytes).expect("the sealed form is written");
    sbar
}

fn load_it(root: &Path) -> Result<Loaded, String> {
    load(root, Vendor::Zerodha, "NIFTY", "1min", 2026, 8)
}

#[test]
fn the_backtest_load_reads_a_sealed_month_through_the_door() {
    let rows = bars(300);

    // RAW ONLY: the answer the other two forms must give.
    let raw_root = root("raw");
    let sealed_bytes = write_raw_and_seal(&raw_root, &rows);
    let raw = load_it(&raw_root).expect("the raw month loads");
    assert_eq!(raw.bars.len(), 300, "every record is read");

    // BOTH FORMS: the door opens the `.sbar` first and answers the same.
    let both_root = root("both");
    let _ = write_raw_and_seal(&both_root, &rows);
    let sbar = place_sealed(&both_root, &sealed_bytes);
    let both = load_it(&both_root).expect("a month in both forms loads");
    let opened = BarFile::open_existing(
        &both_root,
        month(),
        brutex_core::universe::fnv1a("NIFTY") as u32,
    )
    .expect("the door opens it");
    assert_eq!(
        opened.form(),
        store::file::Form::Sealed,
        "{}",
        sbar.display()
    );
    assert_eq!(
        both.bars, raw.bars,
        "the sealed form loads the raw form's candles"
    );
    assert_eq!(both.excluded, raw.excluded);
    assert_eq!(both.key, raw.key);

    // SEALED ALONE (after a retirement): no `.bin`, the same candles.
    let alone_root = root("alone");
    let _ = place_sealed(&alone_root, &sealed_bytes);
    assert!(
        !month().to_path_buf(&alone_root).exists(),
        "this root holds no raw form"
    );
    let alone = load_it(&alone_root).expect("a month held only sealed loads");
    assert_eq!(
        alone.bars, raw.bars,
        "the sealed form alone loads the same candles"
    );
}

#[test]
fn a_damaged_sealed_month_is_refused_by_name_and_never_loaded_from_the_bin() {
    let rows = bars(300);
    let damaged_root = root("damaged");
    let mut bytes = write_raw_and_seal(&damaged_root, &rows);
    // A BYTE OF THE LAST FRAME: the head opens, rows 256..300 are refused.
    let last = bytes.len() - 3;
    bytes[last] ^= 0x10;
    let sbar = place_sealed(&damaged_root, &bytes);
    assert!(
        month().to_path_buf(&damaged_root).exists(),
        "the intact raw form is beside it"
    );
    let why = load_it(&damaged_root).expect_err("a damaged sealed month is refused");
    assert!(why.contains("sealed month is refused"), "{why}");
    assert!(why.contains(&sbar.display().to_string()), "{why}");
    assert!(why.contains("could not be read"), "{why}");
}

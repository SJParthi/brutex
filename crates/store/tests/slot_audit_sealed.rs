//! R4-09a, D-6243: a month served in its SEALED form (`.sbar`) has no header
//! slots, so `BarFile::slot_audit` is `None` for it, no
//! `store.header.slot_damaged` line can come from it, and a damaged sealed head
//! is refused rather than read at some older state.
//!
//! The sealed door (SC-3) opens the `.sbar` first and builds the handle from
//! its one head (`sealed::Head::bar_header`). That head is under its own
//! checksum and there is no second copy to fall back to, so the silent revert
//! `slot_audit` names in the raw form has no way to happen here. These tests
//! hold both halves of that claim: the handle says "no slots" rather than
//! inventing a clean pair, and a damaged head is a refusal.
//!
//! Every bar is invented: a 2031 month closing at seventy rupees, a price NIFTY
//! has never traded at.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::{BarFile, Form};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const SYMBOL: u32 = 0x0049_0943;
const MICROS: i64 = 1_000_000;

/// A scratch root of its own, made fresh under the system temp directory.
fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-r409a-sealed-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&root).expect("a scratch root");
    root
}

/// The invented month: GDFL NSE INDEX NIFTY one-second, March 2031.
fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2031, 3).expect("a month"),
        file: FileKind::Bars,
    })
    .expect("a store path")
}

/// `n` invented one-second bars from 09:15 IST on the month's first day.
fn bars(n: i64) -> Vec<Bar> {
    let start = YearMonth::new(2031, 3)
        .expect("a month")
        .ist_bounds_micros()
        .0
        + 555 * 60 * MICROS;
    (0..n)
        .map(|k| Bar {
            ts_micros: start + k * MICROS,
            open: 7_000 + k,
            high: 7_100 + k,
            low: 6_900 + k,
            close: 7_000 + k,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect()
}

/// Writes the raw month in two appends, then publishes its sealed form beside
/// it by rename, as the sealer does. Returns the `.sbar` path.
fn write_and_seal(root: &Path, batch: &[Bar]) -> PathBuf {
    let at = month();
    let (first, second) = batch.split_at(batch.len() / 2);
    let mut writer = BarFile::open_or_create(root, at, SYMBOL).expect("open");
    let _ = writer.append(first).expect("append 1");
    let _ = writer.append(second).expect("append 2");
    drop(writer);
    let raw = BarFile::open_existing(root, at, SYMBOL).expect("the raw month");
    assert_eq!(raw.form(), Form::Raw);
    assert!(
        raw.slot_audit().is_some_and(|audit| audit.is_clean()),
        "the raw form has its two slots audited: {:?}",
        raw.slot_audit()
    );
    let built = store::sealed::build(at, &raw.header(), batch, None, None).expect("sealed");
    drop(raw);
    let sbar = at.with_file(FileKind::SealedBars).to_path_buf(root);
    let staged = sbar.with_extension("staged");
    std::fs::write(&staged, built.to_bytes()).expect("staged");
    std::fs::rename(&staged, &sbar).expect("published");
    sbar
}

/// THE SEALED FORM HAS NO SLOTS, AND SAYS SO. The handle opened through the
/// sealed door carries `None`, not a made-up clean pair, and reads every
/// record the raw form held.
#[test]
fn a_month_opened_in_its_sealed_form_has_no_slot_audit() {
    let root = scratch("none");
    let batch = bars(40);
    let _ = write_and_seal(&root, &batch);

    let sealed = BarFile::open_existing(&root, month(), SYMBOL).expect("the sealed month");
    assert_eq!(
        sealed.form(),
        Form::Sealed,
        "the door opens the sealed form first"
    );
    assert_eq!(
        sealed.slot_audit(),
        None,
        "a sealed month has one head and no slots to audit"
    );
    assert_eq!(sealed.header().n_valid, 40);
    let last = sealed.read_record(39).expect("the last record");
    assert_eq!(last.close, 7_039);
    let _ = std::fs::remove_dir_all(&root);
}

/// A DAMAGED SEALED HEAD IS REFUSED, NEVER READ AT AN OLDER STATE. Every byte
/// of the head's first 64 is flipped in turn on a fresh copy; each open either
/// refuses or, when the byte is one the head does not cover, reads exactly the
/// month that was sealed. None returns fewer records or other bars.
#[test]
fn a_damaged_sealed_head_is_refused_never_read_short() {
    let root = scratch("head");
    let batch = bars(40);
    let sbar = write_and_seal(&root, &batch);
    let whole = std::fs::read(&sbar).expect("the sealed bytes");
    let mut refused = 0_u32;
    for at in 0..64.min(whole.len()) {
        let mut damaged = whole.clone();
        damaged[at] ^= 0x01;
        std::fs::write(&sbar, &damaged).expect("damaged");
        match BarFile::open_existing(&root, month(), SYMBOL) {
            Err(_) => refused += 1,
            Ok(file) => {
                assert_eq!(file.form(), Form::Sealed, "byte {at}");
                assert_eq!(file.slot_audit(), None, "byte {at}");
                assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
                if let Ok(last) = file.read_record(39) {
                    assert_eq!(last.close, 7_039, "byte {at}");
                }
            }
        }
    }
    std::fs::write(&sbar, &whole).expect("restored");
    assert!(
        refused > 0,
        "a flipped byte in the sealed head is refused by name, not served"
    );
    let _ = std::fs::remove_dir_all(&root);
}

//! A byte-for-byte fingerprint of what the reader decodes out of the **real**
//! lake, so that a refusal added to the reader can be shown not to fire on
//! sound data.
//!
//! # Why a digest rather than assertions on values
//!
//! `tests/real_lake.rs` asserts named values out of two known files, which
//! proves the decode is *correct*. This file answers a different question: did
//! a change to the reader alter what it decodes anywhere across a wide sample?
//! Every field of every bar — the paisa integers, the open-interest sentinel,
//! the eight `f64` greeks by their exact bit pattern, the provenance id — is
//! folded into one 64-bit FNV-1a digest per file. A change that alters one
//! gamma in one row of one file moves that file's digest and no other.
//!
//! The expected digests are **not** hardcoded: they are a property of the
//! operator's 40 GB of data, not of this crate, and `CLAUDE.md` §3 rule 1
//! forbids writing down a number nobody measured. What is asserted here is
//! that every sampled file decodes with no refusal at all. The digests are
//! *printed*, so a before-and-after comparison of two `--nocapture` runs is a
//! diff rather than an opinion.
//!
//! # Why this can be skipped
//!
//! The same reason `tests/real_lake.rs` can: the fixture is `~/.brutex/lake`
//! and it cannot be committed. On a machine without it this test prints that
//! it is skipping and proves nothing. That is stated rather than hidden.

// The same exceptions every test module in this workspace takes: a test that
// cannot panic cannot fail, and the lints that forbid panicking exist to keep
// them out of the READER, not out of its tests.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use lake::reader::LakeFile;
use lake::schema::Layout;

/// How many real files the sample must reach before it is worth believing.
const MINIMUM_FILES: usize = 250;

/// FNV-1a 64, offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64, prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Folds eight bytes into a running FNV-1a digest.
fn eat(mut h: u64, v: u64) -> u64 {
    for b in v.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Folds an optional value, distinguishing absent from every present value.
fn eat_opt(h: u64, v: Option<u64>) -> u64 {
    match v {
        Some(v) => eat(eat(h, 1), v),
        None => eat(h, 0),
    }
}

/// The digest of everything one file decodes to, and how many rows that was.
///
/// Every field enters, including the ones a careless digest would drop: the
/// open-interest sentinel enters as the raw `i64` so an invented null is
/// visible, and each greek enters by `to_bits` so no rounding can hide.
fn digest_file(path: &Path) -> Result<(u64, usize, Layout), String> {
    let file = LakeFile::open(path).map_err(|e| e.to_string())?;
    let layout = file.layout();
    let mut h = eat(FNV_OFFSET, u64::from(u8::from(layout.has_greeks())));
    let mut rows = 0_usize;

    for g in 0..file.row_groups() {
        let batch = file
            .read_row_group(g)
            .map_err(|e| format!("group {g}: {e}"))?;
        for bar in batch.iter() {
            #[expect(
                clippy::cast_sign_loss,
                reason = "the digest wants the bit pattern, not the magnitude"
            )]
            {
                h = eat(h, bar.timestamp_micros as u64);
                h = eat(h, bar.open.raw() as u64);
                h = eat(h, bar.high.raw() as u64);
                h = eat(h, bar.low.raw() as u64);
                h = eat(h, bar.close.raw() as u64);
                h = eat(h, bar.volume as u64);
                // The RAW field, sentinel and all: an invented null is
                // i64::MIN here and it must move the digest.
                h = eat(h, bar.open_interest as u64);
                h = eat_opt(h, bar.spot_at_bar.map(|p| p.raw() as u64));
                h = eat_opt(h, bar.greeks_provenance_id.map(|v| i64::from(v) as u64));
            }
            match bar.greeks {
                Some(g) => {
                    h = eat(h, 1);
                    for v in [
                        g.iv,
                        g.delta,
                        g.gamma,
                        g.theta,
                        g.vega,
                        g.rho,
                        g.t_years_used,
                        g.rate_used,
                    ] {
                        h = eat(h, v.to_bits());
                    }
                }
                None => h = eat(h, 0),
            }
            rows += 1;
        }
    }
    Ok((h, rows, layout))
}

/// Every `<year>/<month>.parquet` under one contract directory, sorted.
fn month_files(contract: &Path, into: &mut Vec<PathBuf>) {
    let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
        return;
    };
    let mut ys: Vec<PathBuf> = years.flatten().map(|e| e.path()).collect();
    ys.sort();
    for y in ys {
        let Ok(months) = std::fs::read_dir(&y) else {
            continue;
        };
        let mut ms: Vec<PathBuf> = months.flatten().map(|e| e.path()).collect();
        ms.sort();
        into.extend(ms);
    }
}

/// A stride sample of one directory's entries, in sorted order so that two
/// runs on the same machine choose the same files.
fn sample_dirs(root: &Path, want: usize) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut all: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    all.sort();
    if all.is_empty() || want == 0 {
        return Vec::new();
    }
    let stride = (all.len() / want).max(1);
    all.into_iter().step_by(stride).take(want).collect()
}

#[test]
fn a_wide_sample_of_the_real_lake_decodes_with_no_refusal_and_a_stable_digest() {
    let Some(home) = std::env::var_os("HOME") else {
        println!("SKIPPING the real-lake digest: no HOME.");
        return;
    };
    let root = Path::new(&home).join(".brutex/lake/bars/NSE");
    if !root.exists() {
        println!("SKIPPING the real-lake digest: ~/.brutex/lake is not on this machine.");
        println!("  The fixture cannot be tracked — CI gate 1 forbids a .parquet in");
        println!("  this repository — so this test proves nothing here.");
        return;
    }

    let mut targets: Vec<PathBuf> = Vec::new();
    for contract in sample_dirs(&root.join("FNO"), 120) {
        month_files(&contract, &mut targets);
    }
    for symbol in sample_dirs(&root.join("CASH"), 20) {
        month_files(&symbol, &mut targets);
    }
    for symbol in sample_dirs(&root.join("INDEX"), 5) {
        month_files(&symbol, &mut targets);
    }
    targets.sort();

    let mut refused: Vec<String> = Vec::new();
    let mut files = 0_usize;
    let mut rows = 0_usize;
    let mut fno = 0_usize;
    let mut cash = 0_usize;
    let mut whole = FNV_OFFSET;

    for path in &targets {
        let shown = path.strip_prefix(&root).unwrap_or(path).display();
        match digest_file(path) {
            Ok((h, n, layout)) => {
                println!("{h:016x} {n:>6} {shown}");
                whole = eat(whole, h);
                files += 1;
                rows += n;
                match layout {
                    Layout::Fno => fno += 1,
                    Layout::Cash => cash += 1,
                }
            }
            Err(e) => refused.push(format!("{shown}: {e}")),
        }
    }

    println!("REAL LAKE DIGEST");
    println!("  files    : {files}");
    println!("  rows     : {rows}");
    println!("  F&O      : {fno}, cash/index: {cash}");
    println!("  refusals : {}", refused.len());
    println!("  WHOLE    : {whole:016x}");
    for line in refused.iter().take(20) {
        println!("    REFUSED {line}");
    }

    assert!(
        refused.is_empty(),
        "a refusal that fires on sound data is a regression:\n  {}",
        refused.join("\n  ")
    );
    assert!(
        files >= MINIMUM_FILES,
        "the sample must reach {MINIMUM_FILES} files to be worth believing, got {files}"
    );
    assert!(
        fno > 0 && cash > 0,
        "both layouts must appear in the sample"
    );
    assert!(rows > 0);
}

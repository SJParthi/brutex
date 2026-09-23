//! Decodes the **real** lake and asserts values read out of it.
//!
//! # Why this test can be skipped, and why that is stated rather than hidden
//!
//! The fixture is `~/.brutex/lake`, 40 GB of operator data. It cannot be
//! committed: CI gate 1 allows exactly `.rs .toml .md .lock .html .css .yml`
//! outside `web/`, so a tracked `.parquet` is a build failure by design, and
//! 40 GB would not belong in git regardless.
//!
//! So when the lake is absent — which is every CI runner — these tests print
//! that they are skipping and why. **They therefore prove nothing on CI.** The
//! decode path that CI does exercise is `synthetic.rs`, which writes its own
//! Parquet files covering the footer, the schema check, the definition levels,
//! null expansion and the paisa conversion — and which now also recompresses
//! those files with `ruzstd`'s own encoder, so the ZSTD page path is driven end
//! to end on every run. What only this file can cover is ZSTD frames as
//! Polars' encoder wrote them and the actual recorded values, because both are
//! properties of the operator's data rather than of this crate.
//!
//! That gap is real and is written down here rather than left for someone to
//! discover from a green tick.

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

use lake::contract::{ContractKind, ContractName};
use lake::reader::LakeFile;
use lake::schema::Layout;

/// The sample F&O file, verified by hand: `PAR1` at both ends, a ZSTD frame at
/// offset 0x19, written by Polars, 2,480 rows in one row group.
const SAMPLE_FNO: &str =
    ".brutex/lake/bars/NSE/FNO/NSE-NIFTY-01Apr20-10000-CE/1minute/2020/03.parquet";

/// A cash file, for the seven-column layout.
const SAMPLE_CASH: &str = ".brutex/lake/bars/NSE/CASH/360ONE/1minute/2023/01.parquet";

/// The lake root, if this machine has one.
fn lake_path(rel: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let p = Path::new(&home).join(rel);
    p.exists().then_some(p)
}

/// Prints the skip reason once, so an absent lake is visible in the log.
fn skipped(what: &str) {
    println!("SKIPPING {what}: ~/.brutex/lake is not on this machine.");
    println!("  The fixture cannot be tracked — CI gate 1 forbids a .parquet in");
    println!("  this repository — so this test proves nothing here. The decode");
    println!("  path is covered by tests/synthetic.rs, which writes its own.");
}

#[test]
#[ignore = "needs ~/.brutex/lake, which cannot be tracked: CI gate 1 forbids a .parquet in this repository. Run with `cargo test -p lake -- --ignored` on a machine that has the lake."]
fn the_real_fno_sample_decodes_to_the_values_it_holds() {
    let Some(path) = lake_path(SAMPLE_FNO) else {
        panic!(
            "MISSING FIXTURE {SAMPLE_FNO} for the real F&O sample: this test is #[ignore]d because \
             the fixture cannot be tracked, so running it explicitly means you believe \
             you have ~/.brutex/lake. Reporting `ok` here would be a test that \
             asserted nothing."
        );
    };

    let f = LakeFile::open(&path).expect("the real sample file must open");
    assert_eq!(f.layout(), Layout::Fno, "17 columns with greeks");
    assert_eq!(f.created_by(), Some("Polars"));
    assert_eq!(f.num_rows(), 2_480);
    assert_eq!(f.row_groups(), 1);

    let b = f.read_row_group(0).expect("the ZSTD pages must decode");
    assert_eq!(b.len(), 2_480);

    // Row 0, read out of the file byte for byte. 2020-03-20 10:38 IST, NIFTY
    // at 8473.1 in the COVID crash, a deep-OTM 10000 CE at 49.50 rupees.
    let r = b.row(0).expect("row 0");
    assert_eq!(r.timestamp_micros, 1_584_685_680_000_000);

    // Prices: 49.5 rupees is 4,950 paisa, exactly.
    assert_eq!(r.open.raw(), 4_950);
    assert_eq!(r.high.raw(), 4_950);
    assert_eq!(r.low.raw(), 4_950);
    assert_eq!(r.close.raw(), 4_950);
    assert_eq!(r.volume, 76);
    assert_eq!(r.open_interest(), Some(75));

    // The recorded spot is money too: 8473.1 rupees is 847,310 paisa.
    assert_eq!(r.spot_at_bar.expect("spot").raw(), 847_310);
    assert_eq!(r.greeks_provenance_id, Some(1_380_610));

    // The greeks keep every bit the file holds.
    let g = r.greeks.expect("row 0 has greeks");
    assert_eq!(g.iv.to_bits(), 0.678_618_464_061_773_3_f64.to_bits());
    assert_eq!(g.delta.to_bits(), 0.103_791_597_602_742_98_f64.to_bits());
    assert_eq!(
        g.gamma.to_bits(),
        0.000_171_426_804_295_494_02_f64.to_bits()
    );
    assert_eq!(g.theta.to_bits(), (-7.788_961_946_081_791_f64).to_bits());
    assert_eq!(g.vega.to_bits(), 2.791_593_074_801_929_3_f64.to_bits());
    assert_eq!(g.rho.to_bits(), 0.276_837_206_382_207_27_f64.to_bits());
    assert_eq!(
        g.t_years_used.to_bits(),
        0.033_280_060_882_800_61_f64.to_bits()
    );
    assert_eq!(g.rate_used.to_bits(), 0.065_f64.to_bits());

    // Every row decodes, and the whole group is walkable.
    assert_eq!(b.iter().count(), 2_480);
}

#[test]
#[ignore = "needs ~/.brutex/lake, which cannot be tracked: CI gate 1 forbids a .parquet in this repository. Run with `cargo test -p lake -- --ignored` on a machine that has the lake."]
fn the_real_cash_sample_has_the_seven_column_layout() {
    let Some(path) = lake_path(SAMPLE_CASH) else {
        panic!(
            "MISSING FIXTURE {SAMPLE_CASH} for the real cash sample: this test is #[ignore]d because \
             the fixture cannot be tracked, so running it explicitly means you believe \
             you have ~/.brutex/lake. Reporting `ok` here would be a test that \
             asserted nothing."
        );
    };

    let f = LakeFile::open(&path).expect("open");
    assert_eq!(f.layout(), Layout::Cash, "no greeks in a cash file");
    assert_eq!(f.num_rows(), 2_143);

    let b = f.read_row_group(0).expect("decode");
    let r = b.row(0).expect("row 0");
    assert_eq!(r.timestamp_micros, 1_674_445_500_000_000);
    assert_eq!(r.open.raw(), 195_000); // 1950.00
    assert_eq!(r.high.raw(), 195_000);
    assert_eq!(r.low.raw(), 193_200); // 1932.00
    assert_eq!(r.close.raw(), 194_300); // 1943.00
    assert_eq!(r.volume, 2_663);

    // Open interest is null for a cash series, and that is not zero.
    assert_eq!(r.open_interest(), None);
    assert_eq!(r.open_interest, i64::MIN);

    // A cash bar carries no greeks at all — the columns are not in the file.
    assert!(!r.has_greeks());
    assert_eq!(r.spot_at_bar, None);
    assert_eq!(r.greeks_provenance_id, None);
}

#[test]
#[ignore = "needs ~/.brutex/lake, which cannot be tracked: CI gate 1 forbids a .parquet in this repository. Run with `cargo test -p lake -- --ignored` on a machine that has the lake."]
fn a_real_contract_directory_name_parses_and_round_trips() {
    let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
        skipped("the contract directory walk");
        return;
    };

    let mut seen = 0_usize;
    let mut futures = 0_usize;
    let entries = std::fs::read_dir(&root).expect("read the F&O directory");
    for e in entries.take(500) {
        let e = e.expect("entry");
        let name = e.file_name();
        let name = name.to_str().expect("a lake name is utf-8");

        let parsed = ContractName::parse(name)
            .unwrap_or_else(|err| panic!("real lake directory {name} was refused: {err}"));
        assert_eq!(
            parsed.to_string(),
            name,
            "round trip must reproduce the lake's own spelling"
        );
        if parsed.kind() == ContractKind::Future {
            futures += 1;
        }
        seen += 1;
    }
    assert!(seen > 0, "the F&O directory must not be empty");
    println!("parsed {seen} real contract names ({futures} futures), all round-tripped");
}

#[test]
#[ignore = "needs ~/.brutex/lake, which cannot be tracked: CI gate 1 forbids a .parquet in this repository. Run with `cargo test -p lake -- --ignored` on a machine that has the lake."]
fn a_spread_of_real_files_decodes_with_no_failures() {
    let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
        skipped("the multi-file decode");
        return;
    };

    let mut files = 0_usize;
    let mut rows = 0_usize;
    let mut null_oi = 0_usize;
    let mut real_oi = 0_usize;
    let mut with_greeks = 0_usize;

    let dirs = std::fs::read_dir(&root).expect("read dir");
    for d in dirs.take(40) {
        let d = d.expect("entry").path();
        let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
            continue;
        };
        for y in years {
            let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
                continue;
            };
            for m in months {
                let p = m.expect("month").path();
                let f = LakeFile::open(&p)
                    .unwrap_or_else(|e| panic!("real lake file {} refused: {e}", p.display()));
                for g in 0..f.row_groups() {
                    let b = f
                        .read_row_group(g)
                        .unwrap_or_else(|e| panic!("{} group {g} refused: {e}", p.display()));
                    for bar in b.iter() {
                        rows += 1;
                        if bar.open_interest().is_none() {
                            null_oi += 1;
                        } else {
                            real_oi += 1;
                        }
                        if bar.has_greeks() {
                            with_greeks += 1;
                        }
                    }
                }
                files += 1;
            }
        }
    }

    assert!(files > 0, "at least one real file must have been read");
    println!("decoded {files} real files, {rows} rows, 0 failures");
    println!("  open interest: {real_oi} real, {null_oi} null");
    println!("  rows carrying greeks: {with_greeks}");

    // The lake genuinely contains both, which is what makes the sentinel
    // load-bearing rather than decorative.
    assert!(rows > 0);
    assert!(null_oi > 0, "the sample must contain a null open interest");
    assert!(real_oi > 0, "and a real one");
}

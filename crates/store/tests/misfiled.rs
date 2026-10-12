//! A bar month filed under another name is refused at every bar door —
//! DB-B65, WP-81.
//!
//! The bar header's identity is the low 32 bits of the FNV-1a of the SYMBOL
//! text. The NIFTY index, every NIFTY option contract and both vendors carry
//! one id, so a month copied by hand to a sibling strike opened cleanly and
//! was served as the other instrument. The `.prv` header (D-3334) binds itself
//! to the whole store path; these tests copy real month files between names
//! and assert each door refuses them, by name, on that evidence — and say
//! plainly what a month with no `.prv` still cannot be refused for.
//!
//! Every bar is invented: prices no index or option ever printed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::{BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::provenance::{self, Day, Landing, Source, SourceKind, WrittenFor};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-misfiled-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
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

/// The header cross-check every writer of the store folds: the low 32 bits of
/// the FNV-1a of the symbol text. One id for the index and every contract.
fn symbol_id(symbol: &str) -> u32 {
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    u32::from_le_bytes(hash[..4].try_into().unwrap())
}

/// One month of one series.
fn series(
    vendor: Vendor,
    segment: &'static str,
    contract: Option<&str>,
    timeframe: Timeframe,
    m: u8,
) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor,
        exchange: "NSE",
        segment,
        symbol: "NIFTY",
        contract: contract.map(|text| Contract::parse(text).expect("a contract")),
        timeframe,
        month: YearMonth::new(2019, m).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// The 12000 call of the 28 Nov 2019 expiry, one-second bars, November.
fn call() -> StorePath<'static> {
    series(
        Vendor::Gdfl,
        "FNO",
        Some("2019-11-28-1200000-CE"),
        Timeframe::SECOND_1,
        11,
    )
}

/// Its sibling strike.
fn sibling() -> StorePath<'static> {
    series(
        Vendor::Gdfl,
        "FNO",
        Some("2019-11-28-1250000-CE"),
        Timeframe::SECOND_1,
        11,
    )
}

/// `n` invented bars of `step` seconds from 09:15 IST on day `dom` of 2019-`m`.
fn bars(m: u8, dom: u8, step: i64, n: i64) -> Vec<Bar> {
    let day = i64::from(days_from_epoch(2019, m, dom));
    let open = day * 86_400_000_000 - 19_800_000_000 + 33_300_000_000;
    (0..n)
        .map(|k| Bar {
            ts_micros: open + k * step * 1_000_000,
            open: 7_777_700 + k,
            high: 7_777_900 + k,
            low: 7_777_500 + k,
            close: 7_777_800 + k,
            volume: 3,
            open_interest: OI_NULL,
        })
        .collect()
}

/// Days from 1970-01-01 to a civil date (Howard Hinnant's `days_from_civil`).
fn days_from_epoch(year: i32, month: u8, day: u8) -> i32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i32::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i32::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Writes `rows` to `path` and records the landing, so the month has a `.prv`.
fn landed(root: &Root, path: StorePath<'_>, rows: &[Bar]) {
    let mut file = BarFile::open_or_create(&root.0, path, symbol_id("NIFTY")).unwrap();
    let appended = file.append(rows).unwrap();
    let landing = Landing {
        source: Source {
            kind: SourceKind::ArchiveMember,
            name: "2019.zip!D28.zip!INVENTED.csv",
            bytes: 4_242,
            crc32: 0x0BAD_F00D,
        },
        day: Day::new(2019, path.month().month(), 4).unwrap(),
        bar_definition: 2,
        journal_ordinal: 7,
        commit: None,
        landed_at_micros: 1_791_547_200_000_000,
    };
    provenance::record(&root.0, &file, path, appended, &landing).unwrap();
}

/// Copies every sibling of the month `from` that exists to the month `to`,
/// except the kinds in `without`: what a restore or a copy by hand does.
fn copy_month(root: &Root, from: StorePath<'_>, to: StorePath<'_>, without: &[FileKind]) {
    for kind in [
        FileKind::Bars,
        FileKind::Checksums,
        FileKind::TimeIndex,
        FileKind::Lock,
        FileKind::Provenance,
    ] {
        if without.contains(&kind) {
            continue;
        }
        let source = from.with_file(kind).to_path_buf(&root.0);
        if !source.exists() {
            continue;
        }
        let target = to.with_file(kind).to_path_buf(&root.0);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(&source, &target).unwrap();
    }
}

fn misfiled(refusal: &StoreError) -> (&Path, &Path, &'static str) {
    match refusal {
        StoreError::Misfiled {
            path,
            sidecar,
            field,
        } => (path, sidecar, field),
        other => panic!("expected Misfiled, found {other:?}"),
    }
}

/// **A CONTRACT MONTH COPIED TO A SIBLING STRIKE IS REFUSED BY NAME, AT EVERY
/// DOOR (DB-B65).** Before, it opened: the two contracts carry one symbol id.
/// The reader's door, the writer's door, the index-rebuild door and the
/// audited door each refuse it, naming the bar file asked for, the sidecar
/// that says otherwise, and what is not this month's. The month it was copied
/// FROM still opens.
#[test]
fn a_contract_month_copied_to_a_sibling_strike_is_refused_at_every_door() {
    let root = Root::new();
    let id = symbol_id("NIFTY");
    landed(&root, call(), &bars(11, 4, 1, 200));
    assert_eq!(
        provenance::written_for(&root.0, call()),
        WrittenFor::ThisMonth
    );
    copy_month(&root, call(), sibling(), &[]);

    let refused = BarFile::open_existing(&root.0, sibling(), id).expect_err("misfiled");
    let (path, sidecar, field) = misfiled(&refused);
    assert_eq!(path, sibling().to_path_buf(&root.0));
    assert_eq!(
        sidecar,
        sibling()
            .with_file(FileKind::Provenance)
            .to_path_buf(&root.0)
    );
    assert_eq!(field, "store path");
    let said = refused.to_string();
    assert!(
        said.contains("2019-11-28-1250000-CE")
            && said.contains("its store path is not this month's")
            && said.contains("DB-B65"),
        "{said}"
    );

    let writer = BarFile::open_or_create(&root.0, sibling(), id).expect_err("no append");
    assert_eq!(misfiled(&writer).2, "store path");
    let reindex = BarFile::index_existing(&root.0, sibling(), id).expect_err("no index");
    assert_eq!(misfiled(&reindex).2, "store path");
    let audited = BarFile::open_existing_audited(&root.0, sibling(), id, u64::MAX)
        .err()
        .expect("not audited");
    assert!(
        audited.contains("its store path is not this month's"),
        "{audited}"
    );

    let original = BarFile::open_existing(&root.0, call(), id).expect("the source month");
    assert_eq!(original.records(), 200);
}

/// **EVERY PART OF THE NAME IS BOUND: THE INDEX, THE VENDOR, THE BAR LENGTH,
/// THE MONTH.** The audit's five files all carried id 433894009: the GDFL
/// index 1s month, two GDFL option contracts, and the Zerodha index minute and
/// day months. Each copy below is one of those confusions.
#[test]
fn a_month_copied_across_the_index_the_vendor_the_rung_or_the_month_is_refused() {
    let root = Root::new();
    let id = symbol_id("NIFTY");
    let index = series(Vendor::Gdfl, "INDEX", None, Timeframe::SECOND_1, 11);
    landed(&root, index, &bars(11, 4, 1, 90));

    // The index's month filed as an option contract's.
    copy_month(&root, index, call(), &[]);
    let refused = BarFile::open_existing(&root.0, call(), id).expect_err("index as contract");
    assert_eq!(misfiled(&refused).2, "store path");

    // GDFL's month filed under Zerodha.
    let other_vendor = series(Vendor::Zerodha, "INDEX", None, Timeframe::SECOND_1, 11);
    copy_month(&root, index, other_vendor, &[]);
    let refused = BarFile::open_existing(&root.0, other_vendor, id).expect_err("vendor");
    assert_eq!(misfiled(&refused).2, "vendor");

    // A minute month filed as a day month: the sidecar's refusal comes first,
    // before the bar header's own bar-length check is reached.
    let minute = series(Vendor::Zerodha, "INDEX", None, Timeframe::MINUTE_1, 11);
    landed(&root, minute, &bars(11, 4, 60, 30));
    let daily = series(Vendor::Zerodha, "INDEX", None, Timeframe::DAY_1, 11);
    copy_month(&root, minute, daily, &[]);
    let refused = BarFile::open_existing(&root.0, daily, id).expect_err("rung");
    assert_eq!(misfiled(&refused).2, "timeframe");

    // November filed as December.
    let december = series(Vendor::Gdfl, "INDEX", None, Timeframe::SECOND_1, 12);
    copy_month(&root, index, december, &[]);
    let refused = BarFile::open_existing(&root.0, december, id).expect_err("month");
    assert_eq!(misfiled(&refused).2, "month");
}

/// **WHAT THE SIDECAR CANNOT SAY, IT DOES NOT REFUSE — AND THAT LIMIT IS
/// EXACT.** A month with no `.prv` (every month written before D-3334, or a
/// copy made without its sidecar) is bound by its symbol hash alone: it opens,
/// and `written_for` says `Unrecorded` so a scrub can count it. A sidecar that
/// fails its own checksum, is unborn, or is a link is no evidence either: the
/// bars open, and `provenance::read` is where that damage is named.
#[test]
fn a_month_with_no_trustworthy_sidecar_is_unrecorded_and_still_opens() {
    let root = Root::new();
    let id = symbol_id("NIFTY");
    landed(&root, call(), &bars(11, 4, 1, 120));

    // Copied WITHOUT its sidecar: the limit, asserted rather than hidden.
    copy_month(&root, call(), sibling(), &[FileKind::Provenance]);
    assert_eq!(
        provenance::written_for(&root.0, sibling()),
        WrittenFor::Unrecorded
    );
    let opened = BarFile::open_existing(&root.0, sibling(), id).expect("unrecorded opens");
    assert_eq!(opened.records(), 120);
    drop(opened);

    let prv = sibling()
        .with_file(FileKind::Provenance)
        .to_path_buf(&root.0);
    let foreign = fs::read(call().with_file(FileKind::Provenance).to_path_buf(&root.0)).unwrap();

    // A header that fails its checksum says nothing.
    let mut rotted = foreign.clone();
    rotted[40] ^= 0x01;
    fs::write(&prv, &rotted).unwrap();
    assert_eq!(
        provenance::written_for(&root.0, sibling()),
        WrittenFor::Unrecorded
    );
    assert!(BarFile::open_existing(&root.0, sibling(), id).is_ok());

    // Shorter than a header, empty, and all zeros: unborn.
    for unborn in [&foreign[..40], &[][..], &[0u8; 320][..]] {
        fs::write(&prv, unborn).unwrap();
        assert_eq!(
            provenance::written_for(&root.0, sibling()),
            WrittenFor::Unrecorded,
            "{} bytes",
            unborn.len()
        );
        assert!(BarFile::open_existing(&root.0, sibling(), id).is_ok());
    }

    // A link is never followed, even to a sidecar that names another month.
    fs::remove_file(&prv).unwrap();
    std::os::unix::fs::symlink(
        call().with_file(FileKind::Provenance).to_path_buf(&root.0),
        &prv,
    )
    .unwrap();
    assert_eq!(
        provenance::written_for(&root.0, sibling()),
        WrittenFor::Unrecorded
    );

    // The whole foreign sidecar, restored: now there is evidence.
    fs::remove_file(&prv).unwrap();
    fs::write(&prv, &foreign).unwrap();
    assert!(matches!(
        provenance::written_for(&root.0, sibling()),
        WrittenFor::Another {
            field: "store path",
            ..
        }
    ));
    assert!(BarFile::open_existing(&root.0, sibling(), id).is_err());
}

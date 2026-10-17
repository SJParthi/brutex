//! `pull::listing`: the listing table read from receipted, invented masters.
//! Every symbol, ISIN and date here is invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test-only assertions"
)]

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use flate2::Compression;
use flate2::write::GzEncoder;

use super::{
    BEFORE_LISTING, HISTORY_ENDS, Listing, MASTER_DIR, Source, Span, Unknown, for_store, master_day,
};
use crate::session::Day;

/// A scratch store root, removed when the test ends.
pub(crate) struct Root(pub(crate) PathBuf);

impl Root {
    pub(crate) fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-listing-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join(MASTER_DIR)).expect("scratch masters");
        Self(path)
    }

    pub(crate) fn masters(&self) -> PathBuf {
        self.0.join(MASTER_DIR)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _best_effort = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn day(year: u16, month: u8, date: u8) -> Day {
    Day::new(year, month, date).expect("test date")
}

/// Seconds from 1980-01-01 to `day`, as a master states a lifecycle date.
pub(crate) fn nse_seconds(day: Day) -> u64 {
    let epoch = Day::new(1980, 1, 1).unwrap().days_from_epoch();
    u64::from(day.days_from_epoch() - epoch) * 86_400
}

/// One invented EQ row: symbol, ISIN, and the three lifecycle cells as written.
pub(crate) struct Row<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) isin: &'a str,
    pub(crate) listed: String,
    pub(crate) removed: String,
    pub(crate) readmitted: String,
}

pub(crate) fn row<'a>(symbol: &'a str, isin: &'a str, listed: Option<Day>) -> Row<'a> {
    Row {
        symbol,
        isin,
        listed: listed.map_or_else(|| "0".to_owned(), |d| nse_seconds(d).to_string()),
        removed: "0".to_owned(),
        readmitted: "0".to_owned(),
    }
}

/// Install a receipted master dated `on` holding `rows` under `root`.
pub(crate) fn install(root: &Path, on: Day, rows: &[Row<'_>]) {
    let mut csv = String::from(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n",
    );
    for (id, row) in rows.iter().enumerate() {
        writeln!(
            csv,
            "{},{},EQ,{},1,{},{},{}",
            9_000 + id,
            row.symbol,
            row.isin,
            row.listed,
            row.removed,
            row.readmitted
        )
        .unwrap();
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(csv.as_bytes()).expect("compress fixture");
    let bytes = encoder.finish().expect("finish gzip");
    crate::cash_session_cache::install(&root.join(MASTER_DIR), on, &bytes)
        .expect("fixture master installs");
}

const ALPHA: &str = "INE000A01001";

#[test]
fn the_newest_master_that_checks_is_used_and_a_bad_newer_one_is_named() {
    let root = Root::new("newest");
    let listed = day(2021, 3, 4);
    install(
        &root.0,
        day(2026, 8, 3),
        &[row("ALPHA", ALPHA, Some(listed))],
    );
    install(
        &root.0,
        day(2026, 8, 4),
        &[row("ALPHA", ALPHA, Some(listed))],
    );
    let table = Listing::load(&root.masters());
    assert_eq!(table.master().map(|(_, on)| on), Some(day(2026, 8, 4)));
    assert!(table.passed_over().is_empty());

    // The newer master's receipt no longer binds its bytes: passed over BY
    // NAME, and the older one answers.
    let receipt = root
        .masters()
        .join("NSE_CM_security_04082026.csv.gz.receipt");
    std::fs::write(&receipt, b"not a receipt\n").expect("damage the fixture receipt");
    let table = Listing::load(&root.masters());
    assert_eq!(
        table.master(),
        Some(("NSE_CM_security_03082026.csv.gz", day(2026, 8, 3)))
    );
    assert_eq!(table.passed_over().len(), 1);
    assert_eq!(table.passed_over()[0].0, "NSE_CM_security_04082026.csv.gz");
    assert!(table.passed_over()[0].1.contains("UNVERIFIED"));
    let span = table
        .span("NSE", "ALPHA", Some(ALPHA))
        .expect("a master row");
    assert_eq!(span.first, Ok(listed));
    assert_eq!(span.source.code(), "nse_security_master");
    let Source::NseSecurityMaster { file, sha256 } = span.source else {
        panic!("a master span names its master");
    };
    assert_eq!(file, "NSE_CM_security_03082026.csv.gz");
    assert_ne!(sha256, &[0_u8; 32]);
}

#[test]
fn a_bar_before_the_listing_day_is_refused_and_the_listing_day_is_kept() {
    let root = Root::new("before");
    let listed = day(2021, 3, 4);
    install(
        &root.0,
        day(2026, 8, 3),
        &[row("ALPHA", ALPHA, Some(listed))],
    );
    let table = Listing::load(&root.masters());
    let span = table.span("NSE", "ALPHA", Some(ALPHA)).unwrap();
    assert!(span.before_listing(day(2021, 3, 3)));
    assert!(
        !span.before_listing(listed),
        "a bar ON the listing day is kept"
    );
    assert!(!span.before_listing(day(2021, 3, 5)));
    assert_eq!(span.listed_on(), Some(listed));
    assert_eq!(BEFORE_LISTING, "before_listing");
}

#[test]
fn epoch_zero_listing_refuses_nothing() {
    let root = Root::new("epoch-zero");
    install(&root.0, day(2026, 8, 3), &[row("ALPHA", ALPHA, None)]);
    let table = Listing::load(&root.masters());
    let span = table.span("NSE", "ALPHA", Some(ALPHA)).unwrap();
    assert_eq!(span.first, Err(Unknown::EpochZero));
    assert_eq!(span.listed_on(), None);
    assert!(!span.before_listing(day(1990, 1, 1)));
    assert_eq!(Unknown::EpochZero.code(), "listing_epoch_zero");
}

#[test]
fn every_unknown_listing_names_why_and_refuses_nothing() {
    let root = Root::new("unknowns");
    let mut blank = row("BLANK", "INE000B01002", None);
    blank.listed = String::new();
    let mut late = row("LATE", "INE000C01003", Some(day(2026, 9, 1)));
    late.removed = "0".to_owned();
    let mut odd = row("ODD", "INE000D01004", None);
    odd.listed = "12345".to_owned();
    let mut backwards = row("BACKWARDS", "INE000E01005", Some(day(2021, 3, 4)));
    backwards.removed = nse_seconds(day(2020, 1, 1)).to_string();
    install(&root.0, day(2026, 8, 3), &[blank, late, odd, backwards]);
    let table = Listing::load(&root.masters());
    for (symbol, isin, why) in [
        ("BLANK", "INE000B01002", Unknown::Blank),
        ("LATE", "INE000C01003", Unknown::AfterMaster),
        ("ODD", "INE000D01004", Unknown::Invalid),
        ("BACKWARDS", "INE000E01005", Unknown::Contradictory),
    ] {
        let span = table.span("NSE", symbol, Some(isin)).unwrap();
        assert_eq!(span.first, Err(why), "{symbol}");
        assert!(!span.before_listing(day(1990, 1, 1)), "{symbol}");
        assert!(span.history_ended(day(2030, 1, 1)).is_none(), "{symbol}");
    }
    let codes = [
        Unknown::Blank,
        Unknown::ColumnAbsent,
        Unknown::Invalid,
        Unknown::Conflicting,
        Unknown::AfterMaster,
        Unknown::Contradictory,
    ]
    .map(Unknown::code);
    assert_eq!(
        codes,
        [
            "listing_blank",
            "listing_column_absent",
            "listing_invalid",
            "listing_conflicting",
            "listing_after_master",
            "listing_contradictory",
        ]
    );
}

#[test]
fn a_name_absent_from_every_master_is_labelled_inferred() {
    let root = Root::new("inferred");
    install(
        &root.0,
        day(2026, 8, 3),
        &[row("ALPHA", ALPHA, Some(day(2021, 3, 4)))],
    );
    let table = Listing::load(&root.masters());
    assert_eq!(table.span("NSE", "GONE", None), None);
    assert_eq!(table.span("NSE", "GONE", Some("INE000H01008")), None);
    let span = Span::inferred(day(2015, 1, 1), day(2019, 6, 28));
    assert_eq!(span.first, Ok(day(2015, 1, 1)));
    assert_eq!(span.last, Some(day(2019, 6, 28)));
    assert_eq!(span.source.code(), "inferred_from_vendor_span");
    // A label only: the vendor's own span accuses none of its own rows.
    assert!(!span.before_listing(day(2014, 1, 1)));
    assert_eq!(span.listed_on(), None);
    assert_eq!(span.history_ended(day(2020, 1, 1)), None);
    // A symbol the master holds under ANOTHER ISIN is not its row, and no ISIN
    // is no row either.
    assert_eq!(table.span("NSE", "ALPHA", Some("INE999Z01999")), None);
    assert_eq!(table.span("NSE", "ALPHA", None), None);
    assert!(table.span("NSE", "ALPHA", Some(ALPHA)).is_some());
}

#[test]
fn history_ends_after_removal_unless_readmitted() {
    let root = Root::new("removal");
    let removed_on = day(2019, 6, 28);
    let mut gone = row("GONE", "INE000F01006", Some(day(2010, 1, 4)));
    gone.removed = nse_seconds(removed_on).to_string();
    let mut back = row("BACK", "INE000G01007", Some(day(2010, 1, 4)));
    back.removed = nse_seconds(removed_on).to_string();
    back.readmitted = nse_seconds(day(2020, 2, 3)).to_string();
    install(&root.0, day(2026, 8, 3), &[gone, back]);
    let table = Listing::load(&root.masters());
    let span = table.span("NSE", "GONE", Some("INE000F01006")).unwrap();
    assert_eq!(span.last, Some(removed_on));
    assert_eq!(
        span.history_ended(removed_on),
        None,
        "the removal day itself"
    );
    assert_eq!(span.history_ended(day(2019, 7, 1)), Some(removed_on));
    let back = table.span("NSE", "BACK", Some("INE000G01007")).unwrap();
    assert_eq!(back.last, None);
    assert_eq!(back.history_ended(day(2019, 7, 1)), None);
    assert_eq!(HISTORY_ENDS, "history_ends_delisted");
}

#[test]
fn no_master_on_disk_answers_nothing_and_names_why() {
    let root = Root::new("empty");
    let table = Listing::load(&root.masters());
    assert_eq!(table.master(), None);
    assert!(table.passed_over().is_empty());
    assert_eq!(table.span("NSE", "ALPHA", Some(ALPHA)), None);
    let missing = Listing::load(&root.0.join("no-such-directory"));
    assert_eq!(missing.master(), None);
    assert_eq!(missing.passed_over().len(), 1);
    // Another exchange has no span here, master or not.
    install(
        &root.0,
        day(2026, 8, 3),
        &[row("ALPHA", ALPHA, Some(day(2021, 3, 4)))],
    );
    let table = Listing::load(&root.masters());
    assert_eq!(table.span("BSE", "ALPHA", Some(ALPHA)), None);
    assert!(table.span("NSE", "ALPHA", Some(ALPHA)).is_some());
}

#[test]
fn a_master_name_is_read_exactly() {
    assert_eq!(
        master_day("NSE_CM_security_04092026.csv.gz"),
        Some(day(2026, 9, 4))
    );
    for name in [
        "NSE_CM_security_04092026.csv.gz.receipt",
        "NSE_CM_security_0409202.csv.gz",
        "NSE_CM_security_0409202x.csv.gz",
        "NSE_CM_security_31022026.csv.gz",
        "NSE_FO_security_04092026.csv.gz",
        "NSE_CM_security_04092026.csv",
    ] {
        assert_eq!(master_day(name), None, "{name}");
    }
}

#[test]
fn a_store_table_is_remembered_only_once_a_master_loads() {
    let root = Root::new("remembered");
    let first = for_store(&root.0);
    assert_eq!(first.master(), None);
    // Installed after a load that found nothing: the next call reads it.
    install(
        &root.0,
        day(2026, 8, 3),
        &[row("ALPHA", ALPHA, Some(day(2021, 3, 4)))],
    );
    let second = for_store(&root.0);
    assert!(second.master().is_some());
    let third = for_store(&root.0);
    assert!(std::sync::Arc::ptr_eq(&second, &third), "loaded once");
}

#[test]
fn a_master_without_the_lifecycle_columns_and_two_rows_that_disagree_state_no_listing() {
    let root = Root::new("columns");
    let csv =
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n9000,ALPHA,EQ,INE000A01001,1\n";
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(csv.as_bytes()).unwrap();
    crate::cash_session_cache::install(
        &root.masters(),
        day(2026, 8, 3),
        &encoder.finish().unwrap(),
    )
    .unwrap();
    let table = Listing::load(&root.masters());
    let span = table.span("NSE", "ALPHA", Some(ALPHA)).unwrap();
    assert_eq!(span.first, Err(Unknown::ColumnAbsent));
    assert_eq!(span.last, None);

    let root = Root::new("conflicting");
    let first = row("ALPHA", ALPHA, Some(day(2021, 3, 4)));
    let second = row("ALPHA", ALPHA, Some(day(2021, 3, 5)));
    let mut csv = String::from(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n",
    );
    for listed in [&first.listed, &second.listed] {
        writeln!(csv, "9000,ALPHA,EQ,{ALPHA},1,{listed},0,0").unwrap();
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(csv.as_bytes()).unwrap();
    crate::cash_session_cache::install(
        &root.masters(),
        day(2026, 8, 3),
        &encoder.finish().unwrap(),
    )
    .unwrap();
    let table = Listing::load(&root.masters());
    let span = table.span("NSE", "ALPHA", Some(ALPHA)).unwrap();
    assert_eq!(span.first, Err(Unknown::Conflicting));
    assert!(!span.before_listing(day(1990, 1, 1)));
}

#[test]
fn a_removal_after_the_masters_own_day_ends_nothing() {
    let root = Root::new("late-removal");
    let mut late = row("LATE", "INE000J01009", Some(day(2010, 1, 4)));
    late.removed = nse_seconds(day(2026, 9, 1)).to_string();
    install(&root.0, day(2026, 8, 3), &[late]);
    let table = Listing::load(&root.masters());
    let span = table.span("NSE", "LATE", Some("INE000J01009")).unwrap();
    assert_eq!(span.first, Ok(day(2010, 1, 4)));
    assert_eq!(span.last, None);
    assert_eq!(span.history_ended(day(2026, 10, 1)), None);
}

#[test]
fn a_name_that_is_not_text_is_not_a_master() {
    use std::os::unix::ffi::OsStrExt as _;
    let root = Root::new("not-text");
    install(
        &root.0,
        day(2026, 8, 3),
        &[row("ALPHA", ALPHA, Some(day(2021, 3, 4)))],
    );
    let odd = std::ffi::OsStr::from_bytes(b"NSE_CM_security_\xff.csv.gz");
    std::fs::write(root.masters().join(odd), b"x").expect("an odd name");
    let table = Listing::load(&root.masters());
    assert_eq!(table.master().map(|(_, on)| on), Some(day(2026, 8, 3)));
    assert!(table.passed_over().is_empty());
}

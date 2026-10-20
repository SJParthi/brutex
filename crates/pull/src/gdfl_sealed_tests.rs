//! The GDFL import over SEALED months (SC-4, D-5904; store-compression
//! design §3, §6 "a late day for a sealed month").
//!
//! A month the import already filled is sealed (built by
//! `store::sealed::build` and published beside its raw form by rename, as
//! SC-3's tests do; the sealer is SC-6's), and the import is run again. The
//! append door refuses the sealed month (`StoreError::Sealed`) and commits
//! nothing; the coordinator files the day again through the back-fill door,
//! which lands a day the raw month would have appended as the sealed month's
//! successor generation. Every answer is compared with the same days
//! imported in order into a store that was never sealed.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

/// Every month file under `<store>/bars`, as store paths with their symbol.
fn months(store: &Path) -> Vec<(StorePath<'static>, &'static str)> {
    let mut found = Vec::new();
    let mut stack = vec![store.join("bars")];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "bin") {
                let rel = path
                    .strip_prefix(store)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                found.push(store_path(&rel));
            }
        }
    }
    found.sort_by_key(|(path, _)| path.to_path_buf(store));
    found
}

/// `bars/<vendor>/<exchange>/<segment>/<symbol>[/<contract>]/<tf>/<yyyy-mm>.bin`.
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
    let contract = (parts.len() == 8).then(|| Contract::parse(parts[5]).unwrap());
    let tf = parts[parts.len() - 2];
    let timeframe = *Timeframe::KNOWN.iter().find(|t| t.as_str() == tf).unwrap();
    let stem = parts[parts.len() - 1].strip_suffix(".bin").unwrap();
    let month =
        store::path::YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap())
            .unwrap();
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

/// Seals every month of `store` beside its raw form, and answers them.
fn seal_every_month(store: &Path) -> Vec<StorePath<'static>> {
    let all = months(store);
    assert!(!all.is_empty(), "the import filled months to seal");
    for &(path, symbol) in &all {
        let raw = BarFile::open_existing(store, path, symbol_id_of(symbol)).unwrap();
        assert_eq!(raw.form(), store::file::Form::Raw);
        let bars = raw.read_range(0, raw.records()).unwrap();
        let prv = std::fs::read(path.with_file(FileKind::Provenance).to_path_buf(store)).ok();
        let built = store::sealed::build(path, &raw.header(), &bars, prv.as_deref(), None).unwrap();
        let at = path.with_file(FileKind::SealedBars).to_path_buf(store);
        let staged = at.with_extension("sbar-staged");
        std::fs::write(&staged, built.to_bytes()).unwrap();
        std::fs::rename(&staged, &at).unwrap();
        assert_eq!(
            BarFile::open_existing(store, path, symbol_id_of(symbol))
                .unwrap()
                .form(),
            store::file::Form::Sealed
        );
    }
    all.into_iter().map(|(path, _)| path).collect()
}

/// Every bar of the month at `path` as every reader opens it.
fn served(store: &Path, path: StorePath<'_>, symbol: &str) -> Vec<Bar> {
    let file = BarFile::open_existing(store, path, symbol_id_of(symbol)).unwrap();
    file.read_range(0, file.records()).unwrap()
}

/// THE LATE DAY, END TO END, for the index files and the option contracts
/// alike: 2024-04-01 and 2024-04-02 imported and every month sealed, then
/// 2024-04-03 imported. No failure; every month of the day is a successor
/// generation holding exactly what an in-order import into a never-sealed
/// store holds; the sealed originals are byte-identical; the repair journal
/// names each successor before and after it; and the same day imported
/// again writes nothing.
#[test]
fn a_late_day_for_a_sealed_month_lands_as_its_successor_generation() {
    for kind in [ImportKind::Indices, ImportKind::Options] {
        let root = scratch(&format!("sc4-late-{}", kind.as_str()));
        world(&root, &[d1(), d2(), d3()]);
        let (sealed, whole) = (root.join("sealed"), root.join("whole"));
        let got = import(&root, Src::Store, kind, d1(), d2(), &sealed);
        assert!(got.failures.is_empty(), "{:?}", got.failures);
        let got = import(&root, Src::Store, kind, d1(), d3(), &whole);
        assert!(got.failures.is_empty(), "{:?}", got.failures);
        let sealed_months = seal_every_month(&sealed);
        let originals = tree(&sealed.join("bars"));

        let got = import(&root, Src::Store, kind, d3(), d3(), &sealed);

        assert!(got.failures.is_empty(), "{:?}", got.failures);
        assert!(got.seconds > 0 && got.files > 0);
        assert_eq!(
            got.seconds_committed, got.seconds,
            "every bar of the day added"
        );
        assert_eq!(got.months_repaired, got.files, "one successor per file");
        assert_eq!(
            tree(&sealed.join("bars")),
            originals,
            "the sealed originals are untouched"
        );
        for (path, symbol) in months(&whole) {
            assert_eq!(
                served(&sealed, path, symbol),
                served(&whole, path, symbol),
                "{path:?}"
            );
            if sealed_months.contains(&path) {
                assert_eq!(
                    store::generation::served(&sealed, path).unwrap().ordinal(),
                    1
                );
            }
        }
        let journal = std::fs::read_to_string(sealed.join("audit").join("gdfl.repairs")).unwrap();
        let repairing = journal
            .lines()
            .filter(|line| line.starts_with("repairing ") && line.contains("sealed-month"))
            .count();
        let repaired = journal
            .lines()
            .filter(|line| line.starts_with("repair "))
            .count();
        assert_eq!((repairing, repaired), (got.files, got.files), "{journal}");

        // THE SAME DAY AGAIN: held by the successor, nothing written.
        let after = (
            tree(&sealed.join("bars")),
            tree(&sealed.join("bar-revisions-v1")),
        );
        let again = import(&root, Src::Store, kind, d3(), d3(), &sealed);
        assert!(again.failures.is_empty(), "{:?}", again.failures);
        assert_eq!((again.seconds_committed, again.months_repaired), (0, 0));
        assert_eq!(
            (
                tree(&sealed.join("bars")),
                tree(&sealed.join("bar-revisions-v1"))
            ),
            after
        );
    }
}

/// A RESUMED IMPORT OVER A SEALED MONTH IS IDEMPOTENT: a day the sealed
/// month holds, imported again, is `AlreadyPresent` at the append door, no
/// failure, nothing written, no generation spent and no repair journaled.
#[test]
fn a_day_a_sealed_month_holds_is_already_present_and_writes_nothing() {
    for kind in [ImportKind::Indices, ImportKind::Options] {
        let root = scratch(&format!("sc4-held-{}", kind.as_str()));
        world(&root, &[d1(), d2()]);
        let sealed = root.join("sealed");
        let got = import(&root, Src::Store, kind, d1(), d2(), &sealed);
        assert!(got.failures.is_empty(), "{:?}", got.failures);
        let sealed_months = seal_every_month(&sealed);
        let before = tree(&sealed);

        let again = import(&root, Src::Store, kind, d2(), d2(), &sealed);

        assert!(again.failures.is_empty(), "{:?}", again.failures);
        assert_eq!((again.seconds_committed, again.months_repaired), (0, 0));
        for path in sealed_months {
            assert_eq!(
                store::generation::served(&sealed, path).unwrap().ordinal(),
                0
            );
        }
        let after = tree(&sealed);
        let changed: Vec<&PathBuf> = after
            .iter()
            .filter(|(path, bytes)| before.get(*path) != Some(*bytes))
            .map(|(path, _)| path)
            .filter(|path| path.starts_with("bars") || path.starts_with("bar-revisions-v1"))
            .collect();
        assert!(changed.is_empty(), "{changed:?}");
        assert!(
            !std::fs::read_to_string(sealed.join("audit").join("gdfl.repairs"))
                .unwrap_or_default()
                .contains("sealed-month"),
        );
    }
}

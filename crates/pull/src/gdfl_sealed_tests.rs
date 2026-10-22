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

// ── D-6247 at the GDFL door: a resumed late day at a revision-served month ─

/// The NIFTY one-second month of April 2024.
fn nifty_april() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `count` invented NIFTY bars of `d`, one a second from 09:15:00 + `from`,
/// at prices no real print of the index was ever made at.
fn nifty_seconds(d: Day, from: u32, count: u32) -> Vec<Bar> {
    (0..count)
        .map(|s| {
            let sod = NINE_FIFTEEN + from + s;
            let secs = i64::from(d.days_from_epoch()) * 86_400 + i64::from(sod) - IST_OFFSET_SECS;
            let price = 7_300_011 + i64::from(s % 89);
            Bar {
                ts_micros: secs * 1_000_000,
                open: price,
                high: price + 6,
                low: price - 6,
                close: price + 2,
                volume: 0,
                open_interest: OI_NULL,
            }
        })
        .collect()
}

/// Offers `batch` to NIFTY's April through the back-fill door the import
/// files every GDFL day through (`ingest::repair_rows`): how it was filed,
/// the bars committed, and every failure's words.
fn back_fill(store: &Path, batch: &[Bar]) -> (crate::ingest::Filed, usize, Vec<String>) {
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: Window::new(d1(), day(2024, 5, 31)).unwrap(),
        granularity: Granularity::Second1,
    };
    let (done, filed) = crate::ingest::repair_rows(
        batch,
        "NIFTY",
        "sc4 revision door",
        store,
        plan(ImportKind::Indices, &request, None),
        "sc4 revision door test",
    );
    let failures = done.failures.into_iter().map(|f| f.why).collect();
    (filed, done.bars_committed, failures)
}

/// The repair journal's lines whose first word is `verb`.
fn journal_lines(store: &Path, verb: &str) -> Vec<String> {
    std::fs::read_to_string(store.join("audit").join("gdfl.repairs"))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.split(' ').next() == Some(verb))
        .map(str::to_owned)
        .collect()
}

/// Every file under `dir`, or nothing when it does not exist.
fn files_or_none(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    if dir.exists() {
        tree(dir)
    } else {
        BTreeMap::new()
    }
}

/// **A RESUMED LATE DAY AT A MONTH SERVED FROM A REVISION** (SC4-17,
/// D-6247; the second checker's blocking finding on 5448750f6). NIFTY's
/// April is made revision-served two ways: a back-filled hole (2024-04-01
/// and 04-03 held, then 04-02), and a sealed month's first late day
/// (04-01..02 held and sealed, then 04-03). Each then takes the first 30 s
/// of 2024-04-04, and then the whole 60 s, the held 30 s byte-equal: the raw
/// month would append both, so the back-fill door files each as the month's
/// NEXT successor (generation 2, then 3), never as a refusal of what
/// `lack_of` sees as an overlap. After every batch the month serves exactly
/// what a never-sealed store that took the same batches serves; the
/// originals and every earlier revision stay byte-identical; each successor
/// is journaled `repairing` (reason `revision successor`) before and
/// `repair` after; and the whole day again writes nothing, journals nothing
/// and spends no generation. Routing a `Follows::Revision` batch to the
/// back-fill judgement instead (the code before D-6247) commits 0 of the
/// whole day and fails here.
#[test]
fn a_resumed_late_day_at_a_revision_served_month_lands_as_its_next_successor() {
    use crate::ingest::Filed;
    let april = nifty_april();
    let generation = |store: &Path| store::generation::served(store, april).unwrap().ordinal();
    for origin in ["hole", "sealed"] {
        let root = scratch(&format!("sc4-revision-{origin}"));
        let (served_from, never) = (root.join("revised"), root.join("never-sealed"));
        for store in [&served_from, &never] {
            std::fs::create_dir_all(store).unwrap();
        }
        let (first, second, third) = (
            nifty_seconds(d1(), 0, 60),
            nifty_seconds(d2(), 0, 60),
            nifty_seconds(d3(), 0, 60),
        );
        // THE NEVER-SEALED STORE: three days in order, each an append.
        for batch in [&first, &second, &third] {
            assert_eq!(back_fill(&never, batch), (Filed::Appended, 60, vec![]));
        }
        // THE SAME THREE DAYS, ENDING SERVED FROM REVISION 1.
        if origin == "hole" {
            for batch in [&first, &third] {
                assert_eq!(
                    back_fill(&served_from, batch),
                    (Filed::Appended, 60, vec![])
                );
            }
            assert_eq!(
                back_fill(&served_from, &second),
                (Filed::NewGeneration, 60, vec![])
            );
        } else {
            for batch in [&first, &second] {
                assert_eq!(
                    back_fill(&served_from, batch),
                    (Filed::Appended, 60, vec![])
                );
            }
            assert_eq!(seal_every_month(&served_from), vec![april]);
            assert_eq!(
                back_fill(&served_from, &third),
                (Filed::NewGeneration, 60, vec![])
            );
        }
        assert_eq!(
            (generation(&served_from), generation(&never)),
            (1, 0),
            "{origin}"
        );
        assert_eq!(
            served(&served_from, april, "NIFTY"),
            served(&never, april, "NIFTY"),
            "{origin}"
        );
        let originals = tree(&served_from.join("bars"));
        let revisions = files_or_none(&served_from.join("bar-revisions-v1"));
        assert!(!revisions.is_empty(), "{origin}: revision 1 is on disk");
        let (intents, repairs) = (
            journal_lines(&served_from, "repairing").len(),
            journal_lines(&served_from, "repair").len(),
        );

        // THE RESUMED DAY: its first half, then the whole day.
        let fourth = nifty_seconds(d4(), 0, 60);
        for (step, batch, to) in [("half", &fourth[..30], 2_u32), ("whole", &fourth[..], 3)] {
            assert_eq!(
                back_fill(&never, batch),
                (Filed::Appended, 30, vec![]),
                "{origin} {step}: the raw month appends it"
            );
            assert_eq!(
                back_fill(&served_from, batch),
                (Filed::NewGeneration, 30, vec![]),
                "{origin} {step}: the month's next successor"
            );
            assert_eq!(generation(&served_from), to, "{origin} {step}");
            assert_eq!(
                served(&served_from, april, "NIFTY"),
                served(&never, april, "NIFTY"),
                "{origin} {step}: what a never-sealed store serves"
            );
        }

        // NOTHING EARLIER WAS REWRITTEN: the original month and revision 1
        // byte for byte; only the two successors were added.
        assert_eq!(tree(&served_from.join("bars")), originals, "{origin}");
        let now = files_or_none(&served_from.join("bar-revisions-v1"));
        for (path, bytes) in &revisions {
            assert_eq!(now.get(path), Some(bytes), "{origin}: {}", path.display());
        }
        assert!(
            now.len() > revisions.len(),
            "{origin}: the successors exist"
        );

        // JOURNALED: each successor's intent before it, its repair after it.
        let new_intents: Vec<String> = journal_lines(&served_from, "repairing").split_off(intents);
        let new_repairs: Vec<String> = journal_lines(&served_from, "repair").split_off(repairs);
        assert_eq!((new_intents.len(), new_repairs.len()), (2, 2), "{origin}");
        for ((intent, repair), (from, offered)) in new_intents
            .iter()
            .zip(&new_repairs)
            .zip([(1_u32, 30_usize), (2, 60)])
        {
            assert!(intent.contains("reason=revision_successor_"), "{intent}");
            assert!(
                intent.contains(&format!(" from_generation={from} ")),
                "{intent}"
            );
            assert!(intent.contains(&format!(" offered={offered} ")), "{intent}");
            assert!(
                repair.contains(&format!(
                    " from_generation={from} to_generation={} ",
                    from + 1
                )),
                "{repair}"
            );
        }

        // THE WHOLE DAY AGAIN: held, nothing written, nothing journaled.
        let (bars_now, revisions_now) = (
            tree(&served_from.join("bars")),
            files_or_none(&served_from.join("bar-revisions-v1")),
        );
        let journal_now = journal_lines(&served_from, "repairing").len();
        assert_eq!(
            back_fill(&served_from, &fourth),
            (Filed::Appended, 0, vec![])
        );
        assert_eq!(generation(&served_from), 3, "{origin}: no generation spent");
        assert_eq!(tree(&served_from.join("bars")), bars_now, "{origin}");
        assert_eq!(
            files_or_none(&served_from.join("bar-revisions-v1")),
            revisions_now,
            "{origin}"
        );
        assert_eq!(journal_lines(&served_from, "repairing").len(), journal_now);
        std::fs::remove_dir_all(root).unwrap();
    }
}

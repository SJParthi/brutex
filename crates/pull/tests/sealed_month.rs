//! The Zerodha pull over SEALED months (SC-4, D-5904; store-compression
//! design §3, §6; WP-62 "the pull must treat Sealed as SC-4 says").
//!
//! The pull lands minutes through `pull::ingest::from_window`, the append
//! door, which also folds every derived rung. A month held in its sealed form
//! refuses every batch it does not hold (`store::file::StoreError::Sealed`);
//! the pull answers each batch exactly as it answers the month in its raw
//! form: a late day after the last held minute is filed as the month's
//! successor generation, a day already held writes nothing, and a batch the
//! raw month would refuse is refused. Every answer is compared with the same
//! days landed into a store that was never sealed. Candles are invented.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test-only assertions"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::fetch::{RawRow, RawWindow};
use pull::session::{Day, IST_OFFSET_SECS, Window};
use store::file::{BarFile, Form};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("brutex-SC4-{tag}-{}", std::process::id()));
        let _best_effort = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

const SYMBOL: &str = "NIFTY";

fn symbol_id() -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(SYMBOL) & u64::from(u32::MAX)).unwrap()
}

/// An invented candle at `minute` past IST midnight on `day`.
fn candle(day: Day, minute: i64) -> RawRow {
    let base = 7_000_000 + (minute % 89) * 17 + i64::from(day.day());
    RawRow {
        timestamp: i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60,
        open: base,
        high: base + 60,
        low: base - 30,
        close: base + 20,
        volume: 0,
        open_interest: None,
    }
}

/// Every minute `from..=to` owes by the calendar of record, one candle each.
fn answer(from: Day, to: Day) -> Vec<RawRow> {
    (from.days_from_epoch()..=to.days_from_epoch())
        .filter_map(|number| {
            let kind = pull::session::with_recorded_window(
                i64::from(number),
                pull::calendar::sourced_kind_of(i64::from(number)),
            );
            let pull::calendar::DayKind::Open(session) = kind else {
                return None;
            };
            let day = Day::from_days(number).unwrap();
            Some(
                (0..1_440_u16)
                    .filter(move |m| session.expects(*m))
                    .map(move |m| candle(day, i64::from(m)))
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .collect()
}

fn land(root: &Path, from: Day, to: Day, rows: Vec<RawRow>) -> pull::ingest::Ingested {
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: Window::new(from, to).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    };
    let plan = pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
        run: 0,
    };
    pull::ingest::from_window(&RawWindow { rows }, SYMBOL, "SC4 TEST", root, plan)
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// Every month file under `<root>/bars`: the store path of each `.bin`.
fn months(root: &Path) -> Vec<StorePath<'static>> {
    let mut found = Vec::new();
    let mut stack = vec![root.join("bars")];
    while let Some(at) = stack.pop() {
        for entry in fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "bin") {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let rel: &'static str = Box::leak(rel.into_boxed_str());
                let parts: Vec<&'static str> = rel.split('/').collect();
                assert_eq!(parts.len(), 7, "{rel}: an index month");
                let timeframe = *Timeframe::KNOWN
                    .iter()
                    .find(|t| t.as_str() == parts[5])
                    .unwrap();
                let stem = parts[6].strip_suffix(".bin").unwrap();
                let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap())
                    .unwrap();
                found.push(
                    StorePath::new(PathParts {
                        vendor: Vendor::Zerodha,
                        exchange: parts[2],
                        segment: parts[3],
                        symbol: parts[4],
                        contract: None,
                        timeframe,
                        month,
                        file: FileKind::Bars,
                    })
                    .unwrap(),
                );
            }
        }
    }
    found.sort_by_key(|path| path.to_path_buf(root));
    found
}

/// Seals every month of `root` beside its raw form.
fn seal_every_month(root: &Path) -> Vec<StorePath<'static>> {
    let all = months(root);
    assert!(all.len() > 1, "the minute month and its derived rungs");
    for &path in &all {
        let raw = BarFile::open_existing(root, path, symbol_id()).unwrap();
        assert_eq!(raw.form(), Form::Raw);
        let bars = raw.read_range(0, raw.records()).unwrap();
        let prv = fs::read(path.with_file(FileKind::Provenance).to_path_buf(root)).ok();
        let built = store::sealed::build(path, &raw.header(), &bars, prv.as_deref(), None).unwrap();
        let at = path.with_file(FileKind::SealedBars).to_path_buf(root);
        let staged = at.with_extension("sbar-staged");
        fs::write(&staged, built.to_bytes()).unwrap();
        fs::rename(&staged, &at).unwrap();
    }
    all
}

fn served(root: &Path, path: StorePath<'_>) -> Vec<store::format::Bar> {
    let file = BarFile::open_existing(root, path, symbol_id()).unwrap();
    file.read_range(0, file.records()).unwrap()
}

fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn repairs(root: &Path) -> String {
    fs::read_to_string(root.join("audit").join("zerodha.repairs")).unwrap_or_default()
}

/// THE LATE DAY: 2024-04-01..03 landed and every month sealed (the minute
/// month and each derived rung), then 2024-04-04. No failure; every month
/// serves exactly what the same landings give a store never sealed, as a
/// successor generation of the sealed one; the sealed originals are
/// byte-identical; the repair journal names each successor before and after;
/// and the same day landed again writes nothing.
#[test]
fn a_late_day_for_a_sealed_month_lands_as_its_successor_and_every_rung_follows() {
    let (sealed, whole) = (Scratch::new("late-sealed"), Scratch::new("late-whole"));
    let (first, last, late) = (day(2024, 4, 1), day(2024, 4, 3), day(2024, 4, 4));
    for root in [&sealed.0, &whole.0] {
        let got = land(root, first, last, answer(first, last));
        assert!(got.failures.is_empty(), "{:?}", got.failures);
    }
    let sealed_months = seal_every_month(&sealed.0);
    let originals = tree(&sealed.0.join("bars"));

    let got = land(&sealed.0, late, late, answer(late, late));
    let raw = land(&whole.0, late, late, answer(late, late));

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert!(raw.failures.is_empty(), "{:?}", raw.failures);
    assert_eq!(
        got.bars_committed, raw.bars_committed,
        "the same bars landed"
    );
    assert!(got.bars_committed > 0);
    assert_eq!(
        tree(&sealed.0.join("bars")),
        originals,
        "the sealed originals are untouched"
    );
    assert_eq!(
        months(&whole.0),
        sealed_months,
        "no month made beside a sealed one"
    );
    for &path in &sealed_months {
        assert_eq!(served(&sealed.0, path), served(&whole.0, path), "{path:?}");
        assert_eq!(
            store::generation::served(&sealed.0, path)
                .unwrap()
                .ordinal(),
            1,
            "{path:?}"
        );
    }
    let journal = repairs(&sealed.0);
    let count = |verb: &str| {
        journal
            .lines()
            .filter(|line| line.split(' ').next() == Some(verb))
            .count()
    };
    assert_eq!(count("repairing"), sealed_months.len(), "{journal}");
    assert_eq!(count("repair"), sealed_months.len(), "{journal}");
    assert!(
        journal
            .lines()
            .filter(|line| line.starts_with("repairing "))
            .all(|line| line.contains("sealed-month")),
        "{journal}"
    );

    // The same late day again: held by every successor, nothing written.
    let before = (
        tree(&sealed.0.join("bars")),
        tree(&sealed.0.join("bar-revisions-v1")),
    );
    let again = land(&sealed.0, late, late, answer(late, late));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(again.bars_committed, 0);
    assert_eq!(
        (
            tree(&sealed.0.join("bars")),
            tree(&sealed.0.join("bar-revisions-v1"))
        ),
        before
    );
}

/// TWO LATE DAYS IN TWO PULLS (SC-4 repair round 1, D-6247): after the first
/// late day every sealed month is served from its successor generation, a
/// repair revision, which the writer door never appends to (D-3350). The
/// second late day, in its own pull, follows that revision's last bar, so
/// the raw month would append it: it is filed as the NEXT successor through
/// the same repair door, with no human step. A re-pull window that re-reads
/// the second day and adds a third (held part byte-equal, new bars after
/// its last) is filed the same way. Every month then serves exactly what a
/// never-sealed store serves; the originals and every earlier revision are
/// byte-identical; each successor is journaled before and after, naming the
/// revision it follows; the same window again writes nothing and spends no
/// generation; and a restated minute of a held late day is refused as the
/// raw month refuses it, nothing written.
#[test]
fn two_late_days_in_two_pulls_after_sealing_serve_what_a_never_sealed_store_serves() {
    let (sealed, whole) = (Scratch::new("late2-sealed"), Scratch::new("late2-whole"));
    let (first, last) = (day(2024, 4, 1), day(2024, 4, 3));
    for root in [&sealed.0, &whole.0] {
        let got = land(root, first, last, answer(first, last));
        assert!(got.failures.is_empty(), "{:?}", got.failures);
    }
    let sealed_months = seal_every_month(&sealed.0);
    let originals = tree(&sealed.0.join("bars"));
    let pulls = [
        (day(2024, 4, 4), day(2024, 4, 4), 1_u32),
        (day(2024, 4, 5), day(2024, 4, 5), 2),
        // A window re-reading the held 04-05 and adding 04-08: the held part
        // byte-equal, the new bars after the last held one.
        (day(2024, 4, 5), day(2024, 4, 8), 3),
    ];
    let mut revisions = BTreeMap::new();
    for (from, to, generation) in pulls {
        let got = land(&sealed.0, from, to, answer(from, to));
        let raw = land(&whole.0, from, to, answer(from, to));
        assert!(got.failures.is_empty(), "{from}..{to}: {:?}", got.failures);
        assert!(raw.failures.is_empty(), "{from}..{to}: {:?}", raw.failures);
        assert!(raw.bars_committed > 0, "{from}..{to}");
        assert_eq!(got.bars_committed, raw.bars_committed, "{from}..{to}");
        for &path in &sealed_months {
            assert_eq!(
                served(&sealed.0, path),
                served(&whole.0, path),
                "{from}..{to} {path:?}"
            );
            assert_eq!(
                store::generation::served(&sealed.0, path)
                    .unwrap()
                    .ordinal(),
                generation,
                "{from}..{to} {path:?}"
            );
        }
        // NEVER IN PLACE: the sealed originals and every revision published
        // by an earlier pull are byte for byte what they were.
        let now = tree(&sealed.0.join("bar-revisions-v1"));
        for (path, bytes) in &revisions {
            assert_eq!(now.get(path), Some(bytes), "{from}..{to} {path:?}");
        }
        revisions = now;
        assert_eq!(tree(&sealed.0.join("bars")), originals, "{from}..{to}");
    }
    assert_eq!(months(&whole.0), sealed_months, "no month made beside one");

    assert_two_successors_after_the_sealed_one(&sealed.0, sealed_months.len());
    assert_held_again_and_restated_refused(&sealed.0, &whole.0, &sealed_months);
}

/// The journal after three late-day pulls over sealed months (D-6247): per
/// month one `repairing`/`repair` pair naming the sealed successor, then two
/// naming the revision successor, generation 1 to 2 and 2 to 3.
fn assert_two_successors_after_the_sealed_one(sealed: &Path, months: usize) {
    let journal = repairs(sealed);
    let lines = |verb: &str| -> Vec<&str> {
        journal
            .lines()
            .filter(|line| line.split(' ').next() == Some(verb))
            .collect()
    };
    let (intents, repairs_done) = (lines("repairing"), lines("repair"));
    assert_eq!(intents.len(), 3 * months, "{journal}");
    assert_eq!(repairs_done.len(), 3 * months, "{journal}");
    let (sealed_lines, revision_lines): (Vec<&str>, Vec<&str>) = intents
        .iter()
        .partition(|line| line.contains("reason=sealed-month_successor_"));
    assert_eq!(sealed_lines.len(), months, "{journal}");
    assert_eq!(revision_lines.len(), 2 * months, "{journal}");
    assert!(
        revision_lines
            .iter()
            .all(|line| line.contains("reason=revision_successor_")),
        "{journal}"
    );
    for link in 1..=2_u32 {
        assert_eq!(
            repairs_done
                .iter()
                .filter(|line| line.contains(&format!(
                    " from_generation={link} to_generation={} ",
                    link + 1
                )))
                .count(),
            months,
            "{journal}"
        );
    }
}

/// After the three pulls (D-6247): the same window again writes nothing,
/// journals nothing and spends no generation, and a restated minute of a held
/// late day is refused as the raw month refuses it, nothing written.
fn assert_held_again_and_restated_refused(
    sealed: &Path,
    whole: &Path,
    sealed_months: &[StorePath<'static>],
) {
    // THE SAME WINDOW AGAIN: held by every successor, nothing written, no
    // generation spent, nothing journaled.
    let (from, to) = (day(2024, 4, 5), day(2024, 4, 8));
    let before = (
        tree(&sealed.join("bars")),
        tree(&sealed.join("bar-revisions-v1")),
        repairs(sealed),
    );
    let again = land(sealed, from, to, answer(from, to));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(again.bars_committed, 0);
    assert_eq!(
        (
            tree(&sealed.join("bars")),
            tree(&sealed.join("bar-revisions-v1")),
            repairs(sealed),
        ),
        before
    );

    // A RESTATED MINUTE OF A HELD LATE DAY is refused as the raw month
    // refuses it: by name, nothing written, no generation spent.
    let held = day(2024, 4, 5);
    let mut restated = answer(held, held);
    restated[10].close += 5;
    let at_revision = land(sealed, held, held, restated.clone());
    let at_raw = land(whole, held, held, restated);
    assert!(!at_raw.failures.is_empty(), "the raw month refuses it");
    assert_eq!(
        at_revision.failures.len(),
        at_raw.failures.len(),
        "{:?} against {:?}",
        at_revision.failures,
        at_raw.failures
    );
    assert_eq!(at_revision.bars_committed, at_raw.bars_committed);
    let words = format!("{:?}", at_revision.failures);
    assert!(
        words.contains("repair revision 3") && words.contains("restates"),
        "{words}"
    );
    assert_eq!(
        (
            tree(&sealed.join("bars")),
            tree(&sealed.join("bar-revisions-v1")),
            repairs(sealed),
        ),
        before
    );
    for &path in sealed_months {
        assert_eq!(served(sealed, path), served(whole, path), "{path:?}");
    }
}

/// A DAY A SEALED MONTH HOLDS writes nothing and spends no generation, and A
/// RESTATED MINUTE is refused exactly as the raw month refuses it: by name,
/// nothing written, nothing repaired.
#[test]
fn a_held_day_writes_nothing_and_a_restated_minute_is_refused_as_raw() {
    let (sealed, raw) = (Scratch::new("held-sealed"), Scratch::new("held-raw"));
    let (first, last) = (day(2024, 4, 1), day(2024, 4, 3));
    for root in [&sealed.0, &raw.0] {
        let got = land(root, first, last, answer(first, last));
        assert!(got.failures.is_empty(), "{:?}", got.failures);
    }
    let sealed_months = seal_every_month(&sealed.0);
    let before = tree(&sealed.0);

    let mid = day(2024, 4, 2);
    let again = land(&sealed.0, mid, mid, answer(mid, mid));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(again.bars_committed, 0);
    for &path in &sealed_months {
        assert_eq!(
            store::generation::served(&sealed.0, path)
                .unwrap()
                .ordinal(),
            0
        );
    }

    let mut restated = answer(mid, mid);
    restated[10].close += 5;
    let at_sealed = land(&sealed.0, mid, mid, restated.clone());
    let at_raw = land(&raw.0, mid, mid, restated);
    assert!(
        !at_raw.failures.is_empty(),
        "the raw month refuses a restated minute"
    );
    assert_eq!(
        at_sealed.failures.len(),
        at_raw.failures.len(),
        "{:?} against {:?}",
        at_sealed.failures,
        at_raw.failures
    );
    let words = format!("{:?}", at_sealed.failures);
    assert!(
        words.contains("sealed") && words.contains("restates"),
        "{words}"
    );
    assert_eq!(at_sealed.bars_committed, at_raw.bars_committed);
    for &path in &sealed_months {
        assert_eq!(
            store::generation::served(&sealed.0, path)
                .unwrap()
                .ordinal(),
            0
        );
    }
    let now = tree(&sealed.0);
    let changed: Vec<&PathBuf> = now
        .iter()
        .filter(|(path, bytes)| before.get(*path) != Some(*bytes))
        .map(|(path, _)| path)
        .filter(|path| {
            path.starts_with(sealed.0.join("bars"))
                || path.starts_with(sealed.0.join("bar-revisions-v1"))
        })
        .collect();
    assert!(changed.is_empty(), "{changed:?}");
    assert!(!repairs(&sealed.0).contains("sealed-month"));
}

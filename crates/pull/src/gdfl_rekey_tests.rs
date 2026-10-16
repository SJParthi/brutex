//! Tests of the re-key (D-3331, D-3316; WP-12c; DB-A17, DB-A18). Every
//! store here is a scratch store in the temporary directory, built from
//! invented bars: the keys follow the shapes the assurance measured (a
//! `2023-06-29` NIFTY long-dated put, three `2020-06-25` strikes a hundred and
//! sixty times the day's ladder) and every price, volume and open interest is
//! made up and cannot be mistaken for a vendor's.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, OptionSide, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{StorePath, YearMonth};

use super::*;
use crate::gdfl_lineage::{RetiredWhy, contract_parts, lineage_path};
use crate::manifest::{EntryKey, Held, Manifest, epoch_path};
use crate::session::Day;

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-rekey-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(root.join("manifest")).unwrap();
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn d(y: u16, m: u8, dd: u8) -> Day {
    Day::new(y, m, dd).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn c(text: &str) -> Contract {
    Contract::parse(text).unwrap()
}

/// One invented one-second bar at `sod` seconds into IST `day`.
fn bar(day: Day, sod: u32, price: i64) -> Bar {
    let ts = (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + i64::from(sod)) * 1_000_000;
    Bar {
        ts_micros: ts,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 7,
        open_interest: 70,
    }
}

/// A key's bars, one month file per month, through the store's writer.
fn put(root: &Path, under: &str, contract: &str, bars: &[Bar]) {
    let mut by_month: BTreeMap<YearMonth, Vec<Bar>> = BTreeMap::new();
    for one in bars {
        let day = ist_day(one.ts_micros).unwrap();
        by_month
            .entry(day.year_month().unwrap())
            .or_default()
            .push(*one);
    }
    let underlying = sym(under);
    for (month, bars) in by_month {
        let at = StorePath::new(month_parts(&underlying, c(contract), month)).unwrap();
        let mut file = BarFile::open_or_create(root, at, symbol_id(underlying)).unwrap();
        file.append(&bars).unwrap();
    }
}

/// The bars one key holds in one month, read back.
fn held(root: &Path, under: &str, contract: &str, month: YearMonth) -> Vec<Bar> {
    read_month(root, sym(under), c(contract), month).unwrap()
}

/// Every byte under `dir`, by path: what "never a byte of the old key" is
/// checked against.
fn bytes_under(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn key_dir(root: &Path, under: &str, contract: &str) -> PathBuf {
    root.join("bars/gdfl/NSE/FNO").join(under).join(contract)
}

/// A census row for one month of one key, from its bar file.
fn census_row_of(root: &Path, under: &str, contract: &str, month: YearMonth) -> Held {
    let underlying = sym(under);
    let at = StorePath::new(month_parts(&underlying, c(contract), month)).unwrap();
    let file = BarFile::open_existing(root, at, symbol_id(underlying)).unwrap();
    census_row(&file, underlying, c(contract), month).unwrap()
}

/// The gdfl census migrated, its FNO partition's first epoch holding `rows`.
fn partitioned(root: &Path, rows: &[Held]) {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 64).unwrap();
    for row in rows {
        census.record_held(*row).unwrap();
    }
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, census.image()).unwrap();
}

/// The current FNO epoch's keys, as `UNDERLYING CONTRACT MONTH`.
fn census_keys(root: &Path) -> (u32, BTreeSet<String>) {
    let epoch = census_store::current_epoch(root, Vendor::Gdfl, Segment::Fno)
        .unwrap()
        .unwrap();
    let census = census_store::read_epoch(
        &epoch_path(root, Vendor::Gdfl, Segment::Fno, epoch),
        Vendor::Gdfl,
        Segment::Fno,
    )
    .unwrap();
    let keys = census
        .held_keys()
        .map(|key| {
            format!(
                "{} {} {}",
                key.symbol.as_str(),
                key.contract
                    .map_or_else(|| "-".to_owned(), |k| k.as_str().to_owned()),
                key.month
            )
        })
        .collect();
    (epoch, keys)
}

/// The store the assurance found, in miniature: a NIFTY ladder on 2019-05-30
/// with three impossible strikes, and one long-dated put stated 2023-06-29
/// with bars in two months of 2020.
struct Fixture {
    scratch: Scratch,
}

const LADDER: [&str; 6] = [
    "2019-05-30-1150000-CE",
    "2019-05-30-1170000-PE",
    "2019-05-30-1180000-CE",
    "2019-06-27-1190000-PE",
    "2019-06-27-1200000-CE",
    "2019-06-27-1220000-PE",
];
const IMPOSSIBLE: [&str; 3] = [
    "2020-06-25-191250000-CE",
    "2020-06-25-191270000-CE",
    "2020-06-25-191300000-CE",
];
const STATED: &str = "2023-06-29-1000000-PE";
const SETTLED: &str = "2023-06-28-1000000-PE";

impl Fixture {
    fn new(tag: &str) -> Self {
        let scratch = Scratch::new(tag);
        let root = scratch.root();
        let on = d(2019, 5, 30);
        for (at, key) in LADDER.iter().enumerate() {
            let price = 101 + i64::try_from(at).unwrap();
            put(
                root,
                "NIFTY",
                key,
                &[bar(on, 33_300, price), bar(on, 33_301, price)],
            );
        }
        for (at, key) in IMPOSSIBLE.iter().enumerate() {
            put(
                root,
                "NIFTY",
                key,
                &[bar(on, 40_000 + u32::try_from(at).unwrap(), 9)],
            );
        }
        // The ladder of the long-dated put's days, so its strike is plausible.
        for day in [d(2020, 6, 15), d(2020, 7, 1)] {
            put(
                root,
                "NIFTY",
                "2020-07-30-1000000-CE",
                &[bar(day, 33_400, 55)],
            );
        }
        put(
            root,
            "NIFTY",
            STATED,
            &[
                bar(d(2020, 6, 15), 34_000, 333),
                bar(d(2020, 6, 15), 34_001, 334),
                bar(d(2020, 7, 1), 35_000, 335),
            ],
        );
        Self { scratch }
    }

    fn root(&self) -> &Path {
        self.scratch.root()
    }

    /// The census as the pre-D-3331 import left it: every key counted.
    fn counted(&self) {
        let root = self.root();
        let mut rows = Vec::new();
        for key in LADDER.iter().chain(&IMPOSSIBLE) {
            rows.push(census_row_of(
                root,
                "NIFTY",
                key,
                YearMonth::new(2019, 5).unwrap(),
            ));
        }
        for month in [
            YearMonth::new(2020, 6).unwrap(),
            YearMonth::new(2020, 7).unwrap(),
        ] {
            rows.push(census_row_of(root, "NIFTY", STATED, month));
            rows.push(census_row_of(root, "NIFTY", "2020-07-30-1000000-CE", month));
        }
        partitioned(root, &rows);
    }
}

/// D-3331's reference strike and its bound, one spelling for the import and
/// the re-key: the LOWER median, and exactly ten times it admitted.
#[test]
fn the_ladder_rule_is_the_decoders_own() {
    assert_eq!(lower_median(&mut []), None);
    assert_eq!(lower_median(&mut [5]), Some(5));
    assert_eq!(lower_median(&mut [9, 1, 5, 7]), Some(5), "lower of 5 and 7");
    assert_eq!(lower_median(&mut [9, 1, 5]), Some(5));
    assert!(!strike_implausible(1_000, 100));
    assert!(strike_implausible(1_001, 100));
    assert!(
        !strike_implausible(i64::MAX, i64::MAX / 2),
        "saturates, never wraps"
    );
}

/// A stored option segment reads back as its expiry, strike and side; a
/// future or a segment not in the store's own spelling does not.
#[test]
fn a_stored_contract_segment_reads_back_as_its_identity() {
    assert_eq!(
        contract_parts(c(STATED)),
        Some((d(2023, 6, 29), 1_000_000, OptionSide::Put))
    );
    assert_eq!(
        contract_parts(c("2020-06-25-191250000-CE")),
        Some((d(2020, 6, 25), 191_250_000, OptionSide::Call))
    );
    for bad in [
        "2023-06-29-FUT",
        "2023-06-29-01000000-PE",
        "2023-02-30-100-CE",
        "2023-06-29-100-XE",
    ] {
        assert_eq!(contract_parts(c(bad)), None, "{bad}");
    }
}

/// The survey reads each key's months and its bars per IST day off the bar
/// files, and names what is not a store path.
#[test]
fn the_survey_reads_each_keys_days_from_its_bar_files() {
    let fixture = Fixture::new("survey");
    let root = fixture.root();
    std::fs::write(root.join("bars/gdfl/NSE/FNO/NIFTY/not-a-contract"), b"x").unwrap();
    let found = survey(root, None).unwrap();
    assert_eq!(found.keys.len(), LADDER.len() + IMPOSSIBLE.len() + 2);
    assert_eq!(found.unreadable, 0);
    assert_eq!(found.strays, 1, "{:?}", found.strays_named);
    let stated = found
        .keys
        .iter()
        .find(|key| key.contract == c(STATED))
        .unwrap();
    assert_eq!(stated.expiry, d(2023, 6, 29));
    assert_eq!(stated.bars(), 3);
    assert_eq!(
        stated
            .days
            .iter()
            .map(|(day, n)| (*day, *n))
            .collect::<Vec<_>>(),
        vec![(d(2020, 6, 15), 2), (d(2020, 7, 1), 1)]
    );
    assert_eq!(stated.months.len(), 2);
    // `only` narrows to the named underlyings, and nothing else is read.
    let none = survey(root, Some(&BTreeSet::from([sym("BANKNIFTY")]))).unwrap();
    assert!(none.keys.is_empty());
    assert_eq!(none.files, 0);
}

/// DB-A17 and DB-A18 on the miniature: the stated 2023-06-29 put moves to
/// 2023-06-28 (the every-underlying `vendor_redated` row), the three strikes
/// a hundred and sixty times the day's ladder are retired by name, and every
/// other key is kept where it is.
#[test]
fn the_plan_moves_a_stated_expiry_and_retires_an_impossible_strike() {
    let fixture = Fixture::new("plan");
    let found = survey(fixture.root(), None).unwrap();
    let decided = plan(&found, &Withdrawn::default());
    assert_eq!(decided.keys, 11);
    assert_eq!(decided.kept, 7, "the ladder and the July call");
    assert!(decided.unresolved.is_empty(), "{:?}", decided.unresolved);
    assert_eq!(decided.moves.len(), 1);
    let moved = &decided.moves[0];
    assert_eq!((moved.from, moved.to), (c(STATED), c(SETTLED)));
    assert_eq!(moved.kind, AliasKind::VendorRedated);
    assert_eq!(moved.bars, 3);
    assert_eq!(
        moved.months,
        vec![
            YearMonth::new(2020, 6).unwrap(),
            YearMonth::new(2020, 7).unwrap()
        ]
    );
    assert_eq!(decided.retired.len(), 3);
    for (record, key) in decided.retired.iter().zip(IMPOSSIBLE) {
        assert_eq!(record.key, c(key));
        assert_eq!(record.why, RetiredWhy::StrikeImplausible);
        assert_eq!(record.day, d(2019, 5, 30));
        // The lower median of the day's nine strikes: 1,200,000.
        assert_eq!(record.median, Some(1_200_000));
        assert_eq!((record.days, record.bars), (1, 1));
        assert!(
            record
                .refusal()
                .starts_with("contract_refused: NSE-FNO-NIFTY-2020-06-25-")
        );
    }
}

/// A key the decoder refuses on some of its days and admits on others is
/// left exactly as it is and named: no new file can drop a day from an old
/// one.
#[test]
fn a_key_refused_on_some_days_only_is_named_and_left() {
    let scratch = Scratch::new("partial");
    let root = scratch.root();
    let (one, two) = (d(2019, 5, 29), d(2019, 5, 30));
    // On the 29th the ladder sits near 12,000; on the 30th near 120.
    for (at, key) in [
        "2019-05-30-1150000-CE",
        "2019-05-30-1170000-PE",
        "2019-05-30-1180000-CE",
    ]
    .iter()
    .enumerate()
    {
        put(
            root,
            "NIFTY",
            key,
            &[bar(one, 33_300 + u32::try_from(at).unwrap(), 9)],
        );
    }
    for (at, key) in [
        "2019-05-30-11500-CE",
        "2019-05-30-11700-PE",
        "2019-05-30-11800-CE",
    ]
    .iter()
    .enumerate()
    {
        put(
            root,
            "NIFTY",
            key,
            &[bar(two, 33_300 + u32::try_from(at).unwrap(), 9)],
        );
    }
    put(
        root,
        "NIFTY",
        "2019-06-27-1200000-CE",
        &[bar(one, 34_000, 9), bar(two, 34_000, 9)],
    );
    let decided = plan(&survey(root, None).unwrap(), &Withdrawn::default());
    assert!(decided.retired.is_empty() && decided.moves.is_empty());
    let left = decided
        .unresolved
        .iter()
        .find(|one| one.key == "NSE-FNO-NIFTY-2019-06-27-1200000-CE")
        .unwrap();
    assert!(
        left.why.contains("refuses 1 of its 2 day(s)"),
        "{}",
        left.why
    );
    assert!(
        left.why.contains("2019-05-30: strike implausible"),
        "{}",
        left.why
    );
}

/// Two stored keys the decoder files under ONE contract, both holding the
/// same day, are two names of one contract that day — the decoder refuses
/// both (`ContractNamedTwice`) — so neither is merged; both are named.
#[test]
fn two_keys_of_one_decoded_contract_on_one_day_are_named_not_merged() {
    let scratch = Scratch::new("twice");
    let root = scratch.root();
    let on = d(2020, 6, 15);
    put(root, "NIFTY", STATED, &[bar(on, 34_000, 9)]);
    put(root, "NIFTY", SETTLED, &[bar(on, 34_500, 9)]);
    let decided = plan(&survey(root, None).unwrap(), &Withdrawn::default());
    assert!(decided.moves.is_empty(), "{:?}", decided.moves);
    let named = &decided.unresolved[0];
    assert_eq!(named.key, format!("NSE-FNO-NIFTY-{STATED}"));
    assert!(named.why.contains("ContractNamedTwice"), "{}", named.why);
}

/// THE RE-KEY, END TO END: the moved key's bars are published under the
/// decoded key as new files, the old key's files keep every byte, the census
/// counts the new months, the ledger names the old key superseded and the
/// three strikes retired, and the census's next epoch lists neither. A second
/// run writes nothing at all.
#[test]
fn applying_publishes_new_files_keeps_the_old_and_withdraws_them_from_the_census() {
    let fixture = Fixture::new("apply");
    fixture.counted();
    let root = fixture.root();
    let old_bytes = bytes_under(&key_dir(root, "NIFTY", STATED));
    let retired_bytes = bytes_under(&key_dir(root, "NIFTY", IMPOSSIBLE[0]));
    let (epoch, before) = census_keys(root);
    assert_eq!(epoch, 1);
    assert!(before.contains(&format!("NIFTY {STATED} 2020-06")));

    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    let done = apply(root, &found, &decided, &ledger).unwrap();

    // New files, the same bars.
    assert_eq!((done.months_written, done.months_already), (2, 0));
    for month in [
        YearMonth::new(2020, 6).unwrap(),
        YearMonth::new(2020, 7).unwrap(),
    ] {
        assert_eq!(
            held(root, "NIFTY", SETTLED, month),
            held(root, "NIFTY", STATED, month)
        );
    }
    // The old key's files and the retired key's files, byte for byte.
    assert_eq!(bytes_under(&key_dir(root, "NIFTY", STATED)), old_bytes);
    assert_eq!(
        bytes_under(&key_dir(root, "NIFTY", IMPOSSIBLE[0])),
        retired_bytes
    );
    // The ledger: one pointer, three refusals, each a line that reads back.
    let text = std::fs::read_to_string(lineage_path(root)).unwrap();
    assert_eq!(text.lines().count(), 4, "{text}");
    let first = text.lines().next().unwrap();
    assert!(
        first.starts_with(&format!(
            "superseded NSE-FNO-NIFTY-{STATED} by=NSE-FNO-NIFTY-{SETTLED} kind=vendor_redated months=2 bars=3 blake3="
        )),
        "{first}"
    );
    let read = withdrawn(root).unwrap();
    assert_eq!(read.counts(), (1, 3));
    let pointer = read.superseded(sym("NIFTY"), c(STATED)).unwrap();
    assert_eq!(pointer.new, c(SETTLED));
    assert_eq!(pointer, &done.moved[0]);
    // The census: the new months counted, then a new epoch without the four
    // withdrawn keys; the first epoch kept.
    let (epoch, after) = census_keys(root);
    assert_eq!(epoch, 2);
    assert!(epoch_path(root, Vendor::Gdfl, Segment::Fno, 1).exists());
    assert!(after.contains(&format!("NIFTY {SETTLED} 2020-06")));
    assert!(after.contains(&format!("NIFTY {SETTLED} 2020-07")));
    for gone in [STATED, IMPOSSIBLE[0], IMPOSSIBLE[1], IMPOSSIBLE[2]] {
        assert!(
            !after.iter().any(|key| key.contains(gone)),
            "{gone} still listed: {after:?}"
        );
    }
    assert_eq!(after.len(), before.len() - 5 + 2);
    let withdrawal = done.withdrawal.as_ref().unwrap();
    assert_eq!(
        (withdrawal.from_epoch, withdrawal.to_epoch, withdrawal.why),
        (Some(1), 2, "rekey")
    );

    // A SECOND RUN WRITES NOTHING: same ledger bytes, no third epoch, the new
    // months already present.
    let ledger_bytes = std::fs::read(lineage_path(root)).unwrap();
    let new_bytes = bytes_under(&key_dir(root, "NIFTY", SETTLED));
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let again = plan(&found, &ledger);
    // DONE, NOT PENDING (AG-02, rekey-3; this test pinned the same move
    // planned again until then): the move and the three retirements are
    // the ledger's, so nothing is to move or retire.
    assert!(again.moves.is_empty() && again.retired.is_empty());
    assert_eq!(
        (again.recorded_moves.len(), again.recorded_retired.len()),
        (1, 3)
    );
    assert_eq!(again.pending(), 0);
    assert!(!again.changes());
    let done = apply(root, &found, &again, &ledger).unwrap();
    assert_eq!((done.months_written, done.months_already), (0, 0));
    assert_eq!(done.lines_written, 0);
    assert!(done.withdrawal.is_none());
    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), ledger_bytes);
    assert_eq!(bytes_under(&key_dir(root, "NIFTY", SETTLED)), new_bytes);
    assert_eq!(census_keys(root).0, 2);
}

/// DB-A17's expected outcome, after the re-key: the import resumed on a
/// D-3331 build files a LATER day of the contract under the decoded key,
/// where it extends the one series the copied bars began, and the census
/// lists that month under the decoded key alone. Without the re-key the same
/// day would open a second series beside the stated key's.
#[test]
fn after_the_rekey_a_resumed_import_extends_the_one_decoded_series() {
    let fixture = Fixture::new("resume");
    fixture.counted();
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let _ = apply(root, &found, &plan(&found, &ledger), &ledger).unwrap();
    // THE RESUMED IMPORT'S DECODER files the stated name under the settled
    // expiry (D-3331), and its writer appends the later day there.
    assert_eq!(
        decoded_key(sym("NIFTY"), c(STATED)),
        Some((c(SETTLED), AliasKind::VendorRedated))
    );
    let july = YearMonth::new(2020, 7).unwrap();
    let copied = held(root, "NIFTY", STATED, july);
    let later = [bar(d(2020, 7, 2), 34_000, 336)];
    put(root, "NIFTY", SETTLED, &later);
    let cycles = census_store::record(
        root,
        Vendor::Gdfl,
        &[census_row_of(root, "NIFTY", SETTLED, july)],
    )
    .unwrap();
    assert_eq!(cycles.iter().map(|one| one.recorded).sum::<usize>(), 1);
    // ONE SERIES: the copied bars, then the later day, in time order.
    let mut expected = copied.clone();
    expected.extend(later);
    assert_eq!(held(root, "NIFTY", SETTLED, july), expected);
    // The stated key keeps exactly what it held, and the census lists the
    // month under the decoded key only.
    assert_eq!(held(root, "NIFTY", STATED, july), copied);
    let (_, keys) = census_keys(root);
    assert!(
        keys.contains(&format!("NIFTY {SETTLED} 2020-07")),
        "{keys:?}"
    );
    assert!(!keys.iter().any(|key| key.contains(STATED)), "{keys:?}");
}

/// A decoded key whose month already holds OTHER bars where the moved ones
/// would go — the import resumed on a D-3331 build between the plan and the
/// apply — refuses that move by name, and nothing of it is written; a
/// retirement in the same plan still lands. (The plan names the same move
/// itself when the bars are there before it is made:
/// `a_move_into_a_held_month_is_unresolved_in_the_plan`.)
#[test]
fn a_target_month_holding_other_bars_refuses_the_move_and_writes_nothing_of_it() {
    let fixture = Fixture::new("collide");
    fixture.counted();
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1, "{:?}", decided.unresolved);
    // THEN the decoded key gains a LATER day of July: the moved July bar
    // would precede it, and a month is never rewritten in place.
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 7, 20), 34_000, 999)]);
    let settled_june = key_dir(root, "NIFTY", SETTLED).join("1s/2020-06.bin");
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert!(done.moved.is_empty());
    assert_eq!(done.refused.len(), 1);
    assert!(
        done.refused[0].why.contains("would precede or contradict"),
        "{}",
        done.refused[0].why
    );
    assert!(
        !settled_june.exists(),
        "no month of a refused move is written"
    );
    let read = withdrawn(root).unwrap();
    assert_eq!(read.counts(), (0, 3), "the retirements still land");
    assert!(read.superseded(sym("NIFTY"), c(STATED)).is_none());
    let (_, after) = census_keys(root);
    assert!(
        after.contains(&format!("NIFTY {STATED} 2020-06")),
        "still listed"
    );
}

/// A single-file census cannot have a key withdrawn without a rewrite in
/// place, so the re-key refuses before anything is written, naming the
/// migration that comes first.
#[test]
fn a_single_file_census_is_refused_before_anything_is_written() {
    let fixture = Fixture::new("single");
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let decided = plan(&found, &Withdrawn::default());
    let why = apply(root, &found, &decided, &Withdrawn::default()).unwrap_err();
    assert!(why.contains("census-migrate gdfl"), "{why}");
    assert!(!key_dir(root, "NIFTY", SETTLED).exists());
    assert!(!lineage_path(root).exists());
}

/// A survey that could not read a month refuses the apply: a ladder missing
/// a file is not the day's ladder.
#[test]
fn an_unreadable_month_refuses_the_apply() {
    let fixture = Fixture::new("unreadable");
    fixture.counted();
    let root = fixture.root();
    let mut found = survey(root, None).unwrap();
    found.note_unreadable(PathBuf::from("x"), "locked".to_owned());
    let decided = plan(&found, &Withdrawn::default());
    let why = apply(root, &found, &decided, &Withdrawn::default()).unwrap_err();
    assert!(why.contains("could not read 1 month file"), "{why}");
    assert!(!lineage_path(root).exists());
}

/// The ledger the re-key and its readers share: torn lines and the other
/// record kinds are passed over; a re-key line that does not parse, or a key
/// both superseded and retired, refuses the whole ledger by line.
#[test]
fn the_ledger_is_read_whole_or_refused_by_line() {
    let pointer = SupersededRecord {
        underlying: sym("NIFTY"),
        old: c(STATED),
        new: c(SETTLED),
        kind: AliasKind::VendorRedated,
        months: 2,
        bars: 3,
        digest: [7; 32],
    };
    let refusal = RetiredRecord {
        underlying: sym("NIFTY"),
        key: c(IMPOSSIBLE[0]),
        why: RetiredWhy::StrikeImplausible,
        day: d(2019, 5, 30),
        median: Some(1_180_000),
        days: 1,
        bars: 5,
    };
    let text = format!(
        "alias * stated=2023-06-29 settled=2023-06-28 kind=vendor_redated first_day=2023-06-28 contracts=12634 v=1\n{}\nsuperseded NSE-FNO-NIFTY-x (torn)\n{}\n",
        pointer.line(),
        refusal.line()
    );
    let read = Withdrawn::parse(&text).unwrap();
    assert_eq!(read.counts(), (1, 1));
    assert!(!read.torn_tail);
    let row = |contract: &str| EntryKey {
        contract: Some(c(contract)),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: sym("NIFTY"),
        timeframe: RUNG,
        month: YearMonth::new(2020, 6).unwrap(),
    };
    assert!(read.holds(&row(STATED)));
    assert!(read.holds(&row(IMPOSSIBLE[0])));
    assert!(!read.holds(&row(SETTLED)));
    assert!(!read.holds(&EntryKey {
        segment: Segment::Index,
        ..row(STATED)
    }));
    assert!(Withdrawn::parse(&pointer.line()).unwrap().torn_tail);
    let bad = Withdrawn::parse(&format!(
        "{}\nretired NSE-FNO-NIFTY-{STATED} why=x\n",
        pointer.line()
    ))
    .unwrap_err();
    assert!(
        bad.contains("line 2 does not parse as a `retired` record"),
        "{bad}"
    );
    let both = RetiredRecord {
        key: c(STATED),
        why: RetiredWhy::ExpiryUnsettled,
        median: None,
        ..refusal
    };
    // AG-02 (rekey-5): the refusal names BOTH lines, so the append-only file
    // can be answered by a later line rather than an edit.
    let bad = Withdrawn::parse(&format!("{}\n{}\n", pointer.line(), both.line())).unwrap_err();
    assert!(
        bad.contains(&format!(
            "line 1 supersedes NSE-FNO-NIFTY-{STATED} by NSE-FNO-NIFTY-{SETTLED} and line 2 \
             retires it; a key is one or the other"
        )),
        "{bad}"
    );
}

/// The re-key's two records are one line each and read back exactly; a
/// field out of place, a strike the median admits, or a key superseded by
/// itself is no record.
#[test]
fn the_rekey_records_read_back_exactly_and_refuse_anything_else() {
    let pointer = SupersededRecord {
        underlying: sym("NIFTY"),
        old: c(STATED),
        new: c(SETTLED),
        kind: AliasKind::VendorRedated,
        months: 2,
        bars: 3,
        digest: [0xab; 32],
    };
    let line = pointer.line();
    assert_eq!(
        line,
        format!(
            "superseded NSE-FNO-NIFTY-{STATED} by=NSE-FNO-NIFTY-{SETTLED} kind=vendor_redated months=2 bars=3 blake3={} v=1",
            "ab".repeat(32)
        )
    );
    assert_eq!(SupersededRecord::parse(&line), Some(pointer));
    for bad in [
        line.replace("superseded", "Superseded"),
        line.replace(
            &format!("by=NSE-FNO-NIFTY-{SETTLED}"),
            &format!("by=NSE-FNO-NIFTY-{STATED}"),
        ),
        line.replace("by=NSE-FNO-NIFTY", "by=NSE-FNO-BANKNIFTY"),
        line.replace("kind=vendor_redated", "kind=renamed"),
        line.replace("months=2", "months=0"),
        line.replace("bars=3", "bars=03"),
        line.replace(&"ab".repeat(32), &"AB".repeat(32)),
        line.replace(&"ab".repeat(32), &"ab".repeat(31)),
        line.replace(" v=1", " v=2"),
        format!("{line} "),
    ] {
        assert_eq!(SupersededRecord::parse(&bad), None, "{bad:?}");
    }
    let refusal = RetiredRecord {
        underlying: sym("NIFTY"),
        key: c(IMPOSSIBLE[0]),
        why: RetiredWhy::StrikeImplausible,
        day: d(2019, 5, 30),
        median: Some(1_180_000),
        days: 1,
        bars: 5,
    };
    let line = refusal.line();
    assert_eq!(
        line,
        "retired NSE-FNO-NIFTY-2020-06-25-191250000-CE why=strike_implausible day=2019-05-30 median=1180000 days=1 bars=5 v=1"
    );
    assert_eq!(RetiredRecord::parse(&line), Some(refusal));
    // The refusal sentence is what a route matches to answer 410: it starts
    // with the word and a colon, and nothing else is taken for it.
    assert!(
        crate::gdfl_lineage::is_contract_refused(&refusal.refusal()),
        "{}",
        refusal.refusal()
    );
    for not in [
        "contract_refused",
        "contract_refused NSE-FNO-NIFTY",
        "contract_refusedX: NSE-FNO-NIFTY",
        "the month: contract_refused: NSE-FNO-NIFTY",
        "",
    ] {
        assert!(!crate::gdfl_lineage::is_contract_refused(not), "{not:?}");
    }
    let unsettled = RetiredRecord {
        why: RetiredWhy::ExpiryUnsettled,
        median: None,
        ..refusal
    };
    assert_eq!(RetiredRecord::parse(&unsettled.line()), Some(unsettled));
    assert!(
        unsettled
            .refusal()
            .contains("expiry_unsettled from 2019-05-30")
    );
    assert!(crate::gdfl_lineage::is_contract_refused(
        &unsettled.refusal()
    ));
    for bad in [
        line.replace("median=1180000", "median=-"),
        line.replace("median=1180000", "median=19125000"),
        line.replace("why=strike_implausible", "why=expiry_unsettled"),
        line.replace("days=1", "days=0"),
        line.replace("day=2019-05-30", "day=2019-5-30"),
        line.replace(" v=1", ""),
    ] {
        assert_eq!(RetiredRecord::parse(&bad), None, "{bad:?}");
    }
}

/// A recount after the re-key walks the old keys' bar files, which are kept,
/// and must not count them back: the census stopped listing them on purpose.
#[test]
fn a_recount_does_not_count_a_withdrawn_key_back() {
    let fixture = Fixture::new("recount");
    fixture.counted();
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    apply(root, &found, &plan(&found, &ledger), &ledger).unwrap();
    let (_, settled) = census_keys(root);
    let span = census_store::Span {
        from: d(2019, 5, 1),
        to: None,
    };
    let recounted = census_store::recount(root, Vendor::Gdfl, span, None).unwrap();
    assert_eq!(recounted.withdrawn, 5, "four keys, five months");
    let (_, after) = census_keys(root);
    assert_eq!(after, settled, "nothing withdrawn came back");
}

// ═══ AG-02: done versus pending, one target per key, held months ══════════

/// The fixture's plan applied whole: what every "after" below starts from.
fn applied(tag: &str) -> Fixture {
    let fixture = Fixture::new(tag);
    fixture.counted();
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let done = apply(root, &found, &plan(&found, &ledger), &ledger).unwrap();
    assert_eq!((done.moved.len(), done.lines_written), (1, 4));
    fixture
}

/// The ledger's text plus `lines`, written back: a ledger as another run (or
/// a later alias table) left it. Only scratch stores are written.
fn append_ledger(root: &Path, lines: &[String]) {
    let path = lineage_path(root);
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
}

/// A superseded line of `old` by `new` under NIFTY, invented digest.
fn pointer(old: &str, new: &str) -> SupersededRecord {
    SupersededRecord {
        underlying: sym("NIFTY"),
        old: c(old),
        new: c(new),
        kind: AliasKind::VendorRedated,
        months: 2,
        bars: 3,
        digest: [7; 32],
    }
}

/// A retired line of `key` under NIFTY.
fn refusal_of(key: &str) -> RetiredRecord {
    RetiredRecord {
        underlying: sym("NIFTY"),
        key: c(key),
        why: RetiredWhy::ExpiryUnsettled,
        day: d(2019, 5, 30),
        median: None,
        days: 1,
        bars: 1,
    }
}

/// rekey-3: AFTER A FULL APPLY NOTHING IS PENDING. A second plan names the
/// move and the three retirements as recorded, a second apply writes no
/// month, no line and no epoch: the acceptance R4-21, ING-10 and
/// DATA-REKEY-APPLY read.
#[test]
fn a_second_plan_after_a_full_apply_has_nothing_pending() {
    let fixture = applied("pending");
    let root = fixture.root();
    let ledger_bytes = std::fs::read(lineage_path(root)).unwrap();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let again = plan(&found, &ledger);
    assert_eq!(
        again.pending(),
        0,
        "{:?} {:?}",
        again.moves,
        again.unresolved
    );
    assert!(again.unresolved.is_empty());
    let done = apply(root, &found, &again, &ledger).unwrap();
    assert_eq!(
        (done.months_written, done.months_already, done.lines_written),
        (0, 0, 0)
    );
    assert!(done.withdrawal.is_none(), "no epoch installed");
    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), ledger_bytes);
}

/// rekey-3: a recorded move is DONE — out of `moves`, out of `changes()`,
/// listed apart with its own counts — while a move not yet recorded stays
/// pending beside it.
#[test]
fn a_recorded_move_is_reported_as_recorded_not_to_move() {
    let fixture = applied("recorded");
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let decided = plan(&found, &withdrawn(root).unwrap());
    assert!(decided.moves.is_empty());
    let recorded = &decided.recorded_moves[0];
    assert_eq!((recorded.from, recorded.to), (c(STATED), c(SETTLED)));
    assert_eq!((recorded.bars, recorded.months.len()), (3, 2));
    assert_eq!(
        decided
            .recorded_retired
            .iter()
            .map(|record| record.key)
            .collect::<Vec<_>>(),
        IMPOSSIBLE.iter().map(|key| c(key)).collect::<Vec<_>>()
    );
    assert!(!decided.changes());
    // THE SAME STORE BEFORE ITS APPLY: pending, nothing recorded.
    let fresh = Fixture::new("recorded-before");
    let before = plan(&survey(fresh.root(), None).unwrap(), &Withdrawn::default());
    assert_eq!((before.moves.len(), before.retired.len()), (1, 3));
    assert!(before.recorded_moves.is_empty() && before.recorded_retired.is_empty());
    assert_eq!(before.pending(), 4);
}

/// rekey-3: the ledger totals a receipt prints follow `--underlying`; an
/// underlying with no record counts none, and no filter counts them all.
#[test]
fn the_ledger_totals_follow_the_underlying_filter() {
    let banknifty = SupersededRecord {
        underlying: sym("BANKNIFTY"),
        ..pointer(STATED, SETTLED)
    };
    let text = format!(
        "{}\n{}\n{}\n",
        pointer(STATED, SETTLED).line(),
        banknifty.line(),
        refusal_of(IMPOSSIBLE[0]).line()
    );
    let read = Withdrawn::parse(&text).unwrap();
    let only = |names: &[&str]| names.iter().map(|name| sym(name)).collect::<BTreeSet<_>>();
    assert_eq!(read.counts_for(None), (2, 1));
    assert_eq!(read.counts_for(Some(&only(&["NIFTY"]))), (1, 1));
    assert_eq!(read.counts_for(Some(&only(&["BANKNIFTY"]))), (1, 0));
    assert_eq!(read.counts_for(Some(&only(&["FINNIFTY"]))), (0, 0));
    assert_eq!(
        read.counts_for(Some(&only(&["NIFTY", "BANKNIFTY"]))),
        (2, 1)
    );
}

/// rekey-5: one key superseded by two different keys is refused, naming both
/// lines; a renamed underlying's same contract is another key.
#[test]
fn a_key_superseded_twice_to_different_targets_is_refused_naming_both_lines() {
    let first = pointer(STATED, SETTLED);
    let second = pointer(STATED, "2023-06-27-1000000-PE");
    let text = format!(
        "alias * stated=2023-06-29 settled=2023-06-28 kind=vendor_redated first_day=2023-06-28 contracts=1 v=1\n{}\n{}\n",
        first.line(),
        second.line()
    );
    let why = Withdrawn::parse(&text).unwrap_err();
    assert!(
        why.contains("line 2 supersedes NSE-FNO-NIFTY-2023-06-29-1000000-PE by NSE-FNO-NIFTY-2023-06-28-1000000-PE and line 3 by NSE-FNO-NIFTY-2023-06-27-1000000-PE"),
        "{why}"
    );
    // THE SAME CONTRACT UNDER ANOTHER UNDERLYING (a rename) is its own key.
    let renamed = SupersededRecord {
        underlying: sym("NIFTYNXT50"),
        ..second
    };
    let read = Withdrawn::parse(&format!("{}\n{}\n", first.line(), renamed.line())).unwrap();
    assert_eq!(read.counts(), (2, 0));
}

/// rekey-5: the same `superseded` line twice — the same fact — reads as
/// once, at its first line. (A second line to the same key with other
/// counts read as the first until repair r1e; it is now refused naming both
/// lines: `one_move_or_retirement_recorded_twice_with_another_account_is_refused`.)
#[test]
fn an_identical_repeated_superseded_line_is_idempotent() {
    let line = pointer(STATED, SETTLED).line();
    let read = Withdrawn::parse(&format!("{line}\n{line}\n{line}\n")).unwrap();
    assert_eq!(read.counts(), (1, 0));
    assert_eq!(read.line_of(sym("NIFTY"), c(STATED)), Some(1));
    assert_eq!(read.superseded(sym("NIFTY"), c(STATED)).unwrap().bars, 3);
    assert_eq!(read.superseded_records().count(), 1);
}

/// rekey-5: superseded and retired, in EITHER order, is refused naming both
/// lines; a repeated `retired` line reads as once.
#[test]
fn a_key_superseded_and_retired_is_refused_naming_both_lines() {
    let moved = pointer(STATED, SETTLED).line();
    let gone = refusal_of(STATED).line();
    let why = Withdrawn::parse(&format!("{moved}\nlineage x\n{gone}\n")).unwrap_err();
    assert!(
        why.contains("line 1 supersedes NSE-FNO-NIFTY-2023-06-29-1000000-PE by NSE-FNO-NIFTY-2023-06-28-1000000-PE and line 3 retires it"),
        "{why}"
    );
    // OUT OF ORDER: the retirement first.
    let why = Withdrawn::parse(&format!("{gone}\n{moved}\n")).unwrap_err();
    assert!(
        why.contains("line 1 retires NSE-FNO-NIFTY-2023-06-29-1000000-PE and line 2 supersedes it by NSE-FNO-NIFTY-2023-06-28-1000000-PE"),
        "{why}"
    );
    let read = Withdrawn::parse(&format!("{gone}\n{gone}\n")).unwrap();
    assert_eq!(read.counts(), (0, 1));
    assert_eq!(read.retired_records().count(), 1);
}

/// rekey-5, the writer: a key the ledger supersedes by ANOTHER key is never
/// moved again; the plan names it with the line it contradicts.
#[test]
fn the_plan_never_retargets_a_superseded_key() {
    let fixture = Fixture::new("retarget");
    let root = fixture.root();
    append_ledger(root, &[pointer(STATED, "2023-06-27-1000000-PE").line()]);
    let decided = plan(&survey(root, None).unwrap(), &withdrawn(root).unwrap());
    assert!(decided.moves.is_empty(), "{:?}", decided.moves);
    let left = decided
        .unresolved
        .iter()
        .find(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}"))
        .unwrap();
    assert!(
        left.why
            .contains("ledger line 1 supersedes it by NSE-FNO-NIFTY-2023-06-27-1000000-PE"),
        "{}",
        left.why
    );
    // A RECORDED MOVE WHOSE KEY GREW since is left too, never moved twice.
    let fixture = applied("grown");
    let root = fixture.root();
    put(root, "NIFTY", STATED, &[bar(d(2020, 7, 2), 30_000, 337)]);
    let decided = plan(&survey(root, None).unwrap(), &withdrawn(root).unwrap());
    assert!(decided.moves.is_empty() && decided.recorded_moves.is_empty());
    assert!(
        decided.unresolved[0]
            .why
            .contains("with 3 bar(s) in 2 month(s), and it holds 4 bar(s)"),
        "{}",
        decided.unresolved[0].why
    );
}

/// rekey-5, the writer: a key the ledger supersedes is never retired, and a
/// key it retires is never moved or kept silently.
#[test]
fn the_plan_never_retires_a_superseded_key() {
    let fixture = Fixture::new("noretire");
    let root = fixture.root();
    append_ledger(
        root,
        &[
            pointer(IMPOSSIBLE[0], "2020-06-25-1150000-CE").line(),
            refusal_of(STATED).line(),
            refusal_of(LADDER[0]).line(),
        ],
    );
    let decided = plan(&survey(root, None).unwrap(), &withdrawn(root).unwrap());
    assert!(
        decided
            .retired
            .iter()
            .all(|record| record.key != c(IMPOSSIBLE[0]))
    );
    assert!(decided.moves.is_empty(), "the retired put is not moved");
    let why = |key: &str| {
        decided
            .unresolved
            .iter()
            .find(|one| one.key == format!("NSE-FNO-NIFTY-{key}"))
            .map_or_else(
                || panic!("{key}: {:?}", decided.unresolved),
                |one| one.why.clone(),
            )
    };
    assert!(why(IMPOSSIBLE[0]).contains("ledger line 1 supersedes it by"));
    assert!(why(STATED).contains("ledger line 2 retires it: a retired key is not moved"));
    assert!(why(LADDER[0]).contains("admits every day where it is, and ledger line 3 retires it"));
}

/// rekey-5: APPLY THEN PARSE ROUND-TRIPS for every plan the planner makes,
/// over every ledger shape here, and a plan that would append a line the
/// parser refuses is refused before a byte is written.
#[test]
fn apply_never_writes_a_line_its_parser_refuses() {
    let shapes: [&[String]; 5] = [
        &[],
        &[pointer(STATED, "2023-06-27-1000000-PE").line()],
        &[pointer(IMPOSSIBLE[0], "2020-06-25-1150000-CE").line()],
        &[refusal_of(STATED).line()],
        &[pointer("2023-06-29-1050000-PE", STATED).line()],
    ];
    for (at, lines) in shapes.iter().enumerate() {
        let fixture = Fixture::new(&format!("roundtrip{at}"));
        fixture.counted();
        let root = fixture.root();
        append_ledger(root, lines);
        for _ in 0..2 {
            let found = survey(root, None).unwrap();
            let ledger = withdrawn(root).unwrap();
            let decided = plan(&found, &ledger);
            let _ = apply(root, &found, &decided, &ledger).unwrap();
            assert!(withdrawn(root).is_ok(), "shape {at}: {:?}", withdrawn(root));
        }
    }
    // A PLAN BUILT BY HAND that re-targets a superseded key is refused whole.
    let fixture = Fixture::new("handmade");
    fixture.counted();
    let root = fixture.root();
    append_ledger(root, &[pointer(STATED, "2023-06-27-1000000-PE").line()]);
    let before = std::fs::read(lineage_path(root)).unwrap();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let mut forced = plan(&found, &Withdrawn::default());
    assert_eq!(forced.moves.len(), 1);
    let why = apply(root, &found, &forced, &ledger).unwrap_err();
    assert!(
        why.contains("ledger line 1 supersedes it by NSE-FNO-NIFTY-2023-06-27-1000000-PE"),
        "{why}"
    );
    assert!(why.contains("nothing was written"), "{why}");
    assert!(!key_dir(root, "NIFTY", SETTLED).exists());
    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), before);
    // Retiring a superseded key, moving a retired one, and moving one key
    // twice or both moving and retiring it: each refused the same way.
    forced.moves.clear();
    forced.retired = vec![RetiredRecord {
        key: c(STATED),
        ..refusal_of(STATED)
    }];
    let why = apply(root, &found, &forced, &ledger).unwrap_err();
    assert!(why.contains("which ledger line 1 supersedes"), "{why}");
    let retired = Withdrawn::parse(&format!("{}\n", refusal_of(STATED).line())).unwrap();
    let moving = plan(&found, &Withdrawn::default());
    let why = apply(root, &found, &moving, &retired).unwrap_err();
    assert!(why.contains("which ledger line 1 retires"), "{why}");
    let mut twice = moving.clone();
    let mut again = twice.moves[0].clone();
    again.to = c("2023-06-27-1000000-PE");
    twice.moves.push(again);
    let why = apply(root, &found, &twice, &Withdrawn::default()).unwrap_err();
    assert!(why.contains("to two keys"), "{why}");
    let mut both = moving;
    both.retired.push(RetiredRecord {
        key: c(STATED),
        ..refusal_of(STATED)
    });
    let why = apply(root, &found, &both, &Withdrawn::default()).unwrap_err();
    assert!(why.contains("both moves and retires"), "{why}");
    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), before);
    // A SECOND ACCOUNT (repair r1e): moving again a key the ledger records
    // as moved to that very key, and retiring a retired key with another
    // account, would each append a line the parser now refuses.
    let recorded = Withdrawn::parse(&format!("{}\n", pointer(STATED, SETTLED).line())).unwrap();
    let moving = plan(&found, &Withdrawn::default());
    let why = apply(root, &found, &moving, &recorded).unwrap_err();
    assert!(why.contains("which ledger line 1 already records"), "{why}");
    let mut other = refusal_of(IMPOSSIBLE[0]);
    other.bars = 5;
    let held_line = Withdrawn::parse(&format!("{}\n", other.line())).unwrap();
    let mut retiring = plan(&found, &Withdrawn::default());
    retiring.moves.clear();
    let why = apply(root, &found, &retiring, &held_line).unwrap_err();
    assert!(
        why.contains("and ledger line 1 retires it as")
            && why.contains("one retirement has one account"),
        "{why}"
    );
    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), before);
}

/// rekey-2: A MOVE INTO A MONTH THAT ALREADY HOLDS LATER BARS is known to
/// the plan, from the month headers the survey read: unresolved by name, so
/// "0 unresolved" predicts an apply that refuses nothing.
#[test]
fn a_move_into_a_held_month_is_unresolved_in_the_plan() {
    let fixture = Fixture::new("held");
    fixture.counted();
    let root = fixture.root();
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 7, 20), 34_000, 999)]);
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert!(decided.moves.is_empty(), "{:?}", decided.moves);
    let left = decided
        .unresolved
        .iter()
        .find(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}"))
        .unwrap();
    assert!(
        left.why.starts_with(&format!(
            "target month held: NSE-FNO-NIFTY-{SETTLED} 2020-07 already holds 1 bar(s)"
        )),
        "{}",
        left.why
    );
    assert!(left.why.contains("R4-21 item 1"), "{}", left.why);
    // AND APPLY AGREES: nothing of the move is written, nothing refused at
    // apply that the plan did not name.
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert!(done.refused.is_empty() && done.moved.is_empty());
    assert!(
        !key_dir(root, "NIFTY", SETTLED)
            .join("1s/2020-06.bin")
            .exists()
    );
    // A LATER BAR IN AN EARLIER MONTH only: the moved months follow it, so
    // the move is planned and lands.
    let fixture = Fixture::new("held-earlier");
    fixture.counted();
    let root = fixture.root();
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 5, 20), 34_000, 998)]);
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1, "{:?}", decided.unresolved);
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert!(done.refused.is_empty(), "{:?}", done.refused);
    assert_eq!(done.months_written, 2);
}

/// rekey-2: two sources whose spans interleave in one target month are
/// named in the plan, the way apply's merge would refuse them.
#[test]
fn moves_whose_spans_interleave_in_one_target_month_are_unresolved() {
    let june = YearMonth::new(2020, 6).unwrap();
    let stored = |contract: &str, first: i64, last: i64| Stored {
        underlying: sym("NIFTY"),
        contract: c(contract),
        expiry: d(2023, 6, 29),
        strike: 1_000_000,
        side: OptionSide::Put,
        months: vec![Month {
            month: june,
            rows: 2,
            first_ts_micros: first,
            last_ts_micros: last,
        }],
        days: BTreeMap::from([(d(2020, 6, 15), 2)]),
    };
    let one = stored(STATED, 10, 30);
    let two = stored("2023-06-30-1000000-PE", 20, 40);
    let held: HashMap<(Symbol, Contract), &Stored> = [&one, &two]
        .into_iter()
        .map(|key| ((key.underlying, key.contract), key))
        .collect();
    let move_of = |key: &Stored| Move {
        underlying: key.underlying,
        from: key.contract,
        to: c(SETTLED),
        kind: AliasKind::VendorRedated,
        months: vec![june],
        bars: 2,
    };
    let mut out = Plan {
        moves: vec![move_of(&one), move_of(&two)],
        ..Plan::default()
    };
    held_months(&mut out, &held, Path::new(""));
    assert!(out.moves.is_empty());
    assert_eq!(out.unresolved.len(), 2);
    assert!(
        out.unresolved[0]
            .why
            .contains("interleaved or repeated seconds in 2020-06")
    );
    // Apart in time, they would be merged and planned.
    let two = stored("2023-06-30-1000000-PE", 31, 40);
    let held: HashMap<(Symbol, Contract), &Stored> = [&one, &two]
        .into_iter()
        .map(|key| ((key.underlying, key.contract), key))
        .collect();
    let mut out = Plan {
        moves: vec![move_of(&one), move_of(&two)],
        ..Plan::default()
    };
    held_months(&mut out, &held, Path::new(""));
    assert_eq!(out.moves.len(), 2);
}

/// FM-273, A RUN CUT SHORT, the next plan tells done from pending truly.
/// After step 1 (the target holds the copy, no ledger line): pending, not a
/// second name, and the next apply finishes it. After step 3 (lines
/// written, no epoch): recorded, nothing pending, and the next apply
/// installs only the epoch.
#[test]
fn a_rekey_cut_short_is_reported_truly_and_finished_by_the_next_apply() {
    // AFTER STEP 1.
    let fixture = Fixture::new("cut1");
    fixture.counted();
    let root = fixture.root();
    let mut copy = held(root, "NIFTY", STATED, YearMonth::new(2020, 6).unwrap());
    copy.extend(held(
        root,
        "NIFTY",
        STATED,
        YearMonth::new(2020, 7).unwrap(),
    ));
    put(root, "NIFTY", SETTLED, &copy);
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1, "{:?}", decided.unresolved);
    assert!(decided.unresolved.is_empty(), "{:?}", decided.unresolved);
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert_eq!((done.months_written, done.months_already), (0, 2));
    assert_eq!(done.lines_written, 4);
    assert_eq!(
        done.withdrawal.as_ref().map(|epoch| epoch.to_epoch),
        Some(2)
    );
    let ledger_text = std::fs::read_to_string(lineage_path(root)).unwrap();

    // AFTER STEP 3: the same copy and the same four lines, the census still
    // at its first epoch.
    let fixture = Fixture::new("cut3");
    fixture.counted();
    let root = fixture.root();
    put(root, "NIFTY", SETTLED, &copy);
    let counted: Vec<Held> = [6, 7]
        .into_iter()
        .map(|m| census_row_of(root, "NIFTY", SETTLED, YearMonth::new(2020, m).unwrap()))
        .collect();
    let _ = census_store::record(root, Vendor::Gdfl, &counted).unwrap();
    append_ledger(
        root,
        &ledger_text.lines().map(str::to_owned).collect::<Vec<_>>(),
    );
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(
        (decided.recorded_moves.len(), decided.recorded_retired.len()),
        (1, 3)
    );
    assert!(
        decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
        "{:?}",
        decided.unresolved
    );
    // STEP 4 IS PENDING (AG-02 repair r1e, blocking item 1; this test read
    // `pending 0` here and then watched the apply install an epoch): the
    // census still lists the four keys the ledger now withdraws.
    let mut listed: Vec<String> = IMPOSSIBLE
        .iter()
        .chain([&STATED])
        .map(|key| format!("NSE-FNO-NIFTY-{key}"))
        .collect();
    listed.sort();
    assert_eq!(decided.census, CensusStep::Listed(listed));
    assert_eq!(decided.pending(), 4);
    assert!(decided.changes());
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert_eq!((done.months_written, done.lines_written), (0, 0));
    assert_eq!(
        done.withdrawal.as_ref().map(|epoch| epoch.to_epoch),
        Some(2)
    );
    let (_, keys) = census_keys(root);
    assert!(!keys.iter().any(|key| key.contains(STATED)), "{keys:?}");
    assert!(
        keys.contains(&format!("NIFTY {SETTLED} 2020-07")),
        "{keys:?}"
    );
    // AND THEN DONE: `pending 0` means an apply of that plan writes no
    // month, appends no line and installs no epoch.
    let found = survey(root, None).unwrap();
    let after = plan(&found, &ledger);
    assert_eq!(after.census, CensusStep::Clear);
    assert_eq!((after.pending(), after.changes()), (0, false));
    let bytes = std::fs::read(lineage_path(root)).unwrap();
    let none = apply(root, &found, &after, &ledger).unwrap();
    assert_eq!((none.months_written, none.lines_written), (0, 0));
    assert!(none.withdrawal.is_none());
    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), bytes);
}

/// The ladder and the three impossible strikes of [`Fixture`] and nothing
/// else, counted: a store whose only re-key work is three retirements.
fn retirements_only(tag: &str) -> Scratch {
    let scratch = Scratch::new(tag);
    let root = scratch.root();
    let on = d(2019, 5, 30);
    let month = YearMonth::new(2019, 5).unwrap();
    let mut rows = Vec::new();
    for (at, key) in LADDER.iter().enumerate() {
        let price = 101 + i64::try_from(at).unwrap();
        put(
            root,
            "NIFTY",
            key,
            &[bar(on, 33_300, price), bar(on, 33_301, price)],
        );
        rows.push(census_row_of(root, "NIFTY", key, month));
    }
    for (at, key) in IMPOSSIBLE.iter().enumerate() {
        put(
            root,
            "NIFTY",
            key,
            &[bar(on, 40_000 + u32::try_from(at).unwrap(), 9)],
        );
        rows.push(census_row_of(root, "NIFTY", key, month));
    }
    partitioned(root, &rows);
    scratch
}

/// AG-02 repair r1e, blocking item 1, THE LOCK RACE: an apply whose only
/// work is retirements skips step 2, appends its lines at step 3, and meets
/// a census lock another ingest holds at step 4. The next plan names the
/// census step pending (also under `--underlying` naming an underlying
/// with no keys, because the apply withdraws the whole ledger's keys); once
/// the lock is free the next apply installs the epoch and nothing else, and
/// after it `pending 0` means an apply changes nothing.
#[test]
fn a_retirements_only_apply_stopped_at_the_census_lock_leaves_step_4_pending() {
    let scratch = retirements_only("lockrace");
    let root = scratch.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let first = plan(&found, &ledger);
    assert_eq!(
        (
            first.moves.len(),
            first.retired.len(),
            first.unresolved.len()
        ),
        (0, 3, 0),
        "{:?}",
        first.unresolved
    );
    assert_eq!(
        (first.census.clone(), first.pending()),
        (CensusStep::Clear, 3)
    );

    // ANOTHER INGEST HOLDS THE CENSUS LOCK.
    let lock_path = census_store::target_for(root, Vendor::Gdfl, Segment::Fno)
        .unwrap()
        .lock_path();
    let holder = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .unwrap();
    holder.lock().unwrap();
    let why = apply(root, &found, &first, &ledger).unwrap_err();
    assert!(
        why.contains("another ingest holds the census lock"),
        "{why}"
    );
    let ledger = withdrawn(root).unwrap();
    assert_eq!(ledger.counts(), (0, 3), "step 3 ran: the lines are written");
    assert_eq!(census_keys(root).0, 1, "step 4 did not run");

    let found = survey(root, None).unwrap();
    let second = plan(&found, &ledger);
    assert_eq!(second.recorded_retired.len(), 3);
    assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
    let listed: Vec<String> = IMPOSSIBLE
        .iter()
        .map(|key| format!("NSE-FNO-NIFTY-{key}"))
        .collect();
    assert_eq!(second.census, CensusStep::Listed(listed));
    assert_eq!((second.pending(), second.changes()), (3, true));
    let only: BTreeSet<Symbol> = [sym("BANKNIFTY")].into_iter().collect();
    let narrowed = plan(&survey(root, Some(&only)).unwrap(), &ledger);
    assert_eq!((narrowed.keys, narrowed.pending()), (0, 3));

    // THE LOCK FREED: the next apply finishes step 4 and only it.
    holder.unlock().unwrap();
    drop(holder);
    let done = apply(root, &found, &second, &ledger).unwrap();
    assert_eq!((done.months_written, done.lines_written), (0, 0));
    assert_eq!(
        done.withdrawal.as_ref().map(|epoch| epoch.to_epoch),
        Some(2)
    );
    let found = survey(root, None).unwrap();
    let third = plan(&found, &ledger);
    assert_eq!(
        (third.census.clone(), third.pending(), third.changes()),
        (CensusStep::Clear, 0, false)
    );
    let bytes = std::fs::read(lineage_path(root)).unwrap();
    let none = apply(root, &found, &third, &ledger).unwrap();
    assert_eq!((none.months_written, none.lines_written), (0, 0));
    assert!(none.withdrawal.is_none(), "PENDING 0 installs no epoch");
    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), bytes);
}

/// A census step the plan cannot answer is pending, never done: a survey
/// built by hand names no store (not read); a garbage current epoch will not
/// read (unreadable, naming the file). A ledger that withdraws nothing needs
/// no census at all, because no census can list a key it withdraws.
#[test]
fn a_census_not_read_or_unreadable_is_pending_never_done() {
    let hand = plan(&Survey::default(), &Withdrawn::default());
    assert_eq!(
        (hand.census.clone(), hand.pending()),
        (CensusStep::NotRead, 1)
    );
    assert!(hand.changes());

    let fixture = applied("garbage-census");
    let root = fixture.root();
    std::fs::write(
        epoch_path(root, Vendor::Gdfl, Segment::Fno, 9),
        b"not a census",
    )
    .unwrap();
    let decided = plan(&survey(root, None).unwrap(), &withdrawn(root).unwrap());
    let CensusStep::Unreadable(why) = &decided.census else {
        panic!("{:?}", decided.census);
    };
    assert!(why.contains("000009"), "{why}");
    assert_eq!(decided.pending(), 1);
    assert!(decided.changes());

    let fresh = Fixture::new("no-ledger");
    let before = plan(&survey(fresh.root(), None).unwrap(), &Withdrawn::default());
    assert_eq!(before.census, CensusStep::Clear);
}

/// AG-02 repair r1e, blocking item 2: A TARGET THAT AGREES WITH THE SOURCE
/// ON EVERY DAY COUNT AND MONTH STAMP BUT HOLDS OTHER BARS is not an apply
/// cut short. The plan used to exempt it on the headers alone and plan a
/// move the apply then refused; now apply's own `target_state` decides, so
/// it is a second name of the contract (`ContractNamedTwice`) and the apply
/// of that plan refuses nothing. Two shapes: other open interest under the
/// same stamps, and that plus an earlier extra day. A third, the same bars
/// plus an earlier extra day, IS a copy: it plans as a move the apply
/// finishes without writing a month.
#[test]
fn a_target_with_the_same_counts_and_other_bars_is_unresolved_not_a_refused_move() {
    for (tag, other_bars, extra_day) in [
        ("other-oi", true, false),
        ("extra-day", true, true),
        ("copy-extra-day", false, true),
    ] {
        let fixture = Fixture::new(tag);
        fixture.counted();
        let root = fixture.root();
        let mut copy: Vec<Bar> = [6, 7]
            .into_iter()
            .flat_map(|m| held(root, "NIFTY", STATED, YearMonth::new(2020, m).unwrap()))
            .collect();
        if other_bars {
            for one in &mut copy {
                one.open_interest += 1;
            }
        }
        if extra_day {
            copy.insert(0, bar(d(2020, 6, 1), 30_000, 444));
        }
        put(root, "NIFTY", SETTLED, &copy);
        let found = survey(root, None).unwrap();
        let ledger = withdrawn(root).unwrap();
        let decided = plan(&found, &ledger);
        let done = apply(root, &found, &decided, &ledger).unwrap();
        assert!(
            done.refused.is_empty(),
            "{tag}: the plan predicts the apply: {:?}",
            done.refused
        );
        if other_bars {
            assert!(decided.moves.is_empty(), "{tag}: {:?}", decided.moves);
            let left = decided
                .unresolved
                .iter()
                .find(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}"))
                .unwrap();
            assert!(
                left.why.contains("ContractNamedTwice"),
                "{tag}: {}",
                left.why
            );
            assert_eq!(done.months_written, 0);
        } else {
            assert_eq!(decided.moves.len(), 1, "{tag}: {:?}", decided.unresolved);
            assert!(
                decided.unresolved.is_empty(),
                "{tag}: {:?}",
                decided.unresolved
            );
            assert_eq!((done.months_written, done.months_already), (0, 2));
            assert_eq!(done.moved.len(), 1);
        }
    }

    // THE SAME BARS WITH ANOTHER DAY BETWEEN TWO OF THE SOURCE'S DAYS: every
    // count and stamp the headers hold agrees, and the bars are not where
    // the apply looks for them (the second agent's shape R2).
    let scratch = Scratch::new("between");
    let root = scratch.root();
    let source = [
        bar(d(2020, 6, 15), 34_000, 333),
        bar(d(2020, 6, 15), 34_001, 334),
        bar(d(2020, 6, 25), 34_000, 336),
    ];
    put(root, "NIFTY", STATED, &source);
    let mut target = source.to_vec();
    target.insert(2, bar(d(2020, 6, 19), 34_000, 777));
    put(root, "NIFTY", SETTLED, &target);
    let june = YearMonth::new(2020, 6).unwrap();
    partitioned(
        root,
        &[
            census_row_of(root, "NIFTY", STATED, june),
            census_row_of(root, "NIFTY", SETTLED, june),
        ],
    );
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert!(decided.moves.is_empty(), "{:?}", decided.moves);
    assert!(
        decided
            .unresolved
            .iter()
            .any(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}")
                && one.why.contains("ContractNamedTwice")),
        "{:?}",
        decided.unresolved
    );
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert!(done.refused.is_empty(), "{:?}", done.refused);
}

/// The plan's held-month verdict IS apply's: a target month holding another
/// bar after the moved ones is unresolved with apply's own words beside the
/// plan's.
#[test]
fn a_held_month_is_decided_by_applys_own_check() {
    let fixture = Fixture::new("held-check");
    fixture.counted();
    let root = fixture.root();
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 7, 20), 34_000, 999)]);
    let found = survey(root, None).unwrap();
    let decided = plan(&found, &withdrawn(root).unwrap());
    let left = decided
        .unresolved
        .iter()
        .find(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}"))
        .unwrap();
    assert!(left.why.starts_with("target month held: "), "{}", left.why);
    assert!(
        left.why.contains("(apply's own check: ")
            && left.why.contains("would precede or contradict"),
        "{}",
        left.why
    );
}

/// An apply killed between the two months of one move (FM-273 step 1 cut
/// short: June copied, July not) plans as the move the next apply finishes,
/// writing July only; the first build named it a second name of the
/// contract, because its header check asked for every month at once.
#[test]
fn an_apply_killed_between_two_months_plans_as_the_move_it_finishes() {
    let fixture = Fixture::new("killed-between-months");
    fixture.counted();
    let root = fixture.root();
    let june = held(root, "NIFTY", STATED, YearMonth::new(2020, 6).unwrap());
    put(root, "NIFTY", SETTLED, &june);
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1, "{:?}", decided.unresolved);
    assert!(decided.unresolved.is_empty(), "{:?}", decided.unresolved);
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert!(done.refused.is_empty(), "{:?}", done.refused);
    assert_eq!((done.months_written, done.months_already), (1, 1));
    assert_eq!(done.moved.len(), 1);
}

/// The follow-up of repair r1e: ONE MOVE OR ONE RETIREMENT RECORDED TWICE
/// WITH ANOTHER ACCOUNT is refused naming both lines, never read as the
/// first; an identical repeat of either reads once at its first line.
#[test]
fn one_move_or_retirement_recorded_twice_with_another_account_is_refused() {
    let first = pointer(STATED, SETTLED);
    let mut more = first;
    more.bars = 4;
    let why = Withdrawn::parse(&format!("{}\n{}\n", first.line(), more.line())).unwrap_err();
    assert!(
        why.contains("line 1 supersedes") && why.contains("and line 2 with 2 month(s), 4 bar(s)"),
        "{why}"
    );
    let mut other = first;
    other.digest = [8; 32];
    let why = Withdrawn::parse(&format!("{}\n\n{}\n", first.line(), other.line())).unwrap_err();
    assert!(
        why.contains("line 1 supersedes") && why.contains("and line 3 with"),
        "{why}"
    );
    assert!(
        why.contains("0707070707070707") && why.contains("0808080808080808"),
        "{why}"
    );

    let retired = refusal_of(IMPOSSIBLE[0]);
    let mut grown = retired;
    grown.bars = 2;
    let why = Withdrawn::parse(&format!("{}\n{}\n", retired.line(), grown.line())).unwrap_err();
    assert!(
        why.contains("line 1 retires") && why.contains("and line 2 as"),
        "{why}"
    );
    let twice = Withdrawn::parse(&format!("{}\n{}\n", retired.line(), retired.line())).unwrap();
    assert_eq!(
        (
            twice.counts(),
            twice.line_of(sym("NIFTY"), c(IMPOSSIBLE[0]))
        ),
        ((0, 1), Some(1))
    );
}

/// A ledger's edges: empty; nothing but a torn tail; a complete record cut
/// before its newline, which is passed over and then written whole by the
/// next apply; a garbage `superseded` line, refused by line; and ten
/// thousand records, read whole.
#[test]
fn a_ledger_reads_empty_torn_garbage_and_long_exactly() {
    let empty = Withdrawn::parse("").unwrap();
    assert_eq!((empty.counts(), empty.torn_tail), ((0, 0), false));
    let torn = Withdrawn::parse("superseded NSE-FNO-NIFTY-2023-0").unwrap();
    assert_eq!((torn.counts(), torn.torn_tail), ((0, 0), true));
    let cut = Withdrawn::parse(&pointer(STATED, SETTLED).line()).unwrap();
    assert_eq!((cut.counts(), cut.torn_tail), ((0, 0), true));
    let why = Withdrawn::parse("superseded garbage\n").unwrap_err();
    assert!(
        why.contains("line 1 does not parse as a `superseded` record"),
        "{why}"
    );

    // THE CUT RECORD, FINISHED: the store's apply marks it torn and writes
    // its own line whole, and the ledger then reads it once.
    let fixture = Fixture::new("torn");
    fixture.counted();
    let root = fixture.root();
    std::fs::create_dir_all(lineage_path(root).parent().unwrap()).unwrap();
    std::fs::write(lineage_path(root), "retired NSE-FNO-NIFTY-2020-06-25-19125").unwrap();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let done = apply(root, &found, &plan(&found, &ledger), &ledger).unwrap();
    assert_eq!(done.lines_written, 4);
    let text = std::fs::read_to_string(lineage_path(root)).unwrap();
    assert!(
        text.starts_with("retired NSE-FNO-NIFTY-2020-06-25-19125 (torn)\n"),
        "{text}"
    );
    assert_eq!(withdrawn(root).unwrap().counts(), (1, 3));

    // TEN THOUSAND RECORDS, one key each.
    let mut long = String::with_capacity(10_000 * 160);
    for at in 0..10_000_i64 {
        let record = SupersededRecord {
            old: c(&format!("2023-06-29-{}-PE", 100_000 + at * 100)),
            new: c(&format!("2023-06-28-{}-PE", 100_000 + at * 100)),
            ..pointer(STATED, SETTLED)
        };
        long.push_str(&record.line());
        long.push('\n');
    }
    let started = std::time::Instant::now();
    let read = Withdrawn::parse(&long).unwrap();
    let took = started.elapsed();
    assert_eq!(read.counts(), (10_000, 0));
    assert_eq!(
        read.line_of(sym("NIFTY"), c("2023-06-29-1099900-PE")),
        Some(10_000)
    );
    eprintln!("LEDGER_PARSE lines=10000 micros={}", took.as_micros());
}

/// NIFTY FIRST, THEN THE SAME CODE ON BANKNIFTY, A STOCK AND AN ILLIQUID
/// STRIKE: one stated-2023-06-29 key of a single bar per underlying, moved,
/// then a second plan pending nothing — no underlying is a special case.
#[test]
fn every_underlying_reads_done_after_its_apply() {
    let scratch = Scratch::new("every");
    let root = scratch.root();
    let mut rows = Vec::new();
    for (under, strike) in [
        ("NIFTY", "1000000"),
        ("BANKNIFTY", "2500000"),
        ("RELIANCE", "180000"),
        ("NIFTY", "1050000"),
    ] {
        let stated = format!("2023-06-29-{strike}-PE");
        put(root, under, &stated, &[bar(d(2020, 6, 15), 36_000, 7)]);
        rows.push(census_row_of(
            root,
            under,
            &stated,
            YearMonth::new(2020, 6).unwrap(),
        ));
    }
    partitioned(root, &rows);
    let found = survey(root, None).unwrap();
    let decided = plan(&found, &withdrawn(root).unwrap());
    assert_eq!(decided.moves.len(), 4, "{:?}", decided.unresolved);
    let ledger = withdrawn(root).unwrap();
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert_eq!(done.moved.len(), 4);
    let found = survey(root, None).unwrap();
    let again = plan(&found, &withdrawn(root).unwrap());
    assert_eq!(again.pending(), 0, "{:?}", again.unresolved);
    assert_eq!(again.recorded_moves.len(), 4);
    let only: BTreeSet<Symbol> = [sym("BANKNIFTY")].into_iter().collect();
    assert_eq!(withdrawn(root).unwrap().counts_for(Some(&only)), (1, 0));
}

/// A synthetic survey of `n` NIFTY option keys: nine in ten a June-2019
/// ladder strike the decoder keeps, one in ten a stated-2023-06-29 put it
/// moves, and a ledger recording the move of half of those. Invented keys,
/// no bar file: the plan reads nothing.
fn synthetic(n: usize) -> (Survey, Withdrawn) {
    let under = sym("NIFTY");
    let mut keys = Vec::with_capacity(n);
    let mut ledger = String::new();
    for at in 0..n {
        let stated = at % 10 == 9;
        let (expiry, day) = if stated {
            (d(2023, 6, 29), d(2020, 6, 15))
        } else {
            (d(2019, 6, 27), d(2019, 6, 3))
        };
        let strike = 1_000_000 + 5_000 * i64::try_from(at / 2).unwrap();
        let side = if at % 2 == 0 { "PE" } else { "CE" };
        let text = format!("{expiry}-{strike}-{side}");
        let contract = c(&text);
        let ts = (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + 34_000) * 1_000_000;
        keys.push(Stored {
            underlying: under,
            contract,
            expiry,
            strike,
            side: if at % 2 == 0 {
                OptionSide::Put
            } else {
                OptionSide::Call
            },
            months: vec![Month {
                month: day.year_month().unwrap(),
                rows: 2,
                first_ts_micros: ts,
                last_ts_micros: ts + 1_000_000,
            }],
            days: BTreeMap::from([(day, 2)]),
        });
        if stated && at % 20 == 19 {
            let to = c(&format!("2023-06-28-{strike}-{side}"));
            let record = SupersededRecord {
                underlying: under,
                old: contract,
                new: to,
                kind: AliasKind::VendorRedated,
                months: 1,
                bars: 2,
                digest: [9; 32],
            };
            ledger.push_str(&record.line());
            ledger.push('\n');
        }
    }
    keys.sort_by(|a, b| a.contract.as_str().cmp(b.contract.as_str()));
    (
        Survey {
            keys,
            ..Survey::default()
        },
        Withdrawn::parse(&ledger).unwrap(),
    )
}

/// THE PLAN'S PER-KEY COST, MEASURED (AG-02): `plan()` over synthetic
/// surveys of 1,000 and 10,000 keys, 200 runs each, nanoseconds per key.
/// Run with `--ignored --nocapture plan_cost`; prints p50, p99 and max.
#[test]
#[ignore = "a measurement, run by hand: --ignored --nocapture plan_cost"]
fn plan_cost_per_key_is_flat_in_the_number_of_keys() {
    for n in [1_000_usize, 10_000] {
        let (found, ledger) = synthetic(n);
        let mut per_key: Vec<u128> = Vec::with_capacity(200);
        let mut judged = 0;
        for _ in 0..200 {
            let started = std::time::Instant::now();
            let decided = plan(&found, &ledger);
            per_key.push(started.elapsed().as_nanos() / n as u128);
            judged = decided.keys;
        }
        assert_eq!(judged, n as u64);
        per_key.sort_unstable();
        eprintln!(
            "PLAN_COST n_keys={n} runs=200 ns_per_key p50={} p99={} max={}",
            per_key[99], per_key[197], per_key[199]
        );
    }
}

/// THE CENSUS STEP'S COST (AG-02 repair r1e): one load of the current FNO
/// epoch and one walk of its keys per plan, flat per entry. A synthetic
/// epoch of 10,000 and of 100,000 invented option entries and a ledger
/// withdrawing 13 of them (the real ledger's count), 200 plans' census step
/// each: p50, p99 and max per plan, and per entry. Run with
/// `--ignored --nocapture census_step_cost`.
#[test]
#[ignore = "a measurement, run by hand: --ignored --nocapture census_step_cost"]
fn census_step_cost_is_one_load_per_plan_flat_per_entry() {
    for n in [10_000_usize, 100_000] {
        let scratch = Scratch::new(&format!("census-cost-{n}"));
        let root = scratch.root();
        let under = sym("NIFTY");
        let month = YearMonth::new(2020, 6).unwrap();
        let mut census =
            Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, u64::try_from(n).unwrap())
                .unwrap();
        let mut ledger = String::new();
        for at in 0..n {
            let key = c(&format!("2023-06-28-{}-PE", 100_000 + at * 5));
            let ts = 1_592_193_400_000_000 + i64::try_from(at).unwrap();
            census
                .record_held(Held::unknown(Entry {
                    key: EntryKey {
                        contract: Some(key),
                        exchange: Exchange::Nse,
                        segment: Segment::Fno,
                        symbol: under,
                        timeframe: RUNG,
                        month,
                    },
                    rows: 1,
                    first_ts_micros: ts,
                    last_ts_micros: ts,
                }))
                .unwrap();
            if at < 13 {
                ledger.push_str(
                    &RetiredRecord {
                        key,
                        ..refusal_of("2019-05-30-1150000-CE")
                    }
                    .line(),
                );
                ledger.push('\n');
            }
        }
        let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, census.image()).unwrap();
        let ledger = Withdrawn::parse(&ledger).unwrap();
        let mut per_plan: Vec<u128> = Vec::with_capacity(200);
        let mut listed = 0;
        for _ in 0..200 {
            let started = std::time::Instant::now();
            let step = census_step(root, &ledger);
            per_plan.push(started.elapsed().as_nanos());
            listed = step.pending();
        }
        assert_eq!(listed, 13);
        per_plan.sort_unstable();
        let per_entry = |ns: u128| ns / n as u128;
        eprintln!(
            "CENSUS_STEP_COST entries={n} plans=200 us_per_plan p50={} p99={} max={} \
             ns_per_entry p50={} p99={} max={}",
            per_plan[99] / 1_000,
            per_plan[197] / 1_000,
            per_plan[199] / 1_000,
            per_entry(per_plan[99]),
            per_entry(per_plan[197]),
            per_entry(per_plan[199])
        );
    }
}

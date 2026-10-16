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
    assert_eq!(again.moves.len(), 1, "the same move, planned again");
    assert_eq!(again.retired.len(), 3);
    let done = apply(root, &found, &again, &ledger).unwrap();
    assert_eq!((done.months_written, done.months_already), (0, 2));
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
/// would go — the import resumed on a D-3331 build before the re-key ran —
/// refuses that move by name, and nothing of it is written; a retirement in
/// the same plan still lands.
#[test]
fn a_target_month_holding_other_bars_refuses_the_move_and_writes_nothing_of_it() {
    let fixture = Fixture::new("collide");
    fixture.counted();
    let root = fixture.root();
    // The decoded key already holds a LATER day of July: the moved July bar
    // would precede it, and a month is never rewritten in place.
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 7, 20), 34_000, 999)]);
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1, "{:?}", decided.unresolved);
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
    let bad = Withdrawn::parse(&format!("{}\n{}\n", pointer.line(), both.line())).unwrap_err();
    assert!(bad.contains("both superseded and retired"), "{bad}");
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

/// A key superseded twice to different targets, or retired twice with
/// different records, refuses the ledger by line; an identical repeat — a
/// crash re-writing the same line — is still one record
/// (cloud-attack-law-20261016, lens 2). The second line silently replaced the
/// first, so the later target won in a ledger read "whole or refused".
#[test]
fn a_key_withdrawn_twice_with_different_records_refuses_the_ledger() {
    let pointer = SupersededRecord {
        underlying: sym("NIFTY"),
        old: c(STATED),
        new: c(SETTLED),
        kind: AliasKind::VendorRedated,
        months: 2,
        bars: 3,
        digest: [7; 32],
    };
    let elsewhere = SupersededRecord {
        new: c("2023-06-27-1000000-PE"),
        ..pointer
    };
    let why = Withdrawn::parse(&format!("{}\n{}\n", pointer.line(), elsewhere.line())).unwrap_err();
    assert!(why.contains("line 2"), "{why}");
    assert!(why.contains("superseded twice"), "{why}");
    assert!(
        why.contains(SETTLED) && why.contains("2023-06-27-1000000-PE"),
        "{why}"
    );
    let repeated = Withdrawn::parse(&format!("{0}\n{0}\n", pointer.line())).unwrap();
    assert_eq!(
        repeated.counts(),
        (1, 0),
        "an identical repeat is one record"
    );

    let refusal = RetiredRecord {
        underlying: sym("NIFTY"),
        key: c(IMPOSSIBLE[0]),
        why: RetiredWhy::StrikeImplausible,
        day: d(2019, 5, 30),
        median: Some(1_180_000),
        days: 1,
        bars: 5,
    };
    let other = RetiredRecord { days: 2, ..refusal };
    let why = Withdrawn::parse(&format!("{}\n{}\n", refusal.line(), other.line())).unwrap_err();
    assert!(why.contains("retired twice"), "{why}");
    let repeated = Withdrawn::parse(&format!("{0}\n{0}\n", refusal.line())).unwrap();
    assert_eq!(repeated.counts(), (0, 1));
}

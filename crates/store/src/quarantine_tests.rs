#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! The quarantine: ledger first, an index probed in one read, a door that
//! drops only named bars and keeps the old generation byte-identical, a whole
//! month recorded rather than published empty. R4-18; ING-01, RT-DATA-02,
//! Z5, Z6. Every price is invented (paisa a real share never printed).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::{
    Applied, ApplyError, BUCKET, CAPACITY, Cause, Entry, INDEX_LEN, LEDGER_HEAD, MAX_REASON, Probe,
    QuarantineError, Record, Scope, Snapshot, Summary, Why, apply_month, check_reason, digest_of,
    index_path, ledger_path, ledger_stamps, merged, probe, read_ledger, rebuild, record, series_key,
};
use crate::file::BarFile;
use crate::format::{Bar, OI_NULL};
use crate::generation::{self, Served};
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::repair::{self, RepairError};

static NEXT: AtomicU64 = AtomicU64::new(0);
const SYMBOL: u32 = 11;

struct Store {
    root: PathBuf,
}

impl Store {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-store-quarantine-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a scratch store root");
        Self { root }
    }

    fn month(&self, path: StorePath<'_>, rows: &[Bar]) {
        let mut writer =
            BarFile::open_or_create(&self.root, path, SYMBOL).expect("the month opens");
        writer.append(rows).expect("the rows commit");
    }

    fn served_bars(&self, path: StorePath<'_>) -> Vec<Bar> {
        let file =
            BarFile::open_existing(&self.root, path, SYMBOL).expect("the served month opens");
        (0..file.header().n_valid)
            .map(|index| file.read_record(index).expect("a record"))
            .collect()
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.root);
    }
}

fn minute(symbol: &'static str, month: YearMonth) -> StorePath<'static> {
    rung(symbol, month, Timeframe::MINUTE_1)
}

fn rung(symbol: &'static str, month: YearMonth, timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn march() -> YearMonth {
    YearMonth::new(2018, 3).expect("2018-03")
}

/// The `index`-th minute from 09:15 IST on 2018-03-01, at an invented level.
fn bar(index: i64, level: i64) -> Bar {
    Bar {
        ts_micros: 1_519_875_900_000_000 + index * 60_000_000,
        open: level,
        high: level + 7,
        low: level - 3,
        close: level + 1,
        volume: 13 + index,
        open_interest: OI_NULL,
    }
}

fn why(cause: Cause) -> Why<'static> {
    Why {
        cause,
        who: "test",
        reason: "invented fixture: another security's rows",
    }
}

fn all_bytes_under(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("a directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            out.extend(all_bytes_under(&path));
        } else {
            out.push((path.clone(), fs::read(&path).expect("bytes")));
        }
    }
    out.sort();
    out
}

#[test]
fn a_quarantine_publishes_a_generation_without_the_named_bars_and_ledgers_each() {
    let store = Store::new("publish");
    let path = minute("QFIXA", march());
    let rows: Vec<Bar> = (0..10)
        .map(|i| bar(i, if i % 3 == 0 { 9_001 } else { 777 }))
        .collect();
    store.month(path, &rows);
    let named: Vec<i64> = rows
        .iter()
        .filter(|b| b.open == 9_001)
        .map(|b| b.ts_micros)
        .collect();
    let applied = apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel))
        .expect("the drop applies");
    let Applied::Dropped { drop, .. } = applied else {
        panic!("expected a dropped generation, got {applied:?}");
    };
    assert_eq!(
        (drop.from, drop.to.ordinal(), drop.held, drop.dropped),
        (0, 1, 10, 4)
    );
    let served = store.served_bars(path);
    let kept: Vec<Bar> = rows.iter().filter(|b| b.open != 9_001).copied().collect();
    assert_eq!(
        served, kept,
        "every other bar is served unchanged, and none named"
    );
    // LEDGERED: the intent before the door, the drop after it, each stamp.
    let ledger = read_ledger(&store.root, Vendor::Zerodha).expect("the ledger reads");
    let scopes: Vec<Scope> = ledger.lines.iter().map(|line| line.scope).collect();
    assert_eq!(scopes, [Scope::Dropping, Scope::Dropped]);
    for line in &ledger.lines {
        assert_eq!(line.stamps, named);
        assert_eq!(line.cause, Cause::ForeignLevel);
        assert_eq!(line.key, "NSE/CASH/QFIXA/1min");
        assert_eq!(line.month, "2018-03");
        let dropped: Vec<Bar> = rows.iter().filter(|b| b.open == 9_001).copied().collect();
        assert_eq!(
            line.digest,
            super::hex(&digest_of(&dropped)),
            "the digest of the bars set aside"
        );
    }
    assert_eq!(ledger.lines[1].revision, 1);
    let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
        panic!("the month is labelled");
    };
    assert_eq!(
        (entry.scope, entry.cause, entry.bars, entry.revision),
        (Scope::Dropped, Cause::ForeignLevel, 4, 1)
    );
    assert_eq!(
        entry.ledger_at, ledger.lines[1].at,
        "the slot names the ledger line"
    );
    assert_eq!(ledger_stamps(&store.root, path).expect("stamps"), named);
}

#[test]
fn the_original_generation_is_byte_identical_after_a_quarantine() {
    let store = Store::new("identical");
    let path = minute("QFIXB", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 500 + i)).collect();
    store.month(path, &rows);
    let bars_dir = store.root.join("bars");
    let before = all_bytes_under(&bars_dir);
    apply_month(
        &store.root,
        path,
        SYMBOL,
        &[rows[1].ts_micros, rows[4].ts_micros],
        why(Cause::BeforeListing),
    )
    .expect("the drop applies");
    assert_eq!(
        all_bytes_under(&bars_dir),
        before,
        "no byte of the original changed"
    );
    let generation = generation::served(&store.root, path).expect("served");
    assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
    assert_eq!(
        store.served_bars(path).len(),
        4,
        "the new generation lacks exactly the two named"
    );
}

#[test]
fn a_second_quarantine_apply_writes_nothing() {
    let store = Store::new("again");
    let path = minute("QFIXC", march());
    let rows: Vec<Bar> = (0..5).map(|i| bar(i, 640)).collect();
    store.month(path, &rows);
    let named = [rows[0].ts_micros, rows[2].ts_micros];
    apply_month(&store.root, path, SYMBOL, &named, why(Cause::BeforeListing)).expect("first");
    let everything = all_bytes_under(&store.root);
    let again =
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::BeforeListing)).expect("second");
    assert_eq!(
        again,
        Applied::Nothing {
            why: "already_dropped"
        }
    );
    assert_eq!(
        all_bytes_under(&store.root),
        everything,
        "the second apply wrote nothing"
    );
}

#[test]
fn a_whole_foreign_month_is_a_record_not_an_empty_generation() {
    let store = Store::new("whole");
    let path = minute("QFIXD", march());
    let rows: Vec<Bar> = (0..4).map(|i| bar(i, 77_713)).collect();
    store.month(path, &rows);
    let named: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    let applied =
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::BeforeListing)).expect("applies");
    assert!(
        matches!(applied, Applied::WholeMonth { bars: 4, .. }),
        "{applied:?}"
    );
    assert!(
        !store.root.join("bar-revisions-v1").exists(),
        "no revision of any kind was published"
    );
    assert!(matches!(
        generation::served(&store.root, path),
        Ok(Served::Original)
    ));
    let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
        panic!("the month is answered by name");
    };
    assert_eq!(
        (entry.scope, entry.cause, entry.bars),
        (Scope::WholeMonth, Cause::BeforeListing, 4)
    );
    let again =
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::BeforeListing)).expect("again");
    assert_eq!(
        again,
        Applied::Nothing {
            why: "already_whole"
        }
    );
    // The door itself refuses an empty generation.
    let header = BarFile::open_existing(&store.root, path, SYMBOL)
        .expect("opens")
        .header();
    let refused = repair::publish_without(
        &store.root,
        path,
        SYMBOL,
        repair::Revision::new(1).expect("1"),
        header,
        &named,
        "whole",
    );
    assert_eq!(refused, Err(RepairError::WholeMonth { bars: 4 }));
}

#[test]
fn a_quarantine_probe_is_one_read_and_a_torn_tail_is_repaired() {
    let store = Store::new("probe");
    let path = minute("QFIXE", march());
    let rows: Vec<Bar> = (0..3).map(|i| bar(i, 321)).collect();
    store.month(path, &rows);
    // No index: clear, by one failed open.
    assert_eq!(probe(&store.root, path), Ok(Probe::Clear));
    apply_month(
        &store.root,
        path,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("applies");
    let index = index_path(&store.root, Vendor::Zerodha);
    assert_eq!(fs::metadata(&index).expect("index").len(), INDEX_LEN);
    // ONE READ: the source of `probe_at` reads one bucket, by one positional read.
    let source = include_str!("quarantine.rs");
    let body = source
        .split("pub fn probe_at(")
        .nth(1)
        .and_then(|rest| rest.split("\nfn refusal_kind").next())
        .expect("probe_at's body");
    assert_eq!(
        body.matches("read_exact_at(").count(),
        1,
        "one positional read"
    );
    assert_eq!(
        body.matches("read_to_end").count(),
        0,
        "never the whole index"
    );
    // A torn index answers by name, never "clear" ...
    let bytes = fs::read(&index).expect("bytes");
    fs::write(&index, &bytes[..bytes.len() - 17]).expect("tear it");
    let torn = probe(&store.root, path);
    assert!(
        matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
        "{torn:?}"
    );
    // ... and is rebuilt from the ledger, the same answer as before.
    let rebuilt = rebuild(&store.root, Vendor::Zerodha).expect("rebuilds");
    assert_eq!((rebuilt.months, rebuilt.torn_tail), (1, false));
    assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
    // A TORN LEDGER TAIL is never a line, and the next append ends it first.
    let ledger = ledger_path(&store.root, Vendor::Zerodha);
    let mut text = fs::read(&ledger).expect("ledger");
    text.extend_from_slice(b"1\tzerodha\tNSE/CASH/QFIXE/1min\t2018-03\tdrop");
    fs::write(&ledger, &text).expect("a torn append");
    let read = read_ledger(&store.root, Vendor::Zerodha).expect("reads");
    assert!(read.torn_tail);
    assert_eq!(read.lines.len(), 2, "the fragment is not a line");
    let other = minute("QFIXF", march());
    store.month(other, &rows);
    let done = record(
        &store.root,
        &Record {
            path: other,
            scope: Scope::WholeMonth,
            cause: Cause::BeforeListing,
            stamps: &[],
            bars: 3,
            revision: 0,
            source_digest: digest_of(&rows),
            who: "test",
            reason: "after a torn tail",
        },
    )
    .expect("records");
    assert!(done.ended_torn);
    let read = read_ledger(&store.root, Vendor::Zerodha).expect("reads");
    assert_eq!(read.lines.len(), 3);
    assert_eq!(
        read.damaged.len(),
        1,
        "the ended fragment is named damaged, never parsed"
    );
    assert!(
        fs::read_to_string(&ledger)
            .expect("text")
            .starts_with(LEDGER_HEAD)
    );
}

#[test]
fn a_damaged_slot_is_named_and_a_record_rebuilds_the_index() {
    let store = Store::new("damaged");
    let path = minute("QFIXG", march());
    let rows: Vec<Bar> = (0..3).map(|i| bar(i, 410)).collect();
    store.month(path, &rows);
    apply_month(
        &store.root,
        path,
        SYMBOL,
        &[rows[1].ts_micros],
        why(Cause::ForeignLevel),
    )
    .expect("applies");
    let index = index_path(&store.root, Vendor::Zerodha);
    let mut bytes = fs::read(&index).expect("bytes");
    let at = bytes
        .iter()
        .enumerate()
        .skip(64)
        .find(|(_, byte)| **byte != 0)
        .map(|(at, _)| at)
        .expect("a held slot");
    bytes[at] ^= 0xff;
    fs::write(&index, &bytes).expect("damage one slot");
    assert!(matches!(
        probe(&store.root, path),
        Err(QuarantineError::Damaged { .. })
    ));
    assert!(matches!(
        Snapshot::load(&store.root, Vendor::Zerodha),
        Err(QuarantineError::Damaged { .. })
    ));
    let other = minute("QFIXH", march());
    store.month(other, &rows);
    let done = apply_month(
        &store.root,
        other,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ForeignLevel),
    )
    .expect("applies");
    assert!(matches!(done, Applied::Dropped { .. }));
    let snapshot = Snapshot::load(&store.root, Vendor::Zerodha).expect("rebuilt and loaded");
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot.get(path).map(|e| e.bars), Some(1));
}

#[test]
fn a_named_stamp_neither_held_nor_ledgered_is_refused_and_nothing_is_written() {
    let store = Store::new("unheld");
    let path = minute("QFIXI", march());
    let rows: Vec<Bar> = (0..4).map(|i| bar(i * 2, 250)).collect();
    store.month(path, &rows);
    let before = all_bytes_under(&store.root);
    let between = rows[0].ts_micros + 60_000_000;
    let refused = apply_month(
        &store.root,
        path,
        SYMBOL,
        &[rows[1].ts_micros, between],
        why(Cause::ForeignLevel),
    );
    assert_eq!(
        refused,
        Err(ApplyError::Repair(RepairError::UnheldStamp(between)))
    );
    assert_eq!(all_bytes_under(&store.root), before, "nothing was written");
}

#[test]
fn a_run_killed_after_the_promotion_is_resumed_from_its_ledgered_intent() {
    let store = Store::new("resume");
    let path = minute("QFIXJ", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 870)).collect();
    store.month(path, &rows);
    let named = [rows[2].ts_micros, rows[3].ts_micros];
    // THE KILLED RUN: the intent is ledgered and the door promoted, and the
    // process died before its `dropped` line.
    let dropped: Vec<Bar> = vec![rows[2], rows[3]];
    record(
        &store.root,
        &Record {
            path,
            scope: Scope::Dropping,
            cause: Cause::ForeignLevel,
            stamps: &named,
            bars: 2,
            revision: 0,
            source_digest: digest_of(&dropped),
            who: "killed",
            reason: "the run that died",
        },
    )
    .expect("the intent");
    repair::drop_month(&store.root, path, SYMBOL, &named, "the run that died").expect("promoted");
    assert_eq!(
        probe(&store.root, path),
        Ok(Probe::Clear),
        "not yet indexed"
    );
    let resumed =
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)).expect("resumes");
    assert!(
        matches!(resumed, Applied::Resumed { bars: 2, .. }),
        "{resumed:?}"
    );
    assert!(
        matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
    );
    let again = apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel))
        .expect("then nothing");
    assert_eq!(
        again,
        Applied::Nothing {
            why: "already_dropped"
        }
    );
}

#[test]
fn a_second_writer_waits_then_names_the_holder() {
    let store = Store::new("writers");
    let path = minute("QFIXK", march());
    let rows: Vec<Bar> = (0..2).map(|i| bar(i, 130)).collect();
    store.month(path, &rows);
    let held = super::writer_lock(&store.root, Vendor::Zerodha, super::WRITER_PATIENCE)
        .expect("the first writer");
    let refused = {
        super::record_bounded(
            &store.root,
            &Record {
                path,
                scope: Scope::WholeMonth,
                cause: Cause::BeforeListing,
                stamps: &[],
                bars: 2,
                revision: 0,
                source_digest: digest_of(&rows),
                who: "second",
                reason: "a second writer",
            },
            std::time::Duration::from_millis(50),
        )
    };
    held.release().expect("released");
    match refused {
        Err(QuarantineError::Held { holder, .. }) => {
            assert!(!holder.is_empty(), "the holder is named");
        }
        // A bound longer than the patience the test grants still names the hold.
        other => panic!("expected the hold named, got {other:?}"),
    }
}

#[test]
fn garbage_reasons_and_names_are_refused_before_anything_is_written() {
    assert!(check_reason("").is_err());
    assert!(check_reason("   ").is_err());
    assert!(check_reason(&"x".repeat(MAX_REASON + 1)).is_err());
    assert!(check_reason("tab\there").is_err());
    assert!(check_reason("line\nbreak").is_err());
    assert!(check_reason(&"x".repeat(MAX_REASON)).is_ok());
    let store = Store::new("garbage");
    let path = minute("QFIXL", march());
    store.month(path, &[bar(0, 90)]);
    let before = all_bytes_under(&store.root);
    let empty = apply_month(&store.root, path, SYMBOL, &[], why(Cause::ZeroPrice));
    assert_eq!(empty, Err(ApplyError::Repair(RepairError::NothingNamed)));
    let no_reason = apply_month(
        &store.root,
        path,
        SYMBOL,
        &[bar(0, 90).ts_micros],
        Why {
            cause: Cause::ZeroPrice,
            who: "t",
            reason: "",
        },
    );
    assert!(matches!(
        no_reason,
        Err(ApplyError::Quarantine(QuarantineError::Reason(_)))
    ));
    assert_eq!(all_bytes_under(&store.root), before);
    assert_eq!(Cause::parse("before_listing"), Some(Cause::BeforeListing));
    assert_eq!(Cause::parse("BEFORE_LISTING"), None);
    for cause in Cause::ALL {
        assert_eq!(Cause::parse(cause.code()), Some(cause));
    }
}

#[test]
fn the_series_key_is_the_paths_series_and_months_of_one_key_do_not_collide() {
    let path = minute("QFIXM", march());
    assert_eq!(series_key(path), "NSE/CASH/QFIXM/1min");
    assert_eq!(
        series_key(rung("QFIXM", march(), Timeframe::DAY_1)),
        "NSE/CASH/QFIXM/1day"
    );
    let store = Store::new("many");
    // MAXIMUM BREADTH: one month for each of 120 months of one key, and the
    // snapshot finds every one at its own month, none at a neighbour's.
    let rows = [bar(0, 60)];
    let mut months = Vec::new();
    for offset in 0..120u16 {
        let month = YearMonth::new(
            2015 + offset / 12,
            u8::try_from(offset % 12 + 1).expect("a month"),
        )
        .expect("month");
        months.push(month);
    }
    for month in &months {
        let path = minute("QFIXM", *month);
        record(
            &store.root,
            &Record {
                path,
                scope: Scope::WholeMonth,
                cause: Cause::BeforeListing,
                stamps: &[],
                bars: 1,
                revision: 0,
                source_digest: digest_of(&rows),
                who: "t",
                reason: "breadth",
            },
        )
        .expect("records");
    }
    let snapshot = Snapshot::load(&store.root, Vendor::Zerodha).expect("loads");
    assert_eq!(snapshot.len(), 120);
    for month in &months {
        assert!(matches!(
            probe(&store.root, minute("QFIXM", *month)),
            Ok(Probe::Quarantined(_))
        ));
        assert!(snapshot.get(minute("QFIXM", *month)).is_some());
    }
    assert!(
        snapshot.get(minute("QFIXN", months[0])).is_none(),
        "another key is clear"
    );
    assert!(
        snapshot
            .get(rung("QFIXM", months[0], Timeframe::DAY_1))
            .is_none(),
        "another rung is clear"
    );
    assert_eq!(
        INDEX_LEN,
        64 + (CAPACITY + BUCKET - 1) * 64,
        "the length the probe checks"
    );
}

#[test]
fn a_linked_audit_directory_is_refused() {
    let store = Store::new("linked");
    let elsewhere = store.root.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("dir");
    std::os::unix::fs::symlink(&elsewhere, store.root.join("audit")).expect("link");
    let path = minute("QFIXO", march());
    let refused = record(
        &store.root,
        &Record {
            path,
            scope: Scope::WholeMonth,
            cause: Cause::BeforeListing,
            stamps: &[],
            bars: 1,
            revision: 0,
            source_digest: [0; 32],
            who: "t",
            reason: "linked",
        },
    );
    assert!(
        matches!(refused, Err(QuarantineError::Linked(_))),
        "{refused:?}"
    );
    assert_eq!(
        fs::read_dir(&elsewhere).expect("dir").count(),
        0,
        "nothing written through the link"
    );
}

/// **A FULL BUCKET IS REFUSED BY NAME, AND THE LEDGER LINE STANDS** (the
/// index's maximum, L-R418-04): nine months whose keys share one home slot
/// fill its bucket of eight; the ninth record is `quarantine_index_full`, its
/// ledger line is written, and the eight indexed months still probe.
#[test]
fn a_ninth_key_on_one_home_slot_is_refused_by_name_and_the_ledger_line_stands() {
    let store = Store::new("full");
    let month = march();
    let target = super::home_of(&super::hash_of("NSE/CASH/QHOME0/1min", month));
    let mut same: Vec<String> = Vec::new();
    for n in 0..2_000_000u32 {
        let symbol = format!("QH{n}");
        if super::home_of(&super::hash_of(&format!("NSE/CASH/{symbol}/1min"), month)) == target {
            same.push(symbol);
            if same.len() == usize::try_from(BUCKET).expect("small") + 1 {
                break;
            }
        }
    }
    assert_eq!(same.len(), 9, "nine keys share one home slot");
    let symbols: Vec<&'static str> = same
        .into_iter()
        .map(|s| &*Box::leak(s.into_boxed_str()))
        .collect();
    let act = |symbol: &'static str| {
        record(
            &store.root,
            &Record {
                path: minute(symbol, month),
                scope: Scope::WholeMonth,
                cause: Cause::BeforeListing,
                stamps: &[],
                bars: 1,
                revision: 0,
                source_digest: [3; 32],
                who: "t",
                reason: "a full bucket",
            },
        )
    };
    for symbol in &symbols[..8] {
        act(symbol).expect("one of eight fits");
    }
    let refused = act(symbols[8]);
    assert!(
        matches!(refused, Err(QuarantineError::Full { .. })),
        "{refused:?}"
    );
    let ledger = read_ledger(&store.root, Vendor::Zerodha).expect("ledger");
    assert_eq!(ledger.lines.len(), 9, "the ninth line stands in the ledger");
    for symbol in &symbols[..8] {
        assert!(matches!(
            probe(&store.root, minute(symbol, month)),
            Ok(Probe::Quarantined(_))
        ));
    }
    assert_eq!(
        probe(&store.root, minute(symbols[8], month)),
        Ok(Probe::Clear)
    );
}

/// R4-18 repair 1, B1: a later plan naming only SOME bars of a month already
/// quarantined whole (a second cause, or a derived bucket covering a minute
/// of one) answers `already_whole` and writes nothing. Before the repair it
/// published a generation without the named subset and its `dropped` line
/// replaced the whole record, so the reader served the rest of the month.
#[test]
fn a_partial_plan_on_a_month_quarantined_whole_keeps_it_whole_and_writes_nothing() {
    let store = Store::new("whole-then-part");
    let path = minute("QFIXW", march());
    let rows: Vec<Bar> = (0..4).map(|i| bar(i, 4_127)).collect();
    store.month(path, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    let whole = apply_month(&store.root, path, SYMBOL, &every, why(Cause::BeforeListing))
        .expect("recorded whole");
    assert!(
        matches!(whole, Applied::WholeMonth { bars: 4, .. }),
        "{whole:?}"
    );
    let before = all_bytes_under(&store.root);
    for subset in [
        vec![rows[1].ts_micros],
        vec![rows[0].ts_micros, rows[3].ts_micros],
        vec![rows[2].ts_micros, rows[2].ts_micros],
    ] {
        let again = apply_month(&store.root, path, SYMBOL, &subset, why(Cause::ZeroPrice))
            .expect("answered");
        assert_eq!(
            again,
            Applied::Nothing {
                why: "already_whole"
            },
            "{subset:?}"
        );
    }
    assert_eq!(all_bytes_under(&store.root), before, "nothing was written");
    assert!(matches!(
        generation::served(&store.root, path),
        Ok(Served::Original)
    ));
    assert_eq!(store.served_bars(path), rows, "no generation was published");
    let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
        panic!("the month is still answered by name");
    };
    assert_eq!(
        (entry.scope, entry.cause, entry.bars),
        (Scope::WholeMonth, Cause::BeforeListing, 4)
    );
    let summary = Summary::load(&store.root, Vendor::Zerodha).expect("a summary");
    assert_eq!(summary.month(path).map(|e| e.scope), Some(Scope::WholeMonth));
    let row = summary
        .instrument("NSE", "CASH", "QFIXW")
        .expect("the share is flagged");
    assert_eq!((row.months, row.whole_months, row.bars), (1, 1, 4));
    // A stamp neither held nor ledgered is still refused on a whole month.
    let between = rows[0].ts_micros + 30_000_000;
    assert_eq!(
        apply_month(&store.root, path, SYMBOL, &[between], why(Cause::ZeroPrice)),
        Err(ApplyError::Repair(RepairError::UnheldStamp(between)))
    );
    assert_eq!(all_bytes_under(&store.root), before, "still nothing written");
}

/// The derived rung the verb applies next: a 2min month recorded whole is
/// not reopened by a bucket of a second cause either.
#[test]
fn a_derived_month_recorded_whole_is_not_reopened_by_one_bucket() {
    let store = Store::new("whole-derived");
    let path = rung("QFIXV", march(), Timeframe::MINUTE_2);
    let rows: Vec<Bar> = (0..3).map(|i| bar(i * 2, 3_331)).collect();
    store.month(path, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    apply_month(&store.root, path, SYMBOL, &every, why(Cause::BeforeListing)).expect("whole");
    let one = apply_month(
        &store.root,
        path,
        SYMBOL,
        &[rows[1].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("answered");
    assert_eq!(
        one,
        Applied::Nothing {
            why: "already_whole"
        }
    );
    assert_eq!(store.served_bars(path), rows);
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
    ));
}

/// A `dropped` line the ledger holds after a `whole_month` line (an older
/// build's, or a killed run's intent finished late) never downgrades the
/// index, its rebuild or the summary: the month stays refused whole.
#[test]
fn a_dropped_line_after_a_whole_line_never_downgrades_index_rebuild_or_summary() {
    let store = Store::new("whole-then-dropped-line");
    let path = minute("QFIXU", march());
    let rows: Vec<Bar> = (0..5).map(|i| bar(i, 2_903)).collect();
    store.month(path, &rows);
    let act = |scope, stamps: &'static [i64], bars, revision| Record {
        path,
        scope,
        cause: if scope == Scope::WholeMonth {
            Cause::BeforeListing
        } else {
            Cause::ZeroPrice
        },
        stamps,
        bars,
        revision,
        source_digest: digest_of(&rows),
        who: "test",
        reason: "invented ledger order",
    };
    record(&store.root, &act(Scope::WholeMonth, &[], 5, 0)).expect("whole");
    let one: &'static [i64] = Box::leak(vec![rows[2].ts_micros].into_boxed_slice());
    record(&store.root, &act(Scope::Dropped, one, 1, 1)).expect("a later dropped line");
    let whole = |why: &str| match probe(&store.root, path) {
        Ok(Probe::Quarantined(e)) => assert_eq!(
            (e.scope, e.cause, e.bars),
            (Scope::WholeMonth, Cause::BeforeListing, 5),
            "{why}"
        ),
        other => panic!("{why}: {other:?}"),
    };
    whole("the index after the line");
    rebuild(&store.root, Vendor::Zerodha).expect("rebuilt");
    whole("the index rebuilt from the ledger");
    let summary = Summary::load(&store.root, Vendor::Zerodha).expect("a summary");
    assert_eq!(
        summary.month(path).map(|e| (e.scope, e.bars)),
        Some((Scope::WholeMonth, 5))
    );
    let row = summary.instrument("NSE", "CASH", "QFIXU").expect("flagged");
    assert_eq!((row.months, row.whole_months, row.bars), (1, 1, 5));
}

/// The one merge rule, every arm: drop on drop adds, drop on whole keeps
/// whole, whole on drop and whole on whole take the newer line.
#[test]
fn the_merge_rule_never_lets_a_drop_lift_a_whole_record() {
    let entry = |scope, bars, ledger_at| Entry {
        scope,
        cause: Cause::ForeignLevel,
        bars,
        ledger_at,
        revision: 1,
        written_at: 7,
    };
    let dropped_a = entry(Scope::Dropped, 3, 10);
    let dropped_b = entry(Scope::Dropped, 4, 20);
    let whole_a = entry(Scope::WholeMonth, 9, 30);
    let whole_b = entry(Scope::WholeMonth, 6, 40);
    assert_eq!(
        merged(dropped_a, dropped_b),
        Entry {
            bars: 7,
            ..dropped_b
        }
    );
    assert_eq!(merged(whole_a, dropped_b), whole_a);
    assert_eq!(merged(dropped_a, whole_b), whole_b);
    assert_eq!(merged(whole_a, whole_b), whole_b);
}

/// A killed run's intent owed on a month later recorded whole is finished
/// (its `dropped` line written) and the month stays whole.
#[test]
fn an_intent_owed_on_a_month_since_recorded_whole_is_finished_and_it_stays_whole() {
    let store = Store::new("whole-resume");
    let path = minute("QFIXT", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 1_709)).collect();
    store.month(path, &rows);
    let named = [rows[2].ts_micros, rows[3].ts_micros];
    record(
        &store.root,
        &Record {
            path,
            scope: Scope::Dropping,
            cause: Cause::ForeignLevel,
            stamps: &named,
            bars: 2,
            revision: 0,
            source_digest: digest_of(&[rows[2], rows[3]]),
            who: "killed",
            reason: "the run that died",
        },
    )
    .expect("the intent");
    repair::drop_month(&store.root, path, SYMBOL, &named, "the run that died").expect("promoted");
    let rest: Vec<i64> = [0, 1, 4, 5].iter().map(|i| rows[*i].ts_micros).collect();
    let whole =
        apply_month(&store.root, path, SYMBOL, &rest, why(Cause::BeforeListing)).expect("whole");
    assert!(
        matches!(whole, Applied::WholeMonth { bars: 4, .. }),
        "{whole:?}"
    );
    let resumed =
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)).expect("resumed");
    assert!(
        matches!(resumed, Applied::Resumed { bars: 2, .. }),
        "{resumed:?}"
    );
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 4
    ));
    assert_eq!(
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)),
        Ok(Applied::Nothing {
            why: "already_whole"
        })
    );
}

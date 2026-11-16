#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! The quarantine: ledger first, an index probed in two positional reads
//! (its header and one bucket), a door that
//! drops only named bars and keeps the old generation byte-identical, a whole
//! month recorded rather than published empty. R4-18; ING-01, RT-DATA-02,
//! Z5, Z6. Every price is invented (paisa a real share never printed).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::{
    Applied, ApplyError, BUCKET, CAPACITY, Cause, Causes, Entry, INDEX_LEN, LEDGER_HEAD,
    MAX_REASON, Probe, QuarantineError, Record, Scope, Snapshot, Summary, Why, apply_month,
    catch_up, check_reason, digest_of, index_path, ledger_path, ledger_stamps, merged, probe,
    read_ledger, rebuild, record, series_key,
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
    // TWO POSITIONAL READS: `probe_at` reads one bucket, `open_index` the
    // 64-byte header (R4-18 repair 2), and neither reads the whole index.
    let source = include_str!("quarantine.rs");
    let body = source
        .split("pub fn probe_at(")
        .nth(1)
        .and_then(|rest| rest.split("\nfn open_index(").next())
        .expect("probe_at's body");
    let opener = source
        .split("\nfn open_index(")
        .nth(1)
        .and_then(|rest| rest.split("\nfn described(").next())
        .expect("open_index's body");
    for (name, text) in [("probe_at", body), ("open_index", opener)] {
        assert_eq!(
            text.matches("read_exact_at(").count(),
            1,
            "{name}: one positional read"
        );
        assert_eq!(
            text.matches("read_to_end").count(),
            0,
            "{name}: never the whole index"
        );
    }
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
    assert!(done.rebuilt, "an image behind its ledger is rebuilt first");
    let read = read_ledger(&store.root, Vendor::Zerodha).expect("reads");
    assert_eq!(read.lines.len(), 3);
    // ENDED, NEVER DAMAGE (R4-18 repair 2): the kill left no act, so the
    // fragment is never parsed and never refuses a rebuild or a listing.
    assert_eq!(read.ended.len(), 1, "the fragment is an ended torn append");
    assert!(read.damaged.is_empty(), "{:?}", read.damaged);
    assert!(
        fs::read_to_string(&ledger)
            .expect("text")
            .contains("drop\ttorn\n"),
        "ended by the torn mark, no byte of it changed"
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
    // THE INTENT IS INDEXED as a drop of no bars (R4-18 repair 2): every
    // reader knows the month is named before its `dropped` line exists.
    assert!(
        matches!(
            probe(&store.root, path),
            Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
        ),
        "the intent is indexed, its bars not yet counted"
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

/// **A FULL BUCKET IS REFUSED BY NAME, AND NOTHING IS WRITTEN** (the index's
/// maximum, L-R418-04): nine months whose keys share one home slot fill its
/// bucket of eight; the ninth record is `quarantine_index_full`, refused
/// BEFORE its ledger line (R4-18 repair 2: a line standing unindexed would
/// leave every reader of the feed answering `quarantine_index_stale`, and no
/// rebuild could index it), and the eight indexed months still probe.
#[test]
fn a_ninth_key_on_one_home_slot_is_refused_by_name_and_nothing_is_written() {
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
    let audit_before = all_bytes_under(&store.root.join("audit"));
    let refused = act(symbols[8]);
    assert!(
        matches!(refused, Err(QuarantineError::Full { .. })),
        "{refused:?}"
    );
    assert_eq!(
        all_bytes_under(&store.root.join("audit")),
        audit_before,
        "refused before its ledger line: nothing written"
    );
    let ledger = read_ledger(&store.root, Vendor::Zerodha).expect("ledger");
    assert_eq!(ledger.lines.len(), 8, "the ninth act has no line");
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
    assert_eq!(
        summary.month(path).map(|e| e.scope),
        Some(Scope::WholeMonth)
    );
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
    assert_eq!(
        all_bytes_under(&store.root),
        before,
        "still nothing written"
    );
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
/// whole, whole on drop takes the newer line and ADDS the bars the drops set
/// aside (R4-18 repair 2: it kept only its own), whole on whole takes the
/// newer line; every arm keeps every cause any line named.
#[test]
fn the_merge_rule_never_lets_a_drop_lift_a_whole_record() {
    let entry = |scope, bars, ledger_at| Entry {
        scope,
        cause: Cause::ForeignLevel,
        causes: Causes::of(Cause::ForeignLevel),
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
    assert_eq!(
        merged(dropped_a, whole_b),
        Entry { bars: 9, ..whole_b },
        "a whole record after a drop counts the dropped bars too"
    );
    assert_eq!(merged(whole_a, whole_b), whole_b);
    // EVERY CAUSE, in every arm: the newest line's `cause`, every line's
    // `causes`.
    let zero = |scope, bars, ledger_at| Entry {
        cause: Cause::ZeroPrice,
        causes: Causes::of(Cause::ZeroPrice),
        ..entry(scope, bars, ledger_at)
    };
    let both = Causes::of(Cause::ForeignLevel).union(Causes::of(Cause::ZeroPrice));
    for (held, newer, scope, cause, bars) in [
        (
            dropped_a,
            zero(Scope::Dropped, 4, 20),
            Scope::Dropped,
            Cause::ZeroPrice,
            7,
        ),
        (
            dropped_a,
            zero(Scope::WholeMonth, 6, 40),
            Scope::WholeMonth,
            Cause::ZeroPrice,
            9,
        ),
        (
            whole_a,
            zero(Scope::Dropped, 4, 20),
            Scope::WholeMonth,
            Cause::ForeignLevel,
            9,
        ),
        (
            whole_a,
            zero(Scope::WholeMonth, 6, 40),
            Scope::WholeMonth,
            Cause::ZeroPrice,
            6,
        ),
    ] {
        let after = merged(held, newer);
        assert_eq!(
            (after.scope, after.cause, after.causes, after.bars),
            (scope, cause, both, bars),
            "{held:?} then {newer:?}"
        );
        assert_eq!(after.causes.said(), "foreign_level and zero_price");
    }
    assert_eq!(
        Causes::of(Cause::BeforeListing)
            .union(both)
            .each()
            .collect::<Vec<_>>(),
        Cause::ALL.to_vec(),
        "code order"
    );
    assert_eq!(
        Causes::of(Cause::BeforeListing).union(both).said(),
        "before_listing, foreign_level and zero_price"
    );
    // AN INTENT (a drop of no bars, revision 0) adds nothing and keeps the
    // revision served; its line is the newest (R4-18 repair 2).
    let intent = Entry {
        bars: 0,
        revision: 0,
        ..entry(Scope::Dropped, 0, 50)
    };
    assert_eq!(
        merged(dropped_a, intent),
        Entry {
            bars: 3,
            revision: 1,
            ..intent
        }
    );
    assert_eq!(merged(whole_a, intent), whole_a);
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
    // THE APPLY THAT RECORDED IT WHOLE FINISHED THE INTENT FIRST (R4-18
    // repair 2, the second check's t2): its `dropped` line precedes the whole
    // line, so the next apply of the intent's own stamps writes nothing.
    let lines = read_ledger(&store.root, Vendor::Zerodha)
        .expect("ledger")
        .lines;
    let scopes: Vec<Scope> = lines.iter().map(|line| line.scope).collect();
    assert_eq!(
        scopes,
        vec![Scope::Dropping, Scope::Dropped, Scope::WholeMonth]
    );
    assert_eq!(lines[1].stamps, named);
    let resumed =
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)).expect("resumed");
    assert_eq!(
        resumed,
        Applied::Nothing {
            why: "already_whole"
        }
    );
    // EVERY BAR SET ASIDE (R4-18 repair 2, D-7292): the 2 the finished intent
    // dropped and the 4 recorded whole after it, and both causes.
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
            && e.bars == 6
            && e.causes.said() == "before_listing and foreign_level"
    ));
    assert_eq!(
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)),
        Ok(Applied::Nothing {
            why: "already_whole"
        })
    );
}

// ---------------------------------------------------------------------------
// R4-18 repair 2. Each test below FAILED on the code the second check read
// (ad0c60248) and names the blocking issue it closes.
// ---------------------------------------------------------------------------

/// The code of an apply's refusal, or a panic naming what came back.
fn refusal_code(done: &Result<Applied, ApplyError>) -> &'static str {
    match done {
        Err(ApplyError::Quarantine(why)) => why.code(),
        other => panic!("expected the quarantine to refuse by name, got {other:?}"),
    }
}

/// Sets the audit directory's mode: `0o555` refuses every new file in it, as
/// a full disk refuses the index's staging file.
fn audit_mode(store: &Store, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(store.root.join("audit"), fs::Permissions::from_mode(mode))
        .expect("the audit directory's mode");
}

/// BLOCKING 1: a ledger line that fails its checksum is NAMED and never
/// rebuilt away. Before, the next writer rebuilt a torn index from the valid
/// lines only: March, recorded whole by the damaged line, read as clear, the
/// apply that rebuilt it answered success, and the summary under-counted.
#[test]
fn a_damaged_ledger_line_is_named_and_never_rebuilt_away() {
    let store = Store::new("ledger-damaged");
    let path = minute("QFIXV", march());
    let rows: Vec<Bar> = (0..4).map(|i| bar(i, 3_119)).collect();
    store.month(path, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    let whole =
        apply_month(&store.root, path, SYMBOL, &every, why(Cause::BeforeListing)).expect("whole");
    assert!(
        matches!(whole, Applied::WholeMonth { bars: 4, .. }),
        "{whole:?}"
    );
    // ONE BYTE of the whole_month line's reason flipped, and the index torn.
    let ledger = ledger_path(&store.root, Vendor::Zerodha);
    let mut bytes = fs::read(&ledger).expect("ledger");
    let at = bytes
        .windows(8)
        .position(|w| w == b"invented")
        .expect("the reason");
    bytes[at] ^= 0x20;
    fs::write(&ledger, &bytes).expect("one byte damaged");
    let index = index_path(&store.root, Vendor::Zerodha);
    let image = fs::read(&index).expect("index");
    fs::write(&index, &image[..100]).expect("torn");
    let damaged_at = read_ledger(&store.root, Vendor::Zerodha)
        .expect("reads")
        .damaged;
    assert_eq!(damaged_at.len(), 1, "the damaged line is counted");
    // THE NEXT APPLY, of another month, refuses by name and writes nothing ...
    let other = minute("QFIXW", march());
    store.month(other, &rows);
    let audit_before = all_bytes_under(&store.root.join("audit"));
    let next = apply_month(
        &store.root,
        other,
        SYMBOL,
        &[rows[1].ts_micros],
        why(Cause::ZeroPrice),
    );
    assert_eq!(refusal_code(&next), "quarantine_ledger_damaged");
    let words = next.expect_err("refused").to_string();
    assert!(
        words.contains(&damaged_at[0].to_string()),
        "the damaged line's offset is named: {words}"
    );
    assert_eq!(
        all_bytes_under(&store.root.join("audit")),
        audit_before,
        "nothing was written"
    );
    assert_eq!(store.served_bars(other).len(), 4, "no generation published");
    // ... a rebuild refuses by name and publishes nothing, so every reader
    // keeps answering "not known", never "clear" ...
    assert_eq!(
        rebuild(&store.root, Vendor::Zerodha).map_err(|why| why.code()),
        Err("quarantine_ledger_damaged")
    );
    let march_now = probe(&store.root, path);
    assert!(
        march_now.is_err(),
        "March is never read as clear: {march_now:?}"
    );
    // ... and the listings name the damage instead of under-counting.
    assert_eq!(
        Summary::load(&store.root, Vendor::Zerodha)
            .map(|summary| summary.is_empty())
            .map_err(|why| why.code()),
        Err("quarantine_ledger_damaged")
    );
}

/// A torn append ended by the next writer is never damage: the kill it
/// records left no act, so it holds no rebuild and no listing refused.
#[test]
fn a_torn_append_ended_by_the_next_writer_is_not_damage() {
    let store = Store::new("torn-not-damage");
    let path = minute("QFIXA2", march());
    let rows: Vec<Bar> = (0..3).map(|i| bar(i, 2_207)).collect();
    store.month(path, &rows);
    let ledger = ledger_path(&store.root, Vendor::Zerodha);
    apply_month(
        &store.root,
        path,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("applies");
    // A WRITER KILLED MID-APPEND: half a line, no newline.
    let mut text = fs::read(&ledger).expect("ledger");
    text.extend_from_slice(b"1792\tzerodha\tNSE/CASH/QFIXA2/1min\t2018-03\twhole_mo");
    fs::write(&ledger, &text).expect("a torn append");
    let other = minute("QFIXB2", march());
    store.month(other, &rows);
    let next = apply_month(
        &store.root,
        other,
        SYMBOL,
        &[rows[2].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("the next writer ends the fragment and writes its act");
    assert!(matches!(next, Applied::Dropped { .. }), "{next:?}");
    let read = read_ledger(&store.root, Vendor::Zerodha).expect("reads");
    assert!(read.damaged.is_empty(), "{:?}", read.damaged);
    rebuild(&store.root, Vendor::Zerodha).expect("a rebuild is not refused");
    let summary = Summary::load(&store.root, Vendor::Zerodha).expect("the summary loads");
    assert_eq!(summary.month(path).map(|e| e.bars), Some(1));
    assert_eq!(summary.month(other).map(|e| e.bars), Some(1));
}

/// BLOCKING 2, a dropped month: the door promoted and the `dropped` line was
/// synced, then the index publish was refused. Before, the readers read the
/// month as clear and the rerun answered `already_dropped` without indexing
/// it, so the stored sweep never refused what `/store.json` called dropped.
#[test]
fn an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply() {
    let store = Store::new("publish-refused-drop");
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 4_211)).collect();
    // An earlier act: the lock, the ledger and the index all exist.
    let first = minute("QFIXX", march());
    store.month(first, &rows);
    apply_month(
        &store.root,
        first,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("an earlier act");
    let path = minute("QFIXY", march());
    store.month(path, &rows);
    let named = [rows[2].ts_micros, rows[3].ts_micros];
    audit_mode(&store, 0o555);
    let refused = apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel));
    audit_mode(&store, 0o755);
    assert!(refused.is_err(), "{refused:?}");
    // UNTIL A WRITER CATCHES UP the readers say "not known", never "clear".
    let between = probe(&store.root, path);
    assert!(
        between.is_err(),
        "an index behind its ledger is not read as clear: {between:?}"
    );
    // THE RERUN finishes it: the month is indexed as dropped, once.
    apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)).expect("the rerun");
    match probe(&store.root, path) {
        Ok(Probe::Quarantined(entry)) => {
            assert_eq!((entry.scope, entry.bars), (Scope::Dropped, 2), "{entry:?}");
        }
        other => panic!("the rerun indexes the drop: {other:?}"),
    }
    assert_eq!(store.served_bars(path).len(), 4);
    let summary = Summary::load(&store.root, Vendor::Zerodha).expect("summary");
    assert_eq!(
        summary.month(path).map(|e| (e.scope, e.bars)),
        Some((Scope::Dropped, 2)),
        "the listing and the index say one thing"
    );
}

/// BLOCKING 2, a month recorded whole: before, the readers served every bar
/// of a month the ledger (and so `/store.json`) recorded whole, until a rerun
/// wrote a second whole line.
#[test]
fn an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply() {
    let store = Store::new("publish-refused-whole");
    let rows: Vec<Bar> = (0..3).map(|i| bar(i, 4_407)).collect();
    let first = minute("QFIXZ", march());
    store.month(first, &rows);
    apply_month(
        &store.root,
        first,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("an earlier act");
    let path = minute("QFIXA1", march());
    store.month(path, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    audit_mode(&store, 0o555);
    let refused = apply_month(&store.root, path, SYMBOL, &every, why(Cause::BeforeListing));
    audit_mode(&store, 0o755);
    assert!(refused.is_err(), "{refused:?}");
    let between = probe(&store.root, path);
    assert!(
        between.is_err(),
        "a month the ledger records whole is not read as clear: {between:?}"
    );
    assert_eq!(
        apply_month(&store.root, path, SYMBOL, &every, why(Cause::BeforeListing)),
        Ok(Applied::Nothing {
            why: "already_whole"
        }),
        "the rerun finds the ledgered act and indexes it"
    );
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
    ));
    let key = series_key(path);
    let wholes = read_ledger(&store.root, Vendor::Zerodha)
        .expect("ledger")
        .lines
        .iter()
        .filter(|line| line.key == key && line.scope == Scope::WholeMonth)
        .count();
    assert_eq!(wholes, 1, "one act, one line");
}

/// BLOCKING 2, killed: a writer killed between its ledger fsync and its
/// index rename leaves the line and the old index. Before, every reader read
/// the month as clear until the next act of that very month.
#[test]
fn a_writer_killed_between_its_ledger_line_and_its_index_is_caught_up_by_the_next() {
    let store = Store::new("killed-before-rename");
    let rows: Vec<Bar> = (0..2).map(|i| bar(i, 4_613)).collect();
    let first = minute("QFIXB1", march());
    store.month(first, &rows);
    apply_month(
        &store.root,
        first,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("an earlier act");
    let path = minute("QFIXC1", march());
    store.month(path, &rows);
    let act = Record {
        path,
        scope: Scope::WholeMonth,
        cause: Cause::BeforeListing,
        stamps: &[],
        bars: 2,
        revision: 0,
        source_digest: digest_of(&rows),
        who: "killed",
        reason: "killed before its rename",
    };
    let line = super::ledger_line(&series_key(path), &act, 1_792_000_000_000_000);
    super::append_line(&ledger_path(&store.root, Vendor::Zerodha), &line).expect("the line");
    let between = probe(&store.root, path);
    assert!(between.is_err(), "not read as clear: {between:?}");
    // THE NEXT WRITER, of any month of the feed, catches the index up first.
    let other = minute("QFIXD1", march());
    store.month(other, &rows);
    apply_month(
        &store.root,
        other,
        SYMBOL,
        &[rows[1].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("the next act");
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
    ));
}

/// The second check's t4: an index of the right length whose header has no
/// magic and no valid checksum (here every byte zero) read as all clear.
#[test]
fn a_zeroed_index_is_not_read_as_clear() {
    let store = Store::new("zeroed-index");
    let path = minute("QFIXE1", march());
    let rows: Vec<Bar> = (0..4).map(|i| bar(i, 4_819)).collect();
    store.month(path, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    apply_month(&store.root, path, SYMBOL, &every, why(Cause::BeforeListing)).expect("whole");
    let index = index_path(&store.root, Vendor::Zerodha);
    fs::write(
        &index,
        vec![0u8; usize::try_from(INDEX_LEN).expect("a length")],
    )
    .expect("zeroed");
    assert_eq!(
        probe(&store.root, path).map_err(|why| why.code()),
        Err("quarantine_index_torn")
    );
    assert_eq!(
        Snapshot::load(&store.root, Vendor::Zerodha)
            .map(|snapshot| snapshot.len())
            .map_err(|why| why.code()),
        Err("quarantine_index_torn")
    );
    rebuild(&store.root, Vendor::Zerodha).expect("rebuilt from the ledger");
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
    ));
}

/// The second check's t2: a killed run's intent (ledgered, promoted, no
/// `dropped` line) was finished only by an apply naming its stamps again; an
/// apply of another subset of the month left it unindexed and uncounted.
#[test]
fn an_intent_a_killed_run_owes_is_finished_by_an_apply_of_another_subset() {
    let store = Store::new("owed-intent");
    let path = minute("QFIXF1", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 5_021)).collect();
    store.month(path, &rows);
    let first = [rows[1].ts_micros, rows[2].ts_micros];
    record(
        &store.root,
        &Record {
            path,
            scope: Scope::Dropping,
            cause: Cause::ForeignLevel,
            stamps: &first,
            bars: 2,
            revision: 0,
            source_digest: digest_of(&[rows[1], rows[2]]),
            who: "killed",
            reason: "killed after its promotion",
        },
    )
    .expect("the intent");
    repair::drop_month(&store.root, path, SYMBOL, &first, "killed").expect("promoted, then killed");
    let second = apply_month(
        &store.root,
        path,
        SYMBOL,
        &[rows[4].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("another subset");
    assert!(matches!(second, Applied::Dropped { .. }), "{second:?}");
    let finished = read_ledger(&store.root, Vendor::Zerodha)
        .expect("ledger")
        .lines
        .iter()
        .any(|line| line.scope == Scope::Dropped && line.stamps == first);
    assert!(finished, "the owed intent has its dropped line");
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.bars == 3
    ));
    let summary = Summary::load(&store.root, Vendor::Zerodha).expect("summary");
    assert_eq!(summary.month(path).map(|e| e.bars), Some(3));
    assert_eq!(store.served_bars(path).len(), 3);
}

/// BLOCKING 3, the restate door (DLY-08): a vendor re-pull still holds the
/// minutes a quarantine set aside, and the restate added them back while the
/// index and the ledger said they were dropped. A month recorded whole is
/// refused by name.
#[test]
fn a_restate_never_re_serves_a_quarantined_bar() {
    let store = Store::new("restate-quarantined");
    let path = minute("QFIXG1", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 5_227)).collect();
    store.month(path, &rows);
    let named = [rows[1].ts_micros, rows[2].ts_micros];
    apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)).expect("dropped");
    let mut offered = rows.clone();
    offered[0].close += 2;
    let restate = repair::Restate {
        open_interest_applies: false,
    };
    let done = repair::restate_month(
        &store.root,
        path,
        SYMBOL,
        &offered,
        restate,
        "vendor restated",
    );
    assert!(done.is_ok(), "{done:?}");
    let served = store.served_bars(path);
    let back: Vec<i64> = named
        .iter()
        .copied()
        .filter(|at| served.iter().any(|bar| bar.ts_micros == *at))
        .collect();
    assert!(back.is_empty(), "set-aside stamps served again: {back:?}");
    assert_eq!(served.len(), 4);
    assert_eq!(served[0], offered[0], "the correction itself lands");
    // A MONTH RECORDED WHOLE is refused by name, and nothing is written.
    let whole = minute("QFIXH1", march());
    store.month(whole, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    apply_month(
        &store.root,
        whole,
        SYMBOL,
        &every,
        why(Cause::BeforeListing),
    )
    .expect("whole");
    let before = all_bytes_under(&store.root);
    let refused = repair::restate_month(
        &store.root,
        whole,
        SYMBOL,
        &offered,
        restate,
        "vendor restated",
    );
    let words = refused
        .as_ref()
        .map_or_else(ToString::to_string, |done| format!("{done:?}"));
    assert!(
        refused.is_err() && words.contains("quarantined"),
        "a whole month is refused by name: {words}"
    );
    assert_eq!(all_bytes_under(&store.root), before, "nothing was written");
}

/// BLOCKING 3, the repair door the ingest calls (`pull::ingest`'s
/// `repair_after`): the same re-pull added the set-aside minutes back as bars
/// the month lacked.
#[test]
fn a_repair_never_re_adds_a_quarantined_bar() {
    let store = Store::new("repair-quarantined");
    let path = minute("QFIXI1", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 5_431)).collect();
    store.month(path, &rows);
    let named = [rows[1].ts_micros, rows[2].ts_micros];
    apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)).expect("dropped");
    let policy = repair::Policy {
        open_interest_applies: false,
        basis_witness: true,
    };
    let done = repair::repair_month(&store.root, path, SYMBOL, &rows, policy);
    assert!(done.is_ok(), "{done:?}");
    let served = store.served_bars(path);
    assert_eq!(served.len(), 4, "set-aside stamps never re-added: {done:?}");
    // A NEW minute the month truly lacks is still added beside them.
    let mut more = rows.clone();
    more.push(bar(9, 5_431));
    let added = repair::repair_month(&store.root, path, SYMBOL, &more, policy);
    assert!(
        matches!(added, Ok(repair::MonthRepair::Repaired { added: 1, .. })),
        "{added:?}"
    );
    assert_eq!(store.served_bars(path).len(), 5);
    // A MONTH RECORDED WHOLE takes no bar, by name.
    let whole = minute("QFIXJ1", march());
    store.month(whole, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    apply_month(
        &store.root,
        whole,
        SYMBOL,
        &every,
        why(Cause::BeforeListing),
    )
    .expect("whole");
    let refused = repair::repair_month(&store.root, whole, SYMBOL, &more, policy);
    let words = refused
        .as_ref()
        .map_or_else(ToString::to_string, |done| format!("{done:?}"));
    assert!(
        refused.is_err() && words.contains("quarantined"),
        "a whole month takes no bar: {words}"
    );
    assert_eq!(store.served_bars(whole).len(), 6, "nothing was written");
}

/// BLOCKING 2 at the second check's own point: the door promoted and the
/// `dropped` line was synced, and the index publish never happened (refused
/// or killed). The readers name it stale; the next writer of any month of
/// the feed indexes the drop once, and a rerun of the plan writes nothing.
#[test]
fn a_dropped_line_synced_and_never_indexed_is_caught_up_by_the_next_writer() {
    let store = Store::new("dropped-never-indexed");
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 5_633)).collect();
    let path = minute("QFIXK1", march());
    store.month(path, &rows);
    let named = [rows[2].ts_micros, rows[3].ts_micros];
    let digest = digest_of(&[rows[2], rows[3]]);
    let act = |scope, revision| Record {
        path,
        scope,
        cause: Cause::ForeignLevel,
        stamps: &named,
        bars: 2,
        revision,
        source_digest: digest,
        who: "refused publish",
        reason: "invented: its index publish never happened",
    };
    record(&store.root, &act(Scope::Dropping, 0)).expect("the intent, indexed");
    let drop = repair::drop_month(&store.root, path, SYMBOL, &named, "refused").expect("promoted");
    let line = super::ledger_line(
        &series_key(path),
        &act(Scope::Dropped, drop.to.ordinal()),
        1_792_000_000_000_000,
    );
    super::append_line(&ledger_path(&store.root, Vendor::Zerodha), &line).expect("synced");
    let between = probe(&store.root, path);
    assert_eq!(
        between.map_err(|why| why.code()),
        Err("quarantine_index_stale"),
        "the readers name the lag"
    );
    let other = minute("QFIXL1", march());
    store.month(other, &rows);
    apply_month(
        &store.root,
        other,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("the next writer");
    assert!(matches!(
        probe(&store.root, path),
        Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
    ));
    assert_eq!(
        apply_month(&store.root, path, SYMBOL, &named, why(Cause::ForeignLevel)),
        Ok(Applied::Nothing {
            why: "already_dropped"
        })
    );
    let dropped = read_ledger(&store.root, Vendor::Zerodha)
        .expect("ledger")
        .lines
        .iter()
        .filter(|line| line.scope == Scope::Dropped && line.key == series_key(path))
        .count();
    assert_eq!(dropped, 1, "indexed once, never a second line");
}

/// THE SELF-CHECK OF REPAIR 2: a reader answered from an image BEHIND its
/// ledger whenever any writer held the lock, taking the lag for that writer's
/// own act in flight. The lag may as well be an act a failed publish left
/// unindexed that the next writer is still bringing into the index (lock
/// taken, rebuild not yet renamed): then a month that act recorded whole read
/// as clear and its bars were served. An image behind its ledger is never the
/// answer now: a reader with no patience (the CLI's, the importer's) is
/// refused by name at once, a patient one (a page's) waits within its
/// patience and is refused by name when it runs out, and one whose writer's
/// level image lands in time answers from that image.
#[test]
fn an_image_behind_its_ledger_is_never_the_answer_while_a_writer_holds_the_lock() {
    let store = Store::new("lag-under-lock");
    let rows: Vec<Bar> = (0..2).map(|i| bar(i, 6_037)).collect();
    let first = minute("QFIXM1", march());
    store.month(first, &rows);
    apply_month(
        &store.root,
        first,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("an earlier act");
    // AN ACT LEDGERED AND NEVER INDEXED: its publish was refused.
    let path = minute("QFIXN1", march());
    store.month(path, &rows);
    let act = Record {
        path,
        scope: Scope::WholeMonth,
        cause: Cause::BeforeListing,
        stamps: &[],
        bars: 2,
        revision: 0,
        source_digest: digest_of(&rows),
        who: "refused publish",
        reason: "invented: its index publish was refused",
    };
    let line = super::ledger_line(&series_key(path), &act, 1_792_000_000_000_000);
    super::append_line(&ledger_path(&store.root, Vendor::Zerodha), &line).expect("never indexed");
    // THE NEXT WRITER HOLDS THE LOCK, its catch-up not yet published.
    let held = super::writer_lock(&store.root, Vendor::Zerodha, super::WRITER_PATIENCE)
        .expect("the next writer's lock");
    assert_eq!(
        probe(&store.root, path).map_err(|why| why.code()),
        Err("quarantine_being_written"),
        "a reader with no patience is refused by name, never the stale image's clear"
    );
    assert_eq!(
        Snapshot::load(&store.root, Vendor::Zerodha)
            .map(|snapshot| snapshot.get(path))
            .map_err(|why| why.code()),
        Err("quarantine_being_written")
    );
    let started = std::time::Instant::now();
    let patient = crate::flock::with_patience(std::time::Duration::from_millis(60), || {
        probe(&store.root, path)
    });
    let waited = started.elapsed();
    let words = patient
        .as_ref()
        .map_or_else(ToString::to_string, |found| format!("{found:?}"));
    assert_eq!(
        patient.map_err(|why| why.code()),
        Err("quarantine_being_written")
    );
    assert!(
        waited >= std::time::Duration::from_millis(60),
        "a patient reader waits its patience first: {waited:?}"
    );
    assert!(
        words.contains(crate::flock::BEING_WRITTEN),
        "the sentence asks again: {words}"
    );
    // THE WRITER'S LEVEL IMAGE LANDS within a waiting reader's patience, the
    // lock still held: built from the ledger beforehand and renamed in while
    // the reader waits, as `publish_image` renames (its syncs left out, so
    // the landing is not at the mercy of a loaded disk).
    let ledger = read_ledger(&store.root, Vendor::Zerodha).expect("the ledger");
    let level = super::image_of(&ledger, &store.root, Vendor::Zerodha).expect("the level image");
    let staged = store.root.join("audit").join(".level-image.test-staging");
    fs::write(&staged, &level).expect("staged");
    let root = store.root.clone();
    let reader = std::thread::spawn(move || {
        crate::flock::with_patience(crate::flock::MAX_PATIENCE, || probe(&root, path))
    });
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::rename(&staged, index_path(&store.root, Vendor::Zerodha)).expect("the level image lands");
    let after = reader.join().expect("the reader ends");
    held.release().expect("the lock let go");
    assert!(
        matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
        "{after:?}"
    );
}

/// THE SELF-CHECK OF REPAIR 2, the doors: a repair judged the quarantine
/// before it opened the served month, so a drop intended and promoted between
/// the two read as nothing set aside and its stamps were added back. The door
/// judges it once the month is open now; this is the other half of that
/// protocol, which holds by the promotion's check: a drop landing after the
/// repair opened the month (here between its publication and its promotion)
/// refuses the repair's promotion, and the repair asked again holds the
/// dropped stamps back and adds only the minute the month truly lacks.
#[test]
fn a_drop_landing_inside_a_repair_refuses_its_promotion_and_the_retry_holds_the_stamps_back() {
    let store = Store::new("repair-meets-drop");
    let path = minute("QFIXO1", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 6_241)).collect();
    store.month(path, &rows);
    let named = [rows[2].ts_micros, rows[3].ts_micros];
    let mut offered = rows.clone();
    offered.push(bar(9, 6_241));
    let policy = repair::Policy {
        open_interest_applies: false,
        basis_witness: true,
    };
    let root = store.root.clone();
    let mut drop_lands = |_revision: &Path| {
        apply_month(&root, path, SYMBOL, &named, why(Cause::ForeignLevel))
            .map(|_| ())
            .map_err(|why| why.to_string())
    };
    let raced = repair::repair_month_with(
        &store.root,
        path,
        SYMBOL,
        &offered,
        policy,
        Some(&mut drop_lands),
    );
    assert_eq!(
        raced.as_ref().err(),
        Some(&RepairError::StaleSource),
        "{raced:?}"
    );
    let served: Vec<i64> = store
        .served_bars(path)
        .iter()
        .map(|b| b.ts_micros)
        .collect();
    assert_eq!(served.len(), 4, "the drop stands: {served:?}");
    assert!(named.iter().all(|at| !served.contains(at)), "{served:?}");
    let again = repair::repair_month(&store.root, path, SYMBOL, &offered, policy);
    assert!(
        matches!(again, Ok(repair::MonthRepair::Repaired { added: 1, .. })),
        "{again:?}"
    );
    let served: Vec<i64> = store
        .served_bars(path)
        .iter()
        .map(|b| b.ts_micros)
        .collect();
    assert_eq!(served.len(), 5, "{served:?}");
    assert!(
        named.iter().all(|at| !served.contains(at)),
        "never re-served: {served:?}"
    );
}

// ---------------------------------------------------------------------------
// R4-18 repair 2, the resumed self-check (24 Oct). Each test below FAILED on
// the code at 304c702f5 and names the defect it closes.
// ---------------------------------------------------------------------------

/// A HEAD TORN BY A KILLED FIRST WRITE never stops a later writer. A feed's
/// first act writes the ledger's head and its line in ONE write, so a writer
/// killed inside it (or refused by a full disk) leaves a strict prefix of the
/// head; the next writer ends that fragment and writes the head again. Before,
/// every writer after that read the ledger's first bytes, found no head there
/// and refused `quarantine_ledger_damaged` at byte 0 for ever — the first
/// apply's own `dropped` line among them, after its door had promoted — while
/// every reader of the ledger read it whole and undamaged.
#[test]
fn a_head_torn_by_a_killed_first_write_never_stops_a_later_writer() {
    for (tag, torn) in [
        ("torn-in-the-head", b"brutex quar".to_vec()),
        ("torn-before-its-newline", LEDGER_HEAD.as_bytes().to_vec()),
        ("two-first-writes-killed", b"brut\ttorn\nbrutex q".to_vec()),
    ] {
        let store = Store::new(tag);
        let path = minute("QFIXH3", march());
        let rows: Vec<Bar> = (0..5).map(|i| bar(i, 1_847)).collect();
        store.month(path, &rows);
        fs::create_dir_all(store.root.join("audit")).expect("the audit directory");
        let ledger = ledger_path(&store.root, Vendor::Zerodha);
        fs::write(&ledger, &torn).expect("what the killed first write left");
        for (round, stamp) in [rows[1].ts_micros, rows[3].ts_micros]
            .into_iter()
            .enumerate()
        {
            let done = apply_month(&store.root, path, SYMBOL, &[stamp], why(Cause::ZeroPrice));
            assert!(
                matches!(done, Ok(Applied::Dropped { .. })),
                "{tag}: apply {round}: {done:?}"
            );
        }
        let read = read_ledger(&store.root, Vendor::Zerodha).expect("reads");
        assert!(read.damaged.is_empty(), "{tag}: {:?}", read.damaged);
        assert_eq!(read.lines.len(), 4, "{tag}: two intents, two dropped lines");
        assert!(
            !read.ended.is_empty(),
            "{tag}: the torn head is an ended append"
        );
        assert!(
            fs::read(&ledger).expect("bytes").starts_with(&torn),
            "{tag}: no byte the killed write left is changed"
        );
        assert!(
            matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
            "{tag}"
        );
        assert_eq!(store.served_bars(path).len(), 3, "{tag}");
    }
}

/// A LEDGER CUT BELOW WHAT ITS INDEX DESCRIBES — put back from an older copy,
/// or truncated — is refused by name and never rebuilt away: the index
/// records acts the ledger no longer holds, and a rebuild from what is left
/// would forget them. Before, the next writer of any month of the feed
/// rebuilt the index from the shorter ledger and answered success, and the
/// month the lost line had recorded whole read as clear and was served.
#[test]
fn a_ledger_cut_below_what_its_index_describes_is_refused_and_never_rebuilt_away() {
    let store = Store::new("ledger-cut");
    let rows: Vec<Bar> = (0..4).map(|i| bar(i, 3_407)).collect();
    let first = minute("QFIXC3", march());
    store.month(first, &rows);
    apply_month(
        &store.root,
        first,
        SYMBOL,
        &[rows[0].ts_micros],
        why(Cause::ZeroPrice),
    )
    .expect("an earlier drop");
    let ledger = ledger_path(&store.root, Vendor::Zerodha);
    let older = fs::read(&ledger).expect("the copy a restore would put back");
    let path = minute("QFIXD3", march());
    store.month(path, &rows);
    let every: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    apply_month(&store.root, path, SYMBOL, &every, why(Cause::BeforeListing)).expect("whole");
    // THE LEDGER PUT BACK FROM THE OLDER COPY: the whole line is gone.
    fs::write(&ledger, &older).expect("restored from an older copy");
    let index = index_path(&store.root, Vendor::Zerodha);
    let files = || {
        (
            fs::read(&ledger).expect("ledger"),
            fs::read(&index).expect("index"),
        )
    };
    let before = files();
    // Every reader names it, never "clear" ...
    assert_eq!(
        probe(&store.root, path).map_err(|why| why.code()),
        Err("quarantine_index_stale")
    );
    // ... the next writer, of another month, refuses by name and writes
    // nothing ...
    let other = minute("QFIXE3", march());
    store.month(other, &rows);
    let next = apply_month(
        &store.root,
        other,
        SYMBOL,
        &[rows[1].ts_micros],
        why(Cause::ZeroPrice),
    );
    assert_eq!(refusal_code(&next), "quarantine_ledger_cut");
    let words = next.expect_err("refused").to_string();
    assert!(
        words.contains(&older.len().to_string()),
        "the ledger's length is named: {words}"
    );
    assert_eq!(files(), before, "nothing was written");
    assert_eq!(store.served_bars(other).len(), 4, "no generation published");
    // ... and so does a catch-up: the month the lost line recorded whole is
    // never read as clear.
    assert_eq!(
        catch_up(&store.root, Vendor::Zerodha).map_err(|why| why.code()),
        Err("quarantine_ledger_cut")
    );
    assert!(probe(&store.root, path).is_err(), "never clear");
    // ... and the listings name it instead of counting without the lost act.
    assert_eq!(
        Summary::load(&store.root, Vendor::Zerodha)
            .map(|summary| summary.is_empty())
            .map_err(|why| why.code()),
        Err("quarantine_ledger_cut")
    );
    assert_eq!(files(), before, "still nothing written");
}

/// A MONTH DROPPED FOR ONE CAUSE AND THEN RECORDED WHOLE FOR ANOTHER counts
/// every bar it set aside and names both causes — in the index, its rebuild,
/// the snapshot and the summary (the second check's follow-up on `merged`).
/// Before, the whole record kept only its own bars (the served generation's,
/// after the drop) and its own cause: the reader said "all 2 of its bars are
/// `zero_price`" of a month that set aside 6, and the listing under-counted.
#[test]
fn a_month_dropped_then_recorded_whole_counts_every_bar_and_names_every_cause() {
    let store = Store::new("drop-then-whole");
    let path = minute("QFIXF3", march());
    let rows: Vec<Bar> = (0..6).map(|i| bar(i, 2_633)).collect();
    store.month(path, &rows);
    let foreign: Vec<i64> = rows[..4].iter().map(|b| b.ts_micros).collect();
    let dropped = apply_month(
        &store.root,
        path,
        SYMBOL,
        &foreign,
        why(Cause::ForeignLevel),
    )
    .expect("dropped");
    assert!(matches!(dropped, Applied::Dropped { .. }), "{dropped:?}");
    let rest: Vec<i64> = rows[4..].iter().map(|b| b.ts_micros).collect();
    let whole = apply_month(&store.root, path, SYMBOL, &rest, why(Cause::ZeroPrice))
        .expect("the rest recorded whole");
    assert!(
        matches!(whole, Applied::WholeMonth { bars: 2, .. }),
        "{whole:?}"
    );
    let both = Causes::of(Cause::ForeignLevel).union(Causes::of(Cause::ZeroPrice));
    let says = |what: &str, entry: Option<Entry>| {
        let entry = entry.unwrap_or_else(|| panic!("{what}: the month is named"));
        assert_eq!(
            (entry.scope, entry.cause, entry.bars),
            (Scope::WholeMonth, Cause::ZeroPrice, 6),
            "{what}"
        );
        assert_eq!(entry.causes, both, "{what}");
        assert_eq!(
            entry.causes.said(),
            "foreign_level and zero_price",
            "{what}"
        );
    };
    let probed = || match probe(&store.root, path) {
        Ok(Probe::Quarantined(entry)) => Some(entry),
        other => panic!("{other:?}"),
    };
    says("the index", probed());
    rebuild(&store.root, Vendor::Zerodha).expect("rebuilt");
    says("the index rebuilt from its ledger", probed());
    says(
        "the snapshot",
        Snapshot::load(&store.root, Vendor::Zerodha)
            .expect("loads")
            .get(path),
    );
    let summary = Summary::load(&store.root, Vendor::Zerodha).expect("a summary");
    says("the summary", summary.month(path));
    let row = summary
        .instrument("NSE", "CASH", "QFIXF3")
        .expect("flagged");
    assert_eq!((row.months, row.whole_months, row.bars), (1, 1, 6));
    assert_eq!(row.causes, vec![Cause::ForeignLevel, Cause::ZeroPrice]);
}

//! Month generations: a repair publishes a NEW generation of a month and every
//! reader opens it, never in place, never dropping or changing a held bar.
//! D-3350, WP-13c; findings DB-N20, DB-N22. Temporary synthetic stores only;
//! every price here is invented.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test-only assertions"
)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::generation::{self, Promoted, Served};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::repair::{self, MonthRepair, Policy, Published, RepairError, Revision};

static NEXT: AtomicU64 = AtomicU64::new(0);

const SYMBOL: u32 = 11;
const INDEX: Policy = Policy {
    open_interest_applies: false,
    basis_witness: false,
};

struct Store(PathBuf);

impl Store {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-generation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn hold(&self, rows: &[Bar]) {
        let mut writer = BarFile::open_or_create(&self.0, month(), SYMBOL).unwrap();
        writer.append(rows).unwrap();
    }

    fn read_all(&self) -> Vec<Bar> {
        let file = BarFile::open_existing(&self.0, month(), SYMBOL).unwrap();
        (0..file.header().n_valid)
            .map(|i| file.read_record(i).unwrap())
            .collect()
    }

    fn original_bytes(&self) -> (Vec<u8>, Vec<u8>) {
        (
            fs::read(month().to_path_buf(&self.0)).unwrap(),
            fs::read(month().with_file(FileKind::Checksums).to_path_buf(&self.0)).unwrap(),
        )
    }

    fn pointer(&self) -> PathBuf {
        generation::pointer_path(&self.0, month())
    }

    fn repair(&self, offered: &[Bar], policy: Policy) -> Result<MonthRepair, RepairError> {
        repair::repair_month(&self.0, month(), SYMBOL, offered, policy)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "FIXTUREIDX",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// 2024-06-01 00:00 IST, in UTC seconds.
const JUNE_IST_MIDNIGHT: i64 = 1_717_180_200;

/// A minute bar on day `day` of June (1-based), `minute` minutes after 09:15.
/// Prices are invented and carry the stamp so no two bars are equal.
fn bar(day: i64, minute: i64) -> Bar {
    let secs = JUNE_IST_MIDNIGHT + (day - 1) * 86_400 + 555 * 60 + minute * 60;
    let price = 1_000_000 + day * 1_000 + minute;
    Bar {
        ts_micros: secs * 1_000_000,
        open: price,
        high: price + 7,
        low: price - 7,
        close: price + 1,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// Five-minute "sessions" on each listed day: enough to be a session-shaped
/// fixture without 375 rows a day.
/// Whole one-minute sessions of 375 bars on `days`: three of them pass the
/// small-month threshold (D-4933), so the month and its revision carry a
/// `.tix`.
fn full_sessions(days: &[i64]) -> Vec<Bar> {
    days.iter()
        .flat_map(|&day| (0..375).map(move |minute| bar(day, minute)))
        .collect()
}

fn sessions(days: &[i64]) -> Vec<Bar> {
    days.iter()
        .flat_map(|&day| (0..5).map(move |minute| bar(day, minute)))
        .collect()
}

/// THE DB-N20 SHAPE, ON A FIXTURE: a month that holds its later days and lacks
/// its first ones. An ordinary append of the first days is refused by the
/// store; a repair serves a new generation with all of them, every reader opens
/// it, and the original is byte-identical.
#[test]
fn a_month_with_a_leading_hole_is_repaired_to_a_new_generation_every_reader_opens() {
    let store = Store::new();
    let held = sessions(&[17, 18, 19]);
    store.hold(&held);
    let before = store.original_bytes();

    // The append-only door refuses the leading days, as on the real store.
    let leading = sessions(&[3, 4]);
    let refused = BarFile::open_or_create(&store.0, month(), SYMBOL)
        .unwrap()
        .append(&leading);
    assert!(
        matches!(refused, Err(StoreError::OverlapDisagrees { .. })),
        "{refused:?}"
    );

    let offered = sessions(&[3, 4, 17, 18, 19]);
    let done = store.repair(&offered, INDEX).unwrap();
    let MonthRepair::Repaired {
        from,
        to,
        held: kept,
        added,
        added_at,
        header,
        published,
        promoted,
        skipped,
    } = done
    else {
        panic!("{done:?}");
    };
    assert_eq!(
        added_at,
        sessions(&[3, 4])
            .iter()
            .map(|b| b.ts_micros)
            .collect::<Vec<_>>(),
        "the stamps it added, ascending"
    );
    assert_eq!((from, to.ordinal(), kept, added), (0, 1, 15, 10));
    assert_eq!(header.n_valid, 25);
    assert_eq!(header.first_ts_micros, bar(3, 0).ts_micros);
    assert_eq!(published, Published::Created);
    assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
    assert!(skipped.is_empty());

    // EVERY READER: the ordinary door now opens the new generation.
    assert_eq!(store.read_all(), offered);
    let served = generation::served(&store.0, month()).unwrap();
    assert_eq!(served.ordinal(), 1);
    assert_eq!(
        generation::served_bars_path(&store.0, month()).unwrap(),
        month().to_path_buf(&store.0.join("bar-revisions-v1/1"))
    );
    let file = BarFile::open_existing(&store.0, month(), SYMBOL).unwrap();
    assert_eq!(file.first_at_or_after(bar(4, 2).ts_micros).unwrap(), 7);
    let audited = BarFile::open_existing_audited(&store.0, month(), SYMBOL, 1 << 20);
    assert!(audited.is_ok(), "{:?}", audited.err());

    // NEVER IN PLACE: the original month is byte for byte what it was.
    assert_eq!(store.original_bytes(), before);
}

/// The writer door refuses the original of a repaired month by name, before it
/// creates or locks anything: an append there would never be served.
#[test]
fn the_writer_door_refuses_a_month_served_from_a_revision() {
    let store = Store::new();
    store.hold(&sessions(&[10]));
    store.repair(&sessions(&[3, 10]), INDEX).unwrap();
    let refused = BarFile::open_or_create(&store.0, month(), SYMBOL).err();
    let said = refused
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    match refused {
        Some(StoreError::Superseded { ordinal, path }) => {
            assert_eq!(ordinal, 1);
            assert_eq!(path, month().to_path_buf(&store.0));
            assert!(said.contains("served from repair revision 1"), "{said}");
        }
        other => panic!("{other:?}"),
    }
}

/// A held bar is never dropped and never changed: an offered bar at a held
/// stamp with other values refuses the month whole, naming the bar, and
/// nothing is written.
#[test]
fn a_held_bar_that_would_change_refuses_the_month_and_writes_nothing() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let mut offered = sessions(&[3, 17]);
    offered[7].close += 5; // day 17, minute 2
    let refused = store.repair(&offered, INDEX).unwrap_err();
    match &refused {
        RepairError::HeldBarDiffers {
            ts_micros, held, ..
        } => {
            assert_eq!(*ts_micros, bar(17, 2).ts_micros);
            assert_eq!(**held, bar(17, 2));
        }
        other => panic!("{other:?}"),
    }
    assert!(refused.to_string().contains("never changes a held bar"));
    assert!(!store.0.join("bar-revisions-v1").exists());
    assert!(!store.pointer().exists());
    assert_eq!(store.read_all(), sessions(&[17]));
}

/// Open interest that is not a measurement (an index, a share, D-3322): a held
/// `0` and an offered null are one absence, and the held value is kept. Where
/// it is a measurement, they differ and the month is refused.
#[test]
fn open_interest_is_compared_only_where_it_is_a_measurement() {
    let store = Store::new();
    let mut held = sessions(&[17]);
    for bar in &mut held {
        bar.open_interest = 0;
    }
    store.hold(&held);
    let offered = sessions(&[3, 17]);
    let strict = Policy {
        open_interest_applies: true,
        basis_witness: false,
    };
    assert!(matches!(
        store.repair(&offered, strict),
        Err(RepairError::HeldBarDiffers { .. })
    ));
    assert!(matches!(
        store.repair(&offered, INDEX).unwrap(),
        MonthRepair::Repaired { added: 5, .. }
    ));
    let read = store.read_all();
    assert_eq!(&read[5..], &held[..], "the held bars keep their own values");
    assert_eq!(&read[..5], &sessions(&[3])[..]);
}

/// A back-adjusted feed (D-3323): bars that share no stamp with the held ones
/// prove nothing about the price basis, and are refused until the window
/// includes a held session.
#[test]
fn a_back_adjusted_feed_needs_a_held_bar_in_the_offered_window() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let shares = Policy {
        open_interest_applies: false,
        basis_witness: true,
    };
    assert_eq!(
        store.repair(&sessions(&[3]), shares),
        Err(RepairError::NoBasisWitness)
    );
    assert!(matches!(
        store.repair(&sessions(&[3, 17]), shares).unwrap(),
        MonthRepair::Repaired { added: 5, .. }
    ));
}

/// Idempotent: a repeat is nothing new and writes nothing. Resumable: a crash
/// after publication and before promotion is finished by the next call, which
/// finds the revision complete and promotes it.
#[test]
fn a_repeat_writes_nothing_and_an_interrupted_repair_resumes() {
    let store = Store::new();
    let held = sessions(&[17]);
    store.hold(&held);
    let offered = sessions(&[3, 17]);

    // THE CRASH: published, never promoted.
    let header = BarFile::open_existing(&store.0, month(), SYMBOL)
        .unwrap()
        .header();
    let merged = sessions(&[3, 17]);
    assert_eq!(
        repair::publish(
            &store.0,
            month(),
            SYMBOL,
            Revision::new(1).unwrap(),
            header,
            &merged
        )
        .unwrap(),
        Published::Created
    );
    assert_eq!(
        store.read_all(),
        held,
        "unpromoted: readers see generation 0"
    );

    let resumed = store.repair(&offered, INDEX).unwrap();
    assert!(
        matches!(
            resumed,
            MonthRepair::Repaired {
                published: Published::Reused,
                ..
            }
        ),
        "{resumed:?}"
    );
    assert_eq!(store.read_all(), offered);

    let pointer_before = fs::read(store.pointer()).unwrap();
    assert_eq!(
        store.repair(&offered, INDEX).unwrap(),
        MonthRepair::NothingNew {
            ordinal: 1,
            n_valid: 10
        }
    );
    assert_eq!(fs::read(store.pointer()).unwrap(), pointer_before);
}

/// An ordinal holding an unfinished reservation is never reopened: the next
/// is used, and the skipped one is named.
#[test]
fn an_abandoned_ordinal_is_stepped_over_and_named() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let reserved = month()
        .to_path_buf(&store.0.join("bar-revisions-v1/1"))
        .with_extension("reserved-v1");
    fs::create_dir_all(reserved.parent().unwrap()).unwrap();
    fs::write(&reserved, b"").unwrap();
    let MonthRepair::Repaired { to, skipped, .. } =
        store.repair(&sessions(&[3, 17]), INDEX).unwrap()
    else {
        panic!("repaired");
    };
    assert_eq!(to.ordinal(), 2);
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].0, 1);
    assert!(skipped[0].1.contains("incomplete repair"), "{skipped:?}");
    assert!(reserved.exists(), "the abandoned evidence is kept");
}

/// A second repair builds on the first: generation 2 holds generation 1's
/// added bars too, and names the generation it replaced.
#[test]
fn a_chain_of_repairs_keeps_every_bar_of_every_generation() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    store.repair(&sessions(&[3, 17]), INDEX).unwrap();
    let second = store.repair(&sessions(&[5, 17]), INDEX).unwrap();
    let MonthRepair::Repaired {
        from, to, promoted, ..
    } = second
    else {
        panic!("{second:?}");
    };
    assert_eq!((from, to.ordinal()), (1, 2));
    assert_eq!(promoted.pointer().previous, 1);
    assert_eq!(store.read_all(), sessions(&[3, 5, 17]));
}

/// A revision published from a generation that is no longer served is not
/// promoted: it would drop the bars the later generation added.
#[test]
fn a_revision_made_from_a_superseded_generation_is_not_promoted() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let original = BarFile::open_existing(&store.0, month(), SYMBOL)
        .unwrap()
        .header();
    // Published from the original, then overtaken by another repair.
    let stale = Revision::new(7).unwrap();
    repair::publish(
        &store.0,
        month(),
        SYMBOL,
        stale,
        original,
        &sessions(&[5, 17]),
    )
    .unwrap();
    store.repair(&sessions(&[3, 17]), INDEX).unwrap();
    assert_eq!(
        generation::promote(&store.0, month(), SYMBOL, stale),
        Err(RepairError::StaleSource)
    );
    assert_eq!(store.read_all(), sessions(&[3, 17]));
    // And promoting the one already served is a no-op.
    assert!(matches!(
        generation::promote(&store.0, month(), SYMBOL, Revision::new(1).unwrap()),
        Ok(Promoted::Already(_))
    ));
}

fn append_raw(path: &Path, bytes: &[u8]) {
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

/// A torn pointer record (a crash inside a promotion) is stepped over: the
/// generation served is the last one whose promotion completed. Damage to
/// every record is a refusal, never a quiet fall back to the original.
#[test]
fn a_torn_or_damaged_pointer_serves_the_last_complete_promotion_or_refuses() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    store.repair(&sessions(&[3, 17]), INDEX).unwrap();
    append_raw(&store.pointer(), &[0xAB; 20]);
    assert_eq!(
        generation::served(&store.0, month()).unwrap().ordinal(),
        1,
        "a torn tail is not a promotion"
    );
    assert_eq!(store.read_all(), sessions(&[3, 17]));

    // A further repair aligns past the torn bytes and is served.
    store.repair(&sessions(&[5, 17]), INDEX).unwrap();
    assert_eq!(generation::served(&store.0, month()).unwrap().ordinal(), 2);
    assert_eq!(fs::metadata(store.pointer()).unwrap().len(), 3 * 64);

    // The newest record damaged: the one before it is served.
    let mut bytes = fs::read(store.pointer()).unwrap();
    bytes[2 * 64 + 20] ^= 0x01;
    fs::write(store.pointer(), &bytes).unwrap();
    assert_eq!(generation::served(&store.0, month()).unwrap().ordinal(), 1);

    // Every record damaged: refused, naming the pointer.
    bytes[20] ^= 0x01;
    fs::write(store.pointer(), &bytes).unwrap();
    match generation::served(&store.0, month()) {
        Err(StoreError::Generation { path, reason }) => {
            assert_eq!(path, store.pointer());
            assert!(
                reason.contains("none of its 3 record(s) verifies"),
                "{reason}"
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        BarFile::open_existing(&store.0, month(), SYMBOL),
        Err(StoreError::Generation { .. })
    ));
}

/// A pointer with no record yet (a crash before the first record landed) is
/// the original month.
#[test]
fn an_empty_pointer_is_the_original_month() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    fs::create_dir_all(store.pointer().parent().unwrap()).unwrap();
    fs::write(store.pointer(), b"").unwrap();
    assert_eq!(
        generation::served(&store.0, month()).unwrap(),
        Served::Original
    );
    assert_eq!(store.read_all(), sessions(&[17]));
}

/// A promoted revision written to outside the repair protocol is not served
/// as the one its promotion recorded.
#[test]
fn a_revision_that_moved_after_promotion_is_refused() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    store.repair(&sessions(&[3, 17]), INDEX).unwrap();
    let revision_root = store.0.join("bar-revisions-v1/1");
    BarFile::open_or_create(&revision_root, month(), SYMBOL)
        .unwrap()
        .append(&sessions(&[20]))
        .unwrap();
    match BarFile::open_existing(&store.0, month(), SYMBOL) {
        Err(StoreError::Generation { reason, .. }) => {
            assert!(
                reason.contains("written outside the repair protocol"),
                "{reason}"
            );
        }
        other => panic!("{other:?}"),
    }
}

/// A month with a record-indexed sibling is not repaired: inserting bars would
/// shift every overlay row.
#[test]
fn a_month_with_an_overlay_is_refused() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let overlay = month().with_file(FileKind::Overlay).to_path_buf(&store.0);
    fs::write(&overlay, b"x").unwrap();
    assert_eq!(
        store.repair(&sessions(&[3, 17]), INDEX),
        Err(RepairError::IndexedSibling(overlay))
    );
}

/// A month that does not exist has nothing to keep: the repair refuses with
/// the store's own `Missing`, and an ordinary append creates it.
#[test]
fn a_missing_month_is_for_an_ordinary_append() {
    let store = Store::new();
    assert!(matches!(
        store.repair(&sessions(&[3]), INDEX),
        Err(RepairError::Store(StoreError::Missing { .. }))
    ));
    assert!(!store.0.join("bar-revisions-v1").exists());
}

/// Nothing outside the bar month has a generation; asking is the original.
#[test]
fn only_bar_months_have_generations() {
    let store = Store::new();
    let overlay = month().with_file(FileKind::Overlay);
    assert_eq!(
        generation::served(&store.0, overlay).unwrap(),
        Served::Original
    );
}

/// Provenance is read beside the bars that are served: a repaired month's
/// revision has no sidecar of its own yet, so the original's — whose records
/// index the original's bars — is never read for it.
#[test]
fn provenance_is_read_beside_the_served_generation() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let original_prv = month()
        .with_file(FileKind::Provenance)
        .to_path_buf(&store.0);
    fs::write(&original_prv, [0xAB_u8; 320]).unwrap();
    assert!(
        store::provenance::read(&store.0, month()).is_err(),
        "the original's is read"
    );
    store.repair(&sessions(&[3, 17]), INDEX).unwrap();
    assert!(matches!(
        store::provenance::read(&store.0, month()),
        Ok(store::provenance::Found::Absent)
    ));
}

/// A linked pointer directory is refused, never followed: the promotion
/// would land wherever the link points.
#[test]
fn a_linked_pointer_directory_is_refused() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let elsewhere = Store::new();
    std::os::unix::fs::symlink(&elsewhere.0, store.0.join(generation::ROOT)).unwrap();
    assert!(matches!(
        store.repair(&sessions(&[3, 17]), INDEX),
        Err(RepairError::Store(StoreError::Symlinked { .. }))
    ));
    assert_eq!(
        store.read_all(),
        sessions(&[17]),
        "nothing is served from a link"
    );
    assert!(fs::read_dir(&elsewhere.0).unwrap().next().is_none());
}

/// THE `.tix` REBUILD INDEXES THE GENERATION EVERY READER OPENS (D-3350,
/// D-3340). `cli tix-rebuild` over a repaired month rebuilds the revision's
/// time index, the one lookups read, and never touches the original's.
#[test]
fn the_time_index_rebuild_indexes_the_served_generation() {
    let store = Store::new();
    store.hold(&full_sessions(&[17, 18, 19]));
    store
        .repair(&full_sessions(&[3, 17, 18, 19]), INDEX)
        .unwrap();
    let revision_tix = month()
        .with_file(FileKind::TimeIndex)
        .to_path_buf(&store.0.join("bar-revisions-v1/1"));
    let original_tix = month().with_file(FileKind::TimeIndex).to_path_buf(&store.0);
    let original = fs::read(&original_tix).unwrap();
    assert!(revision_tix.is_file(), "the revision is born indexed");

    // Kept as published: the rebuild reads the revision and writes nothing.
    let kept = BarFile::index_existing(&store.0, month(), SYMBOL).unwrap();
    assert_eq!(kept.n_valid, 1_500, "the served generation's bars");
    assert_eq!(kept.outcome, store::file::Reindexed::Kept);

    // A revision whose index went missing is rebuilt where it lives.
    fs::remove_file(&revision_tix).unwrap();
    let built = BarFile::index_existing(&store.0, month(), SYMBOL).unwrap();
    assert_eq!(built.n_valid, 1_500);
    assert!(
        matches!(built.outcome, store::file::Reindexed::Built(_)),
        "{:?}",
        built.outcome
    );
    assert!(revision_tix.is_file());
    assert_eq!(
        fs::read(&original_tix).unwrap(),
        original,
        "never the original's"
    );
    let file = BarFile::open_existing(&store.0, month(), SYMBOL).unwrap();
    assert_eq!(file.first_at_or_after(bar(17, 0).ts_micros).unwrap(), 375);
}

/// A WRITER HOLDING THE ORIGINAL MONTH REFUSES ITS PROMOTION (D-3350). A
/// promotion holds the original month shared until its pointer record is
/// written, so a writer mid-append cannot add bars to a generation that stops
/// being served under it: the promotion is refused by name and promotes once
/// the writer is gone, and that writer's later open is refused.
#[test]
fn a_writer_holding_the_month_refuses_its_promotion() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    let header = BarFile::open_existing(&store.0, month(), SYMBOL)
        .unwrap()
        .header();
    let revision = Revision::new(1).unwrap();
    repair::publish(
        &store.0,
        month(),
        SYMBOL,
        revision,
        header,
        &sessions(&[3, 17]),
    )
    .unwrap();

    let writer = BarFile::open_or_create(&store.0, month(), SYMBOL).unwrap();
    let refused = generation::promote(&store.0, month(), SYMBOL, revision);
    assert!(
        matches!(refused, Err(RepairError::Store(StoreError::Locked { .. }))),
        "{refused:?}"
    );
    assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
    drop(writer);
    assert_eq!(store.read_all(), sessions(&[17]), "still generation 0");

    let promoted = generation::promote(&store.0, month(), SYMBOL, revision).unwrap();
    assert!(matches!(promoted, Promoted::Promoted(_)), "{promoted:?}");
    assert_eq!(store.read_all(), sessions(&[3, 17]));
    assert!(matches!(
        BarFile::open_or_create(&store.0, month(), SYMBOL),
        Err(StoreError::Superseded { ordinal: 1, .. })
    ));
}

/// A NON-DIRECTORY INSIDE THE POINTER TREE IS REFUSED, never read as "no
/// promotion": which generation the month serves cannot then be told. A
/// store root that is itself a file is the original's refusal, named by the
/// door that opens it (`write.rs`, D-1522).
#[test]
fn a_file_where_the_pointer_tree_belongs_is_refused() {
    let store = Store::new();
    store.hold(&sessions(&[17]));
    fs::write(store.0.join(generation::ROOT), b"not a directory").unwrap();
    let refused = generation::served(&store.0, month());
    assert!(
        matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
        "{refused:?}"
    );
    assert!(BarFile::open_existing(&store.0, month(), SYMBOL).is_err());
}

/// THE DB-N26 SHAPE: A ONE-SECOND MONTH PAST THE OLD 100,000-ROW CEILING TAKES
/// A DAY IT LACKS AS A NEW GENERATION (WP-12d). Five held "sessions" of 22,500
/// one-second bars (the regular session's length) are 112,500 bars, which
/// `repair_month` refused `RowLimit` before the ceiling became the most one
/// month can hold; a GDFL NIFTY one-second month holds about 409,000. The
/// day between them lands, every held bar is kept, and the original is
/// byte-identical.
#[test]
fn a_one_second_month_past_the_old_ceiling_takes_a_missing_day_as_a_new_generation() {
    const SESSION: i64 = 22_500;
    let store = Store::new();
    let seconds = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "FIXTUREIDX",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    // An invented one-second bar `second` seconds after 09:15 IST on day
    // `day` of June 2024.
    let at = |day: i64, second: i64| {
        let price = 2_000_000 + day * 10_000 + second % 997;
        Bar {
            ts_micros: (JUNE_IST_MIDNIGHT + (day - 1) * 86_400 + 555 * 60 + second) * 1_000_000,
            open: price,
            high: price + 3,
            low: price - 3,
            close: price + 1,
            volume: 0,
            open_interest: OI_NULL,
        }
    };
    let day_of = |day: i64| (0..SESSION).map(move |second| at(day, second));
    let held: Vec<Bar> = [3, 4, 5, 7, 10].into_iter().flat_map(day_of).collect();
    assert!(held.len() > 100_000, "past the ceiling this replaced");
    let mut writer = BarFile::open_or_create(&store.0, seconds, SYMBOL).unwrap();
    writer.append(&held).unwrap();
    drop(writer);
    let original = (
        fs::read(seconds.to_path_buf(&store.0)).unwrap(),
        fs::read(seconds.with_file(FileKind::Checksums).to_path_buf(&store.0)).unwrap(),
    );
    let missing: Vec<Bar> = day_of(6).collect();
    let repaired =
        repair::repair_month(&store.0, seconds, SYMBOL, &missing, INDEX).expect("repaired");
    let MonthRepair::Repaired {
        from,
        to,
        held: kept,
        added,
        added_at,
        ..
    } = repaired
    else {
        panic!("{repaired:?}");
    };
    assert_eq!((from, to.ordinal()), (0, 1));
    assert_eq!((kept, added), (112_500, 22_500));
    assert_eq!(added_at.first(), Some(&missing[0].ts_micros));
    let served = BarFile::open_existing(&store.0, seconds, SYMBOL).unwrap();
    assert_eq!(served.header().n_valid, 135_000);
    let expected: Vec<Bar> = [3, 4, 5, 6, 7, 10].into_iter().flat_map(day_of).collect();
    for (index, bar) in expected.iter().enumerate().step_by(1_009) {
        assert_eq!(
            served.read_record(u64::try_from(index).unwrap()).unwrap(),
            *bar
        );
    }
    assert_eq!(
        served.read_record(134_999).unwrap(),
        *expected.last().unwrap()
    );
    assert_eq!(
        served.first_at_or_after(missing[0].ts_micros).unwrap(),
        3 * 22_500
    );
    assert_eq!(
        (
            fs::read(seconds.to_path_buf(&store.0)).unwrap(),
            fs::read(seconds.with_file(FileKind::Checksums).to_path_buf(&store.0)).unwrap(),
        ),
        original,
        "never in place"
    );
}

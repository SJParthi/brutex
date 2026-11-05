//! Rows of `census.rs` no test reached before BASE-COV-API repair 3 (shard
//! AS1, D-8661): what a `since` holds and read back, the tail's settle at
//! its limit, a census of in-memory partitions and the closes it holds, and
//! a vendor's census refused where its version-4 folder cannot be told.
//! Every key and close is invented.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;
use pull::manifest::{Closes, Entry, Held, Manifest};

/// An invented cash key of `month` 2026.
fn key(symbol: &str, month: u8) -> EntryKey {
    EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        symbol: Symbol::new(symbol).expect("a symbol"),
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2026, month).expect("a month"),
    }
}

/// A held row of `key` with `closes`.
fn held(key: EntryKey, closes: Closes) -> Held {
    Held::new(
        Entry {
            key,
            rows: 5,
            first_ts_micros: 1_782_877_500_000_000,
            last_ts_micros: 1_782_891_900_000_000,
        },
        closes,
    )
}

/// **A `since` SAYS WHAT IT HOLDS AND HOW MUCH WAS READ BACK** (BASE-COV-API
/// repair 3, D-8661): `Since::is_empty`, `len`, `read_back` and `iter`, and
/// `Since::read_back`, which nothing in the workspace called (L-BCA-7).
/// Extremes: nothing at all; only entries read back from the file; only the
/// settled level; only the fresh one; the same key in every level; and, at
/// once, every level holding entries, read in the order older, settled,
/// fresh, and a thousand read back beside one fresh; out of the box, the
/// largest generation.
#[test]
fn a_since_says_what_it_holds_and_how_much_was_read_back() {
    let one = key("CENSUSCOVA", 6);
    let other = key("CENSUSCOVB", 7);
    let empty = Since::default();
    assert!(empty.is_empty());
    assert_eq!((empty.len(), empty.read_back()), (0, 0));
    let older = Since {
        older: vec![(1, one)],
        ..Since::default()
    };
    assert!(!older.is_empty());
    assert_eq!((older.len(), older.read_back()), (1, 1));
    let settled = [(2, one)];
    let only_settled = Since {
        settled: &settled,
        ..Since::default()
    };
    assert!(!only_settled.is_empty());
    assert_eq!((only_settled.len(), only_settled.read_back()), (1, 0));
    let fresh = [(u64::MAX, other)];
    let only_fresh = Since {
        fresh: &fresh,
        ..Since::default()
    };
    assert!(!only_fresh.is_empty());
    assert_eq!(only_fresh.iter().next(), Some(&(u64::MAX, other)));
    let every = Since {
        older: vec![(1, one)],
        settled: &settled,
        fresh: &fresh,
    };
    assert_eq!((every.len(), every.read_back()), (3, 1));
    assert_eq!(
        every
            .iter()
            .map(|(generation, _)| *generation)
            .collect::<Vec<_>>(),
        [1, 2, u64::MAX],
        "older, then settled, then fresh"
    );
    let many = Since {
        older: (0..1_000).map(|generation| (generation, one)).collect(),
        fresh: &fresh,
        ..Since::default()
    };
    assert_eq!((many.len(), many.read_back()), (1_001, 1_000));
}

/// **A TAIL SETTLES ITS FRESH LEVEL ONCE, AT ITS LIMIT, AND NEVER BEFORE**
/// (BASE-COV-API repair 3, D-8661): `Tail::settle`. Extremes: an empty tail;
/// one entry under the limit (nothing moves); exactly the limit (everything
/// moves, the held rows with it); a settled level already holding entries,
/// to which the next settle appends; and, at once, a second settle at the
/// limit over a settled level shared with an earlier part (the earlier part
/// keeps what it saw).
#[test]
fn a_tail_settles_its_fresh_level_once_at_its_limit() {
    let manifest = Manifest::open(Vendor::Groww, &[], &[]).expect("a genesis census");
    let row = key("CENSUSCOVA", 6);
    let mut tail = Tail {
        header: manifest.header(),
        settled: Arc::new(Level::default()),
        fresh: Level::default(),
    };
    tail.settle();
    assert!(tail.settled.changes.is_empty() && tail.fresh.changes.is_empty());
    let fill = |tail: &mut Tail, count: usize| {
        for n in 0..count {
            tail.fresh
                .changes
                .push((u64::try_from(n).expect("small"), row));
        }
        tail.fresh.held.insert(row, held(row, Closes::UNKNOWN));
    };
    fill(&mut tail, FRESH_LIMIT - 1);
    tail.settle();
    assert_eq!(
        (tail.settled.changes.len(), tail.fresh.changes.len()),
        (0, FRESH_LIMIT - 1),
        "one short of the limit, nothing moves"
    );
    fill(&mut tail, 1);
    tail.settle();
    assert_eq!(
        (tail.settled.changes.len(), tail.fresh.changes.len()),
        (FRESH_LIMIT, 0),
        "at the limit, everything moves"
    );
    assert!(tail.settled.held.contains_key(&row) && tail.fresh.held.is_empty());
    let earlier = Arc::clone(&tail.settled);
    fill(&mut tail, FRESH_LIMIT);
    tail.settle();
    assert_eq!(tail.settled.changes.len(), 2 * FRESH_LIMIT);
    assert_eq!(
        earlier.changes.len(),
        FRESH_LIMIT,
        "a part extended earlier keeps the level it saw"
    );
    assert_eq!(tail.entries(), 2 * FRESH_LIMIT);
}

/// **A CENSUS OF IN-MEMORY PARTITIONS IS PARTITIONED, AND A MONTH'S CLOSES
/// ARE THE ROW'S OWN** (BASE-COV-API repair 3, D-8661):
/// `Manifests::partitions` and `Manifests::closes`, which nothing in the
/// workspace called (L-BCA-7). Extremes: no partition; one; every segment's;
/// a key no partition holds; a single census holding the key with known
/// closes; one holding it with closes unknown; and, at once, a partitioned
/// census asked for a cash key it has no cash partition for, and a single
/// census asked for another month of the same share; out of the box, a key
/// of the index segment asked of a single census of cash rows.
#[test]
fn a_census_of_partitions_is_partitioned_and_a_months_closes_are_its_own() {
    let row = key("CENSUSCOVA", 6);
    let none = Manifests::partitions(Vec::new());
    assert!(none.is_partitioned() && none.parts().is_empty());
    assert_eq!(none.closes(&row), None);
    let every = Manifests::partitions(
        SEGMENTS
            .iter()
            .map(|segment| Manifest::partition_absent(Vendor::Groww, *segment))
            .collect(),
    );
    assert_eq!(every.parts().len(), SEGMENTS.len());
    assert_eq!(every.closes(&row), None, "no partition holds it");
    let index_only = Manifests::partitions(vec![Manifest::partition_absent(
        Vendor::Groww,
        Segment::Index,
    )]);
    assert_eq!(index_only.parts().len(), 1);
    assert_eq!(index_only.closes(&row), None, "no cash partition at all");

    let priced = Closes::known(1_234_500, 1_240_000).expect("two prices");
    let mut manifest = Manifest::open(Vendor::Groww, &[], &[]).expect("a genesis census");
    manifest.record_held(held(row, priced)).expect("records");
    let single = Manifests::single(manifest);
    assert_eq!(single.closes(&row), Some(priced));
    assert_eq!(single.closes(&key("CENSUSCOVA", 7)), None, "another month");
    let index_key = EntryKey {
        segment: Segment::Index,
        ..row
    };
    assert_eq!(single.closes(&index_key), None, "another segment's key");
    let mut unknown = Manifest::open(Vendor::Groww, &[], &[]).expect("a genesis census");
    unknown
        .record_held(held(row, Closes::UNKNOWN))
        .expect("records");
    assert_eq!(
        Manifests::single(unknown).closes(&row),
        Some(Closes::UNKNOWN)
    );
}

/// **A VENDOR'S CENSUS IS REFUSED WHERE ITS VERSION-4 FOLDER CANNOT BE
/// TOLD** (BASE-COV-API repair 3, D-8661): `locate`. Extremes: no store at
/// all (the single file); a version-4 folder (partitioned); a file where the
/// vendor's version-4 folder goes (refused: not a directory); a `v4` that is
/// a symbolic link to itself (refused with the loop's own kind); a `v4`
/// path under a file (the single file); and, at once, a file at the vendor's
/// name beside another vendor's folder (each answered for itself); out of
/// the box, a vendor name that is a symbolic link to a folder (refused: a
/// link is not the folder).
#[test]
fn a_vendors_census_is_refused_where_its_v4_folder_cannot_be_told() {
    let root = crate::scratch::path("census-cover-locate");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let kind = |located: Located| match located {
        Located::Single(_) => "single".to_owned(),
        Located::Partitioned(_) => "partitioned".to_owned(),
        Located::Refused(_, Census::Unreadable { reason, fault }) => {
            format!("refused {fault:?}: {reason}")
        }
        Located::Refused(_, other) => panic!("a refusal is unreadable: {other:?}"),
    };
    assert_eq!(kind(locate(&root, Vendor::Groww)), "single");
    let v4 = pull::manifest::v4_root(&root);
    std::fs::create_dir_all(v4.join(Vendor::Dhan.as_str())).expect("dhan's folder");
    std::fs::write(v4.join(Vendor::Groww.as_str()), b"a file").expect("a file");
    assert_eq!(kind(locate(&root, Vendor::Dhan)), "partitioned");
    let refused = kind(locate(&root, Vendor::Groww));
    assert!(
        refused.starts_with("refused Refused") && refused.contains("is not a directory"),
        "{refused}"
    );
    std::fs::create_dir_all(root.join("target")).expect("a folder");
    std::os::unix::fs::symlink(root.join("target"), v4.join(Vendor::Zerodha.as_str()))
        .expect("a link");
    let linked = kind(locate(&root, Vendor::Zerodha));
    assert!(linked.contains("is not a directory"), "{linked}");

    let looped = crate::scratch::path("census-cover-locate-loop");
    let _ = std::fs::remove_dir_all(&looped);
    let manifest_dir = pull::manifest::v4_root(&looped)
        .parent()
        .expect("a manifest folder")
        .to_path_buf();
    std::fs::create_dir_all(&manifest_dir).expect("the manifest folder");
    std::os::unix::fs::symlink("v4", pull::manifest::v4_root(&looped)).expect("a loop");
    let refused = kind(locate(&looped, Vendor::Groww));
    assert!(
        refused.starts_with("refused Io(FilesystemLoop)")
            && refused.contains("could not be measured"),
        "{refused}"
    );

    let under = crate::scratch::path("census-cover-locate-under");
    let _ = std::fs::remove_dir_all(&under);
    std::fs::create_dir_all(&under).expect("a scratch root");
    std::fs::write(under.join("manifest"), b"a file where the folder goes").expect("a file");
    assert_eq!(kind(locate(&under, Vendor::Groww)), "single");
    for dir in [root, looped, under] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

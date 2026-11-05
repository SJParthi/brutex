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
use std::fmt::Write as _;

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

// ═══ The store filter, the census files and their tails (BASE-COV-API repair
// 3, shard AS1, D-8661: the session that continued AS1) ═══

/// A series of `segment` named `symbol`, minute bars.
fn series_of(segment: Segment, symbol: &str) -> Series {
    Series {
        contract: None,
        exchange: Exchange::Nse,
        segment,
        symbol: Symbol::new(symbol).expect("a symbol"),
        timeframe: Timeframe::MINUTE_1,
    }
}

/// **A STORE FILTER KEEPS ONLY WHAT EACH OF ITS FIELDS NAMES, AND SAYS WHEN
/// IT NARROWS** (BASE-COV-API repair 3, D-8661): `StoreFilter::keeps` and
/// `StoreFilter::is_empty`, field by field. Extremes: the default filter,
/// which keeps every row and is empty; another segment; another bar length;
/// a month one before `from`; a month one after `to`; a month equal to each
/// bound, kept; an empty symbol needle, which narrows nothing; a needle the
/// symbol holds; a needle it does not; and, at once, every field set and
/// matched (kept, not empty), and a `from` after its `to` (nothing kept);
/// out of the box, a needle of one character every symbol of the segment
/// holds.
#[test]
fn a_store_filter_keeps_only_what_each_field_names_and_says_when_it_narrows() {
    let cash = series_of(Segment::Cash, "CENSUSCOVA");
    let july = YearMonth::new(2026, 7).expect("a month");
    let june = YearMonth::new(2026, 6).expect("a month");
    let august = YearMonth::new(2026, 8).expect("a month");
    let all = StoreFilter::default();
    assert!(all.is_empty() && all.keeps(&cash, july));
    let judged =
        |filter: StoreFilter, month: YearMonth| (filter.keeps(&cash, month), filter.is_empty());
    let segment = StoreFilter {
        segment: Some(Segment::Fno),
        ..StoreFilter::default()
    };
    assert_eq!(judged(segment, july), (false, false), "another segment");
    let timeframe = || StoreFilter {
        timeframe: Some(Timeframe::SECOND_1),
        ..StoreFilter::default()
    };
    assert_eq!(
        judged(timeframe(), july),
        (false, false),
        "another bar length"
    );
    let from = || StoreFilter {
        from: Some(july),
        ..StoreFilter::default()
    };
    assert_eq!(
        judged(from(), june),
        (false, false),
        "one month before from"
    );
    assert_eq!(judged(from(), july), (true, false), "at from");
    let to = || StoreFilter {
        to: Some(july),
        ..StoreFilter::default()
    };
    assert_eq!(judged(to(), august), (false, false), "one month after to");
    assert_eq!(judged(to(), july), (true, false), "at to");
    let needle = |text: &str| StoreFilter {
        symbol: Some(text.to_owned()),
        ..StoreFilter::default()
    };
    assert_eq!(
        judged(needle(""), july),
        (true, true),
        "an empty needle narrows nothing"
    );
    assert_eq!(judged(needle("COVA"), july), (true, false));
    assert_eq!(judged(needle("COVB"), july), (false, false));
    assert_eq!(judged(needle("C"), july), (true, false), "one character");
    let every = StoreFilter {
        segment: Some(Segment::Cash),
        symbol: Some("CENSUS".to_owned()),
        timeframe: Some(Timeframe::MINUTE_1),
        from: Some(june),
        to: Some(august),
    };
    assert_eq!(
        judged(every, july),
        (true, false),
        "every field set and matched"
    );
    let inverted = StoreFilter {
        from: Some(august),
        to: Some(june),
        ..StoreFilter::default()
    };
    for month in [june, july, august] {
        assert!(!inverted.keeps(&cash, month), "{month}");
    }
}

/// A store root of this test's own, with its `manifest/` folder.
fn store_root(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("census-cover-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("manifest")).expect("the manifest folder");
    dir
}

/// A key of `segment` (no contract) in `month` of 2026.
fn key_in(segment: Segment, symbol: &str, month: u8) -> EntryKey {
    EntryKey {
        segment,
        ..key(symbol, month)
    }
}

/// A row of `rows` bars of `key` whose last bar is `last` seconds after the
/// first, with an invented close.
fn grown(key: EntryKey, rows: u64, last: i64) -> Held {
    let first = 1_782_877_500_000_000_i64;
    Held::new(
        Entry {
            key,
            rows,
            first_ts_micros: first,
            last_ts_micros: first + last * 1_000_000,
        },
        Closes::known(7_001, 7_000 + i64::try_from(rows).expect("small")).expect("a close"),
    )
}

/// A census file the census's own writer wrote, and the census it keeps.
struct Written {
    path: PathBuf,
    manifest: Manifest,
}

impl Written {
    /// `vendor`'s single file under `root`, holding `rows`.
    fn single(root: &Path, vendor: Vendor, rows: &[Held]) -> Self {
        let mut manifest = Manifest::open(vendor, &[], &[]).expect("a genesis census");
        for one in rows {
            let _append = manifest.record_held(*one).expect("records");
        }
        let path = manifest_path(root, vendor);
        std::fs::write(&path, manifest.image()).expect("written");
        Self { path, manifest }
    }

    /// GDFL's `segment` partition as epoch 1 under `root`, declaring
    /// `capacity`, holding `rows`.
    fn partition(root: &Path, segment: Segment, capacity: u64, rows: &[Held]) -> Self {
        let mut manifest =
            Manifest::partition_genesis(Vendor::Gdfl, segment, capacity).expect("a partition");
        for one in rows {
            let _append = manifest.record_held(*one).expect("records");
        }
        std::fs::create_dir_all(segment_dir(root, Vendor::Gdfl, segment)).expect("its folder");
        let path = epoch_path(root, Vendor::Gdfl, segment, 1);
        std::fs::write(&path, manifest.image()).expect("written");
        Self { path, manifest }
    }

    /// Appends `rows` as the writer does: each entry, then the slot counting
    /// it, into the same file.
    fn append(&mut self, rows: &[Held]) {
        for one in rows {
            let append = self.manifest.record_held(*one).expect("records");
            self.poke(append.offset, &append.bytes);
            self.poke(append.commit.offset, &append.commit.bytes);
        }
    }

    /// Writes `bytes` at `at`, in place: the same inode.
    fn poke(&self, at: u64, bytes: &[u8]) {
        use std::os::unix::fs::FileExt as _;
        std::fs::OpenOptions::new()
            .write(true)
            .open(&self.path)
            .expect("opens")
            .write_all_at(bytes, at)
            .expect("written in place");
    }

    /// Appends `rows` as raw images after `before`'s entries and commits a
    /// header that counts `keys` keys and `total` rows, whatever the rows
    /// say: a file no writer of this build produces.
    fn forged(&self, before: ManifestHeader, rows: &[Held], keys: u64, total: u64) {
        let stride = Layout::for_version(before.format_version)
            .expect("a layout")
            .entry_stride();
        for (n, one) in (before.n_valid..).zip(rows) {
            self.poke(HEADER_LEN + n * stride, &one.image());
        }
        let header = before
            .advance(u64::try_from(rows.len()).expect("small"), keys, total)
            .expect("advances");
        let commit = header.commit().expect("commits");
        self.poke(commit.offset, &commit.bytes);
    }

    /// Cuts the file to `len` bytes, in place.
    fn cut(&self, len: u64) {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&self.path)
            .expect("opens")
            .set_len(len)
            .expect("cut");
    }

    /// Puts a FIFO where the file was.
    fn fifo(&self) {
        std::fs::remove_file(&self.path).expect("removed");
        let made = std::process::Command::new("mkfifo")
            .arg(&self.path)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo made the FIFO");
    }
}

/// The census's held half, or a failure naming what it was instead.
fn manifests_of(census: &VendorCensus) -> &Manifests {
    match census.state {
        Census::Held { ref manifest } => manifest,
        Census::Absent | Census::Unreadable { .. } => panic!("not held: {}", census.note()),
    }
}

/// **A SINGLE FILE THAT HOLDS ONE PARTITION'S EPOCH IS REFUSED, NEVER READ AS
/// THE WHOLE CENSUS, AND A SPECIAL FILE IS NAMED FOR WHAT IT IS**
/// (BASE-COV-API repair 3, D-8661). Extremes: the index partition's epoch at
/// GDFL's single-file name; the cash partition's; the F&O partition's; a
/// single file of the vendor's own, held; and, at once, a partition epoch of
/// one row at that name beside no `v4` folder (single, so refused), and the
/// same name holding a whole single census again (held); a character device,
/// named as a device; a FIFO, named as one; out of the box, an empty
/// partition epoch, zero rows, at that name.
#[test]
fn a_single_file_holding_a_partition_epoch_is_refused_and_a_special_file_is_named() {
    let dir = store_root("partition-at-single");
    let path = manifest_path(&dir, Vendor::Gdfl);
    for (segment, rows) in [
        (
            Segment::Index,
            vec![grown(key_in(Segment::Index, "CENSUSIDX", 6), 5, 60)],
        ),
        (Segment::Cash, vec![grown(key("CENSUSCOVA", 6), 5, 60)]),
        (Segment::Fno, Vec::new()),
    ] {
        let mut epoch = Manifest::partition_genesis(
            Vendor::Gdfl,
            segment,
            pull::manifest::capacity_floor(segment),
        )
        .expect("a partition");
        for one in &rows {
            let _append = epoch.record_held(*one).expect("records");
        }
        std::fs::write(&path, epoch.image()).expect("written at the single name");
        let census = read_vendor(&dir, Vendor::Gdfl);
        let Census::Unreadable { reason, fault } = &census.state else {
            panic!("{segment:?}: not refused: {}", census.note());
        };
        assert_eq!(*fault, Fault::Refused);
        assert_eq!(
            *reason,
            format!(
                "it holds a version-4 epoch of the {} partition, not a single-file census; \
                 refused rather than read as all of gdfl's census (D-3316)",
                segment.as_str()
            )
        );
    }
    let _written = Written::single(&dir, Vendor::Gdfl, &[grown(key("CENSUSCOVA", 6), 5, 60)]);
    assert_eq!(manifests_of(&read_vendor(&dir, Vendor::Gdfl)).keys(), 1);
    let device = std::fs::metadata("/dev/null")
        .expect("a device")
        .file_type();
    assert_eq!(kind_of(device), "device or other special file");
    let fifo = dir.join("named-pipe");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo runs");
    assert!(made.success());
    let pipe = std::fs::symlink_metadata(&fifo)
        .expect("a FIFO")
        .file_type();
    assert_eq!(kind_of(pipe), "FIFO");
    let _ = std::fs::remove_dir_all(dir);
}

/// **A READ-BACK FROM A FILE CUT SHORT, NO LONGER A FILE, OR HOLDING ANOTHER
/// SEGMENT'S KEY IS REFUSED, NEVER GUESSED** (BASE-COV-API repair 3, D-8661):
/// `Part::read_back` below a census's base. Extremes: the two appends read
/// back whole; a generation at the base, which reads back nothing; the file
/// cut after its first entry; cut to its header; a FIFO at its name; a
/// partition whose second entry was rewritten in place to a key of another
/// segment; and, at once, a generation at the base asked of the cut file
/// (nothing to read, so nothing refused), and a FIFO asked at the base; out
/// of the box, a generation one past the newest, never answered.
#[test]
fn a_read_back_from_a_cut_or_foreign_file_is_refused_never_guessed() {
    let dir = store_root("read-back-cut");
    let one = grown(key("CENSUSCOVA", 6), 5, 60);
    let two = grown(key("CENSUSCOVB", 6), 6, 70);
    let three = grown(key("CENSUSCOVC", 6), 7, 80);
    let mut written = Written::single(&dir, Vendor::Gdfl, &[one]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let since = manifests_of(&first).parts()[0].header().generation;
    written.append(&[two, three]);
    let whole = read_vendor(&dir, Vendor::Gdfl);
    let part = &manifests_of(&whole).parts()[0];
    let base = part.header().generation;
    assert_eq!(part.changed_since(since).map(|since| since.len()), Some(2));
    assert_eq!(part.read_back(base, 10), Some(Vec::new()), "at the base");
    assert!(part.changed_since(base + 1).is_none(), "past the newest");
    let stride = Layout::for_version(part.header().format_version)
        .expect("a layout")
        .entry_stride();
    written.cut(HEADER_LEN + stride);
    assert!(
        part.changed_since(since).is_none(),
        "cut after its first entry"
    );
    assert_eq!(
        part.read_back(base, 10),
        Some(Vec::new()),
        "at the base, cut"
    );
    written.cut(HEADER_LEN);
    assert!(part.changed_since(since).is_none(), "cut to its header");
    written.fifo();
    assert!(part.changed_since(since).is_none(), "a FIFO at its name");
    assert_eq!(
        part.read_back(base, 10),
        Some(Vec::new()),
        "at the base, a FIFO"
    );

    let dir2 = store_root("read-back-foreign");
    let index = |symbol: &str| key_in(Segment::Index, symbol, 6);
    let mut written = Written::partition(
        &dir2,
        Segment::Index,
        pull::manifest::capacity_floor(Segment::Index),
        &[grown(index("CENSUSIDXA"), 5, 60)],
    );
    let first = read_vendor(&dir2, Vendor::Gdfl);
    let since = manifests_of(&first).parts()[0].header().generation;
    written.append(&[grown(index("CENSUSIDXB"), 6, 70)]);
    let whole = read_vendor(&dir2, Vendor::Gdfl);
    let part = &manifests_of(&whole).parts()[0];
    assert_eq!(part.changed_since(since).map(|since| since.len()), Some(1));
    let stride = Layout::for_version(part.header().format_version)
        .expect("a layout")
        .entry_stride();
    written.poke(
        HEADER_LEN + stride,
        &grown(key("CENSUSCOVA", 6), 6, 70).image(),
    );
    assert!(
        part.changed_since(since).is_none(),
        "a cash key read back from the index partition"
    );
    for dir in [dir, dir2] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// **A TAIL IS TAKEN ONLY WHERE IT REACHES THE CENSUS A WHOLE READ GIVES;
/// EVERY OTHER CHANGE TO THE FILE IS READ WHOLE** (BASE-COV-API repair 3,
/// D-8661): the refusals of `extend`, each shown by the whole read it causes.
/// Extremes: an empty census extended by its first two entries (a tail, no
/// whole read); the last entry read rewritten in place; the newest header
/// copied into the other slot; another vendor's census written over the file;
/// an index partition's epoch written over the single file (read whole, and
/// refused); a partition rewritten at another capacity; a partition rewritten
/// as its own compaction (another lineage); a header that counts an entry the
/// file was cut before; a base read degraded; and, forged after a partition's
/// last entry, a key of another segment, a key whose rows went down, one whose
/// last bar went back, a new key the header does not count, and one whose
/// rows the header does not count; at once, a rewritten last entry beside an
/// append, and a forged header that miscounts both keys and rows (the keys'
/// check speaks first); out of the box, a FIFO where the census was.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every refusal of the tail read, each beside the whole read it causes"
)]
fn a_tail_is_taken_only_where_it_reaches_the_whole_read() {
    let whole_reads = |path: &Path| manifest_reads::of(path);
    let a = grown(key("CENSUSCOVA", 6), 5, 60);
    let b = grown(key("CENSUSCOVB", 6), 6, 70);
    let c = grown(key("CENSUSCOVC", 6), 7, 80);

    // An empty census, extended: a tail, nothing read whole.
    let dir = store_root("tail-empty");
    let mut written = Written::single(&dir, Vendor::Gdfl, &[]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    written.append(&[a, b]);
    let reads = whole_reads(&written.path);
    let extended = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    assert_eq!(
        whole_reads(&written.path),
        reads,
        "an empty census is extended"
    );
    assert_eq!(manifest_reads::tails_of(&written.path).last(), Some(&2));
    assert_eq!(manifests_of(&extended).keys(), 2);

    // Each change below is read whole: `case` reads the census after it and
    // counts one more whole read.
    let read_whole = |dir: &Path, path: &Path, previous: &VendorCensus, what: &str| {
        let reads = whole_reads(path);
        let after = read_vendor_after(dir, Vendor::Gdfl, Some(previous));
        assert_eq!(whole_reads(path), reads + 1, "{what}: read whole");
        after
    };

    // The last entry read, rewritten in place beside an append.
    let dir = store_root("tail-last");
    let mut written = Written::single(&dir, Vendor::Gdfl, &[a, b]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let stride = Layout::for_version(manifests_of(&first).parts()[0].header().format_version)
        .expect("a layout")
        .entry_stride();
    written.append(&[c]);
    written.poke(
        HEADER_LEN + stride,
        &grown(key("CENSUSCOVB", 6), 9, 90).image(),
    );
    let after = read_whole(&dir, &written.path, &first, "the last entry rewritten");
    assert!(
        after
            .note()
            .contains("the header claims 18 rows; the entries hold 21"),
        "the whole read counts the rewritten entry: {}",
        after.note()
    );

    // The newest header copied into the other slot.
    let dir = store_root("tail-slot");
    let mut written = Written::single(&dir, Vendor::Gdfl, &[a]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    written.append(&[b]);
    let newest = written.manifest.header();
    let other = u64::try_from(SLOT_SPACING).expect("small") * ((newest.generation + 1) % 2);
    written.poke(other, &newest.image());
    let _after = read_whole(&dir, &written.path, &first, "a header in the other slot");

    // Another vendor's census written over the file, in place.
    let dir = store_root("tail-vendor");
    let written = Written::single(&dir, Vendor::Gdfl, &[a]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let elsewhere = store_root("tail-vendor-zerodha");
    let zerodha = Written::single(&elsewhere, Vendor::Zerodha, &[a, b]);
    written.poke(0, &std::fs::read(&zerodha.path).expect("read"));
    let after = read_whole(&dir, &written.path, &first, "another vendor's census");
    assert!(
        matches!(after.state, Census::Unreadable { .. }),
        "{}",
        after.note()
    );

    // An index partition's epoch written over the single file.
    let dir = store_root("tail-segment");
    let written = Written::single(&dir, Vendor::Gdfl, &[a]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let mut epoch = Manifest::partition_genesis(
        Vendor::Gdfl,
        Segment::Index,
        pull::manifest::capacity_floor(Segment::Index),
    )
    .expect("a partition");
    let _append = epoch
        .record_held(grown(key_in(Segment::Index, "CENSUSIDXA", 6), 5, 60))
        .expect("records");
    written.poke(0, &epoch.image());
    let after = read_whole(
        &dir,
        &written.path,
        &first,
        "a partition over the single file",
    );
    assert!(
        after.note().contains("holds a version-4 epoch of the"),
        "{}",
        after.note()
    );

    // Partitions: rewritten at another capacity, and as their compaction.
    let index = |symbol: &str| key_in(Segment::Index, symbol, 6);
    let floor = pull::manifest::capacity_floor(Segment::Index);
    let dir = store_root("tail-capacity");
    let written = Written::partition(
        &dir,
        Segment::Index,
        floor,
        &[grown(index("CENSUSIDXA"), 5, 60)],
    );
    let first = read_vendor(&dir, Vendor::Gdfl);
    let mut wider =
        Manifest::partition_genesis(Vendor::Gdfl, Segment::Index, floor * 2).expect("a partition");
    let _append = wider
        .record_held(grown(index("CENSUSIDXA"), 5, 60))
        .expect("records");
    written.poke(0, &wider.image());
    let _after = read_whole(&dir, &written.path, &first, "another capacity");

    let dir = store_root("tail-lineage");
    let written = Written::partition(
        &dir,
        Segment::Index,
        floor,
        &[
            grown(index("CENSUSIDXA"), 5, 60),
            grown(index("CENSUSIDXB"), 6, 70),
        ],
    );
    let first = read_vendor(&dir, Vendor::Gdfl);
    let compacted = written.manifest.compacted().expect("compacts");
    written.poke(0, &compacted.image());
    let _after = read_whole(&dir, &written.path, &first, "another lineage");

    // The newest generation committed as an older version's header, and a
    // partition's counting one entry past its capacity.
    let slot_of = |generation: u64| u64::try_from(SLOT_SPACING).expect("small") * (generation % 2);
    let dir = store_root("tail-version");
    let written = Written::single(&dir, Vendor::Gdfl, &[a]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let mut older_version = written.manifest.header();
    assert_ne!(
        older_version.format_version, 2,
        "the single file is written past version 2"
    );
    older_version.format_version = 2;
    older_version.generation += 1;
    written.poke(slot_of(older_version.generation), &older_version.image());
    let _after = read_whole(&dir, &written.path, &first, "an older version's header");
    let dir = store_root("tail-limit");
    let written = Written::partition(
        &dir,
        Segment::Index,
        floor,
        &[grown(index("CENSUSIDXA"), 5, 60)],
    );
    let first = read_vendor(&dir, Vendor::Gdfl);
    let mut past = written.manifest.header();
    past.n_valid = past.limit() + 1;
    past.generation += 1;
    written.poke(slot_of(past.generation), &past.image());
    let _after = read_whole(&dir, &written.path, &first, "a header past its limit");

    // A header that counts an entry the file was cut before.
    let dir = store_root("tail-cut");
    let mut written = Written::single(&dir, Vendor::Gdfl, &[a]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let len = std::fs::metadata(&written.path).expect("a file").len();
    written.append(&[b]);
    written.cut(len);
    let _after = read_whole(&dir, &written.path, &first, "a counted entry cut off");

    // A base read degraded: its newest header stands in the other slot too
    // (a slot in the wrong position is said, and stepped over).
    let dir = store_root("tail-degraded");
    let mut written = Written::single(&dir, Vendor::Gdfl, &[a, b]);
    let older = u64::try_from(SLOT_SPACING).expect("small")
        * ((written.manifest.header().generation + 1) % 2);
    written.poke(older, &written.manifest.header().image());
    let first = read_vendor(&dir, Vendor::Gdfl);
    assert!(
        manifests_of(&first).parts()[0]
            .base
            .degraded_reason()
            .is_some(),
        "the damaged slot is said"
    );
    written.append(&[c]);
    let _after = read_whole(&dir, &written.path, &first, "a degraded base");

    // Forged after a partition's last entry.
    for (what, rows, keys, total) in [
        (
            "another segment's key",
            vec![grown(key("CENSUSCOVA", 6), 9, 90)],
            3,
            20,
        ),
        (
            "rows went down",
            vec![grown(index("CENSUSIDXA"), 4, 90)],
            2,
            10,
        ),
        (
            "its last bar went back",
            vec![grown(index("CENSUSIDXA"), 5, 50)],
            2,
            11,
        ),
        (
            "a key not counted",
            vec![grown(index("CENSUSIDXC"), 7, 80)],
            2,
            18,
        ),
        (
            "rows not counted",
            vec![grown(index("CENSUSIDXC"), 7, 80)],
            3,
            11,
        ),
        (
            "keys and rows not counted",
            vec![grown(index("CENSUSIDXC"), 7, 80)],
            2,
            11,
        ),
    ] {
        let dir = store_root(&format!("tail-forged-{}", what.replace(' ', "-")));
        let written = Written::partition(
            &dir,
            Segment::Index,
            floor,
            &[
                grown(index("CENSUSIDXA"), 5, 60),
                grown(index("CENSUSIDXB"), 6, 70),
            ],
        );
        let first = read_vendor(&dir, Vendor::Gdfl);
        written.forged(written.manifest.header(), &rows, keys, total);
        let _after = read_whole(&dir, &written.path, &first, what);
    }

    // Out of the box: a FIFO where the census was.
    let dir = store_root("tail-fifo");
    let written = Written::single(&dir, Vendor::Gdfl, &[a]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    written.fifo();
    let after = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    assert!(
        matches!(after.state, Census::Unreadable { .. }),
        "{}",
        after.note()
    );
}

/// A writer that refuses every write: what a formatter sees when its output
/// fails.
struct Refusing;

impl std::fmt::Write for Refusing {
    fn write_str(&mut self, _text: &str) -> std::fmt::Result {
        Err(std::fmt::Error)
    }
}

/// **A CENSUS IS THE SAME READ, OR CONTINUES ITS FILE, ONLY BY ITS OWN
/// EVIDENCE** (BASE-COV-API repair 3, D-8661): `Part::continues_file`,
/// `Part::same_read`, `Manifests::same_read` and `VendorCensus::same_read`,
/// each false on the one field that differs. Extremes: two whole reads of one
/// unchanged file (continued, not the same read); the last entry rewritten
/// in place; the file replaced by rename; the file removed; parts of two
/// segments; epoch 1 and the epoch 2 after it; an epoch rewritten in place
/// as its compaction (another lineage); a degraded read on either side; an
/// empty census read twice; two single files at two roots; a tail kept
/// twice over an unchanged file (the same tail); a single census beside a
/// partitioned one; a partitioned census beside one of no parts; two absent
/// censuses; two vendors; and, at once, a degraded read of a removed file,
/// and a census in another segment, epoch and path; out of the box, a
/// series written to a formatter whose every write fails.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every comparison of two census reads, each false on the one field that differs"
)]
fn a_census_is_the_same_read_or_continues_its_file_only_by_its_own_evidence() {
    let a = grown(key("CENSUSCOVA", 6), 5, 60);
    let b = grown(key("CENSUSCOVB", 6), 6, 70);
    let c = grown(key("CENSUSCOVC", 6), 7, 80);
    let part = |census: &VendorCensus, at: usize| manifests_of(census).parts()[at].clone();

    // One unchanged file read twice: continued, never the same read.
    let dir = store_root("continues");
    let written = Written::single(&dir, Vendor::Gdfl, &[a, b]);
    let one = read_vendor(&dir, Vendor::Gdfl);
    let two = read_vendor(&dir, Vendor::Gdfl);
    assert!(part(&two, 0).continues_file(&part(&one, 0)));
    assert!(!part(&two, 0).same_read(&part(&one, 0)));
    // The last entry rewritten in place: not continued.
    let stride = Layout::for_version(part(&one, 0).header().format_version)
        .expect("a layout")
        .entry_stride();
    written.poke(
        HEADER_LEN + stride,
        &grown(key("CENSUSCOVB", 6), 9, 90).image(),
    );
    assert!(
        !part(&two, 0).continues_file(&part(&one, 0)),
        "last entry rewritten"
    );
    // Cut to its header: the last entry read cannot be read again.
    let held_bytes = std::fs::read(&written.path).expect("read");
    written.cut(HEADER_LEN);
    assert!(
        !part(&two, 0).continues_file(&part(&one, 0)),
        "cut to its header"
    );
    written.poke(0, &held_bytes);
    // Replaced by rename: another inode.
    let bytes = std::fs::read(&written.path).expect("read");
    let tmp = written.path.with_extension("writing");
    std::fs::write(&tmp, &bytes).expect("written");
    std::fs::rename(&tmp, &written.path).expect("renamed");
    assert!(
        !part(&two, 0).continues_file(&part(&one, 0)),
        "another inode"
    );
    // Removed.
    std::fs::remove_file(&written.path).expect("removed");
    assert!(!part(&two, 0).continues_file(&part(&one, 0)), "removed");

    // A header whose generation is below its entry count: no lineage at all.
    let dir = store_root("continues-lineage");
    let written = Written::single(&dir, Vendor::Gdfl, &[a, b]);
    let one = read_vendor(&dir, Vendor::Gdfl);
    let mut forged = written.manifest.header();
    forged.generation = forged.n_valid - 1;
    let blank = vec![0u8; forged.image().len()];
    let spacing = u64::try_from(SLOT_SPACING).expect("small");
    written.poke(0, &blank);
    written.poke(spacing, &blank);
    written.poke(spacing * (forged.generation % 2), &forged.image());
    let two = read_vendor(&dir, Vendor::Gdfl);
    assert_eq!(
        part(&two, 0).header().generation,
        1,
        "the forged header is read"
    );
    assert!(!part(&two, 0).continues_file(&part(&one, 0)), "no lineage");

    // A degraded read (its newest header stands in the other slot too), on
    // either side.
    let dir = store_root("continues-degraded");
    let written = Written::single(&dir, Vendor::Gdfl, &[a, b]);
    let healthy = read_vendor(&dir, Vendor::Gdfl);
    let older = u64::try_from(SLOT_SPACING).expect("small")
        * ((written.manifest.header().generation + 1) % 2);
    written.poke(older, &written.manifest.header().image());
    let degraded = read_vendor(&dir, Vendor::Gdfl);
    let damaged = part(&degraded, 0);
    assert!(damaged.base.degraded_reason().is_some());
    assert!(!damaged.continues_file(&part(&healthy, 0)), "self degraded");
    assert!(
        !part(&healthy, 0).continues_file(&damaged),
        "other degraded"
    );
    assert_eq!(
        damaged.since_floor(),
        damaged.header().generation,
        "no read-back"
    );
    assert!(
        damaged
            .changed_since(damaged.header().generation - 1)
            .is_none()
    );
    std::fs::remove_file(&written.path).expect("removed");
    assert!(
        !damaged.continues_file(&part(&healthy, 0)),
        "degraded and removed"
    );

    // An empty census read twice: nothing to continue from.
    let dir = store_root("continues-empty");
    let _written = Written::single(&dir, Vendor::Gdfl, &[]);
    let empty = read_vendor(&dir, Vendor::Gdfl);
    let again = read_vendor(&dir, Vendor::Gdfl);
    assert!(
        !part(&again, 0).continues_file(&part(&empty, 0)),
        "no first entry"
    );

    // Partitions: two segments, two epochs, another lineage.
    let index = |symbol: &str| key_in(Segment::Index, symbol, 6);
    let floor = pull::manifest::capacity_floor(Segment::Index);
    let dir = store_root("continues-partitions");
    let written = Written::partition(
        &dir,
        Segment::Index,
        floor,
        &[
            grown(index("CENSUSIDXA"), 5, 60),
            grown(index("CENSUSIDXB"), 6, 70),
        ],
    );
    let first = read_vendor(&dir, Vendor::Gdfl);
    let (index_part, cash_part) = (part(&first, 0), part(&first, 1));
    assert!(!cash_part.continues_file(&index_part), "two segments");
    assert!(!cash_part.same_read(&index_part), "two segments");
    let compacted = written.manifest.compacted().expect("compacts");
    let epoch2 = epoch_path(&dir, Vendor::Gdfl, Segment::Index, 2);
    std::fs::write(&epoch2, compacted.image()).expect("epoch 2");
    let second = read_vendor(&dir, Vendor::Gdfl);
    assert_eq!(part(&second, 0).epoch(), Some(2));
    assert!(!part(&second, 0).continues_file(&index_part), "two epochs");
    assert!(!part(&second, 0).same_read(&index_part), "two epochs");
    std::fs::remove_file(&epoch2).expect("epoch 2 removed");
    written.poke(0, &compacted.image());
    let rewritten = read_vendor(&dir, Vendor::Gdfl);
    assert_eq!(part(&rewritten, 0).epoch(), Some(1));
    assert!(
        !part(&rewritten, 0).continues_file(&index_part),
        "the same file at another lineage"
    );

    // Two single files at two roots: two paths.
    let elsewhere = store_root("continues-elsewhere");
    let _written = Written::single(&elsewhere, Vendor::Gdfl, &[a]);
    let there = read_vendor(&elsewhere, Vendor::Gdfl);
    assert!(!part(&there, 0).same_read(&part(&healthy, 0)), "two paths");
    assert!(
        !cash_part.same_read(&part(&there, 0)),
        "another segment, epoch and path"
    );

    // A tail kept twice over an unchanged file is the same read, tail and all.
    let dir = store_root("same-tail");
    let mut written = Written::single(&dir, Vendor::Gdfl, &[a]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    written.append(&[b, c]);
    let extended = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    let kept = read_vendor_after(&dir, Vendor::Gdfl, Some(&extended));
    assert!(part(&kept, 0).same_read(&part(&extended, 0)), "one tail");
    assert!(kept.same_read(&extended));

    // The shapes: single beside partitioned, partitioned beside none.
    assert!(!manifests_of(&healthy).same_read(manifests_of(&second)));
    let no_parts = Manifests::partitions(Vec::new());
    assert!(!manifests_of(&second).same_read(&no_parts));
    // Two absent censuses are the same; two vendors never are.
    let bare = store_root("same-absent");
    let absent = read_vendor(&bare, Vendor::Gdfl);
    assert!(matches!(absent.state, Census::Absent));
    assert!(absent.same_read(&read_vendor(&bare, Vendor::Gdfl)));
    assert!(!absent.same_read(&read_vendor(&bare, Vendor::Zerodha)));

    // Out of the box: a series written where every write fails.
    let series = series_of(Segment::Index, "CENSUSIDXA");
    assert!(write!(Refusing, "{series}").is_err());
    assert_eq!(series.to_string(), "NSE-INDEX-CENSUSIDXA");
    for name in [
        "continues",
        "continues-degraded",
        "continues-empty",
        "continues-lineage",
        "continues-partitions",
        "continues-elsewhere",
        "same-tail",
        "same-absent",
    ] {
        let _ = std::fs::remove_dir_all(crate::scratch::path(&format!("census-cover-{name}")));
    }
}

/// **A PARTITION THE HOST WILL NOT LIST, OR A CENSUS FILE IT WILL NOT OPEN,
/// IS REFUSED WITH THE HOST'S OWN KIND, AND READ AGAIN ONCE IT MAY BE**
/// (BASE-COV-API repair 3, D-8661). Mode bits do not bind root, so the body
/// runs where they bind (`crate::isolated::where_permission_binds`, D-0995).
/// Extremes: a segment folder of mode 000; one of mode 300 (no read); one of
/// mode 500, listed and held; an epoch of mode 000; one of mode 200 (write
/// only); one of mode 400, held; and, at once, a folder of mode 000 over an
/// epoch of mode 000 (the folder's refusal speaks first), and a folder of
/// mode 100 (search, no read) over a readable epoch; out of the box, the
/// single file of mode 000, refused by the single-file read.
#[cfg(unix)]
#[test]
fn a_partition_or_file_the_host_will_not_read_is_refused_with_its_kind() {
    crate::isolated::where_permission_binds(
        "census::cover_tests::a_partition_or_file_the_host_will_not_read_is_refused_with_its_kind",
        a_partition_or_file_the_host_will_not_read_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
#[cfg(unix)]
fn a_partition_or_file_the_host_will_not_read_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = |path: &Path, bits: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).expect("mode set");
    };
    let refused = |census: &VendorCensus| -> (String, Fault) {
        match &census.state {
            Census::Unreadable { reason, fault } => (reason.clone(), *fault),
            Census::Held { .. } | Census::Absent => panic!("not refused: {}", census.note()),
        }
    };
    let denied = Fault::Io(std::io::ErrorKind::PermissionDenied);
    let floor = pull::manifest::capacity_floor(Segment::Index);
    let rows = [grown(key_in(Segment::Index, "CENSUSIDXA", 6), 5, 60)];
    let dir = store_root("perm-partition");
    let written = Written::partition(&dir, Segment::Index, floor, &rows);
    let folder = segment_dir(&dir, Vendor::Gdfl, Segment::Index);
    for (folder_bits, epoch_bits, listed) in [
        (0o000, 0o644, false),
        (0o300, 0o644, false),
        (0o100, 0o644, false),
        (0o000, 0o000, false),
    ] {
        mode(&written.path, epoch_bits);
        mode(&folder, folder_bits);
        assert!(
            std::fs::read_dir(&folder).is_err(),
            "mode {folder_bits:o} binds (D-0995)"
        );
        let census = read_vendor(&dir, Vendor::Gdfl);
        mode(&folder, 0o755);
        mode(&written.path, 0o644);
        let (reason, fault) = refused(&census);
        assert_eq!(fault, denied, "{folder_bits:o}");
        assert!(
            reason.starts_with(&format!("{} could not be listed: ", folder.display())),
            "{folder_bits:o}: {reason}"
        );
        assert!(!listed);
    }
    for bits in [0o000, 0o200] {
        mode(&written.path, bits);
        let census = read_vendor(&dir, Vendor::Gdfl);
        mode(&written.path, 0o644);
        let (reason, fault) = refused(&census);
        assert_eq!(fault, denied, "{bits:o}");
        assert!(
            reason.starts_with(&format!("{}: ", written.path.display())),
            "{bits:o}: {reason}"
        );
    }
    mode(&folder, 0o500);
    mode(&written.path, 0o400);
    assert_eq!(manifests_of(&read_vendor(&dir, Vendor::Gdfl)).keys(), 1);
    mode(&folder, 0o755);
    mode(&written.path, 0o644);
    // Out of the box: the single file, of mode 000.
    let single = store_root("perm-single");
    let file = Written::single(&single, Vendor::Gdfl, &[grown(key("CENSUSCOVA", 6), 5, 60)]);
    mode(&file.path, 0o000);
    let census = read_vendor(&single, Vendor::Gdfl);
    mode(&file.path, 0o644);
    assert_eq!(refused(&census).1, denied);
    for root in [dir, single] {
        let _ = std::fs::remove_dir_all(root);
    }
}

/// **A ROOM IS CAPPED BELOW ITS BOUND AND SAYS WHERE, THE SINGLE FILE BY ITS
/// OWN NAME** (BASE-COV-API repair 3, D-8661): `Room::capped_reason`.
/// Extremes: no headroom at all; one entry free; one below the bound; at the
/// bound (not capped); one above; the largest headroom; the single file; the
/// index, cash and F&O partitions; and, at once, the single file with no
/// headroom, and a partition at exactly one below the bound; out of the box,
/// a room whose committed count is past its capacity (headroom zero, said as
/// it is).
#[test]
fn a_room_is_capped_below_its_bound_and_says_where() {
    let room = |segment: Option<Segment>, headroom: u64| Room {
        segment,
        epoch: segment.map(|_| 1),
        capacity: MAX_ENTRIES,
        committed: MAX_ENTRIES.saturating_sub(headroom),
        headroom,
    };
    let said = |segment: Option<Segment>, headroom: u64| room(segment, headroom).capped_reason();
    let single = said(None, 0).expect("capped");
    assert!(
        single.starts_with(&format!(
            "capped: 0 of {MAX_ENTRIES} entries free in the single file (below {CAPPED_HEADROOM})"
        )),
        "{single}"
    );
    assert!(said(None, 1).expect("capped").contains("1 of"));
    for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
        let one_below = said(Some(segment), CAPPED_HEADROOM - 1).expect("capped");
        assert!(
            one_below.contains(&format!("free in the {} partition", segment.as_str())),
            "{one_below}"
        );
        assert_eq!(said(Some(segment), CAPPED_HEADROOM), None, "at the bound");
    }
    assert_eq!(said(None, CAPPED_HEADROOM + 1), None);
    assert_eq!(said(None, u64::MAX), None);
    let overfull = Room {
        segment: None,
        epoch: None,
        capacity: 4,
        committed: 9,
        headroom: 0,
    };
    assert!(
        overfull
            .capped_reason()
            .expect("capped")
            .starts_with("capped: 0 of 4 entries free in the single file"),
        "said as it is"
    );
}

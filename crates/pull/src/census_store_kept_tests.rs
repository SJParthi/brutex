//! A day's census publish costs only the day's own rows (R3-19, D-6891), and
//! the AG-05 follow-ups R3-19 owns: a segment directory that is not a
//! directory is refused by name before anything is staged, the seal-stub
//! check never waits and never follows, a link at a single-file census is
//! named as a link, the single-file publish's `.writing` temporary is born
//! through `store::staging::create`, and an epoch's refusal says its kind
//! once.
//!
//! Every fixture is invented: keys, counts and stamps, in scratch stores of
//! the tests' own under the host's temporary directory. No vendor row and no
//! price appears anywhere here.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

use super::{
    CensusRead, HEADER_LEN, Kept, Target, WholeStep, current_epoch, holds_sealed_stub,
    install_epoch_with, not_an_epoch, read_epoch, reads, record, record_kept, sealed_stub,
    stub_hook, withdraw,
};
use crate::manifest::{
    Closes, Entry, EntryKey, Held, Manifest, epoch_path, manifest_path, segment_dir, vendor_dir,
};

// ═══ Fixtures ═══════════════════════════════════════════════════════════════

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-census-kept-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(root.join("manifest")).expect("a scratch store");
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

/// The `n`th invented key of `segment`: 50,000 strikes a month, CE and PE,
/// months from 1990 on, so millions of keys are distinct.
fn key_in(segment: Segment, n: u32) -> EntryKey {
    let month = n / 100_000;
    let side = if n.is_multiple_of(2) { "CE" } else { "PE" };
    let strike = 1_000_000 + (n % 100_000) / 2 * 100;
    EntryKey {
        contract: match segment {
            Segment::Fno => Contract::parse(&format!("2019-12-26-{strike}-{side}")),
            _ => None,
        },
        exchange: Exchange::Nse,
        segment,
        symbol: Symbol::new(if segment == Segment::Fno {
            "NIFTY"
        } else {
            "INVENTEDIDX"
        })
        .expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(
            u16::try_from(1_990 + month / 12).expect("a year"),
            u8::try_from(month % 12 + 1).expect("a month"),
        )
        .expect("a month"),
    }
}

fn key(n: u32) -> EntryKey {
    key_in(Segment::Fno, n)
}

/// An invented row of key `n` holding `rows` rows.
fn row_in(segment: Segment, n: u32, rows: u64) -> Held {
    Held::new(
        Entry {
            key: key_in(segment, n),
            rows,
            first_ts_micros: 10,
            last_ts_micros: 10 + i64::try_from(rows).expect("rows"),
        },
        Closes::UNKNOWN,
    )
}

fn row(n: u32, rows: u64) -> Held {
    row_in(Segment::Fno, n, rows)
}

/// Epoch 1 of the gdfl FNO partition, holding `keys` distinct keys, written
/// as a fixture at a capacity with room for every day the test offers.
fn partition_of(root: &Path, keys: u32) -> PathBuf {
    let capacity = u64::from(keys) * 2 + 4_096;
    let mut census =
        Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, capacity).expect("genesis");
    for n in 0..keys {
        census
            .record_held(row(n, u64::from(n % 97) + 1))
            .expect("held");
    }
    std::fs::create_dir_all(vendor_dir(root, Vendor::Gdfl)).expect("v4");
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
    std::fs::write(&path, census.image()).expect("epoch 1");
    path
}

/// The single-file census of `vendor`, holding `keys` distinct keys.
fn single_of(root: &Path, vendor: Vendor, keys: u32) -> PathBuf {
    let mut census = Manifest::open_image(vendor, &[]).expect("genesis");
    for n in 0..keys {
        census
            .record_held(row(n, u64::from(n % 89) + 1))
            .expect("held");
    }
    let path = manifest_path(root, vendor);
    std::fs::write(&path, census.image()).expect("the single file");
    path
}

/// Day `d`'s invented batch over a census of `keys` keys: `fresh` new keys,
/// one key it already holds offered again exactly, and one it holds grown.
fn day_rows(keys: u32, d: u32, fresh: u32) -> Vec<Held> {
    let base = keys + d * fresh;
    let mut out: Vec<Held> = (0..fresh)
        .map(|i| row(base + i, u64::from(d) + 3))
        .collect();
    // The same row the fixture holds: exact, so nothing is written for it.
    out.push(row(1, 2));
    // A row the fixture holds, grown: a changed row.
    out.push(row(2, 3 + u64::from(d) * 10));
    out
}

/// Every file under `dir`, by path relative to it, with its bytes.
fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).expect("listed") {
            let path = entry.expect("an entry").path();
            let meta = std::fs::symlink_metadata(&path).expect("measured");
            if meta.is_dir() {
                stack.push(path);
            } else if !path.to_string_lossy().ends_with(".lock") {
                out.push((
                    path.strip_prefix(dir).expect("under").to_path_buf(),
                    std::fs::read(&path).expect("read"),
                ));
            }
        }
    }
    out.sort();
    out
}

/// The census reads made on this thread since the last call: `(kind,
/// bytes)`, in order.
fn reads_since() -> Vec<(&'static str, u64)> {
    reads::taken()
        .into_iter()
        .map(|(_, kind, bytes)| (kind, bytes))
        .collect()
}

/// p50, p99 and max of `micros`, sorted in place.
fn percentiles(micros: &mut [u128]) -> (u128, u128, u128) {
    micros.sort_unstable();
    let n = micros.len();
    (micros[n / 2], micros[n * 99 / 100], micros[n - 1])
}

// ═══ R3-19: a day's publish costs its own rows ══════════════════════════════

/// **A DAY'S PUBLISH READS ONLY ITS OWN KEYS** (R3-19's named test; a counting
/// reader). The first day of a run reads the census whole, once; every later
/// day reads exactly one header region, 32 KiB, and the same bytes at 1,000
/// and at 20,000 entries — where the per-call door, [`record`], reads every
/// entry every day (which is what this package removes: before it, every day
/// was the last row's line).
#[test]
fn a_days_publish_reads_only_its_own_keys() {
    let mut per_day_kept: Vec<Vec<(&'static str, u64)>> = Vec::new();
    for keys in [1_000_u32, 20_000] {
        let scratch = Scratch::new(&format!("OWNKEYS{keys}"));
        let root = scratch.root();
        let _ = partition_of(root, keys);
        let _ = reads_since();
        let mut kept = Kept::new();
        // Day 1: whole, once, because the run has read nothing yet.
        let cycles = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(keys, 1, 5)).unwrap();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].read, CensusRead::Whole("first"));
        let first = reads_since();
        assert_eq!(first.len(), 1, "{first:?}");
        assert_eq!(first[0].0, "whole");
        assert_eq!(
            first[0].1,
            HEADER_LEN + u64::from(keys) * 128,
            "the first day read every entry"
        );
        // Days 2..=6: one header region each, whatever the census holds.
        for d in 2..=6 {
            let cycles = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(keys, d, 5)).unwrap();
            assert_eq!(cycles[0].read, CensusRead::Kept, "day {d} at {keys}");
            assert_eq!(cycles[0].recorded, 6, "five new keys and one grown row");
            assert_eq!(cycles[0].exact, 1, "the exact row writes nothing");
            let day = reads_since();
            assert_eq!(
                day,
                vec![("header", HEADER_LEN)],
                "day {d} at {keys} entries"
            );
            per_day_kept.push(day);
        }
        // The per-call door reads the whole list for the same kind of day.
        let _ = record(root, Vendor::Gdfl, &day_rows(keys, 7, 5)).unwrap();
        let unkept = reads_since();
        assert_eq!(unkept.len(), 1);
        assert_eq!(unkept[0].0, "whole");
        assert!(
            unkept[0].1 > HEADER_LEN + u64::from(keys) * 128,
            "the per-call door reads every entry: {unkept:?}"
        );
    }
    // The same read at 1,000 and at 20,000 entries: flat.
    assert!(per_day_kept.windows(2).all(|pair| pair[0] == pair[1]));
}

/// **THE CENSUS AFTER N DAYS IS THE SAME BYTES AS BEFORE** (R3-19's named
/// test): one store published day by day through the per-call door (the
/// census as it was), one through a run's kept memory, from the same fixture
/// and the same days — new keys, rows offered again exactly, grown rows and
/// an empty day among them. Every census file is byte for byte the same, and
/// the kept census in memory is the census a fresh read of the file gives.
#[test]
fn the_census_after_n_days_is_the_same_bytes_as_before() {
    for single in [false, true] {
        let before = Scratch::new("SAMEA");
        let after = Scratch::new("SAMEB");
        for scratch in [&before, &after] {
            if single {
                let _ = single_of(scratch.root(), Vendor::Gdfl, 300);
            } else {
                let _ = partition_of(scratch.root(), 300);
            }
        }
        let mut kept = Kept::new();
        for d in 1..=12 {
            let rows = if d == 7 {
                Vec::new()
            } else {
                day_rows(300, d, 9)
            };
            let a = record(before.root(), Vendor::Gdfl, &rows).unwrap();
            let b = record_kept(&mut kept, after.root(), Vendor::Gdfl, &rows).unwrap();
            let strip = |cycles: &[super::Cycle]| -> Vec<(u64, u64, u64, usize)> {
                cycles
                    .iter()
                    .map(|c| (c.generation, c.entries, c.keys, c.recorded))
                    .collect()
            };
            assert_eq!(strip(&a), strip(&b), "day {d}, single {single}");
            assert_eq!(
                tree(&before.root().join("manifest")),
                tree(&after.root().join("manifest")),
                "day {d}: the same bytes, single {single}"
            );
        }
        assert!(
            kept.len() == 1,
            "one census kept for the run (single {single})"
        );
        // The memory is the file.
        let fresh = if single {
            Target::single(after.root(), Vendor::Gdfl)
                .load()
                .unwrap()
                .census
        } else {
            read_epoch(
                &epoch_path(after.root(), Vendor::Gdfl, Segment::Fno, 1),
                Vendor::Gdfl,
                Segment::Fno,
            )
            .unwrap()
        };
        let target = if single {
            Target::single(after.root(), Vendor::Gdfl)
        } else {
            Target::partition(after.root(), Vendor::Gdfl, Segment::Fno)
        };
        let held = &kept.held.get(&target.named()).expect("kept").loaded.census;
        assert_eq!(held.header(), fresh.header());
        assert_eq!(held.image(), fresh.image(), "single {single}");
    }
}

/// **ANOTHER WRITER BETWEEN TWO DAYS IS SEEN, AND THE CENSUS IS READ WHOLE**:
/// an append by the per-call door between two kept days moves the header and
/// the length, so the kept census is not trusted; the next day reads whole,
/// says why, and the file ends where two per-call writers would leave it.
#[test]
fn another_writer_between_days_is_seen_and_the_census_read_whole() {
    let mine = Scratch::new("OTHERA");
    let reference = Scratch::new("OTHERB");
    for scratch in [&mine, &reference] {
        let _ = partition_of(scratch.root(), 200);
    }
    let mut kept = Kept::new();
    for (d, other) in [(1, false), (2, true), (3, false), (4, true), (5, false)] {
        let rows = day_rows(200, d, 4);
        let got = record_kept(&mut kept, mine.root(), Vendor::Gdfl, &rows).unwrap();
        let _ = record(reference.root(), Vendor::Gdfl, &rows).unwrap();
        let expected = match d {
            1 => CensusRead::Whole("first"),
            3 | 5 => CensusRead::Whole("length_moved"),
            _ => CensusRead::Kept,
        };
        assert_eq!(got[0].read, expected, "day {d}");
        if other {
            let theirs = vec![row(90_000 + d, 1)];
            let _ = record(mine.root(), Vendor::Gdfl, &theirs).unwrap();
            let _ = record(reference.root(), Vendor::Gdfl, &theirs).unwrap();
        }
    }
    assert_eq!(
        tree(&mine.root().join("manifest")),
        tree(&reference.root().join("manifest"))
    );
}

/// **AN EPOCH INSTALLED BETWEEN TWO DAYS IS READ WHOLE**: a re-key's withdraw
/// installs epoch 2 between kept days; the next day sees the epoch moved,
/// reads epoch 2 whole and appends there, never into the epoch it kept.
#[test]
fn an_epoch_installed_between_days_is_read_whole() {
    let scratch = Scratch::new("EPOCHMOVED");
    let root = scratch.root();
    let one = partition_of(root, 100);
    let mut kept = Kept::new();
    let _ = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(100, 1, 3)).unwrap();
    let one_bytes = std::fs::read(&one).unwrap();
    let done = withdraw(root, Vendor::Gdfl, Segment::Fno, &|k: &EntryKey| {
        *k == key(5)
    })
    .unwrap()
    .expect("epoch 2 installed");
    assert_eq!(done.to_epoch, 2);
    let got = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(100, 2, 3)).unwrap();
    assert_eq!(got[0].read, CensusRead::Whole("epoch_moved"));
    assert_eq!(got[0].epoch, Some(2));
    assert_eq!(std::fs::read(&one).unwrap(), one_bytes, "epoch 1 untouched");
    let got = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(100, 3, 3)).unwrap();
    assert_eq!(got[0].read, CensusRead::Kept);
    let census = read_epoch(
        &epoch_path(root, Vendor::Gdfl, Segment::Fno, 2),
        Vendor::Gdfl,
        Segment::Fno,
    )
    .unwrap();
    assert!(
        census.held(&key(5)).is_none(),
        "the withdrawn key stays out"
    );
    assert!(
        census.held(&key(100 + 3 * 3)).is_some(),
        "day 3's rows are in epoch 2"
    );
}

/// **A DAY THAT FAILS KEEPS NOTHING**: a row whose count went backwards is
/// refused, nothing of that day is published, the run's memory of the census
/// is dropped (it held the refused day's rows in memory), and the next day
/// reads the census whole and lands as if the refused day never was.
#[test]
fn a_day_that_fails_keeps_nothing_and_the_next_reads_whole() {
    let scratch = Scratch::new("FAILED");
    let root = scratch.root();
    let one = partition_of(root, 100);
    let mut kept = Kept::new();
    let _ = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(100, 1, 3)).unwrap();
    assert_eq!(kept.len(), 1);
    let before = std::fs::read(&one).unwrap();
    let mut bad = day_rows(100, 2, 3);
    bad.push(row(3, 1)); // key 3 holds 4 rows: a count that went backwards.
    let why = record_kept(&mut kept, root, Vendor::Gdfl, &bad).expect_err("refused");
    assert!(why.contains("nothing of this batch"), "{why}");
    assert!(kept.is_empty(), "a failed day keeps nothing");
    assert_eq!(
        std::fs::read(&one).unwrap(),
        before,
        "nothing of the day landed"
    );
    let got = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(100, 3, 3)).unwrap();
    assert_eq!(got[0].read, CensusRead::Whole("first"));
    assert_eq!(kept.len(), 1);
}

/// **AN EMPTY STORE**: no census at all. The first day installs the first
/// epoch whole (nothing to keep before it existed), the second reads it whole
/// once, and from the third on the census is kept; a single file the same.
#[test]
fn a_store_with_no_census_is_kept_from_its_second_day() {
    for single in [false, true] {
        let scratch = Scratch::new("EMPTY");
        let root = scratch.root();
        if !single {
            std::fs::create_dir_all(vendor_dir(root, Vendor::Gdfl)).unwrap();
        }
        let mut kept = Kept::new();
        let reads: Vec<CensusRead> = (1..=4)
            .map(|d| {
                record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(0, d, 2)).unwrap()[0].read
            })
            .collect();
        assert_eq!(
            reads,
            vec![
                CensusRead::Whole("first"),
                CensusRead::Whole("first"),
                CensusRead::Kept,
                CensusRead::Kept
            ],
            "single {single}"
        );
    }
}

/// **A DAY OFFERING ONLY ROWS ALREADY HELD WRITES NO BYTE AND STAYS KEPT**
/// (`CLAUDE.md` §3 rule 5).
#[test]
fn a_rerun_day_writes_nothing_and_stays_kept() {
    let scratch = Scratch::new("RERUN");
    let root = scratch.root();
    let _ = partition_of(root, 50);
    let mut kept = Kept::new();
    let rows = day_rows(50, 1, 2);
    let _ = record_kept(&mut kept, root, Vendor::Gdfl, &rows).unwrap();
    let before = tree(&root.join("manifest"));
    for _ in 0..3 {
        let got = record_kept(&mut kept, root, Vendor::Gdfl, &rows).unwrap();
        assert_eq!((got[0].read, got[0].recorded), (CensusRead::Kept, 0));
    }
    assert_eq!(tree(&root.join("manifest")), before);
}

/// **A FILE REPLACED UNDER THE SAME NAME, EVEN WITH THE SAME BYTES, IS READ
/// WHOLE**: the kept identity is the inode, not the name.
#[test]
fn a_file_replaced_under_its_name_is_read_whole() {
    let scratch = Scratch::new("REPLACED");
    let root = scratch.root();
    let one = partition_of(root, 50);
    let mut kept = Kept::new();
    let _ = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(50, 1, 2)).unwrap();
    let copy = one.with_extension("copy");
    std::fs::copy(&one, &copy).unwrap();
    std::fs::rename(&copy, &one).unwrap();
    let got = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(50, 2, 2)).unwrap();
    assert_eq!(got[0].read, CensusRead::Whole("file_replaced"));
}

/// **BYTES A DEAD WRITER LEFT PAST THE LAST ENTRY ARE READ WHOLE, NOT
/// TRUSTED**: the length moved, so the kept census is dropped and the census
/// reads as a whole read reads it, the same as the per-call door's.
#[test]
fn a_torn_tail_left_by_a_dead_writer_is_read_whole() {
    let mine = Scratch::new("TORNA");
    let reference = Scratch::new("TORNB");
    for scratch in [&mine, &reference] {
        let _ = partition_of(scratch.root(), 60);
    }
    let mut kept = Kept::new();
    let _ = record_kept(&mut kept, mine.root(), Vendor::Gdfl, &day_rows(60, 1, 2)).unwrap();
    let _ = record(reference.root(), Vendor::Gdfl, &day_rows(60, 1, 2)).unwrap();
    for scratch in [&mine, &reference] {
        use std::io::Write as _;
        let path = epoch_path(scratch.root(), Vendor::Gdfl, Segment::Fno, 1);
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        file.write_all(&[0xA5; 77]).unwrap();
    }
    let got = record_kept(&mut kept, mine.root(), Vendor::Gdfl, &day_rows(60, 2, 2));
    let theirs = record(reference.root(), Vendor::Gdfl, &day_rows(60, 2, 2));
    assert_eq!(
        got.as_ref().map(|c| (c[0].entries, c[0].read)),
        theirs
            .as_ref()
            .map(|c| (c[0].entries, CensusRead::Whole("length_moved"))),
        "{got:?} / {theirs:?}"
    );
    assert_eq!(
        tree(&mine.root().join("manifest")),
        tree(&reference.root().join("manifest"))
    );
}

/// **A LINK SWAPPED IN FOR THE KEPT EPOCH IS REFUSED BY NAME, NEVER READ
/// THROUGH**: the kept check opens without following, so it does not trust
/// the memory, and the whole read refuses the link by name; the file the link
/// names is unchanged.
#[test]
fn a_link_swapped_in_for_the_kept_epoch_is_refused_by_name() {
    let scratch = Scratch::new("KEPTLINK");
    let root = scratch.root();
    let one = partition_of(root, 40);
    let mut kept = Kept::new();
    let _ = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(40, 1, 2)).unwrap();
    let aside = root.join("aside.man");
    std::fs::rename(&one, &aside).unwrap();
    std::os::unix::fs::symlink(&aside, &one).unwrap();
    let before = std::fs::read(&aside).unwrap();
    let why = record_kept(&mut kept, root, Vendor::Gdfl, &day_rows(40, 2, 2)).expect_err("refused");
    assert!(why.contains("epoch_not_regular (symlink)"), "{why}");
    assert!(why.contains(&one.display().to_string()), "{why}");
    assert_eq!(
        std::fs::read(&aside).unwrap(),
        before,
        "nothing through the link"
    );
    assert!(kept.is_empty());
}

/// **TWO SEGMENTS IN ONE DAY ARE TWO KEPT CENSUSES**, each read whole once and
/// kept after, each under its own lock.
#[test]
fn two_segments_are_two_kept_censuses() {
    let scratch = Scratch::new("TWOSEG");
    let root = scratch.root();
    let _ = partition_of(root, 30);
    let mut kept = Kept::new();
    for d in 1..=3 {
        let mut rows = day_rows(30, d, 2);
        rows.push(row_in(Segment::Index, d, 1));
        let got = record_kept(&mut kept, root, Vendor::Gdfl, &rows).unwrap();
        let read: Vec<(Option<Segment>, CensusRead)> =
            got.iter().map(|c| (c.segment, c.read)).collect();
        let expected = match d {
            1 => vec![
                (Some(Segment::Index), CensusRead::Whole("first")),
                (Some(Segment::Fno), CensusRead::Whole("first")),
            ],
            // The index partition had no epoch before day 1: read whole once more.
            2 => vec![
                (Some(Segment::Index), CensusRead::Whole("first")),
                (Some(Segment::Fno), CensusRead::Kept),
            ],
            _ => vec![
                (Some(Segment::Index), CensusRead::Kept),
                (Some(Segment::Fno), CensusRead::Kept),
            ],
        };
        assert_eq!(read, expected, "day {d}");
    }
    assert_eq!(kept.len(), 2);
}

/// **TWO RUNS AT ONCE OVER ONE CENSUS**: each takes the partition's lock per
/// day (a day that finds it held is refused by name and offered again) and
/// sees the other's appends as a moved length, so neither trusts a stale
/// memory; every row of both lands and the census loads clean.
#[test]
fn two_runs_at_once_each_see_the_others_days() {
    let scratch = Scratch::new("TWORUNS");
    let root = scratch.root().to_path_buf();
    let _ = partition_of(&root, 100);
    let runs: Vec<_> = [0_u32, 1]
        .into_iter()
        .map(|run| {
            let root = root.clone();
            std::thread::spawn(move || {
                let mut kept = Kept::new();
                let mut moved = 0_u32;
                for d in 0..20 {
                    let rows = vec![row(200_000 + run * 1_000 + d, 1)];
                    let mut tries = 0_u32;
                    let got = loop {
                        match record_kept(&mut kept, &root, Vendor::Gdfl, &rows) {
                            Err(why) if why.contains("another ingest holds the census lock") => {
                                tries += 1;
                                assert!(tries < 10_000, "{why}");
                                std::thread::sleep(std::time::Duration::from_millis(1));
                            }
                            other => break other.unwrap(),
                        }
                    };
                    if got[0].read == CensusRead::Whole("length_moved") {
                        moved += 1;
                    }
                }
                moved
            })
        })
        .collect();
    let moved: u32 = runs
        .into_iter()
        .map(|run| run.join().expect("a run ends"))
        .sum();
    eprintln!("two runs at once: {moved} day(s) saw the other run's append and read whole");
    let census = read_epoch(
        &epoch_path(&root, Vendor::Gdfl, Segment::Fno, 1),
        Vendor::Gdfl,
        Segment::Fno,
    )
    .unwrap();
    assert!(census.degraded_reason().is_none());
    assert_eq!(census.keys(), 140, "100 held, 20 from each run");
}

// ═══ AG-05 follow-ups, part 1: the segment directory (verify-AG-05-r3) ═══════

/// Every name under `dir` (not following links), with its bytes.
fn listing(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    tree(dir)
}

/// **A SYMBOLIC LINK AT A SEGMENT DIRECTORY'S NAME IS REFUSED BY NAME ON
/// INSTALL**, before any byte is staged: the directory it names is byte for
/// byte as it was and no epoch name exists anywhere. Through the install door
/// itself and through a day's publish (whose listing refuses it first).
#[test]
fn a_symlinked_segment_directory_is_refused_by_name_on_install() {
    let scratch = Scratch::new("SEGLINK");
    let root = scratch.root();
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    // A real epoch, but in a directory the census does not own.
    let made = partition_of(root, 8);
    std::fs::rename(&made, elsewhere.join("000001.man")).unwrap();
    let seg = segment_dir(root, Vendor::Gdfl, Segment::Fno);
    std::fs::remove_dir(&seg).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &seg).unwrap();
    let before = listing(&elsewhere);

    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let lock = target.lock().expect("the lock");
    let image = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 64)
        .unwrap()
        .image();
    let why = install_epoch_with(
        &lock,
        root,
        Vendor::Gdfl,
        Segment::Fno,
        2,
        &image,
        &mut |_: WholeStep| Ok(()),
    )
    .expect_err("refused");
    drop(lock);
    for part in [
        "segment_dir_not_directory (symlink)",
        "is a symbolic link, not a directory",
        &seg.display().to_string(),
    ] {
        assert!(why.contains(part), "{part}: {why}");
    }
    let why = record(root, Vendor::Gdfl, &[row(1_000, 1)]).expect_err("refused");
    assert!(why.contains("segment_dir_not_directory (symlink)"), "{why}");
    assert_eq!(
        listing(&elsewhere),
        before,
        "the link's target is as it was"
    );
    for name in ["000002.man", "000002.man.tmp"] {
        assert!(
            std::fs::symlink_metadata(elsewhere.join(name)).is_err(),
            "{name}"
        );
    }
    assert!(
        std::fs::symlink_metadata(&seg)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link is left as it is"
    );
}

/// **A FILE WHERE A SEGMENT DIRECTORY BELONGS IS REFUSED BY NAME**, not with
/// the host's `File exists`, and the file keeps its bytes.
#[test]
fn a_file_where_a_segment_directory_belongs_is_refused_by_name() {
    let scratch = Scratch::new("SEGFILE");
    let root = scratch.root();
    std::fs::create_dir_all(vendor_dir(root, Vendor::Gdfl)).unwrap();
    let seg = segment_dir(root, Vendor::Gdfl, Segment::Fno);
    std::fs::write(&seg, b"an invented file where a directory belongs").unwrap();
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let lock = target.lock().expect("the lock");
    let image = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 64)
        .unwrap()
        .image();
    let why = install_epoch_with(
        &lock,
        root,
        Vendor::Gdfl,
        Segment::Fno,
        1,
        &image,
        &mut |_: WholeStep| Ok(()),
    )
    .expect_err("refused");
    drop(lock);
    assert!(why.contains("segment_dir_not_directory (file)"), "{why}");
    assert!(why.contains("is a regular file, not a directory"), "{why}");
    let why = record(root, Vendor::Gdfl, &[row(1, 1)]).expect_err("refused");
    assert!(why.contains("segment_dir_not_directory (file)"), "{why}");
    assert_eq!(
        std::fs::read(&seg).unwrap(),
        b"an invented file where a directory belongs"
    );
}

/// **A SYMBOLIC LINK AT THE VENDOR'S DIRECTORY IS STILL REFUSED** (a
/// regression guard: D-3316's `locate` refused it before R3-19).
#[test]
fn a_symlinked_vendor_directory_is_still_refused_on_install() {
    let scratch = Scratch::new("VENDORLINK");
    let root = scratch.root();
    let elsewhere = root.join("elsewhere-vendor");
    std::fs::create_dir_all(elsewhere.join("FNO")).unwrap();
    std::fs::create_dir_all(root.join("manifest/v4")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, vendor_dir(root, Vendor::Gdfl)).unwrap();
    let why = record(root, Vendor::Gdfl, &[row(1, 1)]).expect_err("refused");
    assert!(
        why.contains(&vendor_dir(root, Vendor::Gdfl).display().to_string())
            && why.contains("is not a directory"),
        "{why}"
    );
    assert_eq!(
        listing(&elsewhere),
        Vec::new(),
        "nothing written through it"
    );
}

// ═══ AG-05 follow-ups, part 2: the stub check never waits ═══════════════════

/// A FIFO made by `mkfifo` (the external program, from a test only).
fn mkfifo(at: &Path) {
    let made = std::process::Command::new("mkfifo")
        .arg(at)
        .status()
        .expect("mkfifo ran");
    assert!(made.success(), "mkfifo made {}", at.display());
}

/// **A FIFO SWAPPED IN AFTER THE LOOK NEVER BLOCKS THE STUB CHECK**: the
/// hook renames a FIFO over the stub between the moment the check is asked
/// and its open (where the old `lstat` looked). The check runs on a thread;
/// its answer must come within 100 ms, and must be `false`. Before R3-19 the
/// `File::open` that followed the `lstat` waited for a writer that never
/// came: the receive timed out, and the test then opens the FIFO for writing
/// so the thread ends, and fails.
#[test]
fn a_fifo_swapped_in_after_the_look_never_blocks_the_stub_check() {
    let scratch = Scratch::new("STUBFIFO");
    let root = scratch.root();
    let stub = manifest_path(root, Vendor::Gdfl);
    std::fs::write(&stub, sealed_stub(Vendor::Gdfl)).unwrap();
    assert!(holds_sealed_stub(&stub), "the stub itself is a stub");
    let fifo = root.join("swapped.fifo");
    mkfifo(&fifo);
    let (started, start) = std::sync::mpsc::channel();
    let (told, answer) = std::sync::mpsc::channel();
    let checking = {
        let (stub, fifo) = (stub.clone(), fifo.clone());
        std::thread::spawn(move || {
            stub_hook::set(move |at: &Path| {
                std::fs::rename(&fifo, at).expect("the FIFO swapped in");
            });
            let _ = started.send(());
            let _ = told.send(holds_sealed_stub(&stub));
        })
    };
    start.recv().expect("the check started");
    let got = answer.recv_timeout(std::time::Duration::from_millis(100));
    if got.is_err() {
        // Release the open that waits, so the suite does not hang; then fail.
        let _ = std::fs::OpenOptions::new().write(true).open(&stub);
    }
    checking.join().expect("the check ends");
    assert!(
        !got.expect("answered within 100 ms, never waiting on the FIFO"),
        "a FIFO is not a stub"
    );
}

/// **A LINK SWAPPED IN AFTER THE LOOK IS NOT A STUB**: the hook renames a
/// symbolic link to a real stub over the stub's name. Before R3-19 the open
/// followed it and read the target as the stub (`true`).
#[test]
fn a_link_swapped_in_after_the_look_is_not_a_stub() {
    let scratch = Scratch::new("STUBLINK");
    let root = scratch.root();
    let stub = manifest_path(root, Vendor::Gdfl);
    std::fs::write(&stub, sealed_stub(Vendor::Gdfl)).unwrap();
    let real = root.join("real-stub");
    std::fs::write(&real, sealed_stub(Vendor::Gdfl)).unwrap();
    let link = root.join("swapped.link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    stub_hook::set(move |at: &Path| {
        std::fs::rename(&link, at).expect("the link swapped in");
    });
    assert!(!holds_sealed_stub(&stub), "a link is not a stub");
    assert!(
        std::fs::symlink_metadata(&stub)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the swap happened"
    );
}

/// The stub check as it was before R3-19 — `lstat`, then `File::open`, then
/// a 16-byte read — kept here only as the baseline the measurement prints
/// beside the door.
fn holds_sealed_stub_as_before(path: &Path) -> bool {
    use std::io::Read as _;
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.len() != sealed_stub(Vendor::Gdfl).len() as u64 {
        return false;
    }
    let mut head = [0_u8; 16];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut head))
        .is_ok()
        && head.starts_with(b"BRUTEXM")
}

/// The seal-stub check in-process, n = 200 each, before (the `lstat` and
/// `File::open` it was) and after (one no-follow, non-blocking open and its
/// `fstat`): p50, p99 and max printed for `docs/06-limits.md` L-R319-06. No
/// wall-clock bound is asserted (a bound on this shared host measures its
/// load); the answer is.
#[test]
fn the_stub_check_costs_one_open_measured() {
    let scratch = Scratch::new("STUBBENCH");
    let stub = manifest_path(scratch.root(), Vendor::Gdfl);
    std::fs::write(&stub, sealed_stub(Vendor::Gdfl)).unwrap();
    let mut said = Vec::new();
    for (series, check) in [
        (
            "before (lstat, File::open)",
            holds_sealed_stub_as_before as fn(&Path) -> bool,
        ),
        (
            "after (open_read_len)",
            holds_sealed_stub as fn(&Path) -> bool,
        ),
    ] {
        let mut micros = Vec::with_capacity(200);
        for _ in 0..200 {
            let start = std::time::Instant::now();
            assert!(check(&stub), "{series}");
            micros.push(start.elapsed().as_nanos());
        }
        let (p50, p99, max) = percentiles(&mut micros);
        said.push(format!(
            "{series}: p50 {p50} ns, p99 {p99} ns, max {max} ns"
        ));
    }
    eprintln!("R3-19 holds_sealed_stub, n = 200 each: {}", said.join("; "));
}

// ═══ AG-05 follow-ups, part 3: a link at the single census is named ═════════

/// **A LINK AT THE SINGLE-FILE CENSUS IS NAMED AS A LINK AT LOAD** (L-AG05-07),
/// with its path, never in the host's ELOOP words.
#[test]
fn a_link_at_the_single_census_is_named_as_a_link_at_load() {
    let scratch = Scratch::new("SINGLELINK");
    let root = scratch.root();
    let path = single_of(root, Vendor::Gdfl, 4);
    let aside = root.join("aside-census");
    std::fs::rename(&path, &aside).unwrap();
    std::os::unix::fs::symlink(&aside, &path).unwrap();
    for why in [
        Target::single(root, Vendor::Gdfl)
            .load()
            .expect_err("refused"),
        record(root, Vendor::Gdfl, &[row(9, 1)]).expect_err("refused"),
        super::read_vendor(root, Vendor::Gdfl).expect_err("refused"),
    ] {
        assert!(why.contains("symbolic link"), "{why}");
        assert!(why.contains(&path.display().to_string()), "{why}");
        assert!(!why.contains("levels of symbolic links"), "named: {why}");
    }
}

/// **A LINK AT THE CENSUS'S `.writing` NAME IS REFUSED AND ITS TARGET IS
/// UNCHANGED** (L-AG05-01): the single-file census's whole publish (here its
/// first write) creates `<vendor>.man.writing` through
/// `store::staging::create`. A symbolic link there is refused
/// `staging_not_regular`, a hard link `staging_linked`, and the file either
/// names keeps its bytes; no census is published. Before R3-19 `File::create`
/// truncated the target, wrote the census into it, and renamed the link into
/// the census's name.
#[test]
fn a_link_at_the_census_writing_name_is_refused_and_its_target_unchanged() {
    for hard in [false, true] {
        let scratch = Scratch::new("WRITING");
        let root = scratch.root();
        let victim = root.join("victim");
        std::fs::write(&victim, b"invented bytes a census must never overwrite").unwrap();
        let writing = manifest_path(root, Vendor::Zerodha).with_extension("man.writing");
        if hard {
            std::fs::hard_link(&victim, &writing).unwrap();
        } else {
            std::os::unix::fs::symlink(&victim, &writing).unwrap();
        }
        let why = record(root, Vendor::Zerodha, &[row(1, 1)]).expect_err("refused");
        let word = if hard {
            "staging_linked"
        } else {
            "staging_not_regular (symlink)"
        };
        assert!(why.contains(word), "{word}: {why}");
        assert!(why.contains(&writing.display().to_string()), "{why}");
        assert_eq!(
            std::fs::read(&victim).unwrap(),
            b"invented bytes a census must never overwrite"
        );
        assert!(
            std::fs::symlink_metadata(manifest_path(root, Vendor::Zerodha)).is_err(),
            "no census was published"
        );
        // Moved aside, the first write lands.
        std::fs::remove_file(&writing).unwrap();
        let _ = record(root, Vendor::Zerodha, &[row(1, 1)]).expect("landed");
        let meta = std::fs::symlink_metadata(manifest_path(root, Vendor::Zerodha)).unwrap();
        assert!(meta.is_file() && meta.nlink() == 1);
    }
}

// ═══ AG-05 follow-ups, part 4: one parenthetical ════════════════════════════

/// **AN EPOCH REFUSAL NEVER STACKS TWO PARENTHETICALS**: for every kind
/// `store::safe_open::Found` names, the refusal starts with its key and the
/// kind's word once, and holds no `) (`. It said `is a pipe (FIFO) (pipe)`.
#[test]
fn an_epoch_refusal_never_stacks_two_parentheticals() {
    use store::safe_open::Found;
    let path = Path::new("/invented/manifest/v4/gdfl/FNO/000001.man");
    for found in [
        Found::Symlink,
        Found::Pipe,
        Found::Socket,
        Found::CharDevice,
        Found::BlockDevice,
        Found::Directory,
        Found::Unknown,
    ] {
        let said = not_an_epoch(path, found);
        assert!(
            said.starts_with(&format!("epoch_not_regular ({}): ", found.word())),
            "{said}"
        );
        assert!(!said.contains(") ("), "{said}");
        assert!(
            said.contains(&format!("{} is {}", path.display(), found.sentence())),
            "{said}"
        );
    }
}

/// The listing and the epoch read both say the new words for a link at an
/// epoch's name.
#[test]
fn a_link_at_an_epoch_name_is_said_once_by_the_listing_and_the_read() {
    let scratch = Scratch::new("EPOCHWORDS");
    let root = scratch.root();
    let one = partition_of(root, 4);
    let two = epoch_path(root, Vendor::Gdfl, Segment::Fno, 2);
    std::os::unix::fs::symlink(&one, &two).unwrap();
    let listed = current_epoch(root, Vendor::Gdfl, Segment::Fno).expect_err("refused");
    let read = read_epoch(&two, Vendor::Gdfl, Segment::Fno).expect_err("refused");
    for why in [listed, read] {
        assert!(why.starts_with("epoch_not_regular (symlink): "), "{why}");
        assert!(!why.contains(") ("), "{why}");
    }
}

// ═══ R3-19's measurement: the publish at 0.4, 2, 4 and 8.4 million ═════════

/// **THE KEPT PUBLISH IS FLAT FROM 0.4 TO 8.4 MILLION ENTRIES** (R3-19's
/// measured target; `docs/06-limits.md` L-IA-11): n = 50 kept day publishes
/// of 64 new keys each at every size, p50, p99 and max printed, the first
/// day's whole read printed beside them, and the p50 at 8.4 million within
/// the ratio gate's 3x of the p50 at 0.4 million.
///
/// Ignored by default: the 8.4-million census is 1.07 GB on disk and about
/// 3.4 GB decoded. Run it in release with a scratch root on a disk with room:
/// `R319_BENCH_ROOT=<dir> cargo test -p pull --release --lib
/// kept_tests::the_kept_publish_is_flat_measured -- --ignored --nocapture`.
#[test]
#[ignore = "builds censuses of up to 8.4 million entries (about 1.9 GB on disk); run in release"]
fn the_kept_publish_is_flat_measured() {
    const N: u32 = 50;
    const FRESH: u32 = 64;
    let base = std::env::var_os("R319_BENCH_ROOT").map_or_else(std::env::temp_dir, PathBuf::from);
    let mut p50s = Vec::new();
    for keys in [400_000_u32, 2_000_000, 4_000_000, 8_400_000] {
        let root = base.join(format!("brutex-r319-bench-{keys}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("manifest")).expect("a bench root");
        let built = std::time::Instant::now();
        let capacity = u64::from(keys) + u64::from(N * FRESH) + 4_096;
        let mut census =
            Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, capacity).expect("genesis");
        for n in 0..keys {
            census.record_held(row(n, 1)).expect("held");
        }
        let path = epoch_path(&root, Vendor::Gdfl, Segment::Fno, 1);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
        std::fs::write(&path, census.image()).expect("epoch 1");
        drop(census);
        let built = built.elapsed().as_millis();
        let mut kept = Kept::new();
        let whole = std::time::Instant::now();
        let first = record_kept(
            &mut kept,
            &root,
            Vendor::Gdfl,
            &(0..FRESH).map(|i| row(keys + i, 1)).collect::<Vec<_>>(),
        )
        .expect("day 1");
        let whole = whole.elapsed().as_micros();
        assert_eq!(first[0].read, CensusRead::Whole("first"));
        let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
        for d in 1..=N {
            let rows: Vec<Held> = (0..FRESH).map(|i| row(keys + d * FRESH + i, 1)).collect();
            let start = std::time::Instant::now();
            let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
            micros.push(start.elapsed().as_micros());
            assert_eq!(got[0].read, CensusRead::Kept, "{keys}: day {d}");
        }
        let (p50, p99, max) = percentiles(&mut micros);
        eprintln!(
            "R3-19 kept publish at {keys} entries, n = {N} days of {FRESH} new keys: \
             p50 {p50} us, p99 {p99} us, max {max} us; the first day's whole read \
             {whole} us; fixture built in {built} ms; file {} bytes",
            std::fs::metadata(&path).map_or(0, |m| m.len())
        );
        p50s.push(p50);
        drop(kept);
        let _ = std::fs::remove_dir_all(&root);
    }
    let (small, large) = (p50s[0], p50s[3]);
    eprintln!("R3-19 ratio p50(8.4M) / p50(0.4M) = {large} / {small}");
    assert!(
        large <= small.saturating_mul(3).max(1),
        "the kept publish is not flat: {p50s:?}"
    );
}

/// **THE KEPT PUBLISH ON A COPY OF A REAL CENSUS EPOCH** (R3-19's check on
/// real data, never on the real store): `R319_REAL_ROOT` names a scratch
/// store root the caller made, holding a CLONE (`cp -c`) of one real GDFL FNO
/// epoch at `manifest/v4/gdfl/FNO/<epoch>.man`; the clone is written, the
/// real file never is. n = 50 kept day publishes of 64 invented keys each
/// (a symbol no exchange lists), p50, p99 and max printed beside the first
/// day's whole read.
#[test]
#[ignore = "needs R319_REAL_ROOT: a scratch store holding a clone of a real census epoch"]
fn the_kept_publish_on_a_copy_of_a_real_epoch_measured() {
    const N: u32 = 50;
    const FRESH: u32 = 64;
    let root = PathBuf::from(std::env::var_os("R319_REAL_ROOT").expect("R319_REAL_ROOT"));
    let invented = |n: u32| {
        let mut held = row(n, 1);
        held.entry.key.symbol = Symbol::new("INVENTEDR").expect("a symbol");
        held
    };
    let mut kept = Kept::new();
    let whole = std::time::Instant::now();
    let first = record_kept(
        &mut kept,
        &root,
        Vendor::Gdfl,
        &(0..FRESH).map(invented).collect::<Vec<_>>(),
    )
    .expect("day 1");
    let whole = whole.elapsed().as_micros();
    assert_eq!(first[0].read, CensusRead::Whole("first"));
    let entries = first[0].entries;
    let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
    for d in 1..=N {
        let rows: Vec<Held> = (0..FRESH).map(|i| invented(d * FRESH + i)).collect();
        let start = std::time::Instant::now();
        let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
        micros.push(start.elapsed().as_micros());
        assert_eq!(got[0].read, CensusRead::Kept, "day {d}");
    }
    let (p50, p99, max) = percentiles(&mut micros);
    eprintln!(
        "R3-19 kept publish on a clone of a real epoch of {entries} entries, n = {N} days of \
         {FRESH} invented keys: p50 {p50} us, p99 {p99} us, max {max} us; the first day's \
         whole read {whole} us"
    );
}

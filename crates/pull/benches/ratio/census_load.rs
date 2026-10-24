//! C-CHK-04, C-CHK-05 and C-CHK-06 (CC-20, FM-615, D-6803): the census COLD
//! LOAD, `pull::ingest::census_store::Target::load`, "the census, read
//! whole", from the file a run finds on disk. It is O(entries) by design,
//! once per run or per cycle under the census's lock, never per lookup (the
//! warm probe after it is `crates/api/benches/ratio/census.rs`'s, unchanged).
//! These rows assert that shape and print the measured time:
//!
//! - C-CHK-04: a single-file census (`manifest/<vendor>.man`, versions 1 to
//!   3), the cost PER ENTRY of a whole load flat at 1,000, 10,000 and 100,000
//!   entries.
//! - C-CHK-05: a version-4 partition (`manifest/v4/<vendor>/<SEGMENT>/`), one
//!   epoch, the same per entry at the same sizes.
//! - C-CHK-06: the epochs. Finding the current epoch lists the partition's
//!   folder (`current_epoch`, one entry per compaction the partition ever had):
//!   its cost PER EPOCH is flat at 10, 100 and 1,000 epochs; and the whole load
//!   with both grown together (1,000 entries over 10 epochs, 10,000 over 100,
//!   100,000 over 1,000) is flat per entry-or-epoch.
//!
//! The files are written once through the census's own image
//! (`pull::manifest::Manifest::image`) into new folders under the system temp
//! directory, printed and left for clean-up (R4-08's no-delete rule). Each
//! load reads them from the page cache: the device's own read cost is not in
//! the figure, and the row says so. Every key is invented. The harness,
//! `crates/pull/benches/ratio.rs`, fails on a ratio above its ceiling and
//! prints p50, p99 and max beside each.

use std::hint::black_box;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Segment;
use brutex_core::vendor::Vendor;
use pull::ingest::census_store::{Target, current_epoch};
use pull::manifest::{Entry, FNO_FLOOR, Manifest, epoch_path, manifest_path, segment_dir};

use super::{cost_ps, key, partition_bytes, ratio, refuse, spread};

/// Entries at 1x; 10x and 100x hold ten and a hundred times as many.
const BASE_ENTRIES: u32 = 1_000;
/// Epochs at 1x for C-CHK-06.
const BASE_EPOCHS: u32 = 10;

/// A new, empty store root under the system temp directory, printed.
fn root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-chk-census-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    if std::fs::create_dir_all(&dir).is_err() {
        refuse("a scratch store root");
    }
    println!("  C-CHK-04 scratch (left for clean-up): {}", dir.display());
    dir
}

/// Writes `bytes` at `path`, making its folder.
fn put(path: &Path, bytes: &[u8]) {
    let made = path
        .parent()
        .is_some_and(|parent| std::fs::create_dir_all(parent).is_ok());
    if !made || std::fs::write(path, bytes).is_err() {
        refuse("a census file written");
    }
}

/// A store root holding a single-file census of `n` entries.
fn single(n: u32) -> PathBuf {
    let Ok(mut census) = Manifest::open(Vendor::Gdfl, &[], &[]) else {
        refuse("a genesis census")
    };
    for i in 0..n {
        let entry = Entry {
            key: key(i),
            rows: 375,
            first_ts_micros: 1_000,
            last_ts_micros: 2_000,
        };
        if census.record(entry).is_err() {
            refuse("a census entry");
        }
    }
    let at = root(&format!("single-{n}"));
    put(&manifest_path(&at, Vendor::Gdfl), &census.image());
    at
}

/// A store root holding an FNO partition of `epochs` epochs, the newest
/// holding `n` entries and each older one the empty census a compaction
/// left behind.
fn partition(n: u32, epochs: u32) -> PathBuf {
    let at = root(&format!("partition-{n}-{epochs}"));
    let Ok(genesis) = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, FNO_FLOOR) else {
        refuse("a genesis partition")
    };
    let older = genesis.image();
    for epoch in 1..epochs {
        put(&epoch_path(&at, Vendor::Gdfl, Segment::Fno, epoch), &older);
    }
    put(
        &epoch_path(&at, Vendor::Gdfl, Segment::Fno, epochs),
        &partition_bytes(n, 1),
    );
    at
}

/// Entries a whole load of `target` reads, refusing a load that fails.
fn load(target: &Target) -> u64 {
    match target.load() {
        Ok(loaded) => loaded.census.entries(),
        Err(why) => refuse(&format!("the census load: {why}")),
    }
}

/// The extremes a run can find on disk, asked before anything is timed: no
/// census at all, a partition with no epoch, a census of one entry, an epoch
/// half written (`.man.tmp`, never an answer) and a symbolic link at an
/// epoch's name (refused by name, never followed).
fn the_extremes_answer() {
    let bare = root("bare");
    match Target::single(&bare, Vendor::Gdfl).load() {
        Ok(loaded) if !loaded.exists && loaded.census.entries() == 0 => {}
        _ => refuse("a store with no census loads an empty one that does not exist"),
    }
    match Target::partition(&bare, Vendor::Gdfl, Segment::Fno).load() {
        Ok(loaded) if loaded.epoch.is_none() && !loaded.exists => {}
        _ => refuse("a partition with no epoch loads absent"),
    }
    if load(&Target::single(&single(1), Vendor::Gdfl)) != 1 {
        refuse("a census of one entry loads it");
    }
    let torn = partition(1, 2);
    let tmp = segment_dir(&torn, Vendor::Gdfl, Segment::Fno).join("000003.man.tmp");
    put(&tmp, b"half an epoch");
    match current_epoch(&torn, Vendor::Gdfl, Segment::Fno) {
        Ok(Some(2)) => {}
        _ => refuse("an epoch being written is never the current one"),
    }
    let linked = partition(1, 1);
    let link = epoch_path(&linked, Vendor::Gdfl, Segment::Fno, 2);
    if std::os::unix::fs::symlink(epoch_path(&linked, Vendor::Gdfl, Segment::Fno, 1), &link)
        .is_err()
    {
        refuse("a symbolic link at an epoch's name");
    }
    match Target::partition(&linked, Vendor::Gdfl, Segment::Fno).load() {
        Err(why) if why.contains("epoch_not_regular") => {}
        _ => refuse("a symbolic link at an epoch's name is refused by name"),
    }
}

/// Picoseconds per unit of one whole load of `target`, which must read
/// `entries` entries; `units` is what the cost is divided by.
fn per_unit(target: &Target, entries: u32, units: u32) -> u128 {
    if load(target) != u64::from(entries) {
        refuse("the load read every entry the fixture wrote");
    }
    let reps = (BASE_ENTRIES * 100 / entries).max(1);
    let whole = cost_ps(reps, || load(black_box(target)));
    println!(
        "  census cold load of {entries:>6} entries ({units} units): {} us per load",
        whole / 1_000_000
    );
    whole / u128::from(units.max(1))
}

/// C-CHK-04, C-CHK-05 and C-CHK-06.
pub(crate) fn cold_load_is_flat_per_entry() -> bool {
    the_extremes_answer();
    let sizes = [BASE_ENTRIES, BASE_ENTRIES * 10, BASE_ENTRIES * 100];
    let mut ok = true;
    let mut largest = None;

    let singles = sizes.map(|n| (n, Target::single(&single(n), Vendor::Gdfl)));
    let [s1, s10, s100] = singles
        .each_ref()
        .map(|(n, target)| per_unit(target, *n, *n));
    ok &= ratio("C-CHK-04 single-file load per entry, 10x", s1, s10);
    ok &= ratio("C-CHK-04 single-file load per entry, 100x", s1, s100);

    let partitions = sizes.map(|n| {
        let at = partition(n, 1);
        (n, Target::partition(&at, Vendor::Gdfl, Segment::Fno))
    });
    let [p1, p10, p100] = partitions
        .each_ref()
        .map(|(n, target)| per_unit(target, *n, *n));
    ok &= ratio("C-CHK-05 partition load per entry, 10x", p1, p10);
    ok &= ratio("C-CHK-05 partition load per entry, 100x", p1, p100);

    let mut listing = Vec::with_capacity(3);
    let mut whole = Vec::with_capacity(3);
    for (n, epochs) in sizes.into_iter().zip([1, 10, 100].map(|x| x * BASE_EPOCHS)) {
        let at = partition(n, epochs);
        match current_epoch(&at, Vendor::Gdfl, Segment::Fno) {
            Ok(Some(newest)) if newest == epochs => {}
            _ => refuse("the newest epoch is the current one"),
        }
        let reps = (BASE_EPOCHS * 100 / epochs).max(1);
        let listed = cost_ps(reps, || {
            current_epoch(black_box(&at), Vendor::Gdfl, Segment::Fno).ok()
        });
        listing.push(listed / u128::from(epochs));
        let target = Target::partition(&at, Vendor::Gdfl, Segment::Fno);
        whole.push(per_unit(&target, n, n + epochs));
        largest = Some((target, n, epochs));
    }
    let ([e1, e10, e100], [w1, w10, w100]) = (listing.as_slice(), whole.as_slice()) else {
        refuse("three sizes measured")
    };
    ok &= ratio("C-CHK-06 current epoch per epoch, 10x epochs", *e1, *e10);
    ok &= ratio("C-CHK-06 current epoch per epoch, 100x epochs", *e1, *e100);
    ok &= ratio("C-CHK-06 load per entry-or-epoch, 10x both", *w1, *w10);
    ok &= ratio("C-CHK-06 load per entry-or-epoch, 100x both", *w1, *w100);

    let [_, _, (n, single_big)] = &singles;
    spread(
        &format!("C-CHK-04 single-file load, {n} entries"),
        200,
        || {
            black_box(load(single_big));
        },
    );
    let [_, _, (n, partition_big)] = &partitions;
    spread(
        &format!("C-CHK-05 partition load, {n} entries"),
        200,
        || {
            black_box(load(partition_big));
        },
    );
    let Some((target, n, epochs)) = largest else {
        refuse("the largest partition")
    };
    spread(
        &format!("C-CHK-06 load, {n} entries over {epochs} epochs"),
        200,
        || {
            black_box(load(&target));
        },
    );
    ok
}

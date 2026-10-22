//! `/db`'s census reads take the census writer's one listing rule and open an
//! epoch without following it (R3-19, L-AG05-05).
//!
//! `api::census::current_epoch` listed `<epoch>.man` names without their
//! type and `sized_epoch` opened an epoch with `O_NONBLOCK` but not
//! `O_NOFOLLOW`, so a symbolic link at an epoch's name was this reader's
//! current epoch and was read through, where `pull::ingest::census_store`
//! refuses it by name. Every fixture is invented: keys and counts in scratch
//! stores under the host's temporary directory.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::manifest::{
    Closes, Entry, EntryKey, Held, Manifest, epoch_path, manifest_path, parse_epoch_name,
    segment_dir,
};
use store::path::{Timeframe, YearMonth};

use super::{
    Census, Fault, current_epoch, manifest_reads, open_without_waiting, read_vendor, sized_epoch,
};

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-api-epoch-link-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(&root).expect("a scratch store");
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An invented FNO epoch image holding `keys` keys from `from`.
fn epoch_image(from: u32, keys: u32) -> Vec<u8> {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 64).expect("genesis");
    for n in from..from + keys {
        census
            .record_held(Held::new(
                Entry {
                    key: EntryKey {
                        contract: Contract::parse(&format!(
                            "2019-12-26-{}-CE",
                            1_000_000 + n * 100
                        )),
                        exchange: Exchange::Nse,
                        segment: Segment::Fno,
                        symbol: Symbol::new("NIFTY").expect("a symbol"),
                        timeframe: Timeframe::SECOND_1,
                        month: YearMonth::new(2019, 11).expect("a month"),
                    },
                    rows: u64::from(n) + 1,
                    first_ts_micros: 10,
                    last_ts_micros: 11 + i64::from(n),
                },
                Closes::UNKNOWN,
            ))
            .expect("held");
    }
    census.image()
}

/// The vendor's census answer, which must be a refusal naming `words`, with
/// nothing read at `unread`.
fn refused_by_name(root: &Path, words: &[&str], unread: &[&Path]) {
    let census = read_vendor(root, Vendor::Gdfl);
    let Census::Unreadable { reason, fault } = &census.state else {
        panic!("a link must be refused, never read: {}", census.note());
    };
    assert!(matches!(fault, Fault::Refused), "{fault:?}: {reason}");
    for word in words {
        assert!(reason.contains(word), "{word}: {reason}");
    }
    assert!(census.is_loud(), "a refusal is loud, never an empty census");
    for path in unread {
        assert_eq!(
            manifest_reads::any_of(path),
            0,
            "nothing behind the link was read: {}",
            path.display()
        );
    }
}

/// **`/db`'s census refuses a symbolic link at an epoch's name, by name**:
/// the listing names it, the counting reader shows nothing behind it read,
/// and the answer is a named refusal, never an empty census and never the
/// link's target served as the census. A link at the segment's directory is
/// refused the same way.
#[test]
fn api_census_refuses_a_symlinked_epoch_by_name() {
    let scratch = Scratch::new("EPOCH");
    let root = scratch.0.as_path();
    let one = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
    std::fs::create_dir_all(one.parent().expect("a parent")).expect("the segment");
    std::fs::write(&one, epoch_image(0, 3)).expect("epoch 1");
    let elsewhere = root.join("elsewhere.man");
    std::fs::write(&elsewhere, epoch_image(100, 7)).expect("an epoch outside the census");
    let two = epoch_path(root, Vendor::Gdfl, Segment::Fno, 2);
    std::os::unix::fs::symlink(&elsewhere, &two).expect("a link at epoch 2");
    refused_by_name(
        root,
        &["epoch_not_regular (symlink)", &two.display().to_string()],
        &[&two, &elsewhere],
    );
    // Moved aside, epoch 1 is the census again.
    std::fs::remove_file(&two).expect("the link removed");
    let census = read_vendor(root, Vendor::Gdfl);
    assert!(
        matches!(census.state, Census::Held { .. }),
        "{}",
        census.note()
    );

    // A link at the segment's directory.
    let scratch = Scratch::new("SEGMENT");
    let root = scratch.0.as_path();
    let real = root.join("elsewhere-segment");
    std::fs::create_dir_all(&real).expect("a directory outside the census");
    std::fs::write(real.join("000001.man"), epoch_image(0, 3)).expect("an epoch there");
    let seg = segment_dir(root, Vendor::Gdfl, Segment::Fno);
    std::fs::create_dir_all(seg.parent().expect("the vendor")).expect("the vendor");
    std::os::unix::fs::symlink(&real, &seg).expect("a link at the segment");
    let through = seg.join("000001.man");
    refused_by_name(
        root,
        &[
            "segment_dir_not_directory (symlink)",
            &seg.display().to_string(),
        ],
        &[&through, &real.join("000001.man")],
    );
}

/// The listing and the epoch open as they were before R3-19 — names without
/// their type, an open that follows a link — kept only as the baseline the
/// measurement prints beside the shared rule. The bytes read.
fn listed_and_read_as_before(dir: &Path) -> usize {
    use std::io::Read as _;
    let mut newest: Option<u32> = None;
    for entry in std::fs::read_dir(dir).expect("listed") {
        if let Some(epoch) = entry
            .expect("an entry")
            .file_name()
            .to_str()
            .and_then(parse_epoch_name)
        {
            newest = Some(newest.map_or(epoch, |seen| seen.max(epoch)));
        }
    }
    let path = dir.join(format!("{:06}.man", newest.expect("an epoch")));
    let mut bytes = Vec::new();
    let _ = open_without_waiting(&path)
        .expect("opened")
        .read_to_end(&mut bytes)
        .expect("read");
    bytes.len()
}

/// p50, p99 and max of `nanos`, sorted in place.
fn percentiles(nanos: &mut [u128]) -> (u128, u128, u128) {
    nanos.sort_unstable();
    let n = nanos.len();
    (nanos[n / 2], nanos[n * 99 / 100], nanos[n - 1])
}

/// **THE CENSUS COLD READ STAYS WITHIN THE RATIO GATE** (R3-19, L-AG05-05):
/// one segment's listing and epoch read, n = 200, through the old code and
/// through the shared rule (one more `lstat`, the epochs' types, a no-follow
/// open), and the whole cold `read_vendor` of a three-segment census, n = 50,
/// each p50, p99 and max printed for docs/06-limits.md L-R319-04. The new
/// p50 is within 3x of the old (the ratio gate's ceiling).
#[test]
fn the_census_cold_read_is_measured() {
    let scratch = Scratch::new("BENCH");
    let root = scratch.0.as_path();
    for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
        let dir = segment_dir(root, Vendor::Gdfl, segment);
        std::fs::create_dir_all(&dir).expect("the segment");
        if segment == Segment::Fno {
            std::fs::write(dir.join("000001.man"), epoch_image(0, 40)).expect("epoch 1");
        }
    }
    let dir = segment_dir(root, Vendor::Gdfl, Segment::Fno);
    let mut before = Vec::with_capacity(200);
    let mut after = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = std::time::Instant::now();
        let old = listed_and_read_as_before(&dir);
        before.push(start.elapsed().as_nanos());
        let start = std::time::Instant::now();
        let epoch = current_epoch(&dir).expect("listed").expect("an epoch");
        let (bytes, _) = sized_epoch(&epoch_path(root, Vendor::Gdfl, Segment::Fno, epoch))
            .expect("read")
            .expect("an epoch");
        after.push(start.elapsed().as_nanos());
        assert_eq!(bytes.len(), old, "the same bytes either way");
    }
    let mut cold = Vec::with_capacity(50);
    for _ in 0..50 {
        let start = std::time::Instant::now();
        let census = read_vendor(root, Vendor::Gdfl);
        cold.push(start.elapsed().as_nanos());
        assert!(
            matches!(census.state, Census::Held { .. }),
            "{}",
            census.note()
        );
    }
    let said = |nanos: &mut [u128]| {
        let (p50, p99, max) = percentiles(nanos);
        (p50, format!("p50 {p50} ns, p99 {p99} ns, max {max} ns"))
    };
    let (old_p50, old) = said(&mut before);
    let (new_p50, new) = said(&mut after);
    let (_, whole) = said(&mut cold);
    eprintln!(
        "R3-19 api epoch listing and read, n = 200 each: before {old}; after {new}. Cold \
         read_vendor of three segments, n = 50: {whole}"
    );
    assert!(
        new_p50 <= old_p50.saturating_mul(3).max(50_000),
        "the shared rule is within the ratio gate: {new_p50} ns against {old_p50} ns"
    );
}

/// An invented single-file census image for Zerodha holding `months` daily
/// NIFTY index months from 2019-01.
fn single_image(months: u8) -> Vec<u8> {
    let mut census = Manifest::open(Vendor::Zerodha, &[], &[]).expect("a genesis census");
    for m in 1..=months {
        census
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment: Segment::Index,
                    symbol: Symbol::new("NIFTY").expect("a symbol"),
                    timeframe: Timeframe::DAY_1,
                    month: YearMonth::new(2019, m).expect("a month"),
                },
                rows: 1,
                first_ts_micros: 10,
                last_ts_micros: 10,
            })
            .expect("one month");
    }
    census.image()
}

/// **`/db`'s single-file census is never read through a link either**
/// (R3-19, closing L-AG05-05 for `manifest/<vendor>.man` as well as for the
/// epochs): a symbolic link at the single file's name, naming a valid census
/// elsewhere, is a named refusal in the store's reading door's words, nothing
/// behind it is read (the counting reader), and the link's target is never
/// served as the vendor's census. `sized_with_id` opened with `O_NONBLOCK`
/// alone, so before R3-19 the target was read and answered `Census::Held`,
/// where every census writer refuses the link by name.
#[test]
fn api_census_refuses_a_symlinked_single_file_by_name() {
    let scratch = Scratch::new("SINGLE");
    let root = scratch.0.as_path();
    let elsewhere = root.join("elsewhere-single.man");
    std::fs::write(&elsewhere, single_image(3)).expect("a census outside the store's name");
    let at = manifest_path(root, Vendor::Zerodha);
    std::fs::create_dir_all(at.parent().expect("manifest/")).expect("manifest/");
    std::os::unix::fs::symlink(&elsewhere, &at).expect("a link at the single file");
    let census = read_vendor(root, Vendor::Zerodha);
    let Census::Unreadable { reason, fault } = &census.state else {
        panic!("a link must be refused, never read: {}", census.note());
    };
    assert!(matches!(fault, Fault::Refused), "{fault:?}: {reason}");
    for word in [
        "(symlink)",
        "a symbolic link",
        "not a regular file",
        &at.display().to_string(),
    ] {
        assert!(reason.contains(word), "{word}: {reason}");
    }
    assert!(
        !reason.contains("levels of symbolic links"),
        "named as a link, not in the host's ELOOP words: {reason}"
    );
    assert!(census.is_loud(), "a refusal is loud, never an empty census");
    assert_eq!(manifest_reads::any_of(&at), 0, "nothing read at the link");
    assert_eq!(
        manifest_reads::any_of(&elsewhere),
        0,
        "nothing read behind it"
    );
    // The same bytes as a regular file at the name are the census.
    std::fs::remove_file(&at).expect("the link removed");
    let _ = std::fs::copy(&elsewhere, &at).expect("a regular file at the name");
    let census = read_vendor(root, Vendor::Zerodha);
    assert!(
        matches!(census.state, Census::Held { .. }),
        "{}",
        census.note()
    );
}

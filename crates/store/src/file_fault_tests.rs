//! The host refusals a test cannot stage on a real disk, driven through the
//! per-thread seam in `file::tests` (`arm_host_fault`): a write, a read or a
//! flush the host refuses on a file this process has just opened, measured
//! or written. Each test asserts what the refusal must leave behind, not
//! only that it is returned: nothing committed, nothing claimed, and the
//! month whole for the next writer.
//!
//! Also the guards whose answer the rest of the open makes invisible on this
//! target (`refuse_link`, `writer_open`'s `ELOOP`, `refuse_links_below`
//! outside its root, `Gathering`'s drop), called directly against their own
//! contract.
//!
//! Every bar is invented: one-second stamps of 2031-02 at a price no index
//! printed (cloud pass 2026-10-17).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::tests::{arm_host_fault, host_fault_armed};
use super::{
    Action, Appended, BarFile, GATHERED_DIRECTORIES, Gathering, StoreError, TimeLookup,
    refuse_link, refuse_links_below, writer_open,
};
use crate::format::{Bar, OI_NULL};
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::time_index::Why;

const SYMBOL: u32 = 41;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root of this test's own, cleared first and removed after.
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-file-fault-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn at(&self, file: FileKind) -> PathBuf {
        month(file).to_path_buf(&self.0)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A one-second NIFTY month of 2031-02 (GDFL, NSE INDEX).
fn month(file: FileKind) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2031, 2).unwrap(),
        file,
    })
    .unwrap()
}

/// Invented one-second bar `i` of the month, from 09:15 IST on the 1st.
fn bar(i: i64) -> Bar {
    let origin = YearMonth::new(2031, 2).unwrap().ist_bounds_micros().0;
    let close = 5_550_000 + (i * 7_919) % 613;
    Bar {
        ts_micros: origin + (555 * 60 + i) * 1_000_000,
        open: close - 11,
        high: close + 40,
        low: close - 40,
        close,
        volume: (i * 31) % 19,
        open_interest: OI_NULL,
    }
}

fn bars(from: i64, to: i64) -> Vec<Bar> {
    (from..to).map(bar).collect()
}

/// The records a fresh reader finds in the month, read back whole.
fn held(root: &Root) -> Vec<Bar> {
    let reader = BarFile::open_existing(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    (0..reader.records())
        .map(|i| reader.read_record(i).unwrap())
        .collect()
}

/// **A TORN GENESIS WHOSE REPAIR THE HOST WILL NOT WRITE IS THE HOST'S
/// REFUSAL, AND THE TORN FILE IS LEFT FOR THE NEXT OPEN TO REPAIR.**
///
/// Covers the `?` on `initialise` in `open_or_create`'s repair of an
/// interrupted genesis (D-0149): the open finds 4,096 zero bytes and no
/// header, and the zero fill that would repair them is refused (a full
/// disk). The refusal is the write's, naming the bar file; not one byte is
/// written; and the next open, with the disk back, repairs the month to an
/// empty one of the full region.
#[test]
fn a_torn_genesis_whose_repair_is_refused_is_the_writes_refusal_and_is_repaired_later() {
    let root = Root::new("genesis");
    let bars = root.at(FileKind::Bars);
    fs::create_dir_all(bars.parent().unwrap()).unwrap();
    fs::write(&bars, vec![0u8; 4_096]).unwrap();
    arm_host_fault(Action::Write, &bars, 0, ErrorKind::StorageFull);
    let refusal = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap_err();
    assert!(
        !host_fault_armed(),
        "the repair's write was the one refused"
    );
    assert_eq!(
        refusal,
        StoreError::DiskFull {
            path: bars.clone(),
            action: Action::Write
        }
    );
    assert_eq!(
        fs::read(&bars).unwrap(),
        vec![0u8; 4_096],
        "nothing written"
    );
    let repaired = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(repaired.records(), 0);
    assert_eq!(fs::metadata(&bars).unwrap().len(), 32_768);
}

/// **A CHECKSUM WRITE THE HOST REFUSES REFUSES THE APPEND BY THE SIDECAR'S
/// NAME, COMMITS NOTHING, AND THE SAME BATCH COMMITS ONCE THE DISK IS
/// BACK.**
///
/// Covers the `?` on the sidecar's write in `seal_committed`, and pins the
/// name it gives: the sidecar's. It named the bar file, which the host had
/// not refused (fixed by this pass). Three bars are committed first, so the
/// refused append re-seals a block that already has an entry: the entry is
/// unchanged (the write never reached the host), the header still counts
/// three, and the retry verifies the old tail against that entry and
/// commits all five.
#[test]
fn a_refused_checksum_write_names_the_sidecar_and_commits_nothing() {
    let root = Root::new("crc-write");
    let crc = root.at(FileKind::Checksums);
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 3)).unwrap();
    let entry = fs::read(&crc).unwrap();
    arm_host_fault(Action::Write, &crc, 0, ErrorKind::StorageFull);
    assert_eq!(
        writer.append(&bars(3, 5)),
        Err(StoreError::DiskFull {
            path: crc.clone(),
            action: Action::Write
        })
    );
    assert!(!host_fault_armed());
    assert_eq!(fs::read(&crc).unwrap(), entry, "the entry is untouched");
    assert_eq!(writer.records(), 3, "nothing committed");
    drop(writer);
    assert_eq!(held(&root), bars(0, 3));
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(
        writer.append(&bars(3, 5)),
        Ok(Appended::Committed {
            first_index: 3,
            n_valid: 5,
        })
    );
    drop(writer);
    assert_eq!(held(&root), bars(0, 5));
}

/// **A MONTH BORN WITHOUT CHECKSUMS RE-SEALS NOTHING: ITS PARTIAL TAIL IS
/// APPENDED TO WITH NO SIDECAR TO CHECK IT AGAINST.**
///
/// Covers `verify_old_tail_before_reseal`'s answer for a file whose header
/// declares no checksums (`crc_path` is `None`, the first row of its
/// table): a version-2 month written by a build before every month was born
/// sealed. Three bars leave its tail block partial, so the next append
/// reaches the check, which has no entry to protect and must not invent a
/// sidecar either.
#[test]
fn a_month_born_without_checksums_appends_past_a_partial_tail_unverified() {
    let root = Root::new("flagless");
    let bars_path = root.at(FileKind::Bars);
    fs::create_dir_all(bars_path.parent().unwrap()).unwrap();
    let layout = crate::layout::Layout::for_version(2).unwrap();
    let genesis = crate::header::Header::genesis_at(layout, SYMBOL, 1, 0);
    let commit = genesis.commit().unwrap();
    let mut region = vec![0u8; 32_768];
    let at = usize::try_from(commit.offset).unwrap();
    region[at..at + commit.bytes.len()].copy_from_slice(&commit.bytes);
    fs::write(&bars_path, &region).unwrap();
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    assert!(!writer.header().checksums_present(), "the premise");
    writer.append(&bars(0, 3)).unwrap();
    assert_eq!(
        writer.append(&bars(3, 5)),
        Ok(Appended::Committed {
            first_index: 3,
            n_valid: 5,
        })
    );
    assert!(
        !root.at(FileKind::Checksums).exists(),
        "no sidecar invented"
    );
    drop(writer);
    assert_eq!(held(&root), bars(0, 5));
}

/// **EVERY READ OF THE TIME INDEX A READER MAKES TO DECIDE IT, REFUSED, IS
/// SAID AS AN UNREADABLE INDEX, AND THE LOOKUP STILL ANSWERS.**
///
/// Covers `confirm_index`'s and `read_entry`'s arms for a read the host
/// refuses other than short (`Why::Unreadable`): the reader's header read of
/// the `.tix` and each entry read of its confirmation are refused in turn,
/// one per fresh reader. Each time the reader bisects and names the refusal
/// (the index's path, `reading`), and a lookup answers exactly as the bars
/// say. Past the last read the index is decided `Indexed`.
#[test]
fn every_refused_read_of_a_readers_index_is_an_unreadable_index() {
    let root = Root::new("tix-reader");
    let tix = root.at(FileKind::TimeIndex);
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 1_100)).unwrap();
    drop(writer);
    assert!(tix.exists(), "the premise: a month past the threshold");
    let mut refused = 0u32;
    for skip in 0.. {
        let reader = BarFile::open_existing(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
        arm_host_fault(Action::Read, &tix, skip, ErrorKind::PermissionDenied);
        let decided = reader.time_lookup();
        if host_fault_armed() {
            assert_eq!(decided, TimeLookup::Indexed, "past every read");
            break;
        }
        refused += 1;
        assert_eq!(
            decided,
            TimeLookup::Bisection(Why::Unreadable(StoreError::Denied {
                path: tix.clone(),
                action: Action::Read,
            })),
            "read {skip}"
        );
        assert_eq!(reader.first_at_or_after(bar(517).ts_micros).unwrap(), 517);
    }
    assert!(refused >= 2, "the header and at least one entry: {refused}");
    super::tests::disarm_host_fault();
}

/// **EVERY READ OF THE TIME INDEX AN APPEND MAKES, REFUSED, REFUSES THE
/// APPEND BY THE INDEX'S NAME AND COMMITS NOTHING; PAST THE LAST, THE
/// APPEND LANDS.**
///
/// Covers `read_entry`'s `Unreadable` arm on the writer's path, and
/// `index_batch`'s refusal of a resume that fails on an index it has just
/// rebuilt (`index_refused`): the writer's index is damaged under it, so
/// its next append resumes, fails the entry's checksum, rebuilds the index
/// from the bars, and resumes again. Each read of the `.tix` along that path
/// is refused in turn, on a fresh month each time. A refused read of the
/// FIRST resume is routed around like the damage it stands beside (the
/// index is rebuilt and the append lands); a refused read of the resume of
/// the index this append has just rebuilt is the host's own refusal,
/// naming the `.tix` and `reading`, and nothing is committed. Either way the
/// month afterwards holds exactly what the answer says, and its index
/// describes it.
#[test]
fn every_refused_read_of_an_appends_index_is_routed_around_or_refuses_it() {
    let (mut routed, mut refused) = (0u32, 0u32);
    for skip in 0.. {
        let root = Root::new("tix-writer");
        let tix = root.at(FileKind::TimeIndex);
        let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
        writer.append(&bars(0, 1_100)).unwrap();
        let mut bytes = fs::read(&tix).unwrap();
        let from = usize::try_from(crate::time_index::HEADER_LEN).unwrap() + 515 * 16;
        for byte in &mut bytes[from..] {
            *byte ^= 0x5A;
        }
        fs::write(&tix, &bytes).unwrap();
        arm_host_fault(Action::Read, &tix, skip, ErrorKind::PermissionDenied);
        let appended = writer.append(&bars(1_100, 1_150));
        let fired = !host_fault_armed();
        super::tests::disarm_host_fault();
        drop(writer);
        let count = match appended {
            Ok(Appended::Committed { n_valid: 1_150, .. }) => {
                routed += u32::from(fired);
                1_150
            }
            Err(StoreError::Denied { path, action }) if fired => {
                assert_eq!((path, action), (tix.clone(), Action::Read), "read {skip}");
                refused += 1;
                1_100
            }
            other => panic!("read {skip}: {other:?}"),
        };
        assert_eq!(held(&root), bars(0, count), "read {skip}");
        let indexed = BarFile::index_existing(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
        assert_eq!(
            (indexed.n_valid, indexed.outcome),
            (u64::try_from(count).unwrap(), super::Reindexed::Kept),
            "read {skip}: the index describes the month"
        );
        if !fired {
            break;
        }
    }
    assert!(
        routed >= 1 && refused >= 1,
        "{routed} routed, {refused} refused"
    );
}

/// **A DISAGREEING OVERLAP WHOSE READ THE HOST REFUSES IS REFUSED BY THAT
/// READ, NEVER DIAGNOSED FROM BARS NOT READ.**
///
/// Covers `diagnose_overlap`'s three read arms: the lookup of the batch's
/// first stamp, each held record it compares, and the held record past the
/// overlap it names as skipped. The month holds 600 bars over three blocks;
/// a reader-fresh writer is offered a batch that restates bars 0 to 4,
/// changes bar 300, and runs past the held range. Every read of the bar file
/// the refused append makes is refused in turn: each refusal is the read's,
/// naming the bar file, and past the last read the append is refused with
/// the diagnosis. Nothing is committed either way.
#[test]
fn every_refused_read_of_an_overlap_diagnosis_is_that_reads_refusal() {
    let root = Root::new("overlap");
    let bars_path = root.at(FileKind::Bars);
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 600)).unwrap();
    drop(writer);
    let mut offered = bars(0, 650);
    offered[300].close += 1;
    offered[300].high += 1;
    let mut denied = 0u32;
    for skip in 0.. {
        let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
        arm_host_fault(Action::Read, &bars_path, skip, ErrorKind::PermissionDenied);
        let appended = writer.append(&offered);
        if host_fault_armed() {
            super::tests::disarm_host_fault();
            assert!(
                matches!(appended, Err(StoreError::OverlapDisagrees { at: 300, .. })),
                "past every read: {appended:?}"
            );
            break;
        }
        denied += 1;
        assert_eq!(
            appended,
            Err(StoreError::Denied {
                path: bars_path.clone(),
                action: Action::Read,
            }),
            "read {skip}"
        );
    }
    assert!(denied >= 3, "the diagnosis read across blocks: {denied}");
    assert_eq!(held(&root), bars(0, 600));
}

/// **A SKIPPED HELD BAR WHOSE READ THE HOST REFUSES IS THAT READ'S
/// REFUSAL.**
///
/// Covers `diagnose_overlap`'s last read: a batch that restates the first
/// two held bars, skips the third and every one after, and runs past the
/// held range. The overlap matches as far as it goes, so the diagnosis reads
/// the next held bar to name it as skipped; that read refused is the
/// refusal returned.
#[test]
fn a_refused_read_of_the_skipped_held_bar_is_that_reads_refusal() {
    let root = Root::new("skipped");
    let bars_path = root.at(FileKind::Bars);
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 400)).unwrap();
    drop(writer);
    let mut offered = bars(0, 2);
    offered.extend(bars(500, 510));
    let mut outcomes = Vec::new();
    for skip in 0.. {
        let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
        arm_host_fault(Action::Read, &bars_path, skip, ErrorKind::PermissionDenied);
        let appended = writer.append(&offered);
        if host_fault_armed() {
            super::tests::disarm_host_fault();
            assert!(
                matches!(appended, Err(StoreError::OverlapDisagrees { at: 2, .. })),
                "past every read: {appended:?}"
            );
            break;
        }
        outcomes.push(appended);
    }
    assert!(!outcomes.is_empty());
    for (skip, appended) in outcomes.into_iter().enumerate() {
        assert_eq!(
            appended,
            Err(StoreError::Denied {
                path: bars_path.clone(),
                action: Action::Read,
            }),
            "read {skip}"
        );
    }
    assert_eq!(held(&root), bars(0, 400));
}

/// **A NAME THAT IS A SYMBOLIC LINK IS REFUSED BY `refuse_link` ITSELF.**
///
/// Kills `replace refuse_link -> Result<(), StoreError> with Ok(())` and
/// `replace match guard meta.file_type().is_symlink() with false`: on this
/// target every caller's next step is an `O_NOFOLLOW` open that refuses the
/// same link with the same `Symlinked`, so through the doors the guard is
/// invisible; its own contract is that it refuses a link (live or dangling)
/// and passes a file, a directory and an absent name.
#[test]
fn refuse_link_refuses_a_link_and_passes_everything_else() {
    let root = Root::new("refuse-link");
    let file = root.0.join("file");
    fs::write(&file, b"x").unwrap();
    let live = root.0.join("live");
    std::os::unix::fs::symlink(&file, &live).unwrap();
    let dangling = root.0.join("dangling");
    std::os::unix::fs::symlink(root.0.join("nowhere"), &dangling).unwrap();
    for link in [&live, &dangling] {
        assert_eq!(
            refuse_link(link),
            Err(StoreError::Symlinked { path: link.clone() })
        );
    }
    for not_a_link in [&file, &root.0, &root.0.join("absent")] {
        assert_eq!(refuse_link(not_a_link), Ok(()), "{}", not_a_link.display());
    }
}

/// **AN OPEN REFUSED `ELOOP` IS A SYMBOLIC LINK, NAMED; ANY OTHER REFUSAL
/// IS WHAT `classify` SAYS.**
///
/// Covers `writer_open`'s `ELOOP` arm, which on this target only a link
/// swapped in between `refuse_link`'s look and the open reaches.
#[test]
fn a_writer_open_refused_eloop_is_a_symbolic_link() {
    let path = Path::new("/brutex/never/opened.bin");
    let eloop = io::Error::from_raw_os_error(crate::open_flags::ELOOP);
    assert_eq!(
        writer_open(Err(eloop), path).unwrap_err(),
        StoreError::Symlinked {
            path: path.to_path_buf()
        }
    );
    assert_eq!(
        writer_open(Err(io::Error::from(ErrorKind::PermissionDenied)), path).unwrap_err(),
        StoreError::Denied {
            path: path.to_path_buf(),
            action: Action::Open,
        }
    );
}

/// **A LEAF OUTSIDE THE ROOT HAS NO COMPONENT BELOW IT TO REFUSE.**
///
/// Covers `refuse_links_below`'s answer for a leaf the root is not a prefix
/// of: every door passes a leaf under its root, and the function's contract
/// is to look only below the root, never at the operator's own path above
/// it, so a link outside is not its to refuse.
#[test]
fn a_leaf_outside_the_root_has_nothing_below_it_to_refuse() {
    let root = Root::new("below");
    let outside = Root::new("outside");
    let link = outside.0.join("link");
    std::os::unix::fs::symlink(&root.0, &link).unwrap();
    assert_eq!(refuse_links_below(&root.0, &link.join("leaf")), Ok(()));
    assert_eq!(
        refuse_links_below(&outside.0, &link.join("leaf")),
        Err(StoreError::Symlinked { path: link.clone() }),
        "the same leaf under its own root is refused"
    );
}

/// **A GATHERING DROPPED UNFINISHED PUTS BACK WHAT WAS GATHERING BEFORE
/// IT.**
///
/// Kills `replace <impl Drop for Gathering>::drop with ()` and `delete !` in
/// it: `with_directories_flushed_once` finishes every gathering on its `Ok`
/// and `Err` paths, so only an open unwinding from a panic drops one
/// unfinished, and then the thread must not keep gathering for an open that
/// is gone. Asserted for an outer gathering in progress (its list is put
/// back, with what it held) and for none (nothing is gathering after), and a
/// finished gathering's drop changes nothing more.
#[test]
fn a_gathering_dropped_unfinished_restores_the_state_before_it() {
    let outer = Gathering::begin();
    GATHERED_DIRECTORIES.with(|cell| {
        cell.borrow_mut()
            .as_mut()
            .unwrap()
            .push(PathBuf::from("/outer"));
    });
    let inner = Gathering::begin();
    GATHERED_DIRECTORIES.with(|cell| {
        cell.borrow_mut()
            .as_mut()
            .unwrap()
            .push(PathBuf::from("/inner"));
    });
    drop(inner);
    assert_eq!(
        GATHERED_DIRECTORIES.with(|cell| cell.borrow().clone()),
        Some(vec![PathBuf::from("/outer")]),
        "the outer gathering is back, with what it held"
    );
    assert_eq!(outer.finish(), vec![PathBuf::from("/outer")]);
    assert_eq!(
        GATHERED_DIRECTORIES.with(|cell| cell.borrow().clone()),
        None
    );
    drop(Gathering::begin());
    assert_eq!(
        GATHERED_DIRECTORIES.with(|cell| cell.borrow().clone()),
        None,
        "nothing gathers after an unfinished one with none before it"
    );
}

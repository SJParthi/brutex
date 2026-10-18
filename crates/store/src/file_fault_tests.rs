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

use super::tests::{arm_host_fault, host_fault_armed, host_fault_on};
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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

/// Runs `body` on a worker with a deadline of `secs`, and answers what it
/// answered. A body that does not return in time fails the test there,
/// rather than hanging the suite with every other test that holds the sink:
/// a mutant of the overlap's partition makes an append call itself for
/// ever. The file seam is per thread, so a body that arms it arms it on the
/// worker that makes the calls.
fn within<T: Send + 'static>(secs: u64, body: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, answered) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _gone = done.send(body());
    });
    answered
        .recv_timeout(std::time::Duration::from_secs(secs))
        .expect("the body returns within its deadline")
}

/// Every append of `offered` to the month at `root` with one of its
/// bar-file reads refused, the first read, then the second, and so on until
/// a refusal no longer fires: each answer, and whether its refusal fired.
/// One worker and one deadline for the whole walk.
fn appends_with_each_read_refused(
    root: &Path,
    offered: Vec<Bar>,
) -> Vec<(Result<Appended, StoreError>, bool)> {
    let root = root.to_path_buf();
    within(30, move || {
        let bars_path = month(FileKind::Bars).to_path_buf(&root);
        let mut answers = Vec::new();
        for skip in 0.. {
            let mut writer = BarFile::open_or_create(&root, month(FileKind::Bars), SYMBOL).unwrap();
            arm_host_fault(Action::Read, &bars_path, skip, ErrorKind::PermissionDenied);
            let appended = writer.append(&offered);
            let fired = !host_fault_armed();
            super::tests::disarm_host_fault();
            answers.push((appended, fired));
            if !fired {
                break;
            }
        }
        answers
    })
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
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("overlap");
    let bars_path = root.at(FileKind::Bars);
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 600)).unwrap();
    drop(writer);
    let mut offered = bars(0, 650);
    offered[300].close += 1;
    offered[300].high += 1;
    let mut answers = appends_with_each_read_refused(&root.0, offered);
    let (last, fired) = answers.pop().unwrap();
    assert!(!fired, "the walk ends past every read");
    assert!(
        matches!(last, Err(StoreError::OverlapDisagrees { at: 300, .. })),
        "past every read: {last:?}"
    );
    assert!(answers.len() >= 3, "the diagnosis read across blocks");
    for (skip, (appended, _)) in answers.into_iter().enumerate() {
        assert_eq!(
            appended,
            Err(StoreError::Denied {
                path: bars_path.clone(),
                action: Action::Read,
            }),
            "read {skip}"
        );
    }
    assert_eq!(held(&root), bars(0, 600));
}

/// **A SKIPPED HELD BAR WHOSE READ THE HOST REFUSES IS THAT READ'S
/// REFUSAL.**
///
/// Covers `diagnose_overlap`'s last read: a batch that restates the first
/// block of held bars (rows 0 to 72), skips row 73 and every one after, and
/// runs past the held range. The overlap matches as far as it goes, so the
/// diagnosis reads the next held bar to name it as skipped. That bar is the
/// first of the next block, so its read is a read of the disk, not of the
/// block the handle verified last; refused, it is the refusal returned.
#[test]
fn a_refused_read_of_the_skipped_held_bar_is_that_reads_refusal() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("skipped");
    let bars_path = root.at(FileKind::Bars);
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 400)).unwrap();
    drop(writer);
    let mut offered = bars(0, 73);
    offered.extend(bars(500, 510));
    let mut answers = appends_with_each_read_refused(&root.0, offered);
    let (last, fired) = answers.pop().unwrap();
    assert!(!fired, "the walk ends past every read");
    assert!(
        matches!(last, Err(StoreError::OverlapDisagrees { at: 73, .. })),
        "past every read: {last:?}"
    );
    assert!(!answers.is_empty());
    for (skip, (appended, _)) in answers.into_iter().enumerate() {
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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
    let _sink_is_mine = crate::emits::hold_the_sink();
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

/// A month of 1,100 one-second bars whose writer made its `.srt` and
/// `.tix`, closed.
fn sorted_month(tag: &str) -> Root {
    let root = Root::new(tag);
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 1_100)).unwrap();
    drop(writer);
    assert!(root.at(FileKind::SortIndex).exists(), "the premise");
    root
}

fn sort(root: &Root) -> Result<super::SortIndexed, StoreError> {
    BarFile::sort_index_existing(&root.0, month(FileKind::Bars), SYMBOL)
}

/// **EVERY READ OF A WHOLE SORT INDEX THE DOOR MAKES, REFUSED, IS EITHER A
/// WALK THAT FAILED (THE INDEX IS WRITTEN WHOLE AGAIN) OR A BODY THAT COULD
/// NOT BE CHECKED (THE DOOR IS REFUSED BY THE INDEX'S NAME, AND NOTHING IS
/// WRITTEN).**
///
/// Covers `damaged`'s refusal of a body read the host refused
/// (`verify_bodies`' `Why::Unreadable`), and the walk's own refused read:
/// the door over a month whose `.srt` is one whole run reads the run's
/// headers, then its body once. Each of those reads is refused in turn. A
/// refused walk cannot say what the index holds, so the door writes the
/// month whole (`Built`) and the index it leaves is kept by the next run; a
/// refused body read is never taken for a whole body (the door would answer
/// `Kept` over bytes nobody read) nor for a damaged one (it would rewrite an
/// index it could not judge): it refuses, naming the index, and the index
/// is byte for byte what it was.
#[test]
fn every_refused_read_of_a_whole_sort_index_rebuilds_or_refuses_by_name() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = sorted_month("srt-reads");
    let srt = root.at(FileKind::SortIndex);
    let (mut built, mut refused) = (0u32, 0u32);
    for skip in 0.. {
        let before = fs::read(&srt).unwrap();
        arm_host_fault(Action::Read, &srt, skip, ErrorKind::PermissionDenied);
        let answer = sort(&root);
        let fired = !host_fault_armed();
        super::tests::disarm_host_fault();
        match answer {
            Ok(sorted) if !fired => {
                assert_eq!(sorted.outcome, super::Resorted::Kept, "past every read");
                break;
            }
            Ok(sorted) => {
                assert!(
                    matches!(&sorted.outcome, super::Resorted::Built(_)),
                    "read {skip}: {sorted:?}"
                );
                assert_eq!((sorted.n_valid, sorted.runs), (1_100, 1), "read {skip}");
                assert_eq!(
                    sort(&root).unwrap().outcome,
                    super::Resorted::Kept,
                    "read {skip}: what it wrote is whole"
                );
                built += 1;
            }
            Err(StoreError::SortIndex { path, reason }) => {
                assert_eq!(path, srt, "read {skip}");
                assert!(
                    reason.starts_with(&format!("{}: ", srt.display()))
                        && reason.contains("refused by the file test seam"),
                    "read {skip}: {reason}"
                );
                assert_eq!(fs::read(&srt).unwrap(), before, "read {skip}: untouched");
                refused += 1;
            }
            Err(other) => panic!("read {skip}: {other:?}"),
        }
    }
    assert!(
        built >= 1 && refused >= 1,
        "{built} built, {refused} refused"
    );
}

/// **A LOOK AT THE SORT INDEX OR AT ITS STAGING NAME THE HOST REFUSES
/// REFUSES THE DOOR BY THAT NAME, AND NOTHING IS REMOVED OR WRITTEN.**
///
/// Kills `replace match guard why.kind() == ErrorKind::NotFound with true`
/// in `walk_sidecar` and in `clear_staging`, and covers both refusals: any
/// refusal of the `lstat` would be taken for "nothing there", so the door
/// would rebuild an index it could not see, or call a staging name it could
/// not see cleared. The month's own directory refuses a look only when it
/// changes between two calls of one function, so the file test seam
/// refuses it (`EACCES`).
#[test]
fn a_refused_look_at_the_sort_index_or_its_staging_name_refuses_the_door() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = sorted_month("srt-looks");
    let srt = root.at(FileKind::SortIndex);
    let staging = srt.with_extension("new.srt");
    fs::write(&staging, b"a dead build's").unwrap();
    let before = fs::read(&srt).unwrap();
    for looked in [&srt, &staging] {
        arm_host_fault(Action::Measure, looked, 0, ErrorKind::PermissionDenied);
        let answer = sort(&root);
        assert!(!host_fault_armed(), "{}", looked.display());
        match answer {
            Err(StoreError::SortIndex { path, reason }) => {
                assert_eq!(path, srt);
                assert!(
                    reason.starts_with(&format!("{}: ", looked.display())),
                    "{reason}"
                );
            }
            other => panic!("{}: {other:?}", looked.display()),
        }
        assert_eq!(fs::read(&srt).unwrap(), before, "the index is untouched");
        assert_eq!(
            fs::read(&staging).unwrap(),
            b"a dead build's",
            "not removed"
        );
    }
    let kept = sort(&root).unwrap();
    assert_eq!(kept.outcome, super::Resorted::Kept);
    assert!(!staging.exists(), "the next door clears the staging name");
}

/// **A MONTH WHOSE BAR IS OFF ITS RUNG'S GRID IS UNSORTABLE, SAID, AND NO
/// INDEX IS WRITTEN FOR IT.**
///
/// Covers the door's `Unsortable` answer, which only a bar file written
/// outside this crate's writer reaches: the writer refuses an off-grid bar
/// at append. Bar 500's stamp is moved one microsecond on disk and its
/// block's checksum sealed again over the moved bytes, so the month reads
/// clean and every bar verifies; the index is removed, so the door must
/// build one, and the build refuses the bar (a one-second rung's stamps are
/// whole seconds). Nothing is written in its place.
#[test]
fn a_month_with_a_bar_off_its_grid_is_unsortable_and_nothing_is_written() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = sorted_month("srt-off-grid");
    let bars_path = root.at(FileKind::Bars);
    let crc_path = root.at(FileKind::Checksums);
    let layout = crate::layout::Layout::CURRENT;
    let mut image = fs::read(&bars_path).unwrap();
    let at = 32_768 + 56 * 500;
    let moved = i64::from_le_bytes(image[at..at + 8].try_into().unwrap()) + 1;
    image[at..at + 8].copy_from_slice(&moved.to_le_bytes());
    let block = 500 / 73;
    let (start, end) = layout.covered_byte_range(block, 1_100).unwrap();
    let (start, end) = (
        usize::try_from(start).unwrap(),
        usize::try_from(end).unwrap(),
    );
    let sum = crate::block::seal(layout, 1_100, block, &image[start..end]).unwrap();
    let mut sums = fs::read(&crc_path).unwrap();
    let entry = usize::try_from(block * 4).unwrap();
    sums[entry..entry + 4].copy_from_slice(&sum.to_le_bytes());
    fs::write(&bars_path, &image).unwrap();
    fs::write(&crc_path, &sums).unwrap();
    let srt = root.at(FileKind::SortIndex);
    fs::remove_file(&srt).unwrap();
    let reader = BarFile::open_existing(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(
        reader.read_record(500).unwrap().ts_micros,
        moved,
        "it reads clean"
    );
    drop(reader);
    let answer = sort(&root).unwrap();
    assert_eq!((answer.n_valid, answer.runs), (1_100, 0));
    assert!(
        matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
        "{answer:?}"
    );
    assert!(!srt.exists(), "no index written");
    assert!(
        !srt.with_extension("new.srt").exists(),
        "nor its staging name"
    );
}

/// **THE DIRECTORIES AN OPEN WOULD MAKE ARE THOSE STRICTLY BELOW THE ROOT,
/// AND NONE OUTSIDE IT.**
///
/// Kills `replace && with ||` in `missing_below`'s walk
/// (`here != root && here.starts_with(root)`): with `||` the walk would
/// count the root itself when it is missing, and any missing directory of a
/// path outside the root. Every open checks its root before it asks, so
/// through the doors the two agree; the function's own contract is the
/// directories between the root (exclusive) and the leaf (inclusive),
/// outermost first, and a leaf outside the root has none.
#[test]
fn the_missing_directories_are_strictly_below_the_root() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let held = Root::new("missing-below");
    let root = held.0.join("root");
    let leaf = root.join("a").join("b");
    assert_eq!(
        super::missing_below(&root, &leaf),
        vec![root.join("a"), leaf.clone()],
        "a missing root is not counted"
    );
    fs::create_dir_all(root.join("a")).unwrap();
    assert_eq!(super::missing_below(&root, &leaf), vec![leaf.clone()]);
    let outside = held.0.join("elsewhere").join("c");
    assert_eq!(
        super::missing_below(&root, &outside),
        Vec::<PathBuf>::new(),
        "nothing outside the root"
    );
    assert_eq!(super::missing_below(&root, &root), Vec::<PathBuf>::new());
}

/// **INSIDE A DURABLE BATCH, A REFUSED MEASURE OF A FILE THE BATCH DEFERS
/// REFUSES THE STEP THAT DEFERRED IT, BY THAT FILE'S NAME, AND A REFUSED
/// APPEND IS NEVER CONFIRMED AFTERWARDS.**
///
/// Covers the `?` on `DurableBatch::defer_file` in `initialise`, in
/// `sync_index`, in the sort index's sync and in `barrier_in`: under
/// `OncePerDevice` a deferred file is measured for its device first, and the
/// host refusing that `fstat` of a file it has just opened is not
/// stageable, so the file seam refuses it (`EACCES`). Every measure of the
/// bar file, the checksum sidecar, the time index and the sort index that a
/// first open and a 1,100-bar append inside one batch make is refused in
/// turn, on a fresh store each time. Each answer is either the open's or the
/// append's refusal, naming the file and `measuring`, or a commit (the sort
/// index's step never refuses its caller, and some measures are of steps
/// that route around them). After a refused append, this process opens the
/// month again only to be refused `barrier_failed` (when the refusal was a
/// barrier's: the header slot may be on disk, and D-1907 forbids confirming
/// it) or to find nothing committed; never to find the 1,100 bars it was
/// told were refused, which is what a batch's refused barrier did before
/// this pass. After a commit, all 1,100 are there.
#[test]
fn a_refused_measure_of_a_deferred_file_refuses_the_step_that_deferred_it() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for kind in [
        FileKind::Bars,
        FileKind::Checksums,
        FileKind::TimeIndex,
        FileKind::SortIndex,
    ] {
        let (mut refused, mut barred) = (0u32, 0u32);
        for skip in 0.. {
            let root = Root::new("batch-measure");
            let target = root.at(kind);
            let batch = super::DurableBatch::new(super::BatchFlush::OncePerDevice);
            let answer = {
                let _in = batch.enter();
                arm_host_fault(Action::Measure, &target, skip, ErrorKind::PermissionDenied);
                BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL)
                    .and_then(|mut writer| writer.append(&bars(0, 1_100)))
            };
            let fired = !host_fault_armed();
            super::tests::disarm_host_fault();
            let _durable = batch.make_durable(1);
            let reopened = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL);
            match answer {
                Ok(Appended::Committed { n_valid: 1_100, .. }) => {
                    assert_eq!(reopened.map(|month| month.records()), Ok(1_100));
                }
                Err(StoreError::Denied { path, action }) if fired => {
                    assert_eq!((path, action), (target.clone(), Action::Measure));
                    refused += 1;
                    let mut month = reopened
                        .unwrap_or_else(|why| panic!("{kind:?} measure {skip}, reopened: {why}"));
                    let on_disk = month.records();
                    match month.append(&bars(1_100, 1_101)) {
                        Err(StoreError::BarrierFailed { path }) => {
                            assert_eq!(path, root.at(FileKind::Bars), "{kind:?} {skip}");
                            barred += 1;
                        }
                        next => assert_eq!(
                            (on_disk, next),
                            (
                                0,
                                Ok(Appended::Committed {
                                    first_index: 0,
                                    n_valid: 1,
                                })
                            ),
                            "{kind:?} measure {skip}: a refused commit is never confirmed"
                        ),
                    }
                }
                other => panic!("{kind:?} measure {skip}: {other:?}"),
            }
            if !fired {
                break;
            }
        }
        match kind {
            FileKind::SortIndex => assert_eq!(refused, 0, "the sort step never refuses"),
            FileKind::Bars => assert!(barred >= 2, "both bar-file barriers: {barred}"),
            FileKind::Checksums => assert_eq!((refused, barred), (1, 1)),
            _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
        }
    }
}

/// **A SEALED BATCH DEFERS NOTHING: A WRITER INSIDE IT FLUSHES ITS OWN
/// FILES, AND THE BATCH HAS NOTHING LEFT TO MAKE DURABLE.**
///
/// A batch already made durable is sealed, and `DurableBatch::current`
/// answers no batch for it, so a month first opened and filled past the
/// threshold inside it is made durable by its own writer, and a second
/// `make_durable` flushes nothing. The sealed answer of `defer_file` itself
/// needs a batch sealed while a writer holds it: the test below.
#[test]
fn a_sealed_batch_defers_nothing_and_the_writer_flushes_its_own() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("sealed-batch");
    let batch = super::DurableBatch::new(super::BatchFlush::OncePerDevice);
    let first = batch.make_durable(1).unwrap();
    assert_eq!((first.files, first.directories), (0, 0));
    {
        let _in = batch.enter();
        let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
        assert_eq!(
            writer.append(&bars(0, 1_100)),
            Ok(Appended::Committed {
                first_index: 0,
                n_valid: 1_100,
            })
        );
    }
    let again = batch.make_durable(1).unwrap();
    assert_eq!((again.files, again.directories), (0, 0), "nothing deferred");
    assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
    assert_eq!(held(&root), bars(0, 1_100));
}

/// **A BATCH SEALED UNDER A WRITER TAKES NO MORE OF ITS FLUSHES: THE
/// WRITER FLUSHES EACH ONE ITSELF.**
///
/// Covers the `false` answer of `DurableBatch::defer_file` at each writer
/// call site. The batch is sealed by an actor between the open's capture of
/// it and the genesis' flush, so the genesis, the time index's sync at the
/// threshold, the sort index's sync and the append's barrier all meet a
/// sealed batch. Each is flushed by the writer, so the batch's second
/// `make_durable` has nothing left, and the time index counts its own flush.
#[test]
fn a_batch_sealed_under_a_writer_leaves_every_flush_to_it() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("sealed-under");
    let batch = super::DurableBatch::new(super::BatchFlush::OncePerDevice);
    let sealer = batch.clone();
    super::tests::arm_before_genesis_flush(move || {
        let _made = sealer.make_durable(1).expect("sealed");
    });
    let _ = super::tests::index_flushes();
    {
        let _in = batch.enter();
        let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
        assert_eq!(
            writer.append(&bars(0, 1_100)),
            Ok(Appended::Committed {
                first_index: 0,
                n_valid: 1_100,
            })
        );
    }
    assert!(super::tests::index_flushes() > 0, "the writer's own flush");
    let again = batch.make_durable(1).unwrap();
    assert_eq!((again.files, again.directories), (0, 0), "nothing deferred");
    assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
    assert_eq!(held(&root), bars(0, 1_100));
}

/// **A RESUMED BATCH THAT OVERLAPS THE HELD TAIL APPENDS ONLY ITS SUFFIX,
/// AND RETURNS.**
///
/// Kills `replace <= with > in BarFile::suffix_that_follows`' partition
/// (caught before only by a timeout): with it, the whole batch is taken for
/// the suffix, and the append that follows meets the same overlap and calls
/// itself for ever. The append runs on a worker with a deadline, so the
/// mutant fails here at once rather than hanging the suite. A month of 100
/// bars is offered bars 90 to 129: the ten held bars are verified and
/// dropped, and only the thirty new ones are committed.
#[test]
fn a_resumed_overlap_appends_only_its_suffix_and_returns() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("resume-suffix");
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 100)).unwrap();
    let (appended, writer) = within(10, move || (writer.append(&bars(90, 130)), writer));
    assert_eq!(
        appended,
        Ok(Appended::Committed {
            first_index: 100,
            n_valid: 130,
        })
    );
    drop(writer);
    assert_eq!(held(&root), bars(0, 130));
}

/// **A REFUSAL ARMED FOR A NAME THAT IS NOT THERE FIRES ON NO HANDLE.**
///
/// Covers the seam's own arm for a name it cannot measure: a handle is
/// matched by device and inode, and a missing name has neither, so the
/// handle's read goes through and the refusal keeps waiting. Armed by the
/// handle's own name, the same read is refused.
#[test]
fn a_refusal_armed_for_a_missing_name_fires_on_no_handle() {
    let root = Root::new("missing-name");
    let real = root.0.join("real");
    fs::write(&real, b"bytes").unwrap();
    let file = fs::File::open(&real).unwrap();
    arm_host_fault(
        Action::Read,
        &root.0.join("never-made"),
        0,
        ErrorKind::Other,
    );
    assert!(host_fault_on(Action::Read, &file).is_none());
    assert!(host_fault_armed(), "still waiting");
    arm_host_fault(Action::Read, &real, 0, ErrorKind::Other);
    assert_eq!(
        host_fault_on(Action::Read, &file).map(|refused| refused.kind()),
        Some(ErrorKind::Other)
    );
    assert!(!host_fault_armed(), "fired once");
}

/// **A SMALL MONTH WHOSE INDEX NAME CANNOT BE LOOKED AT IS NOT TAKEN FOR ONE
/// WITH NO INDEX.**
///
/// Covers the small-month rule's guard in `index_on_open`: only a look that
/// says the name is not there gives a month under the threshold no `.tix`.
/// A look the host refuses otherwise goes down the path that names what is
/// there. Here the open that follows succeeds, so the index is built as
/// absent, and the next run keeps it.
#[test]
fn a_small_months_refused_index_look_is_not_the_small_month_rule() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("small-look");
    let mut writer = BarFile::open_or_create(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    writer.append(&bars(0, 100)).unwrap();
    drop(writer);
    let tix = root.at(FileKind::TimeIndex);
    assert!(!tix.exists(), "the rule gave it none");
    arm_host_fault(Action::Measure, &tix, 0, ErrorKind::PermissionDenied);
    let indexed = BarFile::index_existing(&root.0, month(FileKind::Bars), SYMBOL).unwrap();
    assert!(!host_fault_armed(), "the look was refused");
    assert_eq!(indexed.outcome, super::Reindexed::Built(Why::Absent));
    assert!(tix.exists(), "built whole");
    assert_eq!(
        BarFile::index_existing(&root.0, month(FileKind::Bars), SYMBOL)
            .unwrap()
            .outcome,
        super::Reindexed::Kept
    );
}

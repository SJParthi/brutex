//! The step that makes a new month's **name** durable: `store::durability::*`.
//!
//! # The step this file is about
//!
//! `BarFile::open_or_create` on a month that does not exist yet writes the
//! header region, `fsync`s the file, and then `fsync`s the **directory**. The
//! last of those three is the one nobody thinks about: without it the bars can
//! be on stable storage inside a file the directory does not yet mention after
//! a crash, which is the same as not having them.
//!
//! Every other test of this crate drives that step down its success path, so
//! the refusal it carries had never run. A refusal that has never run is a
//! refusal nobody has read since it was written, and this one guards the one
//! failure that is invisible afterwards.
//!
//! # How the failure is arranged, and what it assumes
//!
//! A directory with the write and execute bits and **not** the read bit
//! accepts a file being created inside it and refuses to be opened. That is
//! exactly the gap between "the bars were written" and "the directory flush
//! succeeded", and it needs no full disk and no injected fault.
//!
//! **UNIX ONLY, and it assumes this process is not root.** Root bypasses the
//! permission bits, would open the directory anyway, and the test would fail
//! on its own assertion rather than pass silently. CI and the operator's
//! machine both run as an ordinary user.
//!
//! # What this file does NOT prove
//!
//! That an `fsync` reached the platter. Nothing in this repository measures
//! that, and `docs/06-limits.md` says so. What is proven here is that when the
//! host refuses the flush, the refusal is **returned and named** rather than
//! swallowed — which is the part this crate controls.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[cfg(unix)]
mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Action, Appended, BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// Distinguishes two scratch trees taken in the same process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary directory that removes itself.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!("{}-{tag}-{serial}", std::process::id()));
        fs::create_dir_all(&root).expect("a scratch root");
        Self { root }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Best effort: a leaked scratch directory must never fail a test run.
        drop(fs::remove_dir_all(&self.root));
    }
}

/// The month under test.
fn bars_path() -> StorePath<'static> {
    month_path(6)
}

/// A month of 2024 in the same directory as [`bars_path`].
fn month_path(month: u8) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, month).expect("a month of 2024"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

/// A directory that will not open refuses the create, in the host's own words.
///
/// The bars file and the header region are written first — the failure is the
/// directory flush that publishes the file's NAME, and it is returned rather
/// than treated as best effort.
#[cfg(unix)]
#[test]
fn a_directory_flush_the_host_refuses_is_returned_and_named() {
    support::where_permission_binds(
        "a_directory_flush_the_host_refuses_is_returned_and_named",
        a_directory_flush_the_host_refuses_is_returned_and_named_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
#[cfg(unix)]
fn a_directory_flush_the_host_refuses_is_returned_and_named_body() {
    use std::os::unix::fs::PermissionsExt as _;

    let scratch = Scratch::new("NOFLUSH");
    let path = bars_path();
    let file = path.to_path_buf(&scratch.root);
    let dir = file
        .parent()
        .expect("a rendered store path always has parents")
        .to_path_buf();
    fs::create_dir_all(&dir).expect("the month's directory");

    // Write and execute, and NOT read: a file may be created inside, and the
    // directory itself may not be opened.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o300)).expect("close the directory");
    let refused = BarFile::open_or_create(&scratch.root, path, 7);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("reopen the directory");

    let refused = refused.expect_err(
        "the directory cannot be opened, so its flush cannot be issued, so the \
         new file's name is not durable and this must not report success",
    );
    assert_eq!(
        refused,
        StoreError::Denied {
            path: dir.clone(),
            action: Action::Open,
        },
        "refused by name, carrying the DIRECTORY and the operation — not the \
         bar file, which opened fine, and not a generic I/O error"
    );

    let text = refused.to_string();
    assert!(
        text.contains(&dir.display().to_string()),
        "the refusal names the directory an operator has to fix — {text}"
    );

    // The same month with the directory open succeeds, so the refusal is about
    // the permission bits and not about the fixture.
    let opened = BarFile::open_or_create(&scratch.root, bars_path(), 7)
        .expect("the same month, with the directory readable");
    assert_eq!(opened.records(), 0, "a fresh month holds no records");
}

/// The flush that makes a new SIDECAR's name durable is returned when the host
/// refuses it, and only a month with nothing committed asks for it. D-0688.
///
/// `sync_all` on the `.crc` makes its bytes durable and not its directory
/// entry, so the writer's door flushes the month's directory whenever it opens
/// a month whose header commits nothing — the only state in which that door
/// may have just created the sidecar. Lose the entry after the first commit and
/// the month is sealed, holds records, and has no `.crc`, which no door will
/// ever recreate.
///
/// Both months are made first, with the directory readable, so the bar file's
/// own creation flush above is not the one this refuses: June is initialised
/// and empty, and July beside it holds three bars. With the directory closed
/// to reading, June's writer open is refused naming the DIRECTORY, and July's
/// succeeds, because a month with committed records neither created a sidecar
/// on this open nor asks for the flush.
///
/// This proves the flush is ISSUED and its refusal returned. That it reached
/// stable storage is not observable here, as `docs/06-limits.md` says.
#[cfg(unix)]
#[test]
fn a_sidecar_flush_the_host_refuses_is_returned_and_only_an_empty_month_asks_for_one() {
    support::where_permission_binds(
        "a_sidecar_flush_the_host_refuses_is_returned_and_only_an_empty_month_asks_for_one",
        a_sidecar_flush_the_host_refuses_is_returned_and_only_an_empty_month_asks_for_one_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
#[cfg(unix)]
fn a_sidecar_flush_the_host_refuses_is_returned_and_only_an_empty_month_asks_for_one_body() {
    use std::os::unix::fs::PermissionsExt as _;

    let scratch = Scratch::new("NOSIDECARFLUSH");
    let dir = bars_path()
        .to_path_buf(&scratch.root)
        .parent()
        .expect("a rendered store path always has parents")
        .to_path_buf();

    let empty = BarFile::open_or_create(&scratch.root, bars_path(), 7).expect("an empty June");
    assert_eq!(empty.records(), 0, "the premise: June commits nothing");
    drop(empty);
    let sidecar = bars_path()
        .with_file(FileKind::Checksums)
        .to_path_buf(&scratch.root);
    assert!(
        sidecar.is_file(),
        "the premise: June's sidecar exists, so this open will not create it"
    );
    let mut held = BarFile::open_or_create(&scratch.root, month_path(7), 7).expect("a July");
    let bars: Vec<Bar> = (0..3i64)
        .map(|minute| Bar {
            ts_micros: 1_719_805_500_000_000 + minute * 60_000_000,
            open: 2_400_000,
            high: 2_400_500,
            low: 2_399_500,
            close: 2_400_100,
            volume: 1_000,
            open_interest: OI_NULL,
        })
        .collect();
    assert_eq!(
        held.append(&bars),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 3
        }),
        "the premise: July commits three bars"
    );
    drop(held);

    // Write and execute, and NOT read.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o300)).expect("close the directory");
    let refused = BarFile::open_or_create(&scratch.root, bars_path(), 7).map(|file| file.records());
    let committed =
        BarFile::open_or_create(&scratch.root, month_path(7), 7).map(|file| file.records());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("reopen the directory");

    assert_eq!(
        refused,
        Err(StoreError::Denied {
            path: dir.clone(),
            action: Action::Open,
        }),
        "an empty month's writer open flushes the directory, and a refused \
         flush is returned naming the directory rather than reported as success"
    );
    assert_eq!(
        committed,
        Ok(3),
        "a month with committed records asks for no flush, so the closed \
         directory does not refuse it"
    );

    // The permission bits, and not the fixture, were the refusal.
    let reopened = BarFile::open_or_create(&scratch.root, bars_path(), 7)
        .expect("the same month, with the directory readable");
    assert_eq!(reopened.records(), 0);
}

/// The directory flush FOLLOWS the sidecar's creation, so it is the flush that
/// publishes the new `.crc`'s name rather than one issued before the name
/// existed. D-0688.
///
/// AF-41 says the writer's door flushes the month's directory after opening
/// the sidecar, and its test cannot tell the order: June's `.crc` exists there
/// already, so a flush moved ahead of the open is refused in the same words,
/// and every test of this crate still passed with it moved. A flush ahead of
/// the open publishes a directory the new name is not in yet, which leaves
/// unguarded exactly the crash this flush exists for.
///
/// Here June is initialised and empty and its `.crc` is removed, which is the
/// one state in which the writer's door may create the sidecar. With the
/// directory closed to reading, the open is refused naming the directory, as
/// in the test above, and the `.crc` EXISTS afterwards and is empty: the create
/// ran, and then the flush that was to publish it was refused. A flush ahead of
/// the open is refused before any create and leaves no `.crc`.
///
/// This proves the order of the two calls. That the flush reached stable
/// storage is not observable here, as `docs/06-limits.md` says.
#[cfg(unix)]
#[test]
fn the_sidecar_flush_follows_the_sidecars_creation_so_a_refused_flush_leaves_the_new_name() {
    support::where_permission_binds(
        "the_sidecar_flush_follows_the_sidecars_creation_so_a_refused_flush_leaves_the_new_name",
        the_sidecar_flush_follows_the_sidecars_creation_so_a_refused_flush_leaves_the_new_name_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
#[cfg(unix)]
fn the_sidecar_flush_follows_the_sidecars_creation_so_a_refused_flush_leaves_the_new_name_body() {
    use std::os::unix::fs::PermissionsExt as _;

    let scratch = Scratch::new("SIDECARORDER");
    let dir = bars_path()
        .to_path_buf(&scratch.root)
        .parent()
        .expect("a rendered store path always has parents")
        .to_path_buf();
    let sidecar = bars_path()
        .with_file(FileKind::Checksums)
        .to_path_buf(&scratch.root);

    let empty = BarFile::open_or_create(&scratch.root, bars_path(), 7).expect("an empty June");
    assert_eq!(empty.records(), 0, "the premise: June commits nothing");
    drop(empty);
    fs::remove_file(&sidecar).expect("the premise: June's sidecar is there to remove");
    assert!(
        !sidecar.exists(),
        "the premise: June has no sidecar, so the next writer open creates one"
    );

    // Write and execute, and NOT read: the `.crc` may be created inside, and
    // the directory itself may not be opened for its flush.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o300)).expect("close the directory");
    let refused = BarFile::open_or_create(&scratch.root, bars_path(), 7).map(|file| file.records());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("reopen the directory");

    assert_eq!(
        refused,
        Err(StoreError::Denied {
            path: dir.clone(),
            action: Action::Open,
        }),
        "the flush is refused naming the directory"
    );
    assert_eq!(
        fs::metadata(&sidecar)
            .map(|meta| (meta.is_file(), meta.len()))
            .ok(),
        Some((true, 0)),
        "the refused flush came AFTER the sidecar's create: the new `.crc` is \
         on disk, empty, and its name is what the flush was issued to publish"
    );

    // The permission bits, and not the fixture, were the refusal; the sidecar
    // the refused open created is the one this open takes.
    let reopened = BarFile::open_or_create(&scratch.root, bars_path(), 7)
        .expect("the same month, with the directory readable");
    assert_eq!(reopened.records(), 0);
}

/// D-0149 — a month interrupted inside `initialise` opens, at every size the
/// interruption can leave behind.
///
/// # The window
///
/// `initialise` is a 32,768-byte zero fill, then a 64-byte header, then one
/// sync. A process that dies between the two writes leaves a file with bytes
/// and no header. The repair condition was `len == 0`, so every one of those
/// sizes was skipped, `validated` found no header slot, and the month refused
/// to open for good — §3 rule 8 forbids rewriting it.
///
/// The likeliest crash point is the cruellest: after the fill and before the
/// header the file is exactly `REGION_LEN`, the same size a healthy empty month
/// has.
#[test]
fn a_month_interrupted_between_the_zero_fill_and_the_header_still_opens() {
    // Every size the interruption can leave: the first byte, a partial fill,
    // one short of the region, and the whole region with no header.
    for len in [1_u64, 64, 4_096, 16_384, 20_000, 32_767, 32_768] {
        let scratch = Scratch::new(&format!("TORN{len}"));
        let path = bars_path();
        let on_disk = path.to_path_buf(&scratch.root);
        let dir = on_disk
            .parent()
            .expect("a month has a parent")
            .to_path_buf();
        fs::create_dir_all(&dir).expect("the month directory");

        // The state a crash inside `initialise` leaves: zeroes, no header.
        fs::write(&on_disk, vec![0u8; usize::try_from(len).expect("fits")]).expect("the torn file");
        assert_eq!(
            fs::metadata(&on_disk).expect("measurable").len(),
            len,
            "the fixture must be exactly the torn size"
        );

        let opened = BarFile::open_or_create(&scratch.root, bars_path(), 7).unwrap_or_else(|why| {
            panic!(
                "a torn month of {len} bytes must be repaired, not \
                                          refused forever: {why}"
            )
        });

        // Repaired to a healthy EMPTY month -- not to something that pretends
        // to hold bars. The whole argument for repairing rather than refusing
        // is that this file provably held nothing.
        assert_eq!(
            opened.header().n_valid,
            0,
            "{len}: repaired months hold no bars"
        );
        assert_eq!(
            fs::metadata(&on_disk).expect("measurable").len(),
            32_768,
            "{len}: the repair writes the full region"
        );
    }
}

/// And the restraint: a file that has something to lose still refuses.
///
/// "No valid header" is NOT the repair condition, deliberately. A month holding
/// real records with a damaged header must refuse loudly — that one has data,
/// and §3 rule 8 outranks getting it open.
#[test]
fn a_month_with_bytes_that_are_not_zero_is_refused_rather_than_reinitialised() {
    let scratch = Scratch::new("NOTZERO");
    let path = bars_path();
    let on_disk = path.to_path_buf(&scratch.root);
    let dir = on_disk.parent().expect("a parent").to_path_buf();
    fs::create_dir_all(&dir).expect("the month directory");

    // Inside the region, so length alone would admit it -- but one byte is not
    // zero, so this is not an interrupted fill and might be anything.
    let mut damaged = vec![0u8; 4_096];
    damaged[2_048] = 0x01;
    fs::write(&on_disk, &damaged).expect("the damaged file");

    let refused = BarFile::open_or_create(&scratch.root, bars_path(), 7);
    assert!(
        refused.is_err(),
        "a file holding a byte this build did not write must not be silently \
         re-initialised -- that would be the §4 fallback that hides a failure"
    );
    assert_eq!(
        fs::metadata(&on_disk).expect("measurable").len(),
        4_096,
        "and the refusal leaves the file exactly as it was found"
    );
}

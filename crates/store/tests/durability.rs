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

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Action, BarFile, StoreError};
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
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("June 2024"),
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

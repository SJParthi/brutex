//! Every `telemetry::emit` this crate owns, driven through the production call
//! that reaches it and read back off the disk.
//!
//! # What was unproven
//!
//! This crate holds six emit sites and, until this module, not one of them was
//! asserted to reach a file. Each could have been deleted outright — the whole
//! body replaced with `()` — and `cargo test`, `cargo clippy` and the mutation
//! gate would all have stayed green, because nothing anywhere read the bytes
//! the sink writes. An observation nothing observes is worth what an untested
//! branch is worth, which is what `CLAUDE.md` §4's ban on a test that asserts
//! nothing says in the other direction.
//!
//! It is worse than an ordinary coverage hole. Five of these six fire only on
//! a refusal — a header region of zeros, a commit walked back a generation, a
//! block whose bytes are not the bytes that were sealed — so the run that needs
//! them is the run nobody can repeat afterwards. A line that was never proved
//! to be written is not evidence.
//!
//! # Why the emits are driven and never built
//!
//! Every drive below calls the **public** function an operator's process calls:
//! [`Header::read_region`], [`Header::commit`], [`crate::block::verify`],
//! [`BarFile::append`]. None of them constructs a `telemetry::Event`. That
//! distinction is the whole point of the module: elsewhere in this workspace a
//! set of tests fabricated their own `Event` with the target of a production
//! site, on a sink they opened themselves, and asserted it landed — which
//! proves the sink works and says **nothing** about whether the production call
//! still emits. Deleting the emit left those tests green.
//!
//! # Why one test and not six
//!
//! `telemetry::install` writes a process-wide `OnceLock` and *refuses* a second
//! call, so a test binary gets exactly one sink. Six tests would race for it
//! and five would lose. One test, one install, one table — and because the
//! table also fixes how many records the file may hold, an emit that fires on a
//! path that should be silent fails it just as loudly as one that stopped
//! firing.
//!
//! The floor is `Trace`, not the default `Info`, because `store.append` emits
//! at `Debug`: at the default floor that row would be filtered and the test
//! would prove the opposite of what it claims.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;

use crate::block;
use crate::file::{Appended, BarFile};
use crate::format::{Bar, FLAG_CHECKSUMS, FormatError, HEADER_LEN, OI_NULL, RECORD_LEN};
use crate::header::{Commit, Header};
use crate::layout::Layout;
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// [`HEADER_LEN`] as a `usize`, for sizing a region in memory.
///
/// Written as a literal in both widths rather than converted, the same way
/// `crate::header` writes the slot stride, so there is no fallible conversion
/// whose failure arm no test could reach. The assertion keeps the two honest.
const HEADER_BYTES: usize = 32_768;
const _: () = assert!(HEADER_LEN == 32_768 && HEADER_BYTES == 32_768);

/// The symbol id every drive here opens and commits under.
const SYMBOL: u32 = 26_000;

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// The open of the first one-minute bar of 2024-06-03, in microseconds.
const T0: i64 = 1_717_386_300_000_000;

/// One production emit site, and the smallest production call that reaches it.
///
/// The level is part of the row on purpose. Each site's doc comment argues for
/// the level it chose — `Warn` for a fall-back because no byte is wrong,
/// `Error` for a refusal, `Debug` for the per-member write path — and an
/// argument nothing checks is a comment. Recording the level here means a
/// silent promotion of the write path to `Info`, which would put a line in the
/// file for every bar of every backfill, fails a test.
struct Site {
    /// The subsystem the event must name.
    target: &'static str,
    /// The sentence the event must carry.
    message: &'static str,
    /// How loud it must be.
    level: telemetry::Level,
    /// The production call that reaches it, given a scratch store root.
    drive: fn(&Path),
}

/// A header region whose every slot is zeros: `note_header_unreadable`.
///
/// A zeroed slot fails the magic check, which is the *unspecific* refusal, so
/// no slot offers a reason and the search ends on `NoValidHeader` — the
/// "every copy of the header is damaged" answer an operator cannot otherwise
/// tell apart from a month that was never pulled.
fn drive_header_unreadable(_root: &Path) {
    let region = vec![0u8; HEADER_BYTES];
    let refused = Header::read_region(&region, HEADER_LEN)
        .expect_err("a region of zeros holds no committed header");
    assert_eq!(refused, FormatError::NoValidHeader);
}

/// A newest slot the file's length cannot support: `note_header_fell_back`.
///
/// This is the fourth row of `crate::header`'s crash table, built rather than
/// simulated: generation 0 commits an empty file, generation 1 commits a
/// counter of 1000 records, and the region handed to the reader is the header
/// alone — 32768 bytes, capacity zero. Generation 1 is whole and unsupported,
/// so `read_region` walks back to generation 0 instead of condemning the file,
/// which is exactly the recovery that used to happen in silence.
fn drive_header_fell_back(_root: &Path) {
    let genesis = Header::genesis(SYMBOL, 60, 0);
    let unsupported = genesis
        .advance(1_000, T0, T0 + MINUTE)
        .expect("advancing by 1000 records is arithmetically fine");

    let mut region = vec![0u8; HEADER_BYTES];
    place(&mut region, &genesis.commit().expect("genesis commits"));
    place(
        &mut region,
        &unsupported.commit().expect("generation 1 commits"),
    );

    let read = Header::read_region(&region, HEADER_LEN)
        .expect("the previous generation is intact and is returned");
    assert_eq!(read.generation, 0, "the newest commit is not the one used");
    assert_eq!(read.n_valid, 0, "and its counter is the one that came back");
}

/// A counter whose data end is past `u64`: `note_commit_refused`.
///
/// [`Header::commit`] refuses three ways — an unknown version, a stride its
/// version does not define, and this one — and all three reach the single emit
/// site through `commit_image`. The offset is the arm reachable without
/// hand-building a header state no constructor produces.
fn drive_commit_refused(_root: &Path) {
    let past_the_end = Header::genesis(SYMBOL, 60, 0)
        .advance(u64::MAX, T0, T0)
        .expect("a counter of u64::MAX is arithmetically fine; its offset is not");
    let refused = past_the_end
        .commit()
        .expect_err("the end of the data is past u64");
    assert_eq!(refused, FormatError::OffsetOverflow);
}

/// A verification asked of a file carrying no checksums: `note_unverifiable`.
///
/// No writer in this workspace sets [`FLAG_CHECKSUMS`] — `docs/04-invariants.md`
/// S-06 and S-06b — so the operator who first trips this is the one running the
/// first build that turns checksums on, against files every build before it
/// wrote. Their file is not corrupt; it predates the flag, and the line is the
/// only place that distinction survives.
fn drive_block_unverifiable(_root: &Path) {
    let plain = Header::genesis(SYMBOL, 60, 0)
        .advance(1, T0, T0)
        .expect("one record commits");
    assert!(!plain.checksums_present(), "the premise: no checksum flag");
    let refused = block::verify(&plain, Layout::CURRENT, 0, &[7u8; RECORD_LEN], 0)
        .expect_err("there is nothing to verify against");
    assert_eq!(refused, FormatError::ChecksumsAbsent);
}

/// A block whose committed bytes are not the bytes sealed: `note_block_mismatch`.
///
/// The lost write `crate::block` exists for. The zeros below are what a newly
/// allocated extent reads back as, and an all-zero [`Bar`] satisfies
/// `ohlc_is_sane` with a *real* zero open interest rather than [`OI_NULL`], so
/// nothing downstream can tell that extent from data. Only the checksum can.
fn drive_block_mismatch(_root: &Path) {
    let sealed_header = Header::genesis(SYMBOL, 60, FLAG_CHECKSUMS)
        .advance(1, T0, T0)
        .expect("one record commits");
    let written = [7u8; RECORD_LEN];
    let sealed = block::seal(Layout::CURRENT, sealed_header.n_valid, 0, &written)
        .expect("the covered length is one record");

    let lost = [0u8; RECORD_LEN];
    let refused = block::verify(&sealed_header, Layout::CURRENT, 0, &lost, sealed)
        .expect_err("a newly allocated extent is not the bytes that were sealed");
    assert!(
        matches!(refused, FormatError::BlockChecksum { block: 0, .. }),
        "the refusal names the block, got {refused:?}"
    );
}

/// A batch that reached stable storage: the `store.append` emit in
/// `crate::file`.
///
/// The only one of the six on a success path, and the only one that needs a
/// real filesystem: it fires after the second `sync_all`, so the line cannot
/// claim a durability the file does not have. Two bars, one commit, one event.
fn drive_append_committed(root: &Path) {
    let mut file = BarFile::open_or_create(root, bars_path(), SYMBOL).expect("a fresh month opens");
    let landed = file
        .append(&[bar(0), bar(1)])
        .expect("two consecutive bars commit");
    assert_eq!(
        landed,
        Appended::Committed {
            first_index: 0,
            n_valid: 2,
        },
        "the premise: the bars were written, not recognised as already present"
    );
}

/// Copies one commit's 64 bytes into the region at the offset it names.
///
/// Written as a zipped walk rather than a slice assignment for the reason
/// `crate::header::covered` gives: this workspace denies slice indexing, and a
/// header region is exactly where a panicking index would eventually be reached.
fn place(region: &mut [u8], commit: &Commit) {
    let at = usize::try_from(commit.offset).expect("a slot offset fits a usize");
    for (dst, src) in region.iter_mut().skip(at).zip(commit.bytes) {
        *dst = src;
    }
}

/// The `i`-th one-minute bar of the session, in paisa.
fn bar(index: i64) -> Bar {
    Bar {
        ts_micros: T0 + index * MINUTE,
        open: 2_345_600 + index,
        high: 2_345_900 + index,
        low: 2_345_100 + index,
        close: 2_345_700 + index,
        volume: 1_000 + index,
        open_interest: OI_NULL,
    }
}

/// The one month every drive that touches a disk writes to.
fn bars_path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("2024-06"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

/// A temporary path no other live process will name.
///
/// The process id is not a random number and is not meant to be: it is unique
/// among *live* processes, which is exactly the set that can collide. Two
/// concurrent `cargo test` runs over one checkout would otherwise delete each
/// other's sink directory mid-assertion.
fn scratch(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("brutex-store-emits-{tag}-{}", std::process::id()))
}

/// EVERY EMIT IN THIS CRATE REACHES A FILE, through the call that owns it.
///
/// Six production sites, six production calls, one sink, and one read of the
/// bytes on disk. Deleting any one of the six emits fails this test; so does
/// changing a target, a sentence or a level, and so does adding a seventh emit
/// on a path this table already drives.
///
/// # The count is an assertion, not a formality
///
/// `assert_eq!(records.len(), SITES.len())` is what pins the **absence** half.
/// Five of these six sites sit beside a success path that is documented to be
/// silent — the ordinary header read, the commit that succeeds, the block that
/// verifies — and a per-file or per-record emit added there would not fail any
/// presence check. It fails this one. `drive_append_committed` alone opens a
/// month, initialises a 32 KiB header, reads it back and commits, and the table
/// says that whole sequence is worth exactly one line.
///
/// # Why this test may install the global
///
/// `telemetry::install` writes a per-process `OnceLock` and refuses a second
/// call. No other test in `crates/store` installs, opens or reads a sink, and
/// no production path in this crate installs one either — the store emits into
/// whatever its host installed. So there is nothing here to race, and the
/// install is asserted rather than skipped on failure: a test that quietly
/// measures somebody else's sink is worse than one that does not run.
#[test]
fn every_emit_in_this_crate_reaches_the_log_through_its_production_call() {
    const SITES: [Site; 6] = [
        Site {
            target: "store.header",
            message: "no committed header",
            level: telemetry::Level::Error,
            drive: drive_header_unreadable,
        },
        Site {
            target: "store.header",
            message: "fell back to an older generation",
            level: telemetry::Level::Warn,
            drive: drive_header_fell_back,
        },
        Site {
            target: "store.header",
            message: "commit refused",
            level: telemetry::Level::Error,
            drive: drive_commit_refused,
        },
        Site {
            target: "store.block",
            message: "no checksums to verify against",
            level: telemetry::Level::Warn,
            drive: drive_block_unverifiable,
        },
        Site {
            target: "store.block",
            message: "checksum mismatch",
            level: telemetry::Level::Error,
            drive: drive_block_mismatch,
        },
        Site {
            target: "store.append",
            message: "committed",
            level: telemetry::Level::Debug,
            drive: drive_append_committed,
        },
    ];

    let dir = scratch("log");
    let root = scratch("root");
    let _ignored = std::fs::remove_dir_all(&dir);
    let _ignored = std::fs::remove_dir_all(&root);

    // `Trace`, so the floor is not what decides the outcome. `store.append`
    // emits at `Debug` and would be filtered by the default `Info` floor,
    // which would leave this test asserting that a filtered event is absent
    // while claiming it proved the emit.
    let sink =
        telemetry::install(&telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace))
            .expect("nothing else in this test binary installs a sink");

    for site in &SITES {
        (site.drive)(&root);
    }

    let found = telemetry::tail(&dir, sink.keep_files(), &telemetry::Query::last(64));
    assert_eq!(
        found.records.len(),
        SITES.len(),
        "six drives, six lines — no site went silent and no silent path spoke. \
         The file held: {:?}",
        found
            .records
            .iter()
            .map(|line| (line.target.clone(), line.message.clone()))
            .collect::<Vec<_>>()
    );

    for site in &SITES {
        let line = found
            .records
            .iter()
            .find(|line| line.target == site.target && line.message == site.message)
            .unwrap_or_else(|| {
                panic!(
                    "THE EVENT NEVER REACHED THE FILE. Without this line the \
                     emit at {} / \"{}\" is deletable and every gate stays green.",
                    site.target, site.message
                )
            });
        assert_eq!(
            line.level, site.level,
            "{} / \"{}\" changed how loud it is, and each site's doc comment \
             argues for the level it chose",
            site.target, site.message
        );
    }

    let _ignored = std::fs::remove_dir_all(&dir);
    let _ignored = std::fs::remove_dir_all(&root);
}

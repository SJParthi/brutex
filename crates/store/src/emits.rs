//! Every `telemetry::emit` this crate owns, driven through the production call
//! that reaches it and read back off the disk.
//!
//! # What was unproven
//!
//! This crate holds eleven emit sites and, until this module, not one of them
//! was asserted to reach a file. Each could have been deleted outright — the
//! whole body replaced with `()` — and `cargo test`, `cargo clippy` and the
//! mutation gate would all have stayed green, because nothing anywhere read
//! the bytes the sink writes. An observation nothing observes is worth what an
//! untested branch is worth, which is what `CLAUDE.md` §4's ban on a test that
//! asserts nothing says in the other direction.
//!
//! It is worse than an ordinary coverage hole. Ten of these eleven fire only
//! once something has already gone wrong — a header region of zeros, a commit
//! walked back a generation, a block whose bytes are not the bytes that were
//! sealed, an append that died before its header slot reached the disk, a lock
//! whose unlock the host refused, a month with no usable time index — so the
//! run that needs them is the run nobody can repeat afterwards. A line that was
//! never proved to be written is not evidence.
//!
//! Six of those ten are not refusals. `store.open` **accepts** the month and
//! names the damage. Since D-0688 the `store.block` interrupted-append line
//! accepts a tail block whose entry was sealed past the commit and names the
//! append that died. The `store.header` fall-back returns an older committed
//! generation's header and names the newer one it rejected. Since D-2329 the
//! two `store.tix` lines name a month whose time lookup bisects and a month
//! whose time index a writer rebuilt from its bars. Those five hand
//! back working data, so the line is the only trace there is. The sixth is
//! `store.flock` (D-0693): it fires from a `Drop`, which cannot hand anybody
//! anything. Those are the six sites in this crate whose deletion is
//! invisible from outside the log. The other four hand the caller a
//! `FormatError` as well as writing a line, so a lost emit still leaves a
//! trace somewhere: the unreadable header, the refused commit, the block with
//! no checksums to verify against and the checksum mismatch. The test counts
//! what each drive's call handed back and holds this paragraph to the counts.
//!
//! # Why the emits are driven and never built
//!
//! Every drive below calls the **public** function an operator's process calls:
//! [`Header::read_region`], [`Header::commit`], [`crate::block::verify`],
//! [`BarFile::open_or_create`], [`BarFile::append`]. None of them constructs a
//! `telemetry::Event`. That
//! distinction is the whole point of the module: elsewhere in this workspace a
//! set of tests fabricated their own `Event` with the target of a production
//! site, on a sink they opened themselves, and asserted it landed — which
//! proves the sink works and says **nothing** about whether the production call
//! still emits. Deleting the emit left those tests green.
//!
//! # Why one test and not nine
//!
//! `telemetry::install` writes a process-wide `OnceLock` and *refuses* a second
//! call, so a test binary gets exactly one sink. Nine tests would race for it
//! and eight would lose. One test, one install, one table — and because the
//! table also fixes how many records the file may hold, an emit that fires on a
//! path that should be silent fails it just as loudly as one that stopped
//! firing.
//!
//! The floor is `Trace`, not the default `Info`, because `store.append` emits
//! at `Debug`: at the default floor that row would be filtered and the test
//! would prove the opposite of what it claims.
//!
//! # One sink is one sink for the WHOLE binary, not just for this module
//!
//! The paragraph above stops one line short of the consequence. A crate's unit
//! tests are one process on as many threads as the machine has, so the sink
//! installed here is also the sink of every test running beside it — and
//! `crate::file` has two that drive real commits and real reopens through the
//! same production calls this module drives. Their records land in this
//! module's file, between its install and its read, and the count assertion
//! counted them. [`hold_the_sink`] carries the measurements and the lock that
//! makes the two windows disjoint.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use brutex_core::vendor::Vendor;

use crate::block;
use crate::file::{Appended, BarFile};
use crate::flock::Flock;
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

/// Exclusive use of the process-wide sink, for as long as the guard is held.
///
/// **Every test in this binary that can reach a `telemetry::emit` must take
/// this**, including the ones that never look at the log. Emitting is the
/// thing that has to be serialised; reading is only where it is noticed.
///
/// # The defect it closes, which was live and intermittent
///
/// `telemetry::install` writes a process-wide `OnceLock`, and `cargo test`
/// runs a crate's unit tests as one process on N threads. The sink the test
/// below installs is therefore shared with whatever else is running, and
/// `crate::file` has exactly two tests that reach a production emit:
///
/// * `a_torn_tail_past_the_commit_counter_opens_the_month_rather_than_bricking_it`
///   commits twice and reopens a file with seventeen bytes past its counter
///   twice — two `store.append` records and two `store.open` records;
/// * `a_re_pull_from_the_middle_of_a_month_is_already_present_not_a_conflict`
///   commits twice — two more `store.append` records.
///
/// Six records belonging to other tests, arriving in this module's file
/// between its install and its read, in whatever subset the scheduler happened
/// to allow. Five consecutive runs of the unchanged crate on one machine gave
/// **12, 8, 11, 12 and 12** records against six sites; the run that first
/// reported it gave 10. `--test-threads=1` gave six every time, and that is
/// what named the cause rather than a guess: the tests sort `crc` < `emits` <
/// `file`, so serially this module reads its file before either of those two
/// has started.
///
/// The extra rows were correct emits from correct code. The test was wrong,
/// not the crate, and it had been wrong since the second of those tests was
/// written — it only started failing when the first one was added.
///
/// # Why a lock, and not any of the three easier fixes
///
/// *Not a `>=` or a `contains`.* The exact count is the only thing this test
/// proves that a presence check does not: that a path documented to be silent
/// stayed silent. Loosening it would leave a green suite proving less than the
/// red one did.
///
/// *Not a filter on the tail.* `telemetry::Query` selects by target, by level
/// or by run id, and not one of the three separates those records from these:
/// the targets and levels are identical because they come from the same sites,
/// and this crate stamps no run id, so every record carries run zero.
///
/// *Not moving the two tests out of the binary.* They exercise
/// `BarFile::validated` directly — the private door both public ones funnel
/// into, reached with no directory tree and no advisory lock — and an
/// integration test cannot name a private function.
///
/// So the windows are made not to overlap instead. The test below holds this
/// across the install, the drives and the read; each test over there holds it
/// for its whole body.
///
/// # Why a poisoned lock is taken anyway
///
/// It guards a window in time, not a data structure, so a test that panicked
/// while holding it left nothing half-built for the next one to trip over.
/// Propagating the poison would report two failures for one defect, and the
/// second would name this module rather than the test that actually broke.
///
/// No `#[must_use]`: `MutexGuard` already carries one, and a caller who drops
/// this on the floor unlocks it on the same line, which is the mistake the
/// attribute on the guard type is there to catch.
pub(crate) fn hold_the_sink() -> MutexGuard<'static, ()> {
    static PROCESS_SINK: Mutex<()> = Mutex::new(());
    PROCESS_SINK.lock().unwrap_or_else(PoisonError::into_inner)
}

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
    /// The production call that reaches it, given a scratch store root, and
    /// what that call handed back to its caller.
    drive: fn(&Path) -> Handed,
}

/// What the production call a drive makes handed back to its caller.
///
/// Each drive returns this only after its own `expect` or `expect_err` has
/// checked the call's result, so the tally the test takes of it is what the
/// calls did, not what a row declares. The module header counts these, and
/// the test holds the header to the tally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Handed {
    /// A `FormatError`: the caller hears of the damage whether or not the
    /// line reaches a log.
    Refusal,
    /// Working data despite the damage the line names: the line is the only
    /// trace of it.
    Data,
    /// Nothing at all: the emit fires from a `Drop`.
    Nothing,
    /// Nothing went wrong: the line records a success.
    Success,
}

/// `n` as the header spells it, for the eleven sites and the tallies of them.
fn spelt(n: usize) -> &'static str {
    [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven",
    ]
    .get(n)
    .copied()
    .unwrap_or_else(|| panic!("the header spells no count above eleven, got {n}"))
}

/// [`spelt`] opening a sentence.
fn spelt_first(n: usize) -> String {
    let word = spelt(n);
    word.get(..1).map_or_else(String::new, str::to_uppercase) + word.get(1..).unwrap_or("")
}

/// This module's `//!` header as one line of prose: markers removed, lines
/// joined with a space, so a sentence wrapped across lines reads whole.
fn module_header() -> String {
    include_str!("emits.rs")
        .lines()
        .map_while(|line| line.strip_prefix("//!"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A header region whose every slot is zeros: `note_header_unreadable`.
///
/// A zeroed slot fails the magic check, which is the *unspecific* refusal, so
/// no slot offers a reason and the search ends on `NoValidHeader` — the
/// "every copy of the header is damaged" answer an operator cannot otherwise
/// tell apart from a month that was never pulled.
fn drive_header_unreadable(_root: &Path) -> Handed {
    let region = vec![0u8; HEADER_BYTES];
    let refused = Header::read_region(&region, HEADER_LEN)
        .expect_err("a region of zeros holds no committed header");
    assert_eq!(refused, FormatError::NoValidHeader);
    Handed::Refusal
}

/// A newest slot the file's length cannot support: `note_header_fell_back`.
///
/// This is the fourth row of `crate::header`'s crash table, built rather than
/// simulated: generation 0 commits an empty file, generation 1 commits a
/// counter of 1000 records, and the region handed to the reader is the header
/// alone — 32768 bytes, capacity zero. Generation 1 is whole and unsupported,
/// so `read_region` walks back to generation 0 instead of condemning the file,
/// which is exactly the recovery that used to happen in silence.
fn drive_header_fell_back(_root: &Path) -> Handed {
    // Sealed: version 3 refuses to commit a slot without the flag (D-1571).
    let genesis = Header::genesis(SYMBOL, 60, FLAG_CHECKSUMS);
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
    Handed::Data
}

/// A counter whose data end is past `u64`: `note_commit_refused`.
///
/// [`Header::commit`] refuses three ways — an unknown version, a stride its
/// version does not define, and this one — and all three reach the single emit
/// site through `commit_image`. The offset is the arm reachable without
/// hand-building a header state no constructor produces.
fn drive_commit_refused(_root: &Path) -> Handed {
    let past_the_end = Header::genesis(SYMBOL, 60, FLAG_CHECKSUMS)
        .advance(u64::MAX, T0, T0)
        .expect("a counter of u64::MAX is arithmetically fine; its offset is not");
    let refused = past_the_end
        .commit()
        .expect_err("the end of the data is past u64");
    assert_eq!(refused, FormatError::OffsetOverflow);
    Handed::Refusal
}

/// A verification asked of a file carrying no checksums: `note_unverifiable`.
///
/// `store::file::initialise` sets [`FLAG_CHECKSUMS`] on every month it
/// creates, so only a direct `block::verify` of a flag-clear header — a month
/// an earlier build wrote — trips this; `docs/04-invariants.md` S-06b,
/// corrected by D-1448. That file is not corrupt; it predates the flag, and the
/// line is the only place that distinction survives.
fn drive_block_unverifiable(_root: &Path) -> Handed {
    let plain = Header::genesis(SYMBOL, 60, 0)
        .advance(1, T0, T0)
        .expect("one record commits");
    assert!(!plain.checksums_present(), "the premise: no checksum flag");
    let refused = block::verify(&plain, Layout::CURRENT, 0, &[7u8; RECORD_LEN], 0)
        .expect_err("there is nothing to verify against");
    assert_eq!(refused, FormatError::ChecksumsAbsent);
    Handed::Refusal
}

/// A block whose committed bytes are not the bytes sealed: `note_block_mismatch`.
///
/// The lost write `crate::block` exists for. The zeros below are what a newly
/// allocated extent reads back as, and an all-zero [`Bar`] satisfies
/// `ohlc_is_sane` with a *real* zero open interest rather than [`OI_NULL`], so
/// nothing downstream can tell that extent from data. Only the checksum can.
fn drive_block_mismatch(_root: &Path) -> Handed {
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
    Handed::Refusal
}

/// A tail block whose entry an interrupted append sealed past the commit:
/// `note_interrupted_append`. D-0688.
///
/// One of the three sites in this crate that **accept** rather than refuse,
/// and so one whose deletion is invisible outside the log: the block is
/// served, and this line is the only trace of the append that died between
/// sealing the sidecar and writing its header slot. Driven through the public
/// [`block::verify_through`] with bytes laid out here, so it reaches this site
/// and no neighbour — a real file would also fire `store.append` and
/// `store.open`, which this table already drives once each.
fn drive_block_sealed_past_the_commit(_root: &Path) -> Handed {
    let header = Header::genesis(SYMBOL, 60, FLAG_CHECKSUMS)
        .advance(1, T0, T0)
        .expect("one record commits");
    let committed = [7u8; RECORD_LEN];
    let past = [8u8; RECORD_LEN];
    let mut sealed_over_two = committed.to_vec();
    sealed_over_two.extend_from_slice(&past);
    let through = block::verify_through(
        &header,
        Layout::CURRENT,
        0,
        &committed,
        &past,
        crate::crc::crc32c(&sealed_over_two),
    )
    .expect("the entry is proved to cover the committed record and one more");
    assert_eq!(through, 2, "the proof names the extent it matched");
    Handed::Data
}

/// A batch that reached stable storage: the `store.append` emit in
/// `crate::file`.
///
/// The only one of the nine that fires where nothing whatever went wrong, and
/// one of the three that need a real filesystem — [`drive_open_ragged_tail`] and
/// [`drive_flock_unlock_refused`] are the others. It fires after the second
/// `sync_all`, so the line cannot claim a durability the file does not have.
/// Two bars, one commit, one event.
fn drive_append_committed(root: &Path) -> Handed {
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
    Handed::Success
}

/// A month whose last bytes no commit claims: the `store.open` emit in
/// `crate::file`.
///
/// The interrupted append `docs/02-store-format.md` §7 describes, built on a
/// real disk rather than simulated, through the same public door
/// [`drive_append_committed`] uses. It is the only site here that fires on a
/// file the caller then goes on to **use**: the month opens, every committed
/// bar in it is readable, and the seventeen bytes are ignored. Until the fix
/// this reports, that file was `StoreError::RaggedTail` and the whole month was
/// unreachable to every process, permanently, because `CLAUDE.md` §3 rule 8
/// forbids rewriting it to clear them.
///
/// Seventeen bytes and not fifty-six: a whole record past the counter and a
/// torn one reach the same line by the same arithmetic, but only the torn one
/// is impossible to mistake for a record somebody forgot to commit.
///
/// # Why the counter stays at zero
///
/// Committing a bar first would paint a fuller picture of the crash and would
/// also fire `store.append` — a **second** record for a site this table already
/// drives exactly once — and the count below is the half of the test that
/// proves the silent paths stayed silent. A drive that fires a neighbour's site
/// breaks that count as surely as a stray production emit does, and it breaks
/// it in a way that reads like a crate bug.
///
/// Nothing is lost by leaving it empty. `bytes_past_the_counter` is measured
/// from `offset_of(n_valid)`, and with `n_valid` zero that is the end of the
/// 32,768-byte header region: every one of the seventeen bytes is past it, the
/// discarded count is seventeen, and the branch under test is the same branch.
///
/// # Why the handle is dropped before the reopen
///
/// The exclusive advisory lock lives in the [`BarFile`], not in the scope, so
/// reopening with the first handle alive would be `StoreError::Locked` and
/// never reach the open this drive exists to reach.
fn drive_open_ragged_tail(root: &Path) -> Handed {
    let fresh = BarFile::open_or_create(root, bars_path(), SYMBOL).expect("a fresh month opens");
    assert_eq!(fresh.records(), 0, "the premise: nothing is committed yet");
    let bars = fresh.path().to_path_buf();
    drop(fresh);

    let mut torn = std::fs::OpenOptions::new()
        .append(true)
        .open(&bars)
        .expect("the month this drive just created is there");
    torn.write_all(&[9u8; 17])
        .expect("seventeen bytes of a record whose header slot never arrived");
    drop(torn);

    let reopened = BarFile::open_or_create(root, bars_path(), SYMBOL)
        .expect("a month is not lost to bytes no commit claims");
    assert_eq!(reopened.records(), 0, "the counter is untouched");
    assert_eq!(
        std::fs::metadata(&bars).expect("a length").len(),
        HEADER_LEN + 17,
        "the premise: seventeen bytes sit past the committed extent"
    );
    Handed::Data
}

/// A guard dropped without a release, whose unlock the host refused:
/// `note_unreleased` in `crate::flock`. D-0693.
///
/// The one site in this crate that fires from a `Drop`, so nothing but the log
/// ever hears of it. The refusal is injected through the per-thread seam in
/// `crate::flock::tests`, because no real file refuses an unlock on request;
/// everything after the injection is the production `Drop`. The descriptor
/// then closes, and with no duplicate of it open the kernel frees the lock, so
/// the drive leaves nothing held behind it.
fn drive_flock_unlock_refused(root: &Path) -> Handed {
    std::fs::create_dir_all(root).expect("a scratch store root");
    let path = root.join("flock.lock");
    let file = std::fs::File::create(&path).expect("the lock file");
    let held = Flock::lock(file, path).expect("an uncontended lock is taken");
    crate::flock::tests::refuse_next_unlock();
    drop(held);
    Handed::Nothing
}

/// A month with no `.tix`, asked how it looks a timestamp up: the
/// `store.tix` warning in `crate::file`'s `bisecting`. D-2329.
///
/// Empty, so nothing commits and `store.append` stays silent: the writer door
/// makes the month and its index, the index is taken away, and a READ handle
/// is asked. `time_lookup` hands back the reason — working data, and the line
/// is the only other trace of it.
fn drive_tix_absent(root: &Path) -> Handed {
    let made = BarFile::open_or_create(root, bars_path(), SYMBOL).expect("a fresh month opens");
    drop(made);
    std::fs::remove_file(bars_path().with_file(FileKind::TimeIndex).to_path_buf(root))
        .expect("the writer door made the index");
    let reader = BarFile::open_existing(root, bars_path(), SYMBOL).expect("the month reads");
    assert_eq!(
        reader.time_lookup(),
        crate::file::TimeLookup::Bisection(crate::time_index::Why::Absent)
    );
    Handed::Data
}

/// A month holding bars and no `.tix`, opened by a writer: the `store.tix`
/// rebuild line in `crate::file`'s `rebuild_index`. D-2329.
///
/// The two bars are committed by hand — records, block checksum, header slot,
/// exactly the bytes `BarFile::append` writes — because an append would fire
/// `store.append`, a second line for a site this table drives once. Then the
/// index is taken away and the writer door opens the month: it rebuilds the
/// index from the bars and the lookup is served by it.
fn drive_tix_rebuilt(root: &Path) -> Handed {
    let made = BarFile::open_or_create(root, bars_path(), SYMBOL).expect("a fresh month opens");
    let (layout, header) = (made.layout(), made.header());
    let bars = made.path().to_path_buf();
    drop(made);
    let mut image = Vec::new();
    for index in 0..2 {
        crate::format::Row::write_into(&bar(index), &mut image);
    }
    let sum = block::seal(layout, 2, 0, &image).expect("one block of two records");
    let commit = header
        .advance(2, bar(0).ts_micros, bar(1).ts_micros)
        .and_then(|next| next.commit())
        .expect("two bars after nothing");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&bars)
        .expect("the month is there");
    std::os::unix::fs::FileExt::write_all_at(&file, &image, HEADER_LEN).expect("records");
    std::os::unix::fs::FileExt::write_all_at(&file, &commit.bytes, commit.offset).expect("slot");
    std::fs::write(bars.with_extension("crc"), sum.to_le_bytes()).expect("the block sum");
    std::fs::remove_file(bars_path().with_file(FileKind::TimeIndex).to_path_buf(root))
        .expect("the writer door made the index");

    let writer = BarFile::open_or_create(root, bars_path(), SYMBOL).expect("the month opens");
    assert_eq!(writer.records(), 2, "the premise: two committed bars");
    assert_eq!(writer.time_lookup(), crate::file::TimeLookup::Indexed);
    assert_eq!(writer.first_at_or_after(bar(1).ts_micros), Ok(1));
    Handed::Data
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

/// One row per production emit site in this crate, and the drive that reaches it.
///
/// At module scope rather than inside the test that walks it, so the table can
/// grow without the test body growing with it.
const SITES: [Site; 11] = [
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
        target: "store.block",
        message: "tail block sealed past the commit by an interrupted append",
        level: telemetry::Level::Warn,
        drive: drive_block_sealed_past_the_commit,
    },
    Site {
        target: "store.open",
        message: "bytes past the commit counter",
        level: telemetry::Level::Warn,
        drive: drive_open_ragged_tail,
    },
    Site {
        target: "store.append",
        message: "committed",
        level: telemetry::Level::Debug,
        drive: drive_append_committed,
    },
    Site {
        target: "store.flock",
        message: "advisory lock not released by its guard",
        level: telemetry::Level::Warn,
        drive: drive_flock_unlock_refused,
    },
    Site {
        target: "store.tix",
        message: "time lookup falls back to bisection",
        level: telemetry::Level::Warn,
        drive: drive_tix_absent,
    },
    Site {
        target: "store.tix",
        message: "time index rebuilt from the bars",
        level: telemetry::Level::Info,
        drive: drive_tix_rebuilt,
    },
];

/// EVERY EMIT IN THIS CRATE REACHES A FILE, through the call that owns it.
///
/// Nine production sites, nine production calls, one sink, and one read of
/// the bytes on disk. Deleting any one of the nine emits fails this test; so
/// does changing a target, a sentence or a level, and so does adding a tenth
/// emit on a path this table already drives.
///
/// # The count is an assertion, not a formality
///
/// `assert_eq!(records.len(), SITES.len())` is what pins the **absence** half.
/// Eight of these nine sites sit beside a success path that is documented to be
/// silent — the ordinary header read, the commit that succeeds, the block that
/// verifies, the month with nothing past its counter, the lock that unlocks —
/// and a per-file or
/// per-record emit added there would not fail any presence check. It fails this
/// one. `drive_append_committed` alone opens a month, initialises a 32 KiB
/// header, reads it back and commits, and the table says that whole sequence is
/// worth exactly one line.
///
/// It stayed an `assert_eq!` through the failure [`hold_the_sink`] describes,
/// where the honest-looking repair was a `contains` check over seven targets.
/// That would have passed on a run holding twelve records, six of them written
/// by other tests, and it would have kept passing the day one of the silent
/// paths started speaking.
///
/// # Each drive gets its own store root
///
/// Three of the nine touch a real filesystem, and two of them render the *same*
/// [`StorePath`] — one vendor, one symbol, one month, because that is the
/// fixture the module already had. Handed one root they would share a file, and
/// whichever ran second would open what the first left behind: a drive firing a
/// site it does not own, which is the exact failure this count exists to catch,
/// arriving from inside the test rather than from the crate. Each drive is
/// handed `<root>/site-<n>` and never learns the others exist. The six that
/// never open a file are handed one too, so no drive can start depending on
/// which of them is the one with a disk.
///
/// # Why this test may install the global
///
/// `telemetry::install` writes a per-process `OnceLock` and refuses a second
/// call. Nothing else in this binary installs one, and no production path in
/// this crate installs one either — the store emits into whatever its host
/// installed. So the install is asserted rather than skipped on failure: a test
/// that quietly measures somebody else's sink is worse than one that does not
/// run.
///
/// What that paragraph used to say next was "so there is nothing here to race",
/// and it was false, which is how the failure got in. Nothing else *installs*
/// a sink. Plenty else *emits* into it.
#[test]
fn every_emit_in_this_crate_reaches_the_log_through_its_production_call() {
    // NOTHING ELSE IN THIS BINARY MAY EMIT UNTIL THE READ BELOW IS DONE.
    // Taken before the install rather than after, so the window this test owns
    // opens before the sink any other thread could write to exists at all.
    let _sink_is_mine = hold_the_sink();

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

    // ONE ROOT PER DRIVE, by ordinal. Two of these open a real month at the
    // same rendered `StorePath`, so a shared root would hand the second one the
    // first one's file — and a torn tail found by the drive that only meant to
    // commit is a line this table cannot tell from a crate defect.
    let handed: Vec<Handed> = SITES
        .iter()
        .enumerate()
        .map(|(ordinal, site)| {
            // The store root exists before the writer runs: the writer no
            // longer creates a missing root (D-1522).
            let site_root = root.join(format!("site-{ordinal}"));
            std::fs::create_dir_all(&site_root).expect("a scratch store root");
            (site.drive)(&site_root)
        })
        .collect();

    let found = telemetry::tail(&dir, sink.keep_files(), &telemetry::Query::last(64));
    assert_eq!(
        found.records.len(),
        SITES.len(),
        "one line per site and not one line more — no site went silent, no \
         silent path spoke, and nothing outside this test wrote while it ran. \
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

    // THE HEADER COUNTS WHAT THE CALLS HANDED BACK. It said three of the
    // eight failure sites were not refusals and five handed back a
    // `FormatError`, and the header fall-back returns `Ok` with an older
    // generation: four and four. Each count is taken from the drives above.
    let tally = |which: Handed| handed.iter().filter(|&&each| each == which).count();
    let failures = SITES.len() - tally(Handed::Success);
    let silent = tally(Handed::Data) + tally(Handed::Nothing);
    let header = module_header();
    for claim in [
        format!(
            "{} of these {} fire only once something has already gone wrong",
            spelt_first(failures),
            spelt(SITES.len())
        ),
        format!(
            "{} of those {} are not refusals",
            spelt_first(silent),
            spelt(failures)
        ),
        format!(
            "Those {} hand back working data, so the line is the only trace there is",
            spelt(tally(Handed::Data))
        ),
        format!(
            "Those are the {} sites in this crate whose deletion is invisible from \
             outside the log",
            spelt(silent)
        ),
        format!(
            "The other {} hand the caller a `FormatError`",
            spelt(tally(Handed::Refusal))
        ),
    ] {
        assert!(
            header.contains(&claim),
            "the module header must say {claim:?}, which is what the drives handed \
             back: {handed:?}"
        );
    }
}

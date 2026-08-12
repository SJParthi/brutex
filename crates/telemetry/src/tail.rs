//! The reader: the last N events, without reading the file.
//!
//! # What this costs, honestly
//!
//! The file is walked **backwards** from its end in [`READ_BLOCK`]-byte
//! blocks, and stops the moment it has what was asked for. So the cost of
//! "the last twenty events" is:
//!
//! * one `open` per file touched, and one `fstat` on that handle for its
//!   length and its identity;
//! * one `seek` per block;
//! * `ceil(bytes_needed / READ_BLOCK)` reads, where `bytes_needed` is the
//!   total size of the events returned, not the size of the file.
//!
//! It is **O(N × line width), and O(1) in the size of the file** — which is
//! the bound that matters, because the file grows all day and N does not. It
//! is *not* O(1) in N, and no reader can be: returning twenty events means
//! reading twenty events' worth of bytes. That is the minimum, and this is
//! within one block of it.
//!
//! `tail::the_last_events_are_read_without_touching_the_rest_of_the_file` is
//! the proof, and it is written as an assertion on [`Tail::bytes_read`] rather
//! than as a timing, for the same reason
//! `api::audit::the_tail_reads_only_the_records_it_shows` asserts a byte
//! offset: a timing passes on a fast machine with a quadratic algorithm.
//!
//! # Why a filter does not make it unbounded
//!
//! A filter that matches nothing would otherwise walk all 64 MiB. Every query
//! carries [`Query::max_scan_bytes`], the walk stops when it has read that
//! many, and [`Tail::hit_scan_cap`] says so — the answer is then "here is what
//! I found in the newest 8 MiB", which is a true answer, rather than a
//! complete answer that took a second to produce. `CLAUDE.md` §4: degrade
//! loudly and name the reason.
//!
//! A `since` filter is better than bounded: the stream is in time order, so
//! the first event older than `since` ends the walk exactly. Nothing after it
//! can match. That order is not an assumption about how clocks behave — the
//! writer produces it, by stamping `ms` inside its critical section clamped to
//! the previous event's (`Sink::emit`, `Inner::last_at`). Before it did, this
//! sentence was simply wrong and the early exit could return nothing.
//!
//! # What a damaged line does
//!
//! It is counted in [`Tail::malformed`] and stepped over. One line that will
//! not decode never blanks the page around it — the same rule
//! `api::audit::Entry` follows for a record that fails its checksum.
//!
//! # Why the walk keys on the file and not on the path
//!
//! **A ROLL RENAMES, AND THE WALK IS NOT ATOMIC.** It opens one path, reads it,
//! and then opens the next — and a roll landing in between renames the file it
//! has just read onto the path it is about to open. Keyed on the path, the
//! walk then returns every event in that file a second time. Keyed on the file
//! itself — [`FileId`], the pair the operating system uses — it does not.

use std::io::{Read as _, Seek as _, SeekFrom};
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

use crate::level::Level;
use crate::record::Record;
use crate::sink::paths_newest_first;

/// How much is read at a time, walking backwards.
///
/// 8 KiB holds roughly thirty events at the typical line width, so "the last
/// twenty" is one block and one read. Larger would read bytes nobody asked
/// for; smaller would turn one read into several.
pub const READ_BLOCK: u64 = 8 * 1024;

/// The most events one query will ever return.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary. A
/// caller asking for everything gets a page of it rather than an allocation
/// the size of the set.
pub const MAX_LIMIT: usize = 1000;

/// How far back a query walks before it gives up and says so.
///
/// One file's worth at the default bound. A query with a filter that matches
/// nothing therefore costs one file, not the whole set.
pub const DEFAULT_MAX_SCAN_BYTES: u64 = 8 * 1024 * 1024;

/// The widest run of bytes without a newline this reader will accumulate.
///
/// **THE CARRY IS THE ONLY UNBOUNDED THING ON THE READ PATH, AND IT WAS BOTH
/// QUADRATIC AND UNCAPPED.** `walk_back` reads fixed blocks backwards and keeps
/// the bytes before the block's first newline as `carry`, to be joined to the
/// next block. When a block holds no newline at all, the whole block joins the
/// carry — and the carry is COPIED on every iteration, so scanning a file whose
/// "lines" are long costs O(bytes²) in time and grows without limit in space.
/// Measured before this cap: 40 ms over 1 MiB and **2.91 s over 8 MiB**, a 4.0x
/// rise per doubling.
///
/// 64 KiB is far above any line this crate can write. The event ceilings bound
/// one at roughly 2.2 KiB of content, and JSON escaping cannot inflate that
/// past about 14 KiB even if every byte needs a `\u00XX`. So a run longer than
/// this is not a line of ours: it is a corrupt file, a foreign file, or a file
/// somebody hand-edited — all of which a reader handed a log folder must
/// survive rather than exhaust memory on.
///
/// It is the same figure `resume_seq` uses for "further back than any line can
/// reach", and for the same reason.
pub const MAX_LINE_BYTES: usize = 64 * 1024;

/// What to return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// How many events, at most. Capped at [`MAX_LIMIT`].
    pub limit: usize,
    /// The quietest level to return, or every level.
    pub min_level: Option<Level>,
    /// A subsystem, matched exactly or as a dotted prefix.
    ///
    /// `"pull"` returns `"pull"`, `"pull.http"` and `"pull.http.retry"`. It
    /// does **not** return `"pullover"`: the prefix match requires the dot, so
    /// a subsystem name can never accidentally swallow another one.
    pub target: Option<String>,
    /// Only events at or after this instant, in milliseconds since the epoch.
    pub since_unix_millis: Option<i64>,
    /// Only events belonging to one run.
    ///
    /// **The filter that makes a shared log readable.** A file spanning three
    /// backfills is three interleaved stories; this is how a reader takes one.
    pub run: Option<u64>,
    /// How many bytes the walk may read before it stops and says it stopped.
    pub max_scan_bytes: u64,
}

impl Query {
    /// The last `limit` events, unfiltered.
    #[must_use]
    pub const fn last(limit: usize) -> Self {
        Self {
            limit,
            min_level: None,
            target: None,
            since_unix_millis: None,
            run: None,
            max_scan_bytes: DEFAULT_MAX_SCAN_BYTES,
        }
    }

    /// The same query, at or above a level.
    #[must_use]
    pub const fn at_least(mut self, level: Level) -> Self {
        self.min_level = Some(level);
        self
    }

    /// The same query, from one subsystem and everything beneath it.
    #[must_use]
    pub fn from_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// The same query, narrowed to one run.
    #[must_use]
    pub const fn from_run(mut self, run: u64) -> Self {
        self.run = Some(run);
        self
    }

    /// The same query, no older than an instant.
    #[must_use]
    pub const fn since(mut self, unix_millis: i64) -> Self {
        self.since_unix_millis = Some(unix_millis);
        self
    }

    /// The same query, reading at most this many bytes.
    #[must_use]
    pub const fn scanning_at_most(mut self, bytes: u64) -> Self {
        self.max_scan_bytes = bytes;
        self
    }

    /// Whether a decoded record is one this query asked for.
    fn wants(&self, record: &Record) -> bool {
        if let Some(floor) = self.min_level
            && !record.level.at_least(floor)
        {
            return false;
        }
        if let Some(want) = self.run
            && record.run != want
        {
            return false;
        }
        match self.target {
            None => true,
            Some(ref want) => {
                // NO LENGTH GUARD, BECAUSE THE DOT ALREADY IS ONE.
                //
                // This read `record.target.len() > want.len() && ...`. That
                // condition is DEAD: the two that follow it already imply it —
                // a target that starts with `want` and has a `.` at byte
                // `want.len()` is necessarily longer than `want`. So `>` and
                // `>=` are behaviourally identical and no test could ever kill
                // the mutant; it is not a coverage gap but an unfalsifiable
                // line, which `CLAUDE.md` §4 treats as an assertion that
                // asserts nothing.
                //
                // The same guard was removed from `Sink::level_for` for the
                // same reason, and this is its reader-side twin — the two must
                // agree about what a subsystem prefix means, so they now agree
                // about this too.
                record.target == *want
                    || (record.target.starts_with(want.as_str())
                        && record.target.as_bytes().get(want.len()) == Some(&b'.'))
            }
        }
    }
}

/// What the walk found, and what it cost.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tail {
    /// The events, **newest first**.
    pub records: Vec<Record>,
    /// How many bytes were read off disk. The number the bound is asserted on.
    pub bytes_read: u64,
    /// How many files the walk read bytes from.
    ///
    /// A file it opened and did not read is not one of them: an empty file has
    /// nothing to give, and a file already read under its previous name — a
    /// roll renamed it mid-walk — would otherwise be counted twice for the one
    /// set of events it contributed.
    pub files_read: u32,
    /// Lines that would not decode. Stepped over, never repaired.
    pub malformed: u64,
    /// Whether the newest file ended mid-line — a writer caught in the act, or
    /// a torn write. The fragment is skipped, not guessed at.
    pub partial_tail: bool,
    /// Whether the walk stopped because it had read [`Query::max_scan_bytes`].
    ///
    /// When this is true the answer is "what I found in the bytes I read", not
    /// "everything there is".
    pub hit_scan_cap: bool,
    /// Whether every event older than the ones returned was actually looked at.
    ///
    /// False means a non-empty file older than the last one read still holds
    /// bytes the walk never opened — so events older than these exist and are
    /// not on the page.
    ///
    /// **It used to be false on ordinary full pages too**, because the walk
    /// reported "the query is satisfied" and "I gave up early" with the same
    /// `bool`. A page that filled exactly on the oldest file's first line said
    /// older events existed when the set held none. See [`Stop`]: a satisfied
    /// walk that consumed its file to byte 0 now looks for an older file rather
    /// than assuming one is there.
    pub reached_oldest: bool,
    /// Files that exist and could not be read, in the failure's own words.
    ///
    /// An absent file is **not** here: the set is sparse until it has rolled,
    /// and "no such file" is the ordinary state before the first event.
    pub errors: Vec<String>,
    /// **Events the sink NUMBERED and this file does not hold.**
    ///
    /// [`None`] when the walk was filtered by level, target or instant — a
    /// filter is supposed to skip records, so a gap in the sequence says
    /// nothing about loss. [`Some`] on an unfiltered walk is an exact count of
    /// events that existed and are gone.
    ///
    /// # Why the sequence number can answer this at all
    ///
    /// [`crate::Sink::emit`] assigns the number BEFORE it tries to write, and
    /// burns one on an event it then drops. So a hole in the sequence is the
    /// drop's own receipt, written by the fact of the numbering rather than by
    /// any bookkeeping — and it survives a restart, because
    /// [`crate::Sink::open`] resumes the count from the file.
    ///
    /// **This is the first question a reader who did not run the job must
    /// ask.** A log handed to somebody else is evidence, and evidence with
    /// silent holes in it is worse than none: every conclusion drawn from it is
    /// unsound in a way nothing on the page would show. `hit_scan_cap`,
    /// `partial_tail` and `malformed` each report a limit of the READER. This
    /// is the one number that reports a loss by the WRITER.
    ///
    /// Measured once by accident: a live log lost 96 events between two reads
    /// twenty minutes apart while every other flag on every surface read clean.
    ///
    /// # Cost
    ///
    /// O(1). Two `seq` reads off records already in hand and one subtraction —
    /// no second walk, no scan, nothing that grows with the answer. The walk
    /// this rides on is held flat by
    /// `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file` (C-T-03),
    /// and the counting itself by
    /// `telemetry::tail::a_hole_in_the_sequence_is_counted_and_a_filter_is_not_mistaken_for_one`
    /// (T-21).
    pub missing: Option<u64>,
}

/// Which file this is, as the operating system knows it: device and inode.
///
/// **Not its path.** The path is a name a roll moves; this is the thing the
/// name is on, and it is what makes "have I already read this?" answerable
/// during a rename. Taken from the `fstat` of the OPEN handle rather than from
/// a second lookup of the path, so it names the file whose bytes were actually
/// read even if the path stopped pointing at it in between.
type FileId = (u64, u64);

/// One file's bytes at an offset.
///
/// # Errors
///
/// Whatever the filesystem said.
pub(crate) fn read_at(path: &Path, offset: u64, len: u64) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let span = usize::try_from(len).unwrap_or(usize::MAX);
    let mut buf = vec![0u8; span];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

/// The last events in the set at `dir`, newest first.
///
/// Never fails. Every way this can go wrong is a state the caller renders: an
/// absent directory is an empty answer, an unreadable file is a line in
/// [`Tail::errors`], and a line that will not decode is a count in
/// [`Tail::malformed`].
#[must_use]
pub fn tail(dir: &Path, keep_files: u8, query: &Query) -> Tail {
    let mut out = walked(dir, keep_files, query);
    out.missing = missing_between(&out, query);
    out
}

/// Events the sink numbered that this answer does not hold, when that can be
/// known.
///
/// [`None`] under any filter: a filter skips records on purpose, so a hole in
/// the sequence carries no information about loss. Under no filter the walk
/// returns CONSECUTIVE records newest-first, so the span between the highest
/// and lowest sequence number it saw is exactly how many events should be
/// there — and anything short of that is gone.
///
/// One record cannot have a hole, and no records is not evidence of one.
fn missing_between(out: &Tail, query: &Query) -> Option<u64> {
    if query.min_level.is_some()
        || query.target.is_some()
        || query.since_unix_millis.is_some()
        || query.run.is_some()
    {
        return None;
    }
    let newest = out.records.first()?.seq;
    let oldest = out.records.last()?.seq;
    let span = newest.checked_sub(oldest)?.checked_add(1)?;
    Some(span.saturating_sub(out.records.len() as u64))
}

/// [`tail`]'s walk, before the completeness arithmetic.
fn walked(dir: &Path, keep_files: u8, query: &Query) -> Tail {
    let mut out = Tail {
        reached_oldest: true,
        ..Tail::default()
    };
    let limit = query.limit.min(MAX_LIMIT);
    if limit == 0 {
        return out;
    }
    let mut first_file = true;
    // THE SAME FILE IS NOT READ TWICE, WHATEVER IT IS CALLED WHEN THE WALK
    // REACHES IT. At most `keep_files` entries, which is a `u8`, pushed once
    // per file rather than once per record — see `FileId` and
    // `a_roll_landing_mid_walk_never_returns_the_same_event_twice`.
    let mut read_already: Vec<FileId> = Vec::new();
    // THE QUERY IS SATISFIED AND THE LAST FILE WAS READ TO ITS FIRST BYTE.
    //
    // The loop then keeps going, but only far enough to answer one question:
    // does a file OLDER than everything read still hold bytes? That is what
    // `reached_oldest` claims to report, and it used to be answered by assuming
    // the answer was yes on every satisfied query — so an unfiltered page that
    // filled exactly on the oldest file's first line said "older events exist"
    // when nothing older existed at all.
    let mut finished = false;
    for path in paths_newest_first(dir, keep_files) {
        // OPENED FIRST, AND EVERY FACT ABOUT IT TAKEN FROM THAT ONE HANDLE.
        // This was a `metadata` of the path followed by an `open` of the path:
        // two lookups, and a roll landing between them gave the length of one
        // file and the bytes of another. One `open` and one `fstat` cannot
        // disagree with each other.
        let mut file = match std::fs::File::open(&path) {
            // AN ABSENT FILE IS NOT AN ERROR. The set is sparse until it has
            // rolled `keep_files` times, so "no such file" is the ordinary
            // state and not a thing to report.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                continue;
            }
            Ok(file) => file,
        };
        let (len, id) = match file.metadata() {
            Ok(meta) => (meta.len(), (meta.dev(), meta.ino())),
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        if len == 0 {
            continue;
        }
        // A REPEAT IS SKIPPED SILENTLY, AND THAT IS NOT A FALLBACK THAT HIDES A
        // FAILURE. Nothing is lost and nothing is degraded: this file's events
        // are already in `records`, put there by the path it answered to a
        // moment ago, and the walk carries on to the next path — which now
        // holds the file that was one older, so the answer stays complete as
        // well as contiguous.
        if read_already.contains(&id) {
            continue;
        }
        // THE PEEK, AND IT READS NOTHING. Reaching here with `finished` means a
        // non-empty file older than every file read still exists, so the answer
        // is no. `files_read` and `bytes_read` are deliberately NOT touched: this
        // file was opened and stat'd, never read, and the cost bound
        // `the_last_events_are_read_without_touching_the_rest_of_the_file` holds
        // because no block is fetched.
        if finished {
            out.reached_oldest = false;
            return out;
        }
        read_already.push(id);
        out.files_read = out.files_read.saturating_add(1);
        let newest_file = core::mem::take(&mut first_file);
        match walk_back(&mut file, &path, len, newest_file, limit, query, &mut out) {
            // Gave up with bytes still unread in THIS file, so something older
            // is unread by construction and no peek is needed.
            Stop::Stopped => {
                out.reached_oldest = false;
                return out;
            }
            // Satisfied, and this file is spent. Look for an older one.
            Stop::Exhausted => finished = true,
            // Still hungry; carry on normally.
            Stop::Unfinished => {}
        }
    }
    // Falling out of the loop means no older non-empty file was found, so
    // `reached_oldest` keeps the `true` it was initialised with.
    out
}

/// Why [`walk_back`] stopped, which a bare `bool` could not say.
///
/// The three states used to be one `true`, and collapsing them is what made
/// `Tail::reached_oldest` wrong on an ordinary full page: a walk that filled its
/// limit on the last line of a file it had read to byte 0 was reported
/// identically to one that gave up with bytes still unread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// The file was consumed and the query still wants more.
    Unfinished,
    /// The query is satisfied, but this file still holds unread bytes.
    Stopped,
    /// The query is satisfied AND this file was read to its first byte.
    ///
    /// Whether anything older exists is then a question about the NEXT file, not
    /// about this one — so the caller peeks rather than assuming.
    Exhausted,
}

/// Walks one file from its end. Returns why it stopped.
fn walk_back(
    file: &mut std::fs::File,
    path: &Path,
    len: u64,
    newest_file: bool,
    limit: usize,
    query: &Query,
    out: &mut Tail,
) -> Stop {
    let mut pos = len;
    let mut carry: Vec<u8> = Vec::new();
    // The bytes after the final newline of the newest file are a line that is
    // not finished being written. Skipped and reported, never decoded.
    let mut drop_fragment = false;
    let mut first_block = true;

    while pos > 0 {
        if out.bytes_read >= query.max_scan_bytes {
            out.hit_scan_cap = true;
            return Stop::Stopped;
        }
        let take = READ_BLOCK.min(pos);
        pos = pos.saturating_sub(take);
        let block = match read_block(file, pos, take) {
            Ok(block) => block,
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                return Stop::Unfinished;
            }
        };
        out.bytes_read = out.bytes_read.saturating_add(take);

        let mut work = block;
        work.append(&mut carry);
        if core::mem::take(&mut first_block) && newest_file && work.last() != Some(&b'\n') {
            out.partial_tail = true;
            drop_fragment = true;
        }

        let mut end = work.len();
        let mut at = end;
        while at > 0 {
            at = at.saturating_sub(1);
            if work.get(at) != Some(&b'\n') {
                continue;
            }
            let line = work.get(at.saturating_add(1)..end).unwrap_or(&[]);
            end = at;
            if core::mem::take(&mut drop_fragment) {
                continue;
            }
            if take_line(line, limit, query, out) {
                // STOPPED, not exhausted: `end` is still above zero, so this
                // file holds lines the walk never looked at.
                return Stop::Stopped;
            }
        }
        carry = work.get(..end).unwrap_or(&[]).to_vec();
        // REFUSED RATHER THAN ACCUMULATED. Past this width the bytes cannot be
        // a line this crate wrote, and carrying them further is the quadratic.
        // Counted as malformed, which is what the reader already says about
        // bytes it stepped over — one number, one meaning.
        if carry.len() > MAX_LINE_BYTES {
            out.malformed = out.malformed.saturating_add(1);
            carry.clear();
        }
    }
    // The first line of the file has no newline before it.
    //
    // EXHAUSTED, not merely stopped. This runs only after `while pos > 0` ended,
    // so every byte of this file has been read. Whether anything OLDER exists is
    // a question about the NEXT file, and `walked` answers it by looking rather
    // than by assuming the worst.
    if !drop_fragment && take_line(&carry, limit, query, out) {
        return Stop::Exhausted;
    }
    Stop::Unfinished
}

/// One block, by seeking rather than by re-opening.
fn read_block(file: &mut std::fs::File, offset: u64, len: u64) -> std::io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset))?;
    let span = usize::try_from(len).unwrap_or(usize::MAX);
    let mut buf = vec![0u8; span];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

/// Considers one line. Returns whether the whole query is finished.
fn take_line(line: &[u8], limit: usize, query: &Query, out: &mut Tail) -> bool {
    if line.is_empty() {
        return false;
    }
    let Ok(record) = Record::decode(line) else {
        out.malformed = out.malformed.saturating_add(1);
        return false;
    };
    // THE ONE FILTER THAT ENDS THE WALK, and the only one entitled to.
    //
    // Sound because the WRITER enforces the order this depends on rather than
    // this comment asserting it: `Sink::emit` stamps `ms` inside its critical
    // section clamped to `Inner::last_at`, so `ms` is non-decreasing in file
    // order. The walk runs backwards, so the first record older than `since`
    // guarantees every remaining one is older still.
    //
    // IT WAS NOT ALWAYS SOUND. The justification used to be the bare sentence
    // "events are in time order", and that was false: the clock was read
    // BEFORE the sink's lock, so `seq` was in lock-acquisition order while `ms`
    // was in pre-lock order, and a thread preempted between the two wrote a
    // line older than the one before it. A single inversion ended the walk
    // early and returned nothing, and no signal survived to say so —
    // `missing_between` returns `None` whenever a `since` filter is present,
    // and `reached_oldest == false` is also set on the ordinary full-page path,
    // so it cannot distinguish "your page is full" from "I gave up".
    //
    // If the clamp in `Sink::emit` is ever removed, this `return true` must
    // become `return false`, demoting `since` to an ordinary skip-filter.
    if query
        .since_unix_millis
        .is_some_and(|floor| record.at_unix_millis < floor)
    {
        return true;
    }
    if !query.wants(&record) {
        return false;
    }
    out.records.push(record);
    out.records.len() >= limit
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{DEFAULT_MAX_SCAN_BYTES, MAX_LIMIT, MAX_LINE_BYTES, Query, READ_BLOCK, Tail, tail};
    use crate::event::Event;
    use crate::level::Level;
    use crate::sink::{Config, Sink, current_path, rotated_path};
    use crate::value::OwnedValue;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        // Clears what earlier RUNS left behind — see `sweep_stale_scratch`.
        // Emptying only the directory about to be used is what let 9,958
        // of them accumulate.
        crate::sweep_stale_scratch();
        let dir = std::env::temp_dir().join(format!(
            "brutex-telemetry-tail-{}-{name}",
            std::process::id()
        ));
        let _ignored = std::fs::remove_dir_all(&dir);
        dir
    }

    /// A sink holding `n` events, numbered from zero.
    fn filled(dir: &PathBuf, n: u32) -> Sink {
        let sink = Sink::open(&Config::new(dir).with_min_level(Level::Trace)).expect("opens");
        for i in 0..n {
            assert!(
                sink.emit(
                    &Event::info("pull.http", "one of many")
                        .with("i", i)
                        .with("padding", "0123456789012345678901234567890123456789")
                )
                .is_written()
            );
        }
        sink
    }

    /// THE READER DOES NOT READ THE FILE.
    ///
    /// The bound this module exists for, asserted in bytes rather than as a
    /// timing: five events out of a 100 KB file must cost one block, and a
    /// timing assertion would pass on a fast machine with an implementation
    /// that read all of it.
    #[test]
    fn the_last_events_are_read_without_touching_the_rest_of_the_file() {
        let dir = scratch("bytes");
        let sink = filled(&dir, 700);
        let size = std::fs::metadata(current_path(&dir)).unwrap().len();
        assert!(
            size > 80_000,
            "the fixture must be much larger than a block: {size}"
        );

        let found = tail(&dir, sink.keep_files(), &Query::last(5));
        assert_eq!(found.records.len(), 5);
        assert_eq!(
            found.bytes_read, READ_BLOCK,
            "exactly one block, not the file"
        );
        assert!(
            found.bytes_read * 10 < size,
            "{} bytes read out of {size}",
            found.bytes_read
        );
        assert_eq!(found.files_read, 1, "and only the newest file was opened");
        assert_eq!(found.malformed, 0);
        assert!(!found.partial_tail);
        assert!(!found.hit_scan_cap);
        assert!(found.errors.is_empty());

        // NEWEST FIRST, and they are the last five that were written.
        let seqs: Vec<u64> = found.records.iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![700, 699, 698, 697, 696]);
        assert_eq!(
            found.records[0].field("i").and_then(OwnedValue::as_i64),
            Some(699)
        );

        // Asking for more costs proportionally more, and never more than it
        // has to: 400 events of ~130 bytes is ~52 KB, which is seven blocks.
        let more = tail(&dir, sink.keep_files(), &Query::last(400));
        assert_eq!(more.records.len(), 400);
        assert!(
            more.bytes_read < size,
            "{} is not less than {size}",
            more.bytes_read
        );
        assert!(more.bytes_read >= READ_BLOCK * 5, "{}", more.bytes_read);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_or_absent_set_is_an_empty_answer_and_not_a_failure() {
        let dir = scratch("absent");
        let found = tail(&dir, 8, &Query::last(10));
        assert_eq!(
            found,
            Tail {
                reached_oldest: true,
                ..Tail::default()
            }
        );
        assert!(found.errors.is_empty(), "an absent file is not an error");

        // A directory with an empty file in it is the state before the first
        // event, and is also not a failure.
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let found = tail(&dir, sink.keep_files(), &Query::last(10));
        assert!(found.records.is_empty());
        assert_eq!(found.bytes_read, 0, "an empty file is not read at all");
        assert_eq!(found.files_read, 0);

        // And a limit of zero returns nothing, having read nothing.
        assert!(sink.emit(&Event::info("t", "m")).is_written());
        let none = tail(&dir, sink.keep_files(), &Query::last(0));
        assert!(none.records.is_empty());
        assert_eq!(none.bytes_read, 0);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// FILTERING RETURNS EXACTLY THE MATCHES, NOT APPROXIMATELY.
    #[test]
    fn a_level_filter_and_a_target_filter_each_return_exactly_what_matches() {
        let dir = scratch("filter");
        let sink = Sink::open(&Config::new(&dir).with_min_level(Level::Trace)).expect("opens");
        // A deliberate mixture, and a target that is a PREFIX of another so
        // the dotted rule is exercised rather than assumed.
        for spec in [
            (Level::Trace, "pull"),
            (Level::Debug, "pull.http"),
            (Level::Info, "pull.http"),
            (Level::Warn, "pull.ssm"),
            (Level::Error, "api.server"),
            (Level::Info, "pullover"),
            (Level::Error, "pull.http"),
        ] {
            assert!(
                sink.emit(&Event::new(spec.0, spec.1, "m").with("t", spec.1))
                    .is_written()
            );
        }
        let all = |query: &Query| {
            tail(&dir, sink.keep_files(), query)
                .records
                .into_iter()
                .map(|r| (r.level, r.target))
                .collect::<Vec<_>>()
        };

        assert_eq!(
            all(&Query::last(50).at_least(Level::Warn)),
            vec![
                (Level::Error, "pull.http".to_owned()),
                (Level::Error, "api.server".to_owned()),
                (Level::Warn, "pull.ssm".to_owned()),
            ],
            "warn and above, newest first, and nothing quieter"
        );
        assert_eq!(
            all(&Query::last(50).at_least(Level::Trace)).len(),
            7,
            "the quietest floor admits everything"
        );

        // A TARGET IS A DOTTED PREFIX, and `pullover` is not beneath `pull`.
        assert_eq!(
            all(&Query::last(50).from_target("pull")),
            vec![
                (Level::Error, "pull.http".to_owned()),
                (Level::Warn, "pull.ssm".to_owned()),
                (Level::Info, "pull.http".to_owned()),
                (Level::Debug, "pull.http".to_owned()),
                (Level::Trace, "pull".to_owned()),
            ],
            "`pullover` shares five letters with `pull` and is not beneath it"
        );
        assert_eq!(
            all(&Query::last(50).from_target("pull.http")).len(),
            3,
            "a deeper prefix narrows it"
        );
        assert_eq!(all(&Query::last(50).from_target("pullov")).len(), 0);
        assert_eq!(all(&Query::last(50).from_target("nothing")).len(), 0);

        // The two filters compose.
        assert_eq!(
            all(&Query::last(50).from_target("pull").at_least(Level::Warn)),
            vec![
                (Level::Error, "pull.http".to_owned()),
                (Level::Warn, "pull.ssm".to_owned()),
            ]
        );
        // And a limit still applies after filtering.
        assert_eq!(all(&Query::last(1).at_least(Level::Warn)).len(), 1);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A `since` FILTER ENDS THE WALK RATHER THAN FILTERING THE WHOLE SET.
    #[test]
    fn a_since_filter_stops_the_walk_at_the_first_event_older_than_it() {
        let dir = scratch("since");
        let sink = filled(&dir, 400);
        let everything = tail(&dir, sink.keep_files(), &Query::last(MAX_LIMIT));
        assert_eq!(everything.records.len(), 400);
        let newest = everything.records[0].at_unix_millis;
        let oldest = everything.records[399].at_unix_millis;

        // Nothing is older than the epoch, so this reads everything.
        let all = tail(&dir, sink.keep_files(), &Query::last(MAX_LIMIT).since(0));
        assert_eq!(all.records.len(), 400);

        // Everything is older than a moment after the newest, so the walk ends
        // on the very first line it decodes.
        let none = tail(
            &dir,
            sink.keep_files(),
            &Query::last(MAX_LIMIT).since(newest + 1),
        );
        assert!(none.records.is_empty());
        assert_eq!(
            none.bytes_read, READ_BLOCK,
            "one block, and it stopped inside it"
        );
        assert!(
            !none.reached_oldest,
            "it stopped early rather than finishing"
        );
        assert!(oldest <= newest, "time runs forwards in the file");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A FILTER THAT MATCHES NOTHING DOES NOT WALK THE WHOLE SET.
    #[test]
    fn a_query_that_finds_nothing_stops_at_its_scan_bound_and_says_it_did() {
        let dir = scratch("cap");
        let sink = filled(&dir, 700);
        let capped = tail(
            &dir,
            sink.keep_files(),
            &Query::last(10)
                .from_target("never.matches.anything")
                .scanning_at_most(READ_BLOCK * 2),
        );
        assert!(capped.records.is_empty());
        assert!(capped.hit_scan_cap, "it stopped because it was told to");
        assert!(!capped.reached_oldest);
        assert_eq!(capped.bytes_read, READ_BLOCK * 2, "not one block more");

        // Without the cap it reads the file and finds nothing, which is a
        // complete answer rather than a truncated one.
        let whole = tail(
            &dir,
            sink.keep_files(),
            &Query::last(10).from_target("never.matches.anything"),
        );
        assert!(whole.records.is_empty());
        assert!(!whole.hit_scan_cap);
        assert!(whole.reached_oldest);
        assert_eq!(
            whole.bytes_read,
            std::fs::metadata(current_path(&dir)).unwrap().len(),
            "a complete answer read the whole file, and says so"
        );
        assert_eq!(Query::last(1).max_scan_bytes, DEFAULT_MAX_SCAN_BYTES);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// THE WALK CROSSES A ROTATION BOUNDARY IN THE RIGHT ORDER.
    #[test]
    fn the_walk_continues_into_the_rolled_files_newest_first() {
        let dir = scratch("across");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(2048)
                .with_keep_files(4),
        )
        .expect("opens");
        for i in 0..300u32 {
            assert!(sink.emit(&Event::info("t", "m").with("i", i)).is_written());
        }
        assert!(rotated_path(&dir, 1).exists(), "it rolled");

        let found = tail(&dir, sink.keep_files(), &Query::last(60));
        assert_eq!(found.records.len(), 60);
        assert!(found.files_read >= 2, "it crossed at least one boundary");
        // Strictly descending sequence numbers across the boundary is the
        // proof that the file ORDER and the within-file order agree.
        for pair in found.records.windows(2) {
            assert_eq!(
                pair[0].seq,
                pair[1].seq + 1,
                "the stream is contiguous across the roll"
            );
        }
        assert_eq!(found.records[0].seq, 300);
        assert_eq!(found.records[59].seq, 241);

        // Asking for more than the set holds returns the set, not an error.
        let everything = tail(&dir, sink.keep_files(), &Query::last(MAX_LIMIT));
        assert!(everything.reached_oldest);
        assert!(
            everything.records.len() < 300,
            "the oldest events fell off the end, which is what rotation is"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A DAMAGED LINE IS COUNTED AND STEPPED OVER, NOT GUESSED AT.
    #[test]
    fn a_line_that_will_not_decode_never_blanks_the_ones_around_it() {
        let dir = scratch("damaged");
        let sink = filled(&dir, 3);
        drop(sink);
        // Three good lines, then rubbish, then a good one — written by hand
        // because nothing in this crate produces a bad line.
        let mut bytes = std::fs::read(current_path(&dir)).unwrap();
        bytes.extend_from_slice(b"{this is not the format}\n");
        bytes.extend_from_slice(b"not even close\n");
        bytes.extend_from_slice(
            br#"{"seq":99,"ts":"x","ms":5,"level":"warn","target":"t","msg":"last"}"#,
        );
        bytes.push(b'\n');
        std::fs::write(current_path(&dir), &bytes).unwrap();

        let found = tail(&dir, 8, &Query::last(10));
        assert_eq!(found.malformed, 2, "both bad lines are counted");
        assert_eq!(found.records.len(), 4, "and all four good ones survive");
        assert_eq!(found.records[0].seq, 99);
        assert_eq!(found.records[0].message, "last");
        assert_eq!(found.records[3].seq, 1);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A WRITER CAUGHT MID-LINE LEAVES A FRAGMENT, AND IT IS NOT AN EVENT.
    #[test]
    fn a_file_that_ends_mid_line_reports_the_fragment_and_does_not_decode_it() {
        let dir = scratch("torn");
        let sink = filled(&dir, 4);
        drop(sink);
        let mut bytes = std::fs::read(current_path(&dir)).unwrap();
        bytes.extend_from_slice(br#"{"seq":5,"ts":"x","ms":5,"level":"warn","targ"#);
        std::fs::write(current_path(&dir), &bytes).unwrap();

        let found = tail(&dir, 8, &Query::last(10));
        assert!(found.partial_tail, "the torn tail is reported by name");
        assert_eq!(
            found.malformed, 0,
            "and it is NOT counted as a damaged line: it is a line that is not \
             finished, which is a different thing"
        );
        assert_eq!(found.records.len(), 4);
        assert_eq!(found.records[0].seq, 4);

        // A file that is nothing BUT a fragment yields nothing at all.
        let dir = scratch("torn-only");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(current_path(&dir), b"{\"seq\":1,\"ms\"").unwrap();
        let found = tail(&dir, 8, &Query::last(10));
        assert!(found.partial_tail);
        assert!(found.records.is_empty());
        assert_eq!(found.malformed, 0);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_larger_than_one_block_with_no_newline_at_all_is_one_fragment() {
        // The path where the fragment spans block boundaries: the walk must
        // carry it backwards and still refuse it at the start of the file.
        let dir = scratch("one-long-line");
        std::fs::create_dir_all(&dir).unwrap();
        let mut bytes = b"{\"seq\":1,\"ms\":1,".to_vec();
        bytes.resize(usize::try_from(READ_BLOCK * 3).unwrap(), b'x');
        std::fs::write(current_path(&dir), &bytes).unwrap();
        let found = tail(&dir, 8, &Query::last(10));
        assert!(found.partial_tail);
        assert!(found.records.is_empty());
        assert_eq!(found.bytes_read, READ_BLOCK * 3, "it did read the file");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_limit_past_the_ceiling_is_the_ceiling_and_not_an_allocation() {
        let dir = scratch("ceiling");
        let sink = filled(&dir, 5);
        let found = tail(&dir, sink.keep_files(), &Query::last(usize::MAX));
        assert_eq!(found.records.len(), 5);
        assert_eq!(Query::last(usize::MAX).limit, usize::MAX, "unclamped here");
        const { assert!(MAX_LIMIT < usize::MAX, "and clamped in the walk") };
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_exists_and_cannot_be_read_is_a_named_error_and_not_a_silence() {
        let dir = scratch("unreadable");
        std::fs::create_dir_all(&dir).unwrap();
        // A DIRECTORY where the newest file has to be. `metadata` succeeds and
        // reports a length; `open` succeeds on some platforms and the read
        // fails on all of them. Whichever way it lands, it must appear in
        // `errors` and must not silently look like an empty stream.
        std::fs::create_dir_all(current_path(&dir)).unwrap();
        std::fs::write(rotated_path(&dir, 1), b"").unwrap();
        let found = tail(&dir, 8, &Query::last(10));
        assert!(
            !found.errors.is_empty(),
            "a file that cannot be read is named: {found:?}"
        );
        assert!(found.records.is_empty());
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **THE READER'S TWO CONSTANTS ARE PINNED AS RELATIONS, NOT RESTATED.**
    ///
    /// Both survived mutation. `READ_BLOCK`'s own proof —
    /// `the_last_events_are_read_without_touching_the_rest_of_the_file` —
    /// asserts `bytes_read == READ_BLOCK`, which is expressed in terms of the
    /// constant, so changing the constant moves the expectation with it:
    /// `8 * 1024` mutated to `8 + 1024` left every assertion true.
    /// `DEFAULT_MAX_SCAN_BYTES` had no test at all.
    ///
    /// A tautology (`assert_eq!(READ_BLOCK, 8 * 1024)`) would restate the
    /// definition and fail nothing. These assert the two things the values are
    /// CHOSEN for, so a change that breaks the design fails here:
    ///
    /// * `READ_BLOCK` must hold several whole events, or the backwards walk
    ///   re-reads for every answer and the O(1)-in-file-size bound is lost.
    /// * `DEFAULT_MAX_SCAN_BYTES` is "one file's worth at the default bound" —
    ///   its own doc — so it must equal `DEFAULT_MAX_FILE_BYTES` exactly, and
    ///   be a strict fraction of the whole retained window.
    ///
    /// The bound those constants exist to keep is measured by
    /// `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file` (C-T-03),
    /// which reads the same 8,192 bytes at a thousand events and at a hundred
    /// thousand.
    #[test]
    fn the_readers_constants_hold_the_relations_they_were_chosen_for() {
        // A single event's line, measured rather than guessed.
        let dir = scratch("const-relations");
        let sink = crate::Sink::open(&crate::Config::new(&dir)).expect("opens");
        assert!(
            sink.emit(&crate::Event::info("t", "a line of ordinary width"))
                .is_written()
        );
        let one_line = sink.health().current_bytes;
        assert!(one_line > 0);

        assert!(
            READ_BLOCK >= one_line * 16,
            "one read must cover many events or the walk re-reads: READ_BLOCK \
             {READ_BLOCK} against a {one_line}-byte line"
        );
        // A `const` block, because both sides are compile-time constants and a
        // runtime `assert!` over two of those is the constant assertion clippy
        // refuses — and it is right to: this one cannot fail at run time, so it
        // belongs where it CAN fail, which is the build.
        const {
            assert!(
                READ_BLOCK < crate::DEFAULT_MAX_FILE_BYTES,
                "the read block must be a fraction of a file, not the whole of one"
            );
        }

        assert_eq!(
            DEFAULT_MAX_SCAN_BYTES,
            crate::DEFAULT_MAX_FILE_BYTES,
            "its doc says 'one file's worth at the default bound' — that is the \
             claim, and this is it"
        );
        let window = u64::from(crate::DEFAULT_KEEP_FILES) * crate::DEFAULT_MAX_FILE_BYTES;
        assert!(
            DEFAULT_MAX_SCAN_BYTES < window,
            "a default walk must stop short of the whole {window}-byte window, \
             or the cap is not a cap"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **`since` IS INCLUSIVE, AND THE BOUNDARY RECORD IS THE LIKELY QUERY.**
    ///
    /// `Query::since` is documented as "only events at or after this instant".
    /// The walk ends when `record.at_unix_millis < floor`; mutating that to
    /// `<=` makes a record whose timestamp equals `since` EXACTLY terminate the
    /// walk instead of being returned — and an operator pasting a run's own
    /// start millisecond hits precisely that record. The mutant survived
    /// because no test used an exact boundary.
    #[test]
    fn since_returns_the_record_that_sits_exactly_on_the_boundary() {
        let dir = scratch("since-boundary");
        let sink = crate::Sink::open(&crate::Config::new(&dir)).expect("opens");
        for n in 0..6u32 {
            assert!(
                sink.emit(&crate::Event::info("t", "m").with("n", n))
                    .is_written()
            );
        }
        let all = tail(&dir, 1, &Query::last(MAX_LIMIT));
        assert_eq!(all.records.len(), 6, "the premise");

        // The stamp of a record in the middle, taken from the file itself.
        let pivot = all.records.get(2).expect("a third record").at_unix_millis;
        let from = tail(&dir, 1, &Query::last(MAX_LIMIT).since(pivot));
        assert!(
            from.records.iter().any(|r| r.at_unix_millis == pivot),
            "the record AT the boundary is included — `since` is 'at or after', \
             and the boundary is the instant an operator is most likely to paste"
        );
        assert!(
            from.records.iter().all(|r| r.at_unix_millis >= pivot),
            "and nothing older comes back"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A FILE THAT EXISTS AND CANNOT BE READ IS A LINE IN `errors`.**
    ///
    /// `tail`'s own doc promises it, and the module header rests the whole
    /// reader on it: "an unreadable file is a line in `Tail::errors`". The guard
    /// that separates "it was not there" (silent, and correct — the set is
    /// sparse until it has rolled) from "it was there and refused" survived
    /// mutation. Replaced with `true`, every error takes the silent `continue`:
    /// an unreadable file vanishes from the answer with no entry in `errors` and
    /// no flag on the page, and the reader then reports "nothing matched" for a
    /// log it could not read.
    ///
    /// A SELF-REFERENTIAL SYMLINK is how the error is produced: `metadata`
    /// follows links, so a link pointing at itself yields `ELOOP` — which is
    /// emphatically not `NotFound`. It needs no permission games and, unlike
    /// `chmod`, it behaves the same when the suite runs as root.
    #[test]
    fn a_file_that_exists_and_refuses_to_be_read_is_named_in_errors() {
        let dir = scratch("stat-refuses");
        std::fs::create_dir_all(&dir).expect("a scratch directory");

        // A real current file, so the walk has something to succeed at too.
        let sink = crate::Sink::open(&crate::Config::new(&dir)).expect("opens");
        assert!(sink.emit(&crate::Event::info("t", "readable")).is_written());

        // A rotated slot that exists and cannot be stat-ed.
        let loop_path = crate::rotated_path(&dir, 1);
        std::os::unix::fs::symlink(&loop_path, &loop_path).expect("a self-referential link");
        assert!(
            std::fs::symlink_metadata(&loop_path).is_ok(),
            "the premise: the path exists"
        );
        let refused = std::fs::metadata(&loop_path);
        assert!(
            refused
                .as_ref()
                .err()
                .is_some_and(|e| e.kind() != std::io::ErrorKind::NotFound),
            "and stat-ing it fails with something that is NOT absence: {refused:?}"
        );

        let found = tail(&dir, 2, &Query::last(MAX_LIMIT));
        assert_eq!(
            found.errors.len(),
            1,
            "the unreadable file is NAMED, not skipped in silence: {found:?}"
        );
        assert!(
            found.errors[0].contains("events.1"),
            "and named by its path: {:?}",
            found.errors
        );
        assert_eq!(
            found.records.len(),
            1,
            "while the readable file is still read — degrade loudly, not blankly"
        );

        // ABSENCE IS STILL SILENT, which is what the guard separates. A sparse
        // set is the ordinary state and must raise nothing.
        let quiet = scratch("stat-absent");
        std::fs::create_dir_all(&quiet).expect("a scratch directory");
        let sink = crate::Sink::open(&crate::Config::new(&quiet)).expect("opens");
        assert!(sink.emit(&crate::Event::info("t", "readable")).is_written());
        let found = tail(&quiet, 8, &Query::last(MAX_LIMIT));
        assert!(
            found.errors.is_empty(),
            "seven absent rotated files are not seven errors: {found:?}"
        );

        let _ignored = std::fs::remove_file(&loop_path);
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&quiet);
    }

    /// **A LOG WITH A HOLE IN IT SAYS SO, AND A FILTER DOES NOT CRY WOLF.**
    ///
    /// This is the first question a reader who did not run the job must ask.
    /// A log handed to somebody else — or to an agent asked to diagnose a
    /// failed trillion-combination run — is EVIDENCE, and evidence with silent
    /// holes is worse than none: every conclusion drawn from it is unsound in a
    /// way nothing on the page would show.
    ///
    /// `hit_scan_cap`, `partial_tail` and `malformed` each report a limit of the
    /// READER. This is the one number that reports a loss by the WRITER, and it
    /// works because `Sink::emit` assigns the sequence number BEFORE it tries to
    /// write and burns one on an event it then drops. The hole is the drop's own
    /// receipt.
    ///
    /// Measured once by accident: a live log lost 96 events between two reads
    /// twenty minutes apart while every other flag on every surface read clean.
    #[test]
    fn a_hole_in_the_sequence_is_counted_and_a_filter_is_not_mistaken_for_one() {
        let dir = scratch("completeness");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = crate::Sink::open(&crate::Config::new(&dir)).expect("opens");
        for n in 0..8u32 {
            let level = if n % 2 == 0 {
                crate::Level::Error
            } else {
                crate::Level::Info
            };
            assert!(
                sink.emit(&crate::Event::new(level, "t", "m").with("n", n))
                    .is_written()
            );
        }

        // WHOLE: nothing was dropped, so nothing is missing.
        let whole = tail(&dir, 1, &Query::last(MAX_LIMIT));
        assert_eq!(whole.records.len(), 8);
        assert_eq!(
            whole.missing,
            Some(0),
            "a complete log reports zero missing, not `None` — silence and \
             'nothing lost' are different answers"
        );

        // A FILTER IS NOT A HOLE. Half these records are Info, so an
        // Error-only view skips four — and must NOT report them as lost.
        let filtered = tail(
            &dir,
            1,
            &Query::last(MAX_LIMIT).at_least(crate::Level::Error),
        );
        assert_eq!(
            filtered.records.len(),
            4,
            "the premise: the filter skipped some"
        );
        assert_eq!(
            filtered.missing, None,
            "a filter skips records on purpose, so a gap says NOTHING about \
             loss and must not be reported as though it did"
        );
        assert_eq!(
            tail(&dir, 1, &Query::last(MAX_LIMIT).from_target("t")).missing,
            None,
            "and the same for a target filter"
        );

        // A REAL HOLE. Remove a line from the middle of the file, which is what
        // a dropped event leaves behind: a sequence number nothing holds.
        drop(sink);
        let path = crate::current_path(&dir);
        let text = std::fs::read_to_string(&path).expect("the log");
        let kept: Vec<&str> = text
            .lines()
            .filter(|line| !line.contains(r#""seq":4,"#))
            .collect();
        assert_eq!(kept.len(), 7, "the premise: exactly one line was removed");
        std::fs::write(&path, format!("{}\n", kept.join("\n"))).expect("rewrite");

        let holed = tail(&dir, 1, &Query::last(MAX_LIMIT));
        assert_eq!(holed.records.len(), 7);
        assert_eq!(
            holed.missing,
            Some(1),
            "one event was numbered and is not here, and the sequence is the \
             only thing that can still say so: {holed:?}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A SHARED LOG SPLITS BACK INTO THE RUNS THAT WROTE IT.**
    ///
    /// This is what a reader who did not run the job needs first, after knowing
    /// the file is whole. One `events.ndjson` holding three backfills is three
    /// interleaved stories, and without a run stamp it is one unreadable one:
    /// an agent handed the folder can see THAT something failed, not WHICH
    /// run's failure it was, nor how two overlapping runs ordered.
    ///
    /// The stamp lives on the sink rather than on every call site, for the same
    /// reason `emit` reads a global at all — a parameter threaded through every
    /// function between `main` and a note helper is one somebody forgets, and
    /// the site they forget is the one being diagnosed.
    #[test]
    fn a_log_holding_several_runs_splits_back_into_them() {
        let dir = scratch("run-identity");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = crate::Sink::open(&crate::Config::new(&dir)).expect("opens");

        // Outside any run: a served request, a startup line.
        assert!(
            sink.emit(&crate::Event::info("api.serve", "listening"))
                .is_written()
        );

        // Two runs, INTERLEAVED, because that is the case a reader cannot
        // untangle by eye and the one a timestamp cannot settle either.
        sink.set_run(7);
        assert!(
            sink.emit(&crate::Event::info("pull.run", "started"))
                .is_written()
        );
        sink.set_run(9);
        assert!(
            sink.emit(&crate::Event::info("pull.run", "started"))
                .is_written()
        );
        sink.set_run(7);
        assert!(
            sink.emit(&crate::Event::error("pull.member", "did not land"))
                .is_written()
        );
        sink.set_run(9);
        assert!(
            sink.emit(&crate::Event::info("pull.member", "landed"))
                .is_written()
        );
        sink.set_run(0);
        assert!(
            sink.emit(&crate::Event::info("api.serve", "idle"))
                .is_written()
        );

        let all = tail(&dir, 1, &Query::last(MAX_LIMIT));
        assert_eq!(all.records.len(), 6, "the premise: six events on one file");

        // AN EVENT OUTSIDE A RUN CARRIES NO RUN, and says so by omission.
        let outside = all.records.iter().filter(|r| r.run == 0).count();
        assert_eq!(outside, 2, "the two api.serve lines belong to no run");
        let raw = std::fs::read_to_string(crate::current_path(&dir)).expect("the log");
        let serve_line = raw
            .lines()
            .find(|l| l.contains("listening"))
            .expect("the line is on disk");
        assert!(
            !serve_line.contains(r#""run""#),
            "the key is OMITTED rather than written as zero, so a reader never \
             has to decide whether 0 is an identifier or an absence: {serve_line}"
        );

        // AND THE SPLIT ITSELF.
        let seven = tail(&dir, 1, &Query::last(MAX_LIMIT).from_run(7));
        assert_eq!(seven.records.len(), 2, "run 7 wrote two events");
        assert!(seven.records.iter().all(|r| r.run == 7));
        assert!(
            seven.records.iter().any(|r| r.message == "did not land"),
            "including its failure, which is the one a reader came for"
        );

        let nine = tail(&dir, 1, &Query::last(MAX_LIMIT).from_run(9));
        assert_eq!(nine.records.len(), 2, "run 9 wrote two");
        assert!(nine.records.iter().all(|r| r.run == 9));
        assert!(
            !nine.records.iter().any(|r| r.message == "did not land"),
            "and run 9's story does NOT contain run 7's failure — which is the \
             whole point, and what an interleaved file cannot otherwise give"
        );

        // A RUN FILTER IS A FILTER, so it must not be mistaken for loss.
        assert_eq!(
            seven.missing, None,
            "narrowing to one run skips the other's records on purpose"
        );
        assert_eq!(all.missing, Some(0), "while the unfiltered walk is whole");

        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A FILE OF ONE UNBROKEN RUN IS REFUSED, NOT ACCUMULATED.**
    ///
    /// `walk_back` keeps the bytes before a block's first newline as `carry`
    /// and joins them to the next block. When a block holds NO newline the whole
    /// block joins the carry, and the carry is copied every iteration — so a
    /// file whose "lines" are long cost O(bytes²) in time and grew without limit
    /// in space. Measured before the cap: **40 ms over 1 MiB and 2.91 s over
    /// 8 MiB**, 4.0x per doubling. After: 41 / 82 / 168 / 330 ms over
    /// 1 / 2 / 4 / 8 MiB — 2.0x per doubling, which is linear, and 8.8x faster
    /// at 8 MiB.
    ///
    /// It matters because the reader is handed files it did not write. A log
    /// folder given to somebody to diagnose may be truncated, concatenated,
    /// hand-edited or simply not ours, and a reader that exhausts memory on one
    /// is no use at the moment it is needed.
    ///
    /// The refusal is COUNTED, not silent: `malformed` is what this reader
    /// already says about bytes it stepped over, so an oversized run gets the
    /// same number and the same meaning.
    #[test]
    fn a_run_of_bytes_wider_than_any_line_is_refused_rather_than_accumulated() {
        let dir = scratch("no-newline");
        std::fs::create_dir_all(&dir).expect("a scratch directory");

        // Comfortably past the cap, so the carry logic must fire more than once.
        let unbroken = vec![b'x'; MAX_LINE_BYTES * 3];
        std::fs::write(crate::current_path(&dir), &unbroken).expect("write");

        let out = tail(&dir, 1, &Query::last(MAX_LIMIT));
        assert!(
            out.records.is_empty(),
            "none of it is a line, so none of it is a record"
        );
        assert!(
            out.malformed > 0,
            "and the reader SAYS it stepped over bytes rather than returning \
             an empty answer that looks like an empty log: {out:?}"
        );

        // A REAL LINE AFTER THE GARBAGE IS STILL FOUND. Refusing the run must
        // not blind the walk to everything past it.
        let mut mixed = unbroken.clone();
        mixed.extend_from_slice(b"\n");
        let sink_dir = scratch("no-newline-mixed");
        std::fs::create_dir_all(&sink_dir).expect("a scratch directory");
        let sink = crate::Sink::open(&crate::Config::new(&sink_dir)).expect("opens");
        assert!(
            sink.emit(&crate::Event::info("t", "after the garbage"))
                .is_written()
        );
        let good = std::fs::read(crate::current_path(&sink_dir)).expect("the good line");
        drop(sink);
        mixed.extend_from_slice(&good);
        std::fs::write(crate::current_path(&dir), &mixed).expect("write");

        let out = tail(&dir, 1, &Query::last(MAX_LIMIT));
        assert!(
            out.records.iter().any(|r| r.message == "after the garbage"),
            "a whole line after an oversized run is still returned: {out:?}"
        );

        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&sink_dir);
    }

    /// **A ROLL THAT LANDS MID-WALK RETURNED EVERY EVENT IN A FILE TWICE.**
    ///
    /// The walk opens one path, reads it, and opens the next. A roll landing in
    /// between renames the file it has just read onto the path it is about to
    /// open — so, keyed on the path, it read the same file a second time and
    /// returned the same events a second time. Measured with one emit thread
    /// and 2,000 tails at a 2 KiB file bound: **1,497 of 2,000 answers held a
    /// duplicate sequence number, up to 40 in one answer.** The first one seen,
    /// newest first, was `[8,7,6,5,4,3,2,1, 8,7,6,5,4,3,2,1]` with
    /// `files_read = 2`, `malformed = 0` and no errors.
    ///
    /// **A duplicate is a WRONG answer, which is worse than a missing one.**
    /// This reader is an evidence format (D-0088): a reader diagnosing a failed
    /// run counts the event twice, and nothing on the page says otherwise.
    /// Worse, the duplicate SILENCED the one flag that could have shown it —
    /// [`Tail::missing`] subtracts `records.len()` from the sequence span, so
    /// eight events returned twice made a span of 8 minus 16, which saturates
    /// to `Some(0)`: "this log is whole".
    ///
    /// A HARD LINK is the fixture, because it is exactly the state the rename
    /// leaves behind for the walk: one file, reachable at both paths, met once
    /// under each name. It needs no thread and no timing, so it cannot pass by
    /// luck on a slow machine.
    ///
    /// `bytes_read` is the sharp assertion. Skipping the repeat is not merely a
    /// filter over the records — the file's blocks are never read at all, which
    /// is why this fix is cheaper than refusing the records afterwards.
    #[test]
    fn a_roll_landing_mid_walk_never_returns_the_same_event_twice() {
        use std::os::unix::fs::MetadataExt as _;

        let dir = scratch("roll-mid-walk");
        let sink = filled(&dir, 8);
        drop(sink);
        let current = current_path(&dir);
        let rolled = rotated_path(&dir, 1);
        std::fs::hard_link(&current, &rolled).expect("a second name for one file");

        // THE PREMISE, STATED RATHER THAN ASSUMED: two paths, one file.
        let a = std::fs::metadata(&current).expect("stat");
        let b = std::fs::metadata(&rolled).expect("stat");
        assert_eq!(
            (a.dev(), a.ino()),
            (b.dev(), b.ino()),
            "the fixture is one file under two names, which is what a rename \
             leaves for a walk that is between two paths"
        );
        let size = a.len();

        let found = tail(&dir, 8, &Query::last(MAX_LIMIT));
        let seqs: Vec<u64> = found.records.iter().map(|r| r.seq).collect();
        assert_eq!(
            seqs,
            vec![8, 7, 6, 5, 4, 3, 2, 1],
            "each event ONCE, newest first: {found:?}"
        );
        assert_eq!(
            found.files_read, 1,
            "and the file counted once, not once per name it answers to"
        );
        assert_eq!(
            found.bytes_read, size,
            "the repeat cost NO BYTES — it is refused before it is read, not \
             filtered after: {found:?}"
        );
        assert_eq!(found.malformed, 0);
        assert!(
            found.errors.is_empty(),
            "a repeat is not a failure to report"
        );
        assert_eq!(
            found.missing,
            Some(0),
            "and the loss detector works again: a span of 8 over 8 records"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A RESTARTED SEQUENCE IS NOT A REPEAT, AND THIS IS WHY THE CHEAP RULE
    /// WAS REFUSED.**
    ///
    /// Two cheaper duplicate guards were considered and both are unsound, for
    /// the same reason this test builds:
    ///
    /// * *"sequence numbers arrive strictly decreasing within one answer, so a
    ///   number that does not decrease is a repeat and can be dropped with one
    ///   comparison."* They do not. `Sink::open` resumes the count from the
    ///   CURRENT file, and `resume_seq` documents that a wiped, absent or
    ///   corrupted tail restarts the numbering at zero. Walking backwards
    ///   across such a restart the numbers go 3, 2, 1 and then 6 — and that
    ///   rule would throw away every event written before the restart, which is
    ///   precisely the evidence a reader came for.
    /// * *"keep the set of sequence numbers already returned."* Sound against
    ///   the rename, but it discards the four DISTINCT records below whose
    ///   numbers collide with the new lifetime's, it pays per record instead of
    ///   per file, and it still reads and decodes every byte of the repeated
    ///   file before rejecting it.
    ///
    /// The file's own identity has neither problem: it is exact, it costs one
    /// comparison per file against at most `keep_files` entries, and it refuses
    /// the repeat before a byte is read.
    #[test]
    fn a_restarted_sequence_is_not_mistaken_for_a_repeat() {
        let dir = scratch("restart");
        let sink = filled(&dir, 6);
        drop(sink);
        // A ROLL BY HAND, then a sink opened on a set whose current file is
        // gone: the numbering restarts, exactly as `resume_seq` says it does.
        std::fs::rename(current_path(&dir), rotated_path(&dir, 1)).expect("roll");
        let sink = filled(&dir, 3);

        let found = tail(&dir, 8, &Query::last(MAX_LIMIT));
        let seqs: Vec<u64> = found.records.iter().map(|r| r.seq).collect();
        assert_eq!(
            seqs,
            vec![3, 2, 1, 6, 5, 4, 3, 2, 1],
            "nine DISTINCT events, of which three pairs share a number and none \
             of them is a repeat: {found:?}"
        );
        assert_eq!(found.files_read, 2, "two files, and both were read");
        // `missing` is the span from the newest number to the oldest, less the
        // records held; across a restart the newest number is SMALLER than
        // numbers further back, so the span is 3 and the answer saturates to
        // zero. That is a limit of the sequence, stated here so the next reader
        // of this test knows the number is not a claim about this fixture.
        assert_eq!(found.missing, Some(0));
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **The peek must still say `false` when an older file really does hold
    /// bytes** — otherwise the fix trades one wrong answer for its opposite.
    ///
    /// The page fills exactly on the NEWEST file's first line while a rolled file
    /// sits behind it. `reached_oldest` must be false, and the walk must not have
    /// paid to learn it: the older file is opened and stat'd, never read.
    #[test]
    fn a_page_that_fills_at_a_file_boundary_still_sees_the_rolled_file_behind_it() {
        let dir = scratch("boundary-with-roll");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = crate::Sink::open(
            &crate::Config::new(&dir)
                .with_max_file_bytes(crate::MIN_FILE_BYTES)
                .with_keep_files(4),
        )
        .expect("opens");
        for n in 0..60u32 {
            let _written = sink.emit(&crate::Event::info("t", "m").with("n", n));
        }
        assert!(
            crate::rotated_path(&dir, 1).exists(),
            "the premise: the set actually rolled"
        );

        // How many records the CURRENT file alone holds.
        let current_only = tail(&dir, 1, &Query::last(MAX_LIMIT));
        let n = current_only.records.len();
        assert!(n > 0, "the current file holds something");
        assert!(
            current_only.reached_oldest,
            "restricted to one file, that file was read to its start"
        );

        // Now ask for exactly that many across the whole set: the limit fills on
        // the current file's first line, and a rolled file remains behind it.
        let found = tail(&dir, sink.keep_files(), &Query::last(n));
        assert_eq!(found.records.len(), n);
        assert!(
            !found.reached_oldest,
            "a rolled file behind it still holds events, so the walk did NOT \
             reach the oldest: {found:?}"
        );
        assert_eq!(
            found.files_read, 1,
            "and it learned that by LOOKING, not by reading: the older file was \
             opened and stat'd, never walked"
        );
        assert_eq!(
            found.bytes_read, current_only.bytes_read,
            "not one extra byte was fetched to answer the question"
        );
    }

    /// **THE LIMIT REACHED ON THE FILE'S VERY FIRST LINE.**
    ///
    /// `walk_back` scans backwards for newlines, so the first line of the file
    /// is the one line with no newline before it — it is handled after the loop
    /// rather than inside it, by a `take_line` whose `true` return had never
    /// been taken. Every other test asks for more events than the file holds, so
    /// the walk always ran out of file before it ran out of limit.
    ///
    /// The arm matters because it is where "I gave you everything you asked
    /// for" is decided. If it were wrong the reader would either return one
    /// record too few at the exact moment the limit binds, or keep walking into
    /// a rolled file it did not need to open.
    #[test]
    fn a_limit_reached_on_the_first_line_of_the_file_ends_the_walk_there() {
        let dir = scratch("limit-on-first-line");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = crate::Sink::open(&crate::Config::new(&dir)).expect("opens");
        for n in 0..3u32 {
            assert!(
                sink.emit(&crate::Event::info("t", "m").with("n", n))
                    .is_written()
            );
        }
        drop(sink);

        // EXACTLY as many as the file holds, so the third one taken IS the
        // file's first line — the one the loop cannot reach.
        let exact = tail(&dir, 1, &Query::last(3));
        assert_eq!(
            exact.records.len(),
            3,
            "all three, and the last is line one"
        );
        assert_eq!(
            exact.records.last().map(|r| r.seq),
            Some(1),
            "the oldest returned is the file's first line: {exact:?}"
        );
        assert_eq!(exact.missing, Some(0), "and nothing is missing from them");
        // THE FLAG THIS FIXTURE ALWAYS COULD HAVE PINNED AND DID NOT.
        //
        // The page filled on the file's first line, every byte of every existing
        // file was read, and nothing older exists — so the walk DID reach the
        // oldest. This asserted nothing before, and the value was `false`:
        // `api::logs` rendered "Older events exist beyond what was read" on a
        // page where nothing older was there to read.
        assert!(
            exact.reached_oldest,
            "the page filled on the file's FIRST line and no older file exists, \
             so the walk reached the oldest: {exact:?}"
        );

        // One fewer: the walk stops before the first line, which is the other
        // side of the same decision.
        let short = tail(&dir, 1, &Query::last(2));
        assert_eq!(short.records.len(), 2);
        // AND THE OTHER DIRECTION, so a fix that simply always says `true`
        // cannot pass: this one stopped with a line still unread.
        assert!(
            !short.reached_oldest,
            "it stopped with the file's first line unread, so it did NOT reach \
             the oldest: {short:?}"
        );
        assert_eq!(
            short.records.last().map(|r| r.seq),
            Some(2),
            "it stopped at the limit rather than reading on: {short:?}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }
}

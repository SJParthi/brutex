//! The reader: the last N events, without reading the file.
//!
//! # What this costs, honestly
//!
//! The file is walked **backwards** from its end in [`READ_BLOCK`]-byte
//! blocks, and stops the moment it has what was asked for. So the cost of
//! "the last twenty events" is:
//!
//! * one `metadata` call per file touched, for its length;
//! * one `open` and one `seek` per file touched;
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
//! can match.
//!
//! # What a damaged line does
//!
//! It is counted in [`Tail::malformed`] and stepped over. One line that will
//! not decode never blanks the page around it — the same rule
//! `api::audit::Entry` follows for a record that fails its checksum.

use std::io::{Read as _, Seek as _, SeekFrom};
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
        match self.target {
            None => true,
            Some(ref want) => {
                record.target == *want
                    || (record.target.len() > want.len()
                        && record.target.starts_with(want.as_str())
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
    /// How many files were opened.
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
    /// Whether the walk reached the start of the oldest file it could find.
    pub reached_oldest: bool,
    /// Files that exist and could not be read, in the failure's own words.
    ///
    /// An absent file is **not** here: the set is sparse until it has rolled,
    /// and "no such file" is the ordinary state before the first event.
    pub errors: Vec<String>,
}

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
    let mut out = Tail {
        reached_oldest: true,
        ..Tail::default()
    };
    let limit = query.limit.min(MAX_LIMIT);
    if limit == 0 {
        return out;
    }
    let mut first_file = true;
    for path in paths_newest_first(dir, keep_files) {
        let len = match std::fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                continue;
            }
            Ok(meta) => meta.len(),
        };
        if len == 0 {
            continue;
        }
        let mut file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        out.files_read = out.files_read.saturating_add(1);
        let newest_file = core::mem::take(&mut first_file);
        if walk_back(&mut file, &path, len, newest_file, limit, query, &mut out) {
            out.reached_oldest = false;
            return out;
        }
    }
    out
}

/// Walks one file from its end. Returns whether the whole query is finished.
fn walk_back(
    file: &mut std::fs::File,
    path: &Path,
    len: u64,
    newest_file: bool,
    limit: usize,
    query: &Query,
    out: &mut Tail,
) -> bool {
    let mut pos = len;
    let mut carry: Vec<u8> = Vec::new();
    // The bytes after the final newline of the newest file are a line that is
    // not finished being written. Skipped and reported, never decoded.
    let mut drop_fragment = false;
    let mut first_block = true;

    while pos > 0 {
        if out.bytes_read >= query.max_scan_bytes {
            out.hit_scan_cap = true;
            return true;
        }
        let take = READ_BLOCK.min(pos);
        pos = pos.saturating_sub(take);
        let block = match read_block(file, pos, take) {
            Ok(block) => block,
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                return false;
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
                return true;
            }
        }
        carry = work.get(..end).unwrap_or(&[]).to_vec();
    }
    // The first line of the file has no newline before it.
    if !drop_fragment && take_line(&carry, limit, query, out) {
        return true;
    }
    false
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
    // THE ONE FILTER THAT ENDS THE WALK. Events are in time order and the walk
    // runs backwards, so the first one older than `since` guarantees every
    // remaining one is older still.
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
    use super::{DEFAULT_MAX_SCAN_BYTES, MAX_LIMIT, Query, READ_BLOCK, Tail, tail};
    use crate::event::Event;
    use crate::level::Level;
    use crate::sink::{Config, Sink, current_path, rotated_path};
    use crate::value::OwnedValue;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
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
}

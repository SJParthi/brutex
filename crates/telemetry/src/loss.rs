//! The loss ledger: what a sink could not write, kept where the next sink on
//! the same directory reads it.
//!
//! # Why a second file at all (sobs-2, D-4410)
//!
//! A sink numbers an event before it tries to write it, so an event the disk
//! refused burns its number, and [`crate::Tail::missing`] reads that hole as
//! the drop's own receipt. That receipt only exists while a LATER event lands
//! in the same file. A process whose disk stayed full until it exited left no
//! line above the hole, so the next process resumed the numbering from the
//! last line it could read and handed the lost numbers out again: measured by
//! the observability audit (probe P8), 185 events dropped, a restart at
//! `next_seq` 21 re-issuing 21..=205, `missing = Some(0)`, and the dropping
//! process's `Health::dropped` gone with it. The loss was reported nowhere.
//!
//! The log itself cannot carry the fact: the disk that refused the events
//! refuses a line about them too. So the fact is kept in a record of FIXED
//! LENGTH, made while the disk still has room — at [`crate::Sink::open`] —
//! and afterwards only overwritten in place. On a filesystem that rewrites a
//! file's existing blocks (ext4, xfs, tmpfs) an overwrite needs no new block,
//! so it lands on a disk that refuses every append. Measured on this tree on a
//! real 64 KiB tmpfs driven to ENOSPC (D-4410). A copy-on-write filesystem
//! (btrfs, ZFS, APFS) may need a new block for the overwrite as well; there the
//! write can fail, and that failure is named in the append failure's own
//! notice rather than assumed away.
//!
//! # What it holds
//!
//! One line, always [`RECORD_BYTES`] long:
//!
//! ```text
//! brutex telemetry loss v1 issued=<20 digits> lost=<20 digits> first=<20 digits> sum=<16 hex>
//! ```
//!
//! * `issued` — the highest sequence number a sink of this directory had
//!   handed out when it last recorded a drop. The next sink resumes at or above
//!   it, so a number is never issued twice.
//! * `lost` — events dropped and not yet reported in the log. The next sink
//!   that opens writes one `Error` event saying so and, only once that event
//!   has landed, sets this back to zero.
//! * `first` — the number of the first of those, zero when `lost` is zero.
//! * `sum` — FNV-1a over every byte before ` sum=`, so a torn or hand-edited
//!   record is refused by name rather than read as a smaller number.
//!
//! # Cost
//!
//! Nothing on the write path that lands. One positioned write of
//! [`RECORD_BYTES`] bytes on each DROPPED event, inside the emit lock that
//! already failed an append, and one read and possibly one write at open.
//! Constant, and paid only when something has already gone wrong.

use std::fs::{File, OpenOptions};
use std::io::Read as _;
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

/// The ledger's name, beside the set.
pub const LEDGER_NAME: &str = "events.loss";

/// Everything before the first number.
const HEAD: &[u8] = b"brutex telemetry loss v1 issued=";
/// Between `issued` and `lost`.
const LOST: &[u8] = b" lost=";
/// Between `lost` and `first`.
const FIRST: &[u8] = b" first=";
/// Between `first` and the checksum.
const SUM: &[u8] = b" sum=";
/// `u64::MAX` is twenty decimal digits.
const DIGITS: usize = 20;
/// A `u64` is sixteen hex digits.
const HEX: usize = 16;

/// Where the checksummed body ends and ` sum=` begins.
const BODY_BYTES: usize = HEAD.len() + DIGITS + LOST.len() + DIGITS + FIRST.len() + DIGITS;

/// The length of every record, newline included.
pub(crate) const RECORD_BYTES: usize = BODY_BYTES + SUM.len() + HEX + 1;

/// What the ledger says about this directory's lost events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Loss {
    /// The highest sequence number handed out as of the last recorded drop.
    pub(crate) issued: u64,
    /// Dropped events no log line has reported yet.
    pub(crate) lost: u64,
    /// The number of the first of them; zero when `lost` is zero.
    pub(crate) first: u64,
}

impl Loss {
    /// One more event, numbered `seq`, could not be written.
    pub(crate) fn dropped(&mut self, seq: u64) {
        if self.lost == 0 {
            self.first = seq;
        }
        self.lost = self.lost.saturating_add(1);
        self.issued = self.issued.max(seq);
    }

    /// Every loss up to now has been reported in the log, by the event
    /// numbered `seq`.
    pub(crate) const fn reported(self, seq: u64) -> Self {
        Self {
            issued: if seq > self.issued { seq } else { self.issued },
            lost: 0,
            first: 0,
        }
    }

    /// The record, exactly [`RECORD_BYTES`] long.
    #[must_use]
    pub(crate) fn render(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(RECORD_BYTES);
        out.extend_from_slice(HEAD);
        out.extend_from_slice(format!("{:020}", self.issued).as_bytes());
        out.extend_from_slice(LOST);
        out.extend_from_slice(format!("{:020}", self.lost).as_bytes());
        out.extend_from_slice(FIRST);
        out.extend_from_slice(format!("{:020}", self.first).as_bytes());
        let sum = fnv1a(&out);
        out.extend_from_slice(SUM);
        out.extend_from_slice(format!("{sum:016x}").as_bytes());
        out.push(b'\n');
        out
    }

    /// One record, or why these bytes are not one.
    ///
    /// # Errors
    ///
    /// A sentence naming what is wrong: the length, the head, a separator, a
    /// number, or a checksum that does not match the body.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != RECORD_BYTES {
            return Err(format!(
                "{} bytes where a loss record is exactly {RECORD_BYTES}",
                bytes.len()
            ));
        }
        let (body, tail) = bytes.split_at(BODY_BYTES);
        let mut at = Cursor { bytes: body, at: 0 };
        at.literal(HEAD, "the record's head")?;
        let issued = at.number()?;
        at.literal(LOST, "` lost=`")?;
        let lost = at.number()?;
        at.literal(FIRST, "` first=`")?;
        let first = at.number()?;
        let (marker, rest) = tail.split_at(SUM.len());
        if marker != SUM {
            return Err("no ` sum=` after the three numbers".to_owned());
        }
        let (hex, newline) = rest.split_at(HEX);
        if newline != b"\n" {
            return Err("the record does not end in a newline".to_owned());
        }
        let said = core::str::from_utf8(hex)
            .ok()
            .filter(|text| text.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|text| u64::from_str_radix(text, 16).ok())
            .ok_or_else(|| "the checksum is not sixteen hex digits".to_owned())?;
        let computed = fnv1a(body);
        if said != computed {
            return Err(format!(
                "the checksum says {said:016x} and the body sums to {computed:016x}: \
                 the record was torn or edited"
            ));
        }
        Ok(Self {
            issued,
            lost,
            first,
        })
    }
}

/// A reading position in a record body.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    /// The next bytes are exactly `want`, or the record is refused naming
    /// `what`.
    fn literal(&mut self, want: &[u8], what: &str) -> Result<(), String> {
        let end = self.at.saturating_add(want.len());
        if self.bytes.get(self.at..end) == Some(want) {
            self.at = end;
            Ok(())
        } else {
            Err(format!("{what} is not where a loss record keeps it"))
        }
    }

    /// The next twenty bytes as a decimal `u64`.
    fn number(&mut self) -> Result<u64, String> {
        let end = self.at.saturating_add(DIGITS);
        let digits = self.bytes.get(self.at..end).unwrap_or(&[]);
        self.at = end;
        // `u64::from_str` accepts a leading `+`; a record this crate wrote
        // never carries one, so every byte is checked to be a digit first.
        core::str::from_utf8(digits)
            .ok()
            .filter(|text| text.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|text| text.parse::<u64>().ok())
            .ok_or_else(|| "a number is not twenty decimal digits of a u64".to_owned())
    }
}

/// FNV-1a, 64-bit. A torn-write detector, not a defence against an attacker:
/// the file sits beside a log the same user can edit.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The ledger, open for the life of the sink that holds the directory.
#[derive(Debug)]
pub(crate) struct Ledger {
    file: File,
    path: PathBuf,
    /// What the file says, kept in step with every write that landed.
    pub(crate) loss: Loss,
}

/// What [`Ledger::open`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Found {
    /// A record was read.
    Read(Loss),
    /// No record was there, and one holding no loss was made.
    Made,
    /// The ledger is not usable as found, in its own words. Either the bytes
    /// were not a record — a new one replaces them, so what an earlier sink
    /// lost cannot be known — or the file could not be read or written.
    Unusable(String),
}

impl Ledger {
    /// Opens the ledger in `dir`, making it when it is not there.
    ///
    /// `resumed` is the sequence number the log itself resumes from, which a
    /// new record starts at. `None` when the file cannot be opened at all, with
    /// the reason in [`Found::Unusable`].
    pub(crate) fn open(dir: &Path, resumed: u64) -> (Option<Self>, Found) {
        let path = dir.join(LEDGER_NAME);
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
        {
            Ok(file) => {
                let found = hooked(Op::Read, || read_bounded(&file));
                Self::settle(file, path, resumed, found)
            }
            Err(e) => (
                None,
                Found::Unusable(format!(
                    "{}: the loss ledger cannot be opened — {e}",
                    path.display()
                )),
            ),
        }
    }

    /// Decides what an opened ledger holds, from what reading it gave.
    ///
    /// Split from [`Self::open`] so each answer a read can give is its own arm
    /// a test reaches with a real file: an empty one, a record, bytes that are
    /// not one, and a read that failed.
    fn settle(
        file: File,
        path: PathBuf,
        resumed: u64,
        found: std::io::Result<Vec<u8>>,
    ) -> (Option<Self>, Found) {
        let fresh = Loss {
            issued: resumed,
            lost: 0,
            first: 0,
        };
        let mut ledger = Self {
            file,
            path,
            loss: fresh,
        };
        let verdict = match found {
            // NOT TAKEN FOR AN EMPTY LEDGER. That would overwrite whatever an
            // earlier sink recorded, which is the one fact this file exists to
            // keep.
            Err(e) => Found::Unusable(format!(
                "{}: the loss ledger cannot be read — {e}; what an earlier sink lost \
                 is unknown",
                ledger.path.display()
            )),
            Ok(bytes) if bytes.is_empty() => match ledger.write(fresh) {
                Ok(()) => Found::Made,
                Err(why) => Found::Unusable(why),
            },
            Ok(bytes) => match Loss::parse(&bytes) {
                Ok(loss) => {
                    ledger.loss = loss;
                    Found::Read(loss)
                }
                Err(bad) => Found::Unusable(match ledger.write(fresh) {
                    Ok(()) => format!(
                        "{}: the loss ledger was not a record this crate wrote ({bad}); \
                         it has been replaced, so any loss an earlier sink recorded in it \
                         is unknown and its sequence numbers may be issued again",
                        ledger.path.display()
                    ),
                    Err(also) => format!(
                        "{}: the loss ledger was not a record this crate wrote ({bad}), \
                         and replacing it failed: {also}",
                        ledger.path.display()
                    ),
                }),
            },
        };
        (Some(ledger), verdict)
    }

    /// Overwrites the record with `loss`, and keeps it only if that landed.
    ///
    /// # Errors
    ///
    /// The failure in its own words, naming the ledger's path.
    pub(crate) fn write(&mut self, loss: Loss) -> Result<(), String> {
        let bytes = loss.render();
        let file = &self.file;
        // AT OFFSET ZERO, AND THEN THE LENGTH. The positioned write reuses the
        // blocks the record already has; `set_len` matters only when a longer
        // file was found and is being replaced, and costs nothing otherwise.
        let landed = hooked(Op::Write, || {
            file.write_all_at(&bytes, 0)
                .and_then(|()| file.set_len(u64::try_from(bytes.len()).unwrap_or(u64::MAX)))
        });
        self.named("written", landed)?;
        self.loss = loss;
        Ok(())
    }

    /// Records one dropped event numbered `seq`.
    ///
    /// # Errors
    ///
    /// The write's failure, from [`Self::write`]. The in-memory count moves
    /// only when the record did, so the two never disagree.
    pub(crate) fn note_drop(&mut self, seq: u64) -> Result<(), String> {
        let mut next = self.loss;
        next.dropped(seq);
        self.write(next)
    }

    /// Makes the record durable.
    ///
    /// # Errors
    ///
    /// The failure in its own words, naming the ledger's path.
    pub(crate) fn sync(&self) -> Result<(), String> {
        self.named("synced", hooked(Op::Sync, || self.file.sync_data()))
    }

    /// One failure, worded once for every operation.
    fn named(&self, what: &str, done: std::io::Result<()>) -> Result<(), String> {
        done.map_err(|e| {
            format!(
                "{}: the loss ledger could not be {what} — {e}",
                self.path.display()
            )
        })
    }
}

/// At most one byte more than a record, so a longer file is told apart from a
/// record without reading the whole of whatever it is.
fn read_bounded(mut file: &File) -> std::io::Result<Vec<u8>> {
    let mut found = Vec::with_capacity(RECORD_BYTES.saturating_add(1));
    let limit = u64::try_from(RECORD_BYTES.saturating_add(1)).unwrap_or(u64::MAX);
    file.by_ref().take(limit).read_to_end(&mut found)?;
    Ok(found)
}

/// The operations the ledger — and the sink's own directory sync — ask of the
/// disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    /// Reading the ledger at open.
    Read,
    /// Overwriting the ledger's record.
    Write,
    /// Syncing the ledger's record.
    Sync,
    /// Syncing the directory that holds the set (D-4411).
    Directory,
}

/// One disk operation, through the test fault injector.
///
/// The injector is compiled out of production and ADDS a branch under test
/// rather than replacing one, so the line that ships is the line every test
/// runs — the shape of `store::file::sync_hooked` (D-2740). It is what lets a
/// test see a ledger that cannot be read, written or synced, none of which a
/// developer's disk does on request.
pub(crate) fn hooked<T>(op: Op, real: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<T> {
    #[cfg(test)]
    if tests::fault_fires(op) {
        return Err(std::io::Error::other(format!("injected {op:?} fault")));
    }
    #[cfg(not(test))]
    let _ = op;
    real()
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
pub(crate) mod tests {
    use super::{Found, LEDGER_NAME, Ledger, Loss, Op, RECORD_BYTES, fnv1a};
    use std::cell::Cell;

    thread_local! {
        /// The one ledger operation to fail on this thread, once.
        static FAULT: Cell<Option<Op>> = const { Cell::new(None) };
    }

    /// Whether the armed fault is `op`; it fires once and disarms.
    pub(super) fn fault_fires(op: Op) -> bool {
        FAULT.with(|armed| {
            let fires = armed.get() == Some(op);
            if fires {
                armed.set(None);
            }
            fires
        })
    }

    /// Arms the next ledger write on this thread to fail.
    pub(crate) fn fail_next_write() {
        FAULT.with(|armed| armed.set(Some(Op::Write)));
    }

    /// Arms the next ledger sync on this thread to fail.
    pub(crate) fn fail_next_sync() {
        FAULT.with(|armed| armed.set(Some(Op::Sync)));
    }

    /// Arms the next directory sync on this thread to fail.
    pub(crate) fn fail_next_directory_sync() {
        FAULT.with(|armed| armed.set(Some(Op::Directory)));
    }

    /// Clears an armed fault that a test proved was never asked for.
    pub(crate) fn disarm() {
        FAULT.with(|armed| armed.set(None));
    }

    /// Arms the next ledger read on this thread to fail.
    pub(crate) fn fail_next_read() {
        FAULT.with(|armed| armed.set(Some(Op::Read)));
    }

    /// Whether a fault is still armed: a test that armed one asserts it fired.
    pub(crate) fn armed() -> bool {
        FAULT.with(Cell::get).is_some()
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        crate::tests::sweep_stale_scratch();
        let dir = std::env::temp_dir().join(format!(
            "brutex-telemetry-loss-{}-{name}",
            std::process::id()
        ));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the directory");
        dir
    }

    /// Every record is the same length, so an overwrite never grows the file,
    /// and every value a `u64` can hold round-trips — zero, one, and the top.
    #[test]
    fn a_record_is_fixed_length_and_every_extreme_round_trips() {
        for (issued, lost, first) in [
            (0, 0, 0),
            (1, 1, 1),
            (u64::MAX, u64::MAX, u64::MAX),
            (205, 185, 21),
            (u64::MAX, 0, 0),
        ] {
            let loss = Loss {
                issued,
                lost,
                first,
            };
            let bytes = loss.render();
            assert_eq!(bytes.len(), RECORD_BYTES, "{loss:?}");
            assert_eq!(Loss::parse(&bytes), Ok(loss));
        }
        assert_eq!(
            RECORD_BYTES, 127,
            "the length is arithmetic: 32 + 20 + 6 + 20 + 7 + 20 + 5 + 16 + 1"
        );
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(
            fnv1a(b"a"),
            0xaf63_dc4c_8601_ec8c,
            "the published FNV-1a vector"
        );
    }

    /// A record that is not one is refused by name, in every place it can be
    /// wrong — never read as a smaller number.
    #[test]
    fn a_damaged_record_is_refused_by_name_wherever_it_is_damaged() {
        let good = Loss {
            issued: 205,
            lost: 185,
            first: 21,
        }
        .render();
        let said = |bytes: &[u8]| Loss::parse(bytes).expect_err("refused");

        assert!(said(&good[..RECORD_BYTES - 1]).contains("bytes where"));
        let mut longer = good.clone();
        longer.push(b'\n');
        assert!(said(&longer).contains("bytes where"));

        let flip = |at: usize, to: u8| {
            let mut bad = good.clone();
            bad[at] = to;
            bad
        };
        assert!(said(&flip(0, b'B')).contains("head"));
        assert!(said(&flip(32 + 20 + 1, b'L')).contains("lost="));
        assert!(said(&flip(32 + 20 + 6 + 20 + 1, b'F')).contains("first="));
        // A digit replaced by a sign: `u64::from_str` would read `+` happily.
        assert!(said(&flip(32, b'+')).contains("twenty decimal digits"));
        assert!(said(&flip(32 + 20 + 6, b'x')).contains("twenty decimal digits"));
        assert!(said(&flip(32 + 20 + 6 + 20 + 7, b'-')).contains("twenty decimal digits"));
        // Twenty nines is past `u64::MAX`.
        let mut past = good.clone();
        past[32..52].copy_from_slice(b"99999999999999999999");
        assert!(said(&past).contains("twenty decimal digits"));
        // A digit changed inside the body: the checksum catches it.
        assert!(said(&flip(40, b'9')).contains("torn or edited"));
        assert!(said(&flip(RECORD_BYTES - 22, b'X')).contains("` sum=`"));
        assert!(said(&flip(RECORD_BYTES - 2, b'g')).contains("sixteen hex"));
        assert!(said(&flip(RECORD_BYTES - 2, b'+')).contains("sixteen hex"));
        assert!(said(&flip(RECORD_BYTES - 1, b' ')).contains("newline"));
        // And non-UTF-8 where a number goes.
        assert!(said(&flip(33, 0xff)).contains("twenty decimal digits"));
        assert!(said(&flip(RECORD_BYTES - 3, 0xff)).contains("sixteen hex"));
    }

    /// The first drop names itself as `first`; later ones only count, and the
    /// high-water never moves backwards. Reporting clears the count and keeps
    /// the high-water at or above the reporting event.
    #[test]
    fn drops_count_from_the_first_and_a_report_clears_only_the_count() {
        let mut loss = Loss {
            issued: 20,
            lost: 0,
            first: 0,
        };
        loss.dropped(21);
        loss.dropped(22);
        loss.dropped(7);
        assert_eq!(
            loss,
            Loss {
                issued: 22,
                lost: 3,
                first: 21
            }
        );
        assert_eq!(
            loss.reported(23),
            Loss {
                issued: 23,
                lost: 0,
                first: 0
            }
        );
        assert_eq!(
            loss.reported(5),
            Loss {
                issued: 22,
                lost: 0,
                first: 0
            },
            "a report numbered below the high-water keeps the high-water"
        );
        let mut top = Loss {
            issued: u64::MAX,
            lost: u64::MAX,
            first: 1,
        };
        top.dropped(u64::MAX);
        assert_eq!(top.lost, u64::MAX, "saturates rather than wrapping to zero");
        assert_eq!(top.first, 1);
    }

    /// Made when absent, read when present, replaced and named when damaged,
    /// and a longer file is cut back to one record rather than left with a
    /// tail that would refuse every later read.
    #[test]
    fn the_ledger_is_made_read_and_a_damaged_one_replaced_by_name() {
        let dir = scratch("open");
        let (made, found) = Ledger::open(&dir, 20);
        assert_eq!(found, Found::Made);
        let mut made = made.expect("a ledger");
        assert_eq!(made.path, dir.join(LEDGER_NAME));
        assert_eq!(
            std::fs::read(dir.join(LEDGER_NAME)).expect("the file"),
            Loss {
                issued: 20,
                lost: 0,
                first: 0
            }
            .render()
        );
        made.note_drop(21).expect("lands");
        made.note_drop(22).expect("lands");
        made.sync().expect("syncs");
        drop(made);

        let (read, found) = Ledger::open(&dir, 3);
        let want = Loss {
            issued: 22,
            lost: 2,
            first: 21,
        };
        assert_eq!(found, Found::Read(want));
        assert_eq!(read.expect("a ledger").loss, want);

        let mut long = Loss::default().render();
        long.extend_from_slice(&[b'x'; 300]);
        std::fs::write(dir.join(LEDGER_NAME), &long).expect("damaged");
        let (replaced, found) = Ledger::open(&dir, 9);
        let Found::Unusable(why) = found else {
            panic!("a damaged ledger is named: {found:?}")
        };
        assert!(why.contains("not a record this crate wrote"), "{why}");
        assert!(why.contains("may be issued again"), "{why}");
        assert_eq!(
            replaced.expect("a ledger").loss,
            Loss {
                issued: 9,
                lost: 0,
                first: 0
            }
        );
        assert_eq!(
            std::fs::read(dir.join(LEDGER_NAME))
                .expect("the file")
                .len(),
            RECORD_BYTES,
            "cut back to one record"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A ledger that cannot be opened is named and the sink carries on without
    /// one. A directory where the file goes refuses the open, as root too.
    #[test]
    fn a_ledger_that_cannot_be_opened_is_named() {
        let dir = scratch("refused");
        std::fs::create_dir_all(dir.join(LEDGER_NAME)).expect("a directory where the file goes");
        let (none, found) = Ledger::open(&dir, 1);
        assert!(none.is_none());
        let Found::Unusable(why) = found else {
            panic!("named: {found:?}")
        };
        assert!(why.contains("cannot be opened"), "{why}");
        assert!(why.contains(LEDGER_NAME), "names the path: {why}");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A write or a sync that cannot land is named, and the in-memory count
    /// stays where the file is — the two never disagree.
    #[test]
    fn a_write_or_sync_that_cannot_land_is_named_and_moves_nothing() {
        let dir = scratch("write-fault");
        let (ledger, found) = Ledger::open(&dir, 4);
        assert_eq!(found, Found::Made);
        let mut ledger = ledger.expect("a ledger");
        let before = std::fs::read(dir.join(LEDGER_NAME)).expect("the file");

        fail_next_write();
        let why = ledger.note_drop(5).expect_err("refused");
        assert!(!armed(), "the write asked the injector");
        assert!(why.contains("could not be written"), "{why}");
        assert!(why.contains("injected Write fault"), "{why}");
        assert!(why.contains(LEDGER_NAME), "names the path: {why}");
        assert_eq!(
            ledger.loss,
            Loss {
                issued: 4,
                lost: 0,
                first: 0
            },
            "nothing moved"
        );
        assert_eq!(
            std::fs::read(dir.join(LEDGER_NAME)).expect("the file"),
            before
        );

        fail_next_sync();
        let why = ledger.sync().expect_err("refused");
        assert!(why.contains("could not be synced"), "{why}");
        ledger.sync().expect("and the next one lands");

        // The fault fires once: the next drop lands and is counted.
        ledger.note_drop(5).expect("lands");
        assert_eq!(ledger.loss.lost, 1);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A read that fails is named and NOT taken for an empty ledger — which
    /// would overwrite whatever an earlier sink recorded. An empty file whose
    /// first record cannot be written, and damaged bytes whose replacement
    /// cannot be written, are named too.
    #[test]
    fn a_read_or_a_first_write_that_fails_is_named_and_overwrites_nothing() {
        let dir = scratch("read-fault");
        let path = dir.join(LEDGER_NAME);
        let kept = Loss {
            issued: 9,
            lost: 4,
            first: 6,
        }
        .render();
        std::fs::write(&path, &kept).expect("a record");
        fail_next_read();
        let (ledger, found) = Ledger::open(&dir, 1);
        assert!(!armed(), "the read asked the injector");
        let Found::Unusable(why) = found else {
            panic!("named: {found:?}")
        };
        assert!(why.contains("cannot be read"), "{why}");
        assert!(why.contains("unknown"), "{why}");
        assert!(
            ledger.is_some(),
            "kept, so a later drop can still be recorded"
        );
        assert_eq!(std::fs::read(&path).expect("the file"), kept, "untouched");

        std::fs::write(&path, b"").expect("emptied");
        fail_next_write();
        let (_ledger, found) = Ledger::open(&dir, 1);
        let Found::Unusable(why) = found else {
            panic!("named: {found:?}")
        };
        assert!(why.contains("could not be written"), "{why}");
        assert!(std::fs::read(&path).expect("the file").is_empty());

        std::fs::write(&path, b"garbage").expect("damaged");
        fail_next_write();
        let (_ledger, found) = Ledger::open(&dir, 1);
        let Found::Unusable(why) = found else {
            panic!("named: {found:?}")
        };
        assert!(why.contains("not a record this crate wrote"), "{why}");
        assert!(why.contains("replacing it failed"), "{why}");
        assert_eq!(std::fs::read(&path).expect("the file"), b"garbage");
        let _ignored = std::fs::remove_dir_all(&dir);
    }
}

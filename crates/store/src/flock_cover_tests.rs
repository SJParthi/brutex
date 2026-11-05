#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the lock registry's answers no other test reached. A
//! reader that outlives a patient writer's turn, a registry entry one side
//! leaves while the other still holds it, an absent registry, the
//! refusal sentence a formatter will not take, and two answers only a FRESH
//! process can show: a note-clearing failure said once per process, and a
//! turn that ends before this process recorded any lock. And (D-8561) the
//! host refusals no lock file makes on request, through the flock test
//! seam: a noted lock's refused `fstat`, a patient shared or exclusive lock
//! the host refuses, and a holder's note the host will not clear.

use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Write as _};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::tests::{HostCall, host_refusal_armed, refuse_next_host};
use super::{
    Flock, Held, Holder, Impatient, Side, Turn, TurnState, Turns, forget_in, key_of, note_unnoted,
    registry, whole, with_patience,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A lock file of this test's own, in a fresh directory.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-flock-cover-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("month.lock");
    File::create(&path).unwrap();
    path
}

/// Runs `body` in a child of this test binary that runs `test` alone, so the
/// body meets this process's statics as a fresh process has them: no lock
/// recorded, no note said, no telemetry sink installed. The parent asserts
/// the child ran exactly one test and passed. The environment is inherited,
/// so a coverage run's profile path reaches the child.
fn in_a_fresh_process(test: &str, body: impl FnOnce()) {
    const CHILD: &str = "BRUTEX_LS1_FRESH_PROCESS_CHILD";
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let (stdout, stderr) = (
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let said = format!("child {test} failed or ran no test:\n{stdout}\n{stderr}");
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{said}"
    );
}

/// A formatter sink that refuses its `refuse`-th write (from 0) and takes
/// every other one.
struct Refuses {
    refuse: usize,
    calls: usize,
    took: String,
}

impl fmt::Write for Refuses {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let call = self.calls;
        self.calls += 1;
        if call == self.refuse {
            return Err(fmt::Error);
        }
        self.took.push_str(s);
        Ok(())
    }
}

/// **A READER THAT OUTLIVES A PATIENT WRITER'S TURN KEEPS ITS ENTRY**: the
/// writer waits its bound beside a shared lock this process holds on the
/// same path, gives up naming the hold, and its turn's end clears the queue
/// but leaves the reader's registration; the reader's own end removes the
/// entry.
#[test]
fn a_reader_that_outlives_a_patient_writers_turn_keeps_its_entry() {
    let path = scratch("reader-outlives");
    let key = key_of(&path);
    let reader = Flock::try_lock_shared_patiently(File::open(&path).unwrap(), path.clone())
        .expect("nobody holds the month");
    let writer_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let refused = Flock::try_lock_patiently(
        writer_file,
        path.clone(),
        Duration::from_millis(20),
        "an LS1 test writer",
    );
    assert!(
        matches!(
            &refused,
            Err(Impatient::Held {
                ahead: 0,
                holder: Some(_),
                ..
            })
        ),
        "{refused:?}"
    );
    {
        let held = registry();
        let entry = held
            .as_ref()
            .and_then(|holders| holders.get(&key))
            .expect("the reader's entry outlives the writer's turn");
        assert_eq!(entry.readers, 1);
        assert!(entry.writer.is_none() && entry.queue.is_none());
    }
    drop(reader);
    assert!(
        registry()
            .as_ref()
            .is_none_or(|holders| !holders.contains_key(&key)),
        "the reader's end removes the entry"
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// **ONE SIDE LEAVING KEEPS AN ENTRY THE OTHER SIDE STILL HOLDS, AND AN
/// ABSENT REGISTRY HAS NOTHING TO FORGET**, asked of the registry's own
/// update directly.
#[test]
fn one_side_leaving_keeps_an_entry_the_other_still_holds() {
    let mut absent: Option<HashMap<u64, Held>> = None;
    forget_in(&mut absent, 7, Side::Writer);
    forget_in(&mut absent, 7, Side::Reader);
    assert!(absent.is_none(), "nothing is made to forget nothing");
    let mut held = Some(HashMap::from([(
        7,
        Held {
            writer: Some(Holder {
                thread: "an LS1 writer".to_owned(),
                since_unix_ms: 1,
                doing: None,
            }),
            readers: 1,
            queue: None,
        },
    )]));
    forget_in(&mut held, 7, Side::Reader);
    let entry = held
        .as_ref()
        .unwrap()
        .get(&7)
        .expect("the writer still holds");
    assert_eq!(entry.readers, 0);
    assert!(entry.writer.is_some());
    forget_in(&mut held, 7, Side::Writer);
    assert!(held.as_ref().unwrap().is_empty(), "nothing left: removed");
}

/// **A HOLD'S SENTENCE THAT A FORMATTER REFUSES ENDS AT THE REFUSAL**: the
/// holder's note is never written after a refused first piece, and a sink
/// that takes everything gets the whole sentence with the note last.
#[test]
fn a_hold_sentence_a_formatter_refuses_ends_at_the_refusal() {
    let held = Impatient::Held {
        bound: Duration::from_millis(20),
        waited: Duration::from_millis(21),
        attempts: 3,
        ahead: 1,
        holder: Some("pid 1 holds it".to_owned()),
    };
    let mut refused = Refuses {
        refuse: 0,
        calls: 0,
        took: String::new(),
    };
    assert!(write!(refused, "{held}").is_err());
    assert_eq!(refused.took, "", "nothing after the refused piece");
    let whole_sentence = held.to_string();
    assert!(
        whole_sentence.starts_with("still held after waiting 21 ms of a 20 ms bound")
            && whole_sentence.ends_with("; pid 1 holds it"),
        "{whole_sentence}"
    );
}

/// **A HOLDER'S NOTE THAT CANNOT BE CLEARED IS SAID ONCE PER PROCESS**: in a
/// fresh process with its own sink, two refusals on one lock file are one
/// `store.flock` line, naming the file and the first refusal.
#[test]
fn a_note_that_cannot_be_cleared_is_said_once_per_process() {
    in_a_fresh_process(
        "flock::cover_tests::a_note_that_cannot_be_cleared_is_said_once_per_process",
        || {
            let log = scratch("note-log").with_file_name("log");
            std::fs::create_dir_all(&log).unwrap();
            let sink = telemetry::install(
                &telemetry::Config::new(&log).with_min_level(telemetry::Level::Trace),
            )
            .expect("a fresh process has no sink");
            let lock = Path::new("/brutex-ls1/QNOTE/2031-02.lock");
            note_unnoted(lock, &io::Error::other("the first refusal"));
            note_unnoted(lock, &io::Error::other("the second refusal"));
            let found = telemetry::tail(
                &log,
                sink.keep_files(),
                &telemetry::Query::last(64).from_target("store.flock"),
            );
            assert_eq!(found.records.len(), 1, "{:?}", found.records);
            let line = &found.records[0];
            let field = |key: &str| {
                line.fields
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value.clone())
            };
            assert_eq!(
                field("file"),
                Some(telemetry::OwnedValue::Str(lock.display().to_string()))
            );
            assert_eq!(
                field("why"),
                Some(telemetry::OwnedValue::Str("the first refusal".to_owned()))
            );
        },
    );
}

/// **A TURN THAT ENDS BEFORE THIS PROCESS RECORDED ANY LOCK MAKES NO
/// REGISTRY**: in a fresh process, a served ticket ending hands the queue
/// on (nothing waits) and creates no registry to forget it in.
#[test]
fn a_turn_ended_before_any_lock_was_recorded_makes_no_registry() {
    in_a_fresh_process(
        "flock::cover_tests::a_turn_ended_before_any_lock_was_recorded_makes_no_registry",
        || {
            assert!(
                registry().is_none(),
                "nothing is recorded in a fresh process"
            );
            let turns = Arc::new(Turns {
                state: Mutex::new(TurnState {
                    next: 1,
                    serving: 0,
                    gone: BTreeSet::new(),
                }),
                moved: Condvar::new(),
            });
            drop(Turn {
                key: 7,
                turns: Arc::clone(&turns),
                ticket: 0,
            });
            assert!(registry().is_none(), "the turn's end made none");
            let state = whole(&turns.state);
            assert_eq!(
                (state.serving, state.next),
                (1, 1),
                "the ticket was handed on"
            );
        },
    );
}

/// The lock file opened to read and write, as a month's writer opens it.
fn writable(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap()
}

/// Whether this process records no writer of `key` (an absent registry or
/// entry records none).
fn no_writer_recorded(key: u64) -> bool {
    registry()
        .as_ref()
        .is_none_or(|holders| holders.get(&key).is_none_or(|entry| entry.writer.is_none()))
}

/// **A NOTED LOCK WHOSE `fstat` THE HOST REFUSES IS STILL TAKEN AND NOTED,
/// AND ITS END PUTS BACK NO TIME** (D-8561): the times a holder's note is
/// cleared back to are the ones the `fstat` read before the note. Refused
/// (the flock test seam, after the call ran), there are none, so the clear
/// leaves the modification time where its truncate put it. The control: an
/// `fstat` that answers puts the old time back.
#[test]
fn a_noted_lock_whose_fstat_is_refused_puts_back_no_time() {
    let path = scratch("noted-fstat");
    let old = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let make_old = || writable(&path).set_modified(old).unwrap();
    let modified = || std::fs::metadata(&path).unwrap().modified().unwrap();
    make_old();
    let held = Flock::try_lock_noted(writable(&path), path.clone()).expect("nobody holds it");
    assert!(
        std::fs::metadata(&path).unwrap().len() > 0,
        "the note is left"
    );
    held.release().unwrap();
    assert_eq!(modified(), old, "the answered fstat's time is put back");
    make_old();
    refuse_next_host(HostCall::Measure, io::ErrorKind::PermissionDenied);
    let held = Flock::try_lock_noted(writable(&path), path.clone())
        .expect("a refused fstat leaves the lock taken");
    assert!(!host_refusal_armed(), "the fstat asked the seam");
    assert!(
        std::fs::metadata(&path).unwrap().len() > 0,
        "the note is left all the same"
    );
    held.release().unwrap();
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0, "and cleared");
    assert_ne!(modified(), old, "no time was read, so none is put back");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// **A PATIENT SHARED LOCK THE HOST REFUSES IS REFUSED IN THE HOST'S KIND,
/// AT ONCE, RECORDING NOTHING** (D-8561): the refusal (the flock test seam,
/// before the call) is no holder, so it is never waited out, even with
/// patience to spend, and no reader is registered. The control: the next
/// attempt takes the lock.
#[test]
fn a_patient_shared_lock_the_host_refuses_is_the_hosts_error_at_once() {
    let path = scratch("shared-host");
    let key = key_of(&path);
    refuse_next_host(HostCall::Registered, io::ErrorKind::Unsupported);
    let refused = with_patience(Duration::from_secs(5), || {
        Flock::try_lock_shared_patiently(File::open(&path).unwrap(), path.clone())
    });
    match refused {
        Err(std::fs::TryLockError::Error(why)) => {
            assert_eq!(why.kind(), io::ErrorKind::Unsupported, "{why}");
        }
        other => panic!("the host's refusal, not a wait or a lock: {other:?}"),
    }
    assert!(!host_refusal_armed());
    assert!(
        registry()
            .as_ref()
            .is_none_or(|holders| holders.get(&key).is_none_or(|entry| entry.readers == 0)),
        "a refused attempt records no reader"
    );
    let held = Flock::try_lock_shared_patiently(File::open(&path).unwrap(), path.clone())
        .expect("the control takes it");
    held.release().unwrap();
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// **A PATIENT EXCLUSIVE LOCK THE HOST REFUSES IS `Impatient::Host`, AT
/// ONCE, RECORDING NO WRITER** (D-8561): within a five-second bound the
/// refusal (the flock test seam, before the call) is answered as the host's,
/// never as a hold waited out, and no holder is named. The control: the
/// next attempt takes the lock.
#[test]
fn a_patient_exclusive_lock_the_host_refuses_is_impatient_host() {
    let path = scratch("patient-host");
    let key = key_of(&path);
    refuse_next_host(HostCall::Registered, io::ErrorKind::Unsupported);
    let refused = Flock::try_lock_patiently(
        writable(&path),
        path.clone(),
        Duration::from_secs(5),
        "an LS1 test writer",
    );
    match refused {
        Err(Impatient::Host(why)) => assert_eq!(why.kind(), io::ErrorKind::Unsupported, "{why}"),
        other => panic!("the host's refusal, not a hold: {other:?}"),
    }
    assert!(!host_refusal_armed());
    assert!(no_writer_recorded(key), "a refused attempt records none");
    let held = Flock::try_lock_patiently(
        writable(&path),
        path.clone(),
        Duration::ZERO,
        "an LS1 test writer",
    )
    .expect("the control takes it");
    assert!(!no_writer_recorded(key), "the control is recorded");
    held.release().unwrap();
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// **A HOLDER'S NOTE WHOSE CLEAR THE HOST REFUSES IS SAID, AND THE LOCK IS
/// STILL LET GO** (D-8561): in a fresh process with its own sink, a noted
/// lock whose truncate-and-restore is refused as it ends (the flock test
/// seam, after the calls ran) says so once on `store.flock`, naming the lock
/// file and the refusal, and its release still answers `Ok`: the same file
/// locks again at once.
#[test]
fn a_note_the_host_will_not_clear_is_said_and_the_lock_still_let_go() {
    in_a_fresh_process(
        "flock::cover_tests::a_note_the_host_will_not_clear_is_said_and_the_lock_still_let_go",
        || {
            let path = scratch("note-clear");
            let log = path.with_file_name("log");
            std::fs::create_dir_all(&log).unwrap();
            let sink = telemetry::install(
                &telemetry::Config::new(&log).with_min_level(telemetry::Level::Trace),
            )
            .expect("a fresh process has no sink");
            let held =
                Flock::try_lock_noted(writable(&path), path.clone()).expect("nobody holds it");
            refuse_next_host(HostCall::Clear, io::ErrorKind::StorageFull);
            held.release()
                .expect("a refused clear is no refused unlock");
            assert!(!host_refusal_armed(), "the clear asked the seam");
            let found = telemetry::tail(
                &log,
                sink.keep_files(),
                &telemetry::Query::last(64).from_target("store.flock"),
            );
            assert_eq!(found.records.len(), 1, "{:?}", found.records);
            let line = &found.records[0];
            let field = |key: &str| {
                line.fields
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value.clone())
            };
            assert_eq!(
                field("file"),
                Some(telemetry::OwnedValue::Str(path.display().to_string()))
            );
            assert_eq!(
                field("why"),
                Some(telemetry::OwnedValue::Str(
                    "refused by the flock test seam".to_owned()
                ))
            );
            let again = Flock::try_lock(writable(&path), path.clone()).expect("it was let go");
            again.release().unwrap();
            let _ = std::fs::remove_dir_all(path.parent().unwrap());
        },
    );
}

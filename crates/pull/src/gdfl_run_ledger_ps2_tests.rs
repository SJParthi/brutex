#![cfg(test)]
//! BASE-COV-PULL-PS2 (fork B): the runs ledger's arms the suite left
//! unreached, each driven by the store it is asked of: two damaged last
//! records and a ledger behind its claims, each with an alert the host will
//! not take; a claims line too long to be a record's; claims that are gone;
//! a journal id at the largest an id can be; claims the host will not let
//! it append to; a journal it cannot open; and the longest record there is.
//! Every id and clock is invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt as _;

use super::*;
use crate::gdfl_fixtures::scratch;

/// One second the calendar admits: 2023-11-14.
const NOW: u64 = 1_700_000_000;

fn entry() -> Entry {
    Entry {
        id: 0,
        mode: "import".to_owned(),
        kind: "stocks".to_owned(),
        by: "cli".to_owned(),
        pid: 42,
        started: NOW,
        resumes: 0,
    }
}

/// A store whose `audit/alerts` is a directory, so no alert can be put.
fn alerts_refused(store: &Path) {
    std::fs::create_dir_all(store.join("audit").join("alerts")).unwrap();
}

/// Two damaged last records refuse the allocation by offset, and the alert
/// that could not be put leaves the queue's place as it was.
#[test]
fn two_damaged_last_records_refuse_and_an_alert_that_cannot_be_put_changes_nothing() {
    let store = scratch("ledger-ps2-damaged");
    let ledger = store.join(LEDGER);
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    std::fs::write(&ledger, [b'x'; 256]).unwrap();
    alerts_refused(&store);
    assert_eq!(
        allocate(&store, &entry()),
        Err(RunsLedgerFault::Damaged { offset: 0 })
    );
    assert!(store.join("audit").join("alerts").is_dir());
    assert_eq!(std::fs::read(&ledger).unwrap(), [b'x'; 256]);
    // Three bad records: the two at the tail, so the offset is the second
    // last's.
    std::fs::write(&ledger, [b'y'; 384]).unwrap();
    assert_eq!(
        allocate(&store, &entry()),
        Err(RunsLedgerFault::Damaged { offset: 128 })
    );
}

/// A ledger that lost records whole ends behind its claims: the new id is
/// raised past the claims' last, though the alert saying so cannot be put.
#[test]
fn a_ledger_behind_its_claims_raises_past_them_though_its_alert_cannot_be_put() {
    let store = scratch("ledger-ps2-behind");
    let ids: Vec<u64> = (0..3)
        .map(|_| allocate(&store, &entry()).unwrap())
        .collect();
    assert_eq!(ids, [NOW, NOW + 1, NOW + 2]);
    // The seed and the first record only: the claims name two ids past it.
    let ledger = store.join(LEDGER);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&ledger)
        .unwrap()
        .set_len(2 * RECORD)
        .unwrap();
    alerts_refused(&store);
    assert_eq!(allocate(&store, &entry()), Ok(NOW + 3));
    assert!(store.join("audit").join("alerts").is_dir());
}

/// A claims line too long to be a record's body is never read as an id:
/// counted damaged, and the ledger's own last record decides.
#[test]
fn a_claims_line_too_long_for_a_record_is_never_read() {
    let store = scratch("ledger-ps2-long-claim");
    assert_eq!(allocate(&store, &entry()), Ok(NOW));
    let claims = store.join(CLAIMS);
    let mut text = std::fs::read_to_string(&claims).unwrap();
    let _ = writeln!(text, "run {} crc=0badc0de", "9".repeat(200));
    std::fs::write(&claims, &text).unwrap();
    assert_eq!(allocate(&store, &entry()), Ok(NOW + 1));
    // Exactly at the bound plus one: still not read.
    let mut text = std::fs::read_to_string(&claims).unwrap();
    let _ = writeln!(text, "run {} crc=0badc0de", "9".repeat(BODY + 1));
    std::fs::write(&claims, &text).unwrap();
    assert_eq!(allocate(&store, &entry()), Ok(NOW + 2));
}

/// Claims that are gone read as no tail: the ledger's last record decides,
/// and the claims are written again from the next id.
#[test]
fn a_ledger_whose_claims_are_gone_reads_its_own_last_record() {
    let store = scratch("ledger-ps2-no-claims");
    assert_eq!(allocate(&store, &entry()), Ok(NOW));
    std::fs::remove_file(store.join(CLAIMS)).unwrap();
    assert_eq!(allocate(&store, &entry()), Ok(NOW + 1));
    let claims = std::fs::read_to_string(store.join(CLAIMS)).unwrap();
    assert_eq!(claims.lines().count(), 1);
    assert!(claims.starts_with(&format!("run {} ", NOW + 1)), "{claims}");
}

/// A damaged last record written over, past a journal whose largest run id
/// is the largest an id can be: no later id exists, so none is given and
/// nothing is written.
#[test]
fn a_journal_id_at_the_largest_an_id_can_be_refuses_the_allocation() {
    let store = scratch("ledger-ps2-journal-max");
    let ledger = store.join(LEDGER);
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    let first = Entry { id: 5, ..entry() }.record().unwrap();
    let mut bytes = first.to_vec();
    bytes.extend_from_slice(&[b'z'; 128]);
    std::fs::write(&ledger, &bytes).unwrap();
    let journal = crate::gdfl_import::journal_path(&store);
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    std::fs::write(&journal, format!("run {} x\n", u64::MAX)).unwrap();
    let refused = allocate(&store, &entry()).unwrap_err();
    assert_eq!(
        refused,
        RunsLedgerFault::Entry {
            why: format!(
                "the journal's largest run id is {}, the largest an id can be: no later id \
                 exists, so none was allocated and nothing was written",
                u64::MAX
            )
        }
    );
    assert_eq!(std::fs::read(&ledger).unwrap(), bytes);
    // One below the largest: the next is the largest, and it is given.
    std::fs::write(&journal, format!("run {} x\n", u64::MAX - 1)).unwrap();
    assert_eq!(allocate(&store, &entry()), Ok(u64::MAX));
}

/// Claims the host will not let the ledger append to: the id's record is
/// written, the id is not handed out, and the refusal says both.
#[test]
fn claims_the_host_will_not_append_to_refuse_the_id() {
    crate::support::where_permission_binds(
        "gdfl_run::ledger::ps2_tests::claims_the_host_will_not_append_to_refuse_the_id",
        || {
            let store = scratch("ledger-ps2-claims-ro");
            assert_eq!(allocate(&store, &entry()), Ok(NOW));
            let claims = store.join(CLAIMS);
            let before = std::fs::read(&claims).unwrap();
            std::fs::set_permissions(&claims, std::fs::Permissions::from_mode(0o444)).unwrap();
            let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
                panic!("not an io fault");
            };
            assert!(
                why.starts_with("its claims line was not written ("),
                "{why}"
            );
            assert!(why.contains("ermission denied"), "{why}");
            assert!(
                why.ends_with(&format!(
                    "); run {}'s record is in the ledger and the id was not handed out",
                    NOW + 1
                )),
                "{why}"
            );
            assert_eq!(std::fs::read(&claims).unwrap(), before);
            let records = read_all(&store).unwrap();
            assert_eq!(records.len(), 3, "the seed, the first id and this one");
        },
    );
}

/// A journal that will not open (a link back to itself) refuses an unborn
/// ledger's seed, naming the journal and the rule for it, which follows a
/// link (its own writer does); the claims' refusal adds that no link is
/// followed.
#[test]
fn a_journal_that_will_not_open_is_named_by_its_own_rule() {
    let store = scratch("ledger-ps2-journal-loop");
    let journal = crate::gdfl_import::journal_path(&store);
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&journal, &journal).unwrap();
    let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
        panic!("not an io fault");
    };
    assert!(
        why.starts_with(&format!("{}: ", journal.display())),
        "{why}"
    );
    assert!(
        why.ends_with("; its ids are read only from a regular file"),
        "{why}"
    );
    std::fs::remove_file(&journal).unwrap();
    let claims = store.join(CLAIMS);
    std::fs::create_dir_all(claims.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&claims, &claims).unwrap();
    let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
        panic!("not an io fault");
    };
    assert!(
        why.ends_with("; its ids are read only from a regular file, never through a link"),
        "{why}"
    );
}

/// The longest record any entry makes fits its body: the largest ids, pid
/// and clock, the longest words of every list, and a resume. So no entry is
/// refused for its length.
#[test]
fn the_longest_record_any_entry_makes_fits_its_body() {
    let longest = |words: &[&str]| {
        words
            .iter()
            .max_by_key(|word| word.len())
            .unwrap()
            .to_string()
    };
    let widest = Entry {
        id: u64::MAX,
        mode: longest(&MODES),
        kind: longest(&KINDS),
        by: longest(&BY),
        pid: u32::MAX,
        started: u64::MAX,
        resumes: u64::MAX,
    };
    let record = widest.record().unwrap();
    assert_eq!(Entry::of(&record), Some(widest.clone()));
    let body = std::str::from_utf8(&record[..BODY]).unwrap().trim_end();
    assert!(body.len() < BODY, "{} of {BODY}: {body}", body.len());
    assert!(body.ends_with(&format!(" resumes={}", u64::MAX)), "{body}");
}

/// Each alert the ledger could not put is said by an error line naming it
/// (the damaged offset, or the claims run it is behind) and the host's
/// refusal, read back from a sink of this child's own: the line is what
/// keeps the unsaved alert from being silent.
#[test]
fn an_alert_the_ledger_could_not_put_is_said_by_name() {
    crate::support::where_permission_binds(
        "gdfl_run::ledger::ps2_tests::an_alert_the_ledger_could_not_put_is_said_by_name",
        || {
            use telemetry::{Config, Level, MAX_LIMIT, OwnedValue, Query};
            let home = scratch("ledger-ps2-sink");
            let sink =
                telemetry::install(&Config::new(&home).with_min_level(Level::Trace)).unwrap();
            let not_put = |since: u64| {
                telemetry::tail(
                    &home,
                    sink.keep_files(),
                    &Query::last(MAX_LIMIT).from_target(TARGET),
                )
                .records
                .into_iter()
                .filter(|record| {
                    record.seq >= since && record.message == "runs ledger alert not put"
                })
                .collect::<Vec<_>>()
            };
            // Two damaged last records.
            let store = scratch("ledger-ps2-sink-damaged");
            let ledger = store.join(LEDGER);
            std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
            std::fs::write(&ledger, [b'x'; 256]).unwrap();
            alerts_refused(&store);
            let since = sink.health().next_seq;
            assert_eq!(
                allocate(&store, &entry()),
                Err(RunsLedgerFault::Damaged { offset: 0 })
            );
            let said = not_put(since);
            assert_eq!(said.len(), 1, "{said:?}");
            assert_eq!(
                said[0].field("offset").and_then(OwnedValue::as_u64),
                Some(0)
            );
            assert!(
                said[0]
                    .field("why")
                    .and_then(OwnedValue::as_str)
                    .is_some_and(|why| why.contains("alerts")),
                "{said:?}"
            );
            // A ledger behind its claims.
            let store = scratch("ledger-ps2-sink-behind");
            for _ in 0..3 {
                allocate(&store, &entry()).unwrap();
            }
            std::fs::OpenOptions::new()
                .write(true)
                .open(store.join(LEDGER))
                .unwrap()
                .set_len(2 * RECORD)
                .unwrap();
            alerts_refused(&store);
            let since = sink.health().next_seq;
            assert_eq!(allocate(&store, &entry()), Ok(NOW + 3));
            let said = not_put(since);
            assert_eq!(said.len(), 1, "{said:?}");
            let key = format!("runs_ledger_behind_claims:{}", NOW + 2);
            assert_eq!(
                said[0].field("key").and_then(OwnedValue::as_str),
                Some(key.as_str())
            );
            assert!(said[0].field("why").is_some(), "{said:?}");
        },
    );
}

/// Cuts the ledger at `at` to nothing: another writer's truncation.
fn cut(at: &std::path::Path) {
    std::fs::OpenOptions::new()
        .write(true)
        .open(at)
        .unwrap()
        .set_len(0)
        .unwrap();
}

/// A ledger cut after this allocation measured it (the seam after
/// `metadata`): the reads under the lock meet the end of the file, so the
/// two last records read as damaged and the allocation is refused by offset,
/// never an id from bytes that are not there; a one-record ledger cut so
/// reads as its only record lost, and the id is raised past the claims,
/// never one already given.
#[test]
fn a_ledger_cut_after_it_was_measured_never_gives_an_id_twice() {
    let store = scratch("ledger-ps2-cut");
    assert_eq!(allocate(&store, &entry()), Ok(NOW));
    assert_eq!(allocate(&store, &entry()), Ok(NOW + 1));
    let ledger = store.join(LEDGER);
    assert_eq!(std::fs::metadata(&ledger).unwrap().len(), 3 * RECORD);
    seam::plant("ledger.measured", cut);
    assert_eq!(
        allocate(&store, &entry()),
        Err(RunsLedgerFault::Damaged { offset: RECORD })
    );
    assert_eq!(
        std::fs::metadata(&ledger).unwrap().len(),
        0,
        "nothing written"
    );
    // One record, cut: its only record is read as lost.
    std::fs::write(&ledger, Entry { id: 9, ..entry() }.record().unwrap()).unwrap();
    seam::plant("ledger.measured", cut);
    assert_eq!(allocate(&store, &entry()), Ok(NOW + 2));
    assert_eq!(std::fs::metadata(&ledger).unwrap().len(), 2 * RECORD);
}

/// A LEDGER THE HOST CANNOT READ under the lock (a fault other than its end,
/// planted on the read of the last record) refuses the allocation in the
/// host's words: no id is given and no byte is written. The next allocation
/// reads the ledger as it was and goes on from its last id.
#[test]
fn a_ledger_the_host_cannot_read_gives_no_id_and_writes_nothing() {
    let store = scratch("ledger-ps2-read");
    assert_eq!(allocate(&store, &entry()), Ok(NOW));
    let ledger = store.join(LEDGER);
    let before = std::fs::read(&ledger).unwrap();
    seam::fail("ledger.read");
    assert_eq!(
        allocate(&store, &entry()),
        Err(RunsLedgerFault::Io {
            why: "invented: the host fault planted at ledger.read".to_owned(),
        })
    );
    assert_eq!(std::fs::read(&ledger).unwrap(), before, "nothing written");
    assert_eq!(allocate(&store, &entry()), Ok(NOW + 1));
}

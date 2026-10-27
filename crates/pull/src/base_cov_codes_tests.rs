#![cfg(test)]
//! BASE-COV-PULL (D-7332): every refusal code, state word and reason word of
//! the crate that no test had named on 27 Oct 2026 (Route B,
//! `state/fix/round2/base-cov-pull-gaps-a1.txt`). A code is a contract: pages,
//! events, the journal and `/logs` carry it, and it is "never renamed or
//! reused". So each table below states every variant's word, the ones already
//! covered included, and checks the words of one table are distinct (two
//! variants under one word would be one refusal to a reader). Values are
//! invented; nothing here is vendor data.

#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::HashSet;
use std::path::PathBuf;

use crate::session::Day;

/// Every word of `table` is the one stated, and no word is used twice.
fn words(table: &[(&'static str, &'static str)], shared: &[&str]) {
    let mut seen = HashSet::new();
    for &(got, want) in table {
        assert_eq!(got, want);
        assert!(
            seen.insert(got) || shared.contains(&got),
            "the word {got} names two variants"
        );
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

#[test]
fn every_alert_error_has_its_own_code() {
    use crate::alerts::AlertError as E;
    let p = PathBuf::from("/invented");
    words(
        &[
            (
                E::Unavailable { root: p.clone() }.code(),
                "alerts_unavailable",
            ),
            (E::KindUnknown { kind: 255 }.code(), "alert_kind_unknown"),
            (E::KeyEmpty.code(), "alert_key_empty"),
            (E::Damaged { offset: 0 }.code(), "alerts_damaged"),
            (
                E::RecordDamaged {
                    ordinal: u32::MAX,
                    offset: u64::MAX,
                }
                .code(),
                "alert_record_damaged",
            ),
            (
                E::VersionUnknown { found: u16::MAX }.code(),
                "alerts_version_unknown",
            ),
            (E::NotAFile { path: p }.code(), "alerts_not_a_file"),
            (E::NoSuchAlert { ordinal: 0 }.code(), "no_such_alert"),
            (
                E::AlreadyAcknowledged { ordinal: 1 }.code(),
                "already_acknowledged",
            ),
            (
                E::AckOutsideWindow { ordinal: 2 }.code(),
                "ack_outside_window",
            ),
            (E::NotDamaged.code(), "alerts_not_damaged"),
            (E::Full.code(), "alerts_full"),
        ],
        &[],
    );
}

#[test]
fn every_fno_membership_day_status_has_a_word_and_a_reason() {
    use crate::fno_membership::DayStatus as S;
    let table = [
        (S::Known, "known", "known"),
        (
            S::NotSession,
            "not_a_session",
            "not a session (the IST calendar says closed)",
        ),
        (
            S::NoSource,
            "no_source",
            "no GDFL options day file for this session",
        ),
        (
            S::SourceRefused,
            "source_refused",
            "the GDFL options day file was refused",
        ),
        (
            S::NoCalendar,
            "no_calendar",
            "the IST calendar does not cover this day",
        ),
        (
            S::NoShareOptions,
            "no_share_options",
            "the options day file lists no share option",
        ),
        (
            S::SourceOnClosedDay,
            "source_on_closed_day",
            "an options day file exists on a closed day",
        ),
    ];
    assert_eq!(table.len(), S::ALL.len(), "every status is in the table");
    for (status, _, _) in table {
        assert!(S::ALL.contains(&status));
    }
    words(&table.map(|(s, w, _)| (s.word(), w)), &[]);
    words(&table.map(|(s, _, r)| (s.reason(), r)), &[]);
}

#[test]
fn the_stored_index_absent_refusal_has_its_own_code() {
    use crate::gdfl_import::ImportRefusal;
    let refusal = ImportRefusal::StoredIndexAbsent {
        stem: "INVENTED".to_owned(),
    };
    assert_eq!(refusal.code(), "stored_index_absent");
    assert!(refusal.to_string().contains("INVENTED"), "{refusal}");
}

#[test]
fn every_run_state_has_its_own_word() {
    use crate::gdfl_run::RunState as R;
    words(
        &[
            (R::Running.as_str(), "running"),
            (R::Stopping.as_str(), "stopping"),
            (R::Finished.as_str(), "finished"),
            (R::Stopped.as_str(), "stopped"),
            (R::Shutdown.as_str(), "shutdown"),
            (R::SourceLost.as_str(), "source_lost"),
            (R::DiskFull.as_str(), "disk_full"),
            (R::Refused.as_str(), "refused"),
            (R::Census.as_str(), "census"),
            (R::Durability.as_str(), "durability"),
        ],
        &[],
    );
}

#[test]
fn every_lock_and_ledger_fault_has_its_own_code() {
    use crate::gdfl_run::ledger::RunsLedgerFault as L;
    use crate::gdfl_run::locks::LockFault as F;
    let lock = || "imports/invented.lock".to_owned();
    let faults = [
        (
            F::Busy {
                lock: lock(),
                holder: String::new(),
            },
            "run_busy",
        ),
        (
            F::Refused {
                lock: lock(),
                why: "invented".to_owned(),
            },
            "run_lock_refused",
        ),
        (F::Replaced { lock: lock() }, "lock_replaced"),
        (
            F::Stalled {
                lock: lock(),
                waited_ms: u64::MAX,
            },
            "lock_stalled",
        ),
        (
            F::Layout {
                why: "invented".to_owned(),
            },
            "census_unreadable",
        ),
    ];
    words(&faults.each_ref().map(|(f, w)| (f.code(), *w)), &[]);
    // A ledger's lock fault is named by the lock's own code.
    for (fault, want) in faults {
        assert_eq!(L::Lock(fault).code(), want);
    }
    words(
        &[
            (L::Damaged { offset: 7 }.code(), "runs_ledger_damaged"),
            (
                L::ClockImplausible {
                    now: 0,
                    why: "invented".to_owned(),
                }
                .code(),
                "clock_implausible",
            ),
            (L::Io { why: String::new() }.code(), "runs_ledger_io"),
            (L::Entry { why: String::new() }.code(), "runs_ledger_entry"),
        ],
        &[],
    );
}

#[test]
fn every_start_gate_and_stop_refusal_has_its_own_word() {
    use crate::gdfl_run::start::Gate as G;
    use crate::gdfl_run::stop::StopRefusal as S;
    words(
        &[
            (G::Go.as_str(), "go"),
            (G::Release.as_str(), "release"),
            (G::Silent.as_str(), "silent"),
        ],
        &[],
    );
    words(
        &[
            (
                S::NotLive {
                    asked: String::new(),
                }
                .code(),
                "run_not_live",
            ),
            (
                S::ClaimBeingWritten {
                    lock: String::new(),
                }
                .code(),
                "claim_being_written",
            ),
            (
                S::ByUnknown {
                    by: "nobody".to_owned(),
                }
                .code(),
                "stop_by_unknown",
            ),
            (
                S::NotStoppable {
                    lock: String::new(),
                    run: u64::MAX,
                    mode: "census-recount".to_owned(),
                }
                .code(),
                "run_not_stoppable",
            ),
            (S::Unwritten { why: String::new() }.code(), "stop_unwritten"),
        ],
        &[],
    );
}

#[test]
fn every_snapshot_answer_has_its_own_word() {
    use crate::master_snapshots::{Stamp, TokenOn as T, Wrote};
    let stamp = Stamp {
        day: day(2031, 2, 3),
        ordinal: 2,
    };
    assert_eq!(
        (Wrote::New(stamp).word(), Wrote::New(stamp).stamp()),
        ("new", stamp)
    );
    assert_eq!(
        (
            Wrote::AlreadyHeld(stamp).word(),
            Wrote::AlreadyHeld(stamp).stamp()
        ),
        ("already_held", stamp)
    );
    words(
        &[
            (
                T::Ambiguous {
                    older: stamp,
                    newer: stamp,
                }
                .word(),
                "token_ambiguous",
            ),
            (T::NotListed { snapshot: stamp }.word(), "token_not_listed"),
            (T::Duplicated { snapshot: stamp }.word(), "token_duplicated"),
            (
                T::BeforeFirst { first: stamp }.word(),
                "before_first_snapshot",
            ),
            (T::NoSnapshot.word(), "no_snapshot"),
            (
                T::Unreadable {
                    snapshot: stamp,
                    why: "invented".to_owned(),
                }
                .word(),
                "snapshot_unreadable",
            ),
        ],
        &[],
    );
}

#[test]
fn every_reference_standing_and_refusal_kind_has_its_own_word() {
    use crate::reference_data::{RefusalKind as K, Standing as S};
    words(
        &[
            (S::Sourced.word(), "sourced"),
            (S::Measured.word(), "measured"),
            (S::Unverified.word(), "unverified"),
        ],
        &[],
    );
    words(
        &[
            (K::Read.word(), "read"),
            (K::TooLarge.word(), "too_large"),
            (K::NotText.word(), "not_text"),
            (K::Magic.word(), "magic"),
            (K::Version.word(), "version"),
            (K::WrongTable.word(), "wrong_table"),
            (K::Generation.word(), "generation"),
            (K::Header.word(), "header"),
            (K::Columns.word(), "columns"),
            (K::Row.word(), "row"),
            (K::Duplicate.word(), "duplicate"),
            (K::RowCount.word(), "row_count"),
            (K::Checksum.word(), "checksum"),
            (K::TooMany.word(), "too_many"),
            (K::Absent.word(), "absent"),
        ],
        &[],
    );
}

#[test]
fn every_refusal_axis_repair_gap_and_claim_standing_has_its_own_word() {
    use crate::ingest::repair_plan::Missing as M;
    use crate::refusal::Axis as A;
    use crate::vendor::ClaimStanding as C;
    words(
        &[
            (A::Status.word(), "status"),
            (A::ErrorName.word(), "error_name"),
        ],
        &[],
    );
    words(
        &[
            (M::Front.code(), "front"),
            (M::Interior.code(), "interior"),
            (M::Tail.code(), "tail"),
            (M::Absent.code(), "absent"),
            (M::BeforeFirst.code(), "before_first"),
        ],
        &[],
    );
    words(
        &[
            (C::VendorDocument.word(), "vendor_doc"),
            (C::OperatorObservation.word(), "operator"),
        ],
        &[],
    );
}

#[test]
fn every_rehearsal_refusal_has_its_own_code_and_part() {
    use crate::rehearsal::{Refusal as R, Stamp};
    let kind = || "invented-kind".to_owned();
    let stamp = || Stamp {
        name: "0123abcd".to_owned(),
        unix: i64::MIN,
    };
    let table = [
        (
            R::Stale {
                kind: kind(),
                rehearsed: stamp(),
                writer: stamp(),
            },
            "stale",
            "rehearsed_commit_older_than_writer_changed",
        ),
        (
            R::NotReady {
                kind: kind(),
                waits_for: Vec::new(),
            },
            "not_ready",
            "prerequisites",
        ),
        (
            R::OperatorPending {
                kind: kind(),
                step: String::new(),
                items: vec!["one".to_owned()],
            },
            "operator_pending",
            "operator",
        ),
        (
            R::StepClosed {
                kind: kind(),
                step: String::new(),
                state: "closed".to_owned(),
            },
            "step_closed",
            "state",
        ),
        (
            R::NeverAutomatic {
                kind: kind(),
                word: "invented",
            },
            "never_automatic",
            "kind",
        ),
        (
            R::ListUnreadable {
                kind: kind(),
                why: String::new(),
            },
            "list_unreadable",
            "list",
        ),
    ];
    words(&table.each_ref().map(|(r, c, _)| (r.code(), *c)), &[]);
    words(&table.each_ref().map(|(r, _, p)| (r.part(), *p)), &[]);
}

#[test]
fn every_keep_reason_and_keep_error_has_its_own_word() {
    use crate::response_keep::{KeepError as K, Reason as R};
    words(
        &[
            (R::Kept.word(), "kept"),
            (R::PackFull.word(), "pack_full"),
            (R::StorageFull.word(), "storage_full"),
            (R::DuplicateBody.word(), "duplicate_body"),
            (R::LaterMonth.word(), "later_month"),
            (R::RefusedAnswer.word(), "refused_answer"),
        ],
        &[],
    );
    let p = || PathBuf::from("/invented");
    words(
        &[
            (
                K::Path(store::path::PathError::EmptySegment { field: "symbol" }).word(),
                "path_refused",
            ),
            (K::ReceiptsFull { pack: p() }.word(), "receipts_full"),
            (
                K::LockStalled {
                    lock: p(),
                    waited_ms: 0,
                }
                .word(),
                "lock_stalled",
            ),
            (K::NotRegular { path: p() }.word(), "not_a_regular_file"),
            (K::WindowsFull { journal: p() }.word(), "windows_full"),
            (K::TooLarge { len: u64::MAX }.word(), "source_refused"),
            (
                K::BodyMismatch {
                    path: p(),
                    what: "length",
                }
                .word(),
                "body_mismatch",
            ),
            (
                K::NotKept {
                    reason: R::LaterMonth,
                }
                .word(),
                "body_not_kept",
            ),
            (
                K::Io {
                    op: "open",
                    path: p(),
                    kind: std::io::ErrorKind::PermissionDenied,
                }
                .word(),
                "io",
            ),
        ],
        &[],
    );
}

#[test]
fn every_spool_error_has_its_own_code() {
    use crate::zerodha_verify::spool::SpoolError as S;
    let p = || PathBuf::from("/invented");
    words(
        &[
            (
                S::Damaged {
                    path: p(),
                    record: 0,
                    why: "crc",
                }
                .code(),
                "spool_damaged",
            ),
            (
                S::Header {
                    path: p(),
                    why: String::new(),
                }
                .code(),
                "spool_header_refused",
            ),
            (
                S::LogDamaged {
                    path: p(),
                    record: u64::MAX,
                    why: "crc",
                }
                .code(),
                "merge_log_damaged",
            ),
            (S::Record { why: String::new() }.code(), "record_refused"),
            (S::NotRegular { path: p() }.code(), "not_regular"),
        ],
        &[],
    );
}

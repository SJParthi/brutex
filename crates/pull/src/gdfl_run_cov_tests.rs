//! BASE-COV-PULL: the words, bytes and faults of a GDFL run that no test had
//! reached (Route B, recipe v4): a progress record's bytes and a refusal's
//! sentence are contracts with `/ingest` and with a resumed run.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::PathBuf;

use super::tests::sample;
use super::*;
use crate::gdfl_fixtures::scratch;

#[test]
fn every_byte_of_a_progress_record_names_one_thing() {
    assert_eq!(
        [
            SourceId::CmZip.code(),
            SourceId::NfoZips.code(),
            SourceId::TickStore.code()
        ],
        [1, 2, 3]
    );
    assert_eq!([By::Cli.code(), By::Api.code()], [1, 2]);
    assert_eq!(By::Api.as_str(), "api");
    assert_eq!(
        ImportKind::ALL.map(kind_code),
        [1, 2, 3],
        "indices, stocks, options"
    );
    let bytes: Vec<u8> = RunState::ALL.into_iter().map(RunState::code).collect();
    assert_eq!(bytes, (1..=10).collect::<Vec<u8>>());
    for state in RunState::ALL {
        assert_eq!(RunState::of_code(state.code()), Some(state));
    }
    assert_eq!(RunState::of_code(0), None);
    assert_eq!(RunState::of_code(11), None);
    assert_eq!(RunState::of_code(u8::MAX), None);
}

#[test]
fn a_set_of_kinds_is_never_empty() {
    assert_eq!(Kinds::one(ImportKind::Stocks).len(), 1);
    assert!(!Kinds::one(ImportKind::Options).is_empty());
    let refused = Kinds::of(&[]).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "request_line_invalid: the request line is not one: no kind is named"
    );
    assert_eq!(
        Kinds::of(&ImportKind::ALL).unwrap().len(),
        ImportKind::ALL.len()
    );
}

#[test]
fn a_day_of_any_other_shape_is_refused_by_name() {
    assert_eq!(day_of("2024-03-01"), Ok(Day::new(2024, 3, 1).unwrap()));
    for bad in [
        "2024-03",
        "2024-03-01-02",
        "24-03-01",
        "2024-3-01",
        "2024-03-1",
        "",
        "--",
    ] {
        assert!(
            day_of(bad).unwrap_err().contains("is not a YYYY-MM-DD day"),
            "{bad}"
        );
    }
    assert!(day_of("2024-13-01").is_err());
    assert!(day_of("2024-99-99").is_err(), "digits, and no such day");
}

#[test]
fn a_head_that_runs_on_past_its_word_is_refused() {
    let line = format!("{REQUEST_HEAD}x a b");
    let refused = Request::from_line(&line).unwrap_err();
    assert!(refused.to_string().contains("the head is not"), "{refused}");
}

#[test]
fn every_lock_fault_is_the_refusal_a_page_names() {
    use locks::LockFault as F;
    let lock = || "imports/invented.lock".to_owned();
    let replaced: RunRefusal = F::Replaced { lock: lock() }.into();
    assert_eq!(
        replaced.to_string(),
        "lock_replaced: imports/invented.lock was removed or replaced after it was opened, so a \
         lock on it excludes no one; nothing was written (no command deletes a lock file)"
    );
    let stalled: RunRefusal = F::Stalled {
        lock: lock(),
        waited_ms: u64::MAX,
    }
    .into();
    assert!(
        stalled.to_string().contains(&u64::MAX.to_string()),
        "{stalled}"
    );
    let layout: RunRefusal = F::Layout {
        why: "an invented layout".to_owned(),
    }
    .into();
    assert_eq!(layout.code(), "census_unreadable");
    assert_eq!(layout.to_string(), "census_unreadable: an invented layout");
    let refused: RunRefusal = F::Refused {
        lock: lock(),
        why: "an invented refusal".to_owned(),
    }
    .into();
    assert_eq!(
        refused.to_string(),
        "run_lock_refused: imports/invented.lock: an invented refusal"
    );
    let busy = RunRefusal::RunBusy {
        lock: lock(),
        holder: String::new(),
    };
    assert!(busy.to_string().contains("no claim names it yet"), "{busy}");
}

#[test]
fn a_damaged_ledger_stops_every_run_by_its_offset() {
    let damaged = RunRefusal::RunsLedgerDamaged { offset: u64::MAX };
    assert!(
        damaged.to_string().contains("no run starts until"),
        "{damaged}"
    );
    assert!(
        damaged.to_string().contains(&u64::MAX.to_string()),
        "{damaged}"
    );
    let reason = RunRefusal::RepairReasonInvalid { bytes: 0 };
    assert!(reason.to_string().contains("(0 bytes)"), "{reason}");
    for why in ["an invented why"] {
        for refusal in [
            RunRefusal::Unstamped {
                why: why.to_owned(),
            },
            RunRefusal::Scope {
                why: why.to_owned(),
            },
            RunRefusal::CensusLayoutChanged {
                why: why.to_owned(),
            },
            RunRefusal::CensusUnreadable {
                why: why.to_owned(),
            },
            RunRefusal::ClockImplausible {
                why: why.to_owned(),
            },
            RunRefusal::RunsLedgerUnwritten {
                why: why.to_owned(),
            },
            RunRefusal::ClaimUnwritten {
                why: why.to_owned(),
            },
        ] {
            assert_eq!(refusal.to_string(), format!("{}: {why}", refusal.code()));
        }
    }
}

#[test]
fn a_progress_fault_says_which_one_it_is() {
    assert_eq!(
        ProgressFault::Length(3).to_string(),
        format!("progress: 3 bytes, not {PROGRESS_BYTES}")
    );
    assert_eq!(
        ProgressFault::Magic.to_string(),
        "progress: not a progress record"
    );
    assert_eq!(
        ProgressFault::Crc.to_string(),
        "progress: CRC mismatch (torn or damaged)"
    );
    assert_eq!(
        ProgressFault::Field("a slot").to_string(),
        "progress: a slot is not one"
    );
}

/// A record whose slot says it holds more text than the slot can hold is
/// refused, whatever its CRC says.
#[test]
fn a_slot_longer_than_its_room_is_refused() {
    let mut raw = [0_u8; 8];
    raw[0] = 3;
    let got = Get(&raw).text((0, 1, 2));
    assert_eq!(got, Err(ProgressFault::Field("a slot's text length")));
}

/// A run refused outside the importer closes its record `refused`, its
/// refusal in a slot; a record of a path that is a directory is named and the
/// writes stay at zero.
#[test]
fn a_refused_run_closes_its_record_refused_and_a_directory_is_no_record() {
    let root = scratch("cov-progress");
    let progress = ProgressFile::of_record(root.join("p"), sample());
    progress.refused(&RunRefusal::SourceUnset { var: "ZQ_VAR" });
    let record = progress.record();
    assert_eq!(record.state, RunState::Refused.code());
    let slot = record.refusals.last().unwrap();
    assert_eq!(
        (slot.code.as_str(), slot.about.as_str()),
        ("source_unset", "source")
    );
    assert_eq!(progress.writes(), 1);
    assert_eq!(progress.fault(), None);
    let dir = root.join("a-dir");
    std::fs::create_dir_all(&dir).unwrap();
    let blocked = ProgressFile::of_record(PathBuf::from(&dir), sample());
    blocked.refused(&RunRefusal::SourceUnset { var: "ZQ_VAR" });
    assert_eq!(blocked.writes(), 0);
    assert!(blocked.fault().is_some());
    std::fs::remove_dir_all(root).unwrap();
}

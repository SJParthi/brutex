//! BASE-COV-PULL: the words, bytes and faults of a GDFL run that no test had
//! reached (Route B, recipe v4): a progress record's bytes and a refusal's
//! sentence are contracts with `/ingest` and with a resumed run.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
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

// ───── BASE-COV-PULL repair 2 (D-7334) ─────

fn on(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// A plan of `kind` alone over the source `word`, its root `root` named by
/// `var`, for April 1st 2024 into `store`.
fn one_kind(
    store: &std::path::Path,
    (kind, word): (ImportKind, SourceWord),
    var: &'static str,
    root: &std::path::Path,
) -> Plan {
    let log = LogState::Installed(store.to_path_buf());
    let process = Process {
        store_root: store,
        commit: None,
        log: &log,
        unstamped: None,
    };
    let request = Request::new(
        Kinds::of(&[kind]).unwrap(),
        word,
        (on(2024, 4, 1), on(2024, 4, 1)),
    );
    let root = root.to_path_buf();
    let env = move |name: &str| (name == var).then(|| root.clone().into_os_string());
    plan(request, &process, &env, &mut SourceCache::default()).unwrap()
}

/// The faults of a lock or the runs ledger no test had turned into a run's
/// refusal: a census layout that changed under the locks, a clock the ledger
/// will not take, a lock fault met inside the ledger, and an I/O or entry
/// fault of the ledger, each the refusal a page names, its words the
/// fault's own.
#[test]
fn a_changed_layout_a_bad_clock_and_an_unwritten_ledger_are_run_refusals() {
    use telemetry::run_dirs::GdflLayout;
    let changed = locks::LockFault::LayoutChanged {
        locked: GdflLayout::V3,
        found: GdflLayout::V4,
    };
    let said = changed.to_string();
    let refusal: RunRefusal = changed.into();
    assert_eq!(refusal.code(), "census_layout_changed");
    assert_eq!(
        refusal.to_string(),
        format!("census_layout_changed: {said}")
    );
    let clock = ledger::RunsLedgerFault::ClockImplausible {
        now: 0,
        why: "an invented clock".to_owned(),
    };
    let said = clock.to_string();
    let refusal: RunRefusal = clock.into();
    assert_eq!(refusal.code(), "clock_implausible");
    assert_eq!(refusal.to_string(), format!("clock_implausible: {said}"));
    let inner: RunRefusal = ledger::RunsLedgerFault::Lock(locks::LockFault::Replaced {
        lock: "imports/invented.lock".to_owned(),
    })
    .into();
    assert_eq!(inner.code(), "lock_replaced");
    for fault in [
        ledger::RunsLedgerFault::Io {
            why: "an invented io".to_owned(),
        },
        ledger::RunsLedgerFault::Entry {
            why: "an invented entry".to_owned(),
        },
    ] {
        let said = fault.to_string();
        let refusal: RunRefusal = fault.into();
        assert_eq!(refusal.code(), "runs_ledger_unwritten");
        assert_eq!(
            refusal.to_string(),
            format!("runs_ledger_unwritten: {said}")
        );
    }
}

/// The claim a plan makes names its asker and its source by their words, and
/// the api's auto-resume only when asked; a claim of a kind no run claims is
/// refused before any lock, a claim that names its start keeps it, and a
/// sidecar name a claim cannot clear (a directory there) is refused by name
/// with the lock left free (a combination: the api, each zip source and a
/// sidecar in the way at once).
#[test]
fn a_claim_names_its_asker_and_source_and_refuses_what_it_cannot_write() {
    let root = scratch("cov-claims");
    let (store, ts) = (root.join("store"), root.join("ts"));
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(&ts).unwrap();
    let planned = one_kind(
        &store,
        (ImportKind::Indices, SourceWord::TickStore),
        TICKSTORE_VAR,
        &ts,
    );
    let ask = |auto_resume| ClaimAsk {
        by: By::Api,
        pid: 7,
        host: "",
        resumes: 0,
        resume_day: None,
        auto_resume,
        legacy_ack: false,
        unstamped: false,
        retry: 0,
    };
    let mut kind = planned.kinds[0].clone();
    let mut words = Vec::new();
    for source in [SourceId::CmZip, SourceId::NfoZips, SourceId::TickStore] {
        kind.source = source;
        let made = claim_template(&planned, &kind, &ask(false));
        words.push((made.source, made.by, made.auto_resume));
    }
    let made = claim_template(&planned, &kind, &ask(true));
    words.push((made.source.clone(), made.by.clone(), made.auto_resume));
    assert_eq!(
        words,
        [
            ("cm_zip".to_owned(), "api".to_owned(), false),
            ("nfo_zips".to_owned(), "api".to_owned(), false),
            ("tickstore".to_owned(), "api".to_owned(), false),
            ("tickstore".to_owned(), "api".to_owned(), true),
        ]
    );
    let mut futures = made.clone();
    futures.kind = "futures".to_owned();
    let refused = claim_with(&store, futures, 0, None, 1_790_812_800).unwrap_err();
    assert_eq!(refused.code(), "claim_unwritten");
    assert!(
        refused
            .to_string()
            .contains("futures is not a kind a run claims"),
        "{refused}"
    );
    let mut started = made.clone();
    started.started = 1_790_812_000;
    let claimed = claim_with(&store, started, 0, None, 1_790_812_800).unwrap();
    assert_eq!(claimed.claim.started, 1_790_812_000, "its own start kept");
    let sidecar = claimed.locks.sidecar_path();
    claimed.release().unwrap();
    std::fs::create_dir_all(sidecar.join("in-the-way")).unwrap();
    let blocked = claim_with(&store, made.clone(), 0, None, 1_790_812_900).unwrap_err();
    assert_eq!(blocked.code(), "claim_unwritten");
    assert!(
        blocked.to_string().contains(&sidecar.display().to_string()),
        "{blocked}"
    );
    std::fs::remove_dir_all(sidecar).unwrap();
    let again = claim_with(&store, made, 0, None, 1_790_813_000).unwrap();
    again.release().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// Every source runs its own reader, an options tick store, the yearly
/// options zips and a capital-market zip that opens, each ending with its
/// one day not held; and a claimed run whose stop was asked before it began
/// ends refused `stopped`, through the claimed door (IA-2).
#[test]
fn every_source_runs_its_own_reader_and_a_stopped_claim_ends_stopped() {
    use crate::gdfl_fixtures::{Method, put, zip};
    let root = scratch("cov-sources");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let (ts, nfo) = (root.join("ts"), root.join("nfo"));
    std::fs::create_dir_all(&ts).unwrap();
    std::fs::create_dir_all(&nfo).unwrap();
    let cm = put(
        &root,
        "cm.zip",
        &zip(&[("readme.txt", b"invented", Method::Stored)]),
    );
    let mut ends = Vec::new();
    for (asked, var, at) in [
        (
            (ImportKind::Options, SourceWord::TickStore),
            TICKSTORE_VAR,
            &ts,
        ),
        (
            (ImportKind::Options, SourceWord::Zip),
            NFO_ARCHIVES_VAR,
            &nfo,
        ),
        ((ImportKind::Indices, SourceWord::Zip), CM_ARCHIVE_VAR, &cm),
    ] {
        let planned = one_kind(&store, asked, var, at);
        let source = planned.kinds[0].source;
        let missing = match execute_kind(&planned, &planned.kinds[0], None) {
            KindEnd::Report(report) => report.days_missing.len(),
            other => panic!("{source:?}: {other:?}"),
        };
        ends.push((source, missing));
    }
    assert_eq!(
        ends,
        [
            (SourceId::TickStore, 1),
            (SourceId::NfoZips, 1),
            (SourceId::CmZip, 1)
        ]
    );
    let planned = one_kind(
        &store,
        (ImportKind::Indices, SourceWord::TickStore),
        TICKSTORE_VAR,
        &ts,
    );
    let ask = ClaimAsk {
        by: By::Cli,
        pid: 7,
        host: "",
        resumes: 0,
        resume_day: None,
        auto_resume: false,
        legacy_ack: false,
        unstamped: false,
        retry: 0,
    };
    let claimed = claim_with(
        &store,
        claim_template(&planned, &planned.kinds[0], &ask),
        0,
        None,
        1_790_812_800,
    )
    .unwrap();
    stop::request_stop(
        &store,
        stop::StopTarget::Run(claimed.run),
        "cli",
        1_790_812_805,
    )
    .unwrap();
    let end = execute_claimed(&planned, &planned.kinds[0], None, &claimed);
    let KindEnd::Refused(refusal) = &end else {
        panic!("{end:?}");
    };
    assert_eq!(refusal.code(), "stopped", "{refusal}");
    claimed.release().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// A repair of a month the clock has closed is no open-month refusal: the
/// plan is made (the other side of `RepairInOpenMonth`).
#[test]
fn a_repair_of_a_closed_month_is_planned() {
    let root = scratch("cov-repair-closed");
    let (store, ts) = (root.join("store"), root.join("ts"));
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(&ts).unwrap();
    let log = LogState::Installed(store.clone());
    let process = Process {
        store_root: &store,
        commit: None,
        log: &log,
        unstamped: None,
    };
    let mut request = Request::new(
        Kinds::of(&[ImportKind::Indices]).unwrap(),
        SourceWord::TickStore,
        (on(2024, 4, 1), on(2024, 4, 30)),
    );
    request.repair_reason = Some("an invented repair".to_owned());
    let env = |name: &str| (name == TICKSTORE_VAR).then(|| ts.clone().into_os_string());
    let planned = plan(request, &process, &env, &mut SourceCache::default()).unwrap();
    assert_eq!(
        planned.request.repair_reason.as_deref(),
        Some("an invented repair")
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A progress record over a device, not a file, is refused by name once and
/// the run goes on; a day whose counts did not move is written again while
/// no write was ever made (`due`); a day's refusals are copied into the
/// record's slots; and a refused run's state is `census` for either census
/// refusal, `durability` for an fsync refusal and `refused` for any other,
/// a refusal that carries no report adding only its own slot (extremes at
/// once: a device, an empty report and a refusal with no report).
#[test]
fn a_progress_record_says_each_end_and_a_device_is_no_record() {
    let device = ProgressFile::of_record(PathBuf::from("/dev/null"), sample());
    assert_eq!(device.fault(), None);
    device.day_passed(ImportKind::Indices, on(2024, 4, 1), &Report::default());
    let fault = device.fault().expect("a device is refused by name");
    assert!(fault.contains("not a regular file"), "{fault}");
    assert_eq!(device.writes(), 0);
    let root = scratch("cov-progress-ends");
    let mut report = Report::default();
    report.recent.push((
        crate::gdfl_import::Cause::open(crate::gdfl_import::Stage::File, "invented_code"),
        crate::ingest::Failure {
            day: Some(on(2024, 4, 1)),
            ..crate::ingest::Failure::new("ZQ.csv", "an invented refusal")
        },
    ));
    let copied = ProgressFile::of_record(root.join("copied"), sample());
    copied.day_passed(ImportKind::Indices, on(2024, 4, 1), &report);
    let slot = copied.record().refusals.last().cloned().unwrap();
    assert_eq!(
        (slot.day, slot.code.as_str(), slot.about.as_str()),
        (20_240_401, "invented_code", "ZQ.csv")
    );
    let done = || Box::new(Report::default());
    let census = crate::gdfl_import::ImportRefusal::CensusRefused {
        kind: ImportKind::Indices,
        day: on(2024, 4, 1),
        room: Err("an invented room".to_owned()),
        why: "an invented census".to_owned(),
        done: done(),
    };
    let durability = crate::gdfl_import::ImportRefusal::Durability {
        kind: ImportKind::Indices,
        day: on(2024, 4, 1),
        why: "an invented fsync".to_owned(),
        done: done(),
    };
    let lineage = crate::gdfl_import::ImportRefusal::Lineage {
        why: "an invented ledger".to_owned(),
    };
    let mut states = Vec::new();
    for (name, refusal) in [("c", census), ("d", durability), ("l", lineage)] {
        let progress = ProgressFile::of_record(root.join(name), sample());
        progress.finished(ImportKind::Indices, Err(&refusal));
        let record = progress.record();
        states.push((record.state, record.refusals.last().unwrap().code.clone()));
    }
    assert_eq!(
        states,
        [
            (RunState::Census.code(), "census_refused".to_owned()),
            (RunState::Durability.code(), "durability".to_owned()),
            (RunState::Refused.code(), "lineage".to_owned()),
        ]
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A record whose CRC does not check is read once more before it is said,
/// and then refused as a CRC fault, never read as a record.
#[test]
fn a_torn_progress_record_is_read_again_then_refused() {
    let root = scratch("cov-progress-torn");
    let progress = ProgressFile::of_record(root.join("p"), sample());
    progress.refused(&RunRefusal::SourceUnset { var: "ZQ_VAR" });
    let path = progress.path().to_path_buf();
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[40] ^= 0xFF;
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(
        Progress::read(&path).unwrap_err(),
        "progress: CRC mismatch (torn or damaged)"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A write past a record's end is dropped, never a panic, and a write that
/// fits lands where it was put.
#[test]
fn a_write_past_the_records_end_is_dropped() {
    let mut raw = [0_u8; PROGRESS_BYTES];
    Put(&mut raw).bytes(PROGRESS_BYTES - 1, &[1, 2]);
    assert_eq!(raw, [0_u8; PROGRESS_BYTES]);
    Put(&mut raw).bytes(PROGRESS_BYTES - 2, &[1, 2]);
    assert_eq!(raw[PROGRESS_BYTES - 2..], [1, 2]);
}

/// **WITH A SINK, THE RECORD CARRIES THE SINK'S HEALTH, AND THE PROCESS'S
/// LOG STATE NAMES ITS DIRECTORY** (repair 2): run in a process of its own,
/// the one install there.
#[test]
fn with_a_sink_the_record_carries_its_health_and_the_log_names_its_directory() {
    crate::support::where_permission_binds(
        "gdfl_run::cov_tests::with_a_sink_the_record_carries_its_health_and_the_log_names_its_directory",
        with_a_sink_body,
    );
}

fn with_a_sink_body() {
    let root = scratch("cov-progress-sink");
    let home = root.join("logs");
    let sink = telemetry::install(&telemetry::Config::new(&home)).expect("the only install");
    assert_eq!(
        LogState::of_process("unused"),
        LogState::Installed(sink.dir().to_path_buf())
    );
    let progress = ProgressFile::of_record(root.join("p"), sample());
    progress.refused(&RunRefusal::SourceUnset { var: "ZQ_VAR" });
    let health = sink.health();
    let record = progress.record();
    assert_eq!(
        (record.clock_held, record.log_dropped, record.log_failures),
        (health.clock_held, health.dropped, health.rotation_failures)
    );
    assert_eq!(Progress::read(progress.path()).unwrap(), record);
}

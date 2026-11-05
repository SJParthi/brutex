#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

//! BASE-COV-PULL-PS2 (D-7336): the filing's arms a whole import day does not
//! reach on its own, each called with the input that reaches it. Every
//! fixture is invented.

use std::path::PathBuf;

use super::*;
use crate::gdfl_run::stop::StopControl;

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-filing-ps2-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// THE LOSS PROBE: the root the run began on answers nothing; another
/// directory at its path, a file there, and nothing there each say why.
#[test]
fn the_loss_probe_names_another_directory_a_file_and_nothing() {
    use std::os::unix::fs::MetadataExt as _;
    let root = scratch("LOSS");
    let meta = std::fs::metadata(&root).unwrap();
    let id = (meta.dev(), meta.ino());
    assert_eq!(store_root_lost(&root, Some(id)), None);
    assert_eq!(
        store_root_lost(&root, None),
        None,
        "a fresh store: a directory"
    );
    let other = store_root_lost(&root, Some((id.0, id.1 ^ 1))).expect("another directory");
    assert!(
        other.contains("is another directory than the run began on"),
        "{other}"
    );
    let file = root.join("a-file");
    std::fs::write(&file, b"invented").unwrap();
    let not_dir = store_root_lost(&file, Some(id)).expect("a file");
    assert!(
        not_dir.contains("is no longer a directory: the store's root went away"),
        "{not_dir}"
    );
    let gone = store_root_lost(&root.join("gone"), None).expect("nothing there");
    assert!(gone.contains("could not reach"), "{gone}");
    std::fs::remove_dir_all(&root).unwrap();
}

/// THE STOP OF A STORE THAT TAKES NO WRITES carries the host's refusal first
/// and then every fix it names.
#[test]
fn the_not_writable_stop_names_its_refusal_and_each_fix() {
    let said = not_writable_said("invented: Permission denied (os error 13)");
    assert!(
        said.starts_with(
            "invented: Permission denied (os error 13); the store does not take writes"
        ),
        "{said}"
    );
    for fix in [
        "chflags nouchg <file>",
        "chmod -N <file>",
        "remount the volume read-write",
    ] {
        assert!(said.contains(fix), "{fix}: {said}");
    }
}

/// A FILE NOT TAKEN AFTER AN EARLIER FILE'S FAULT names that fault: a full
/// disk, a lost store, and any other code a store that takes no writes.
#[test]
fn a_file_not_taken_after_an_earlier_fault_names_that_fault() {
    let day = Day::new(2019, 11, 14).unwrap();
    for (code, word) in [
        (1, "disk_full"),
        (2, "store_lost"),
        (3, "store_not_writable"),
        (9, "store_not_writable"),
    ] {
        let refused = not_taken(ImportKind::Options, day, code);
        assert!(
            refused.failure.why.contains(&format!(
                "an earlier file of the day met {word}, so the run ends"
            )),
            "{code}: {}",
            refused.failure.why
        );
    }
}

/// WORKERS ASKED TO STOP BEFORE THEIR FIRST FILE take no job, and the pool
/// answers the nothing they did.
#[test]
fn workers_asked_to_stop_take_no_job() {
    let root = scratch("STOP");
    let stop = StopControl::new(root.join("stop"), 7);
    stop.ask("cli", 0);
    let gate = BeginGate::new(None, 2);
    let jobs = [1, 2, 3];
    // A JOB TAKEN WOULD PANIC, and the pool resumes a worker's panic here.
    let done = on_workers(
        2,
        &jobs,
        &gate,
        &mut || Ok(1),
        &|_: &i32| -> i32 { unreachable!("a stopped worker takes no job") },
        Some(&stop),
    )
    .expect("no refusal");
    assert!(done.is_empty(), "{done:?}");
    std::fs::remove_dir_all(&root).unwrap();
}

/// A JOB THAT PANICS PANICS ITS CALLER with its own payload: a worker's
/// fault is never swallowed into a short list.
#[test]
fn a_job_that_panics_panics_its_caller() {
    let gate = BeginGate::new(None, 2);
    let jobs = [1, 2];
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        on_workers(
            2,
            &jobs,
            &gate,
            &mut || Ok(1),
            &|job: &i32| {
                assert!(*job != 2, "invented job fault");
                *job
            },
            None,
        )
    }));
    let payload = caught.expect_err("the panic comes back");
    let said = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("a panic message");
    assert_eq!(said, "invented job fault");
}

/// A BEGIN ATTEMPT THAT LEFT NEITHER A LINE NOR A REFUSAL is said as the line
/// not written: a worker that asked never waits for ever and never files
/// under a line nobody wrote.
#[test]
fn a_begin_attempt_that_left_no_line_and_no_refusal_is_said_not_written() {
    let gate = std::sync::Arc::new(BeginGate::new(None, 1));
    let asker = {
        let gate = std::sync::Arc::clone(&gate);
        std::thread::spawn(move || gate.line())
    };
    loop {
        let state = gate.lock();
        if state.asked {
            break;
        }
        drop(state);
        std::thread::yield_now();
    }
    {
        let mut state = gate.lock();
        state.asked = false;
        state.attempts += 1;
        gate.changed.notify_all();
    }
    let refused = asker.join().unwrap().expect_err("no line");
    assert!(
        matches!(
            &refused,
            ImportRefusal::Journal { why } if why == "the day's begin line was not written"
        ),
        "{refused:?}"
    );
}

/// A HOST THAT STARTS NO WORKER refuses the pool in its own words, and no job
/// runs.
#[test]
fn a_pool_the_host_starts_no_worker_for_is_refused_in_its_words() {
    inject::refuse_spawn(0);
    let gate = BeginGate::new(None, 2);
    let jobs = [1, 2, 3];
    let refused = on_workers(
        2,
        &jobs,
        &gate,
        &mut || Ok(1),
        &|_: &i32| -> i32 { unreachable!("no worker started, so no job runs") },
        None,
    )
    .expect_err("no worker started");
    assert_eq!(refused, "invented: the host refused worker 0 a thread");
}

/// A HOST THAT STARTS FEWER WORKERS THAN ASKED: the pool runs every job on
/// the ones it started, in the jobs' order.
#[test]
fn a_pool_with_fewer_workers_than_asked_runs_every_job() {
    inject::refuse_spawn(1);
    let gate = BeginGate::new(None, 2);
    let jobs = [1, 2, 3, 4];
    let done = on_workers(2, &jobs, &gate, &mut || Ok(1), &|job: &i32| job * 10, None)
        .expect("one worker ran them all");
    assert_eq!(done, vec![10, 20, 30, 40]);
}

/// A day's filer over an invented run whose journal is absent, and its
/// run-wide hold-back map filled to the cap when `full`.
fn with_filer(full: bool, check: impl FnOnce(&mut DayFiler<'_>)) {
    let root = scratch("FILER");
    let day = Day::new(2019, 11, 14).unwrap();
    let run = super::super::Run {
        kind: ImportKind::Options,
        from: day,
        to: day,
        only: &[],
        store_root: &root,
    };
    let journal = super::super::journal::Journal::load(root.join("journal")).unwrap();
    let month = day.year_month().unwrap();
    let held_back = (0..if full { super::super::HELD_BACK_CAP } else { 0 })
        .map(|n| ((format!("INVENTED{n}"), None, month), day))
        .collect();
    let ctx = Ctx {
        run: &run,
        journal: &journal,
        options: super::super::RunOptions::default(),
        id: 1,
        started: 1,
        announced: std::cell::Cell::new(false),
        scoped: None,
        held_back: std::cell::RefCell::new(held_back),
        store_id: std::cell::Cell::new(None),
    };
    let request = BarRequest {
        instrument_id: "INVENTED".to_owned(),
        listing: crate::vendor::Listing::Index,
        window: crate::session::Window::new(day, day).unwrap(),
        granularity: crate::vendor::Granularity::Minute1,
    };
    let mut report = Report::default();
    let mut filer = DayFiler::of_day(&ctx, ("options 2019-11-14 *", day), &request, &mut report);
    check(&mut filer);
    drop(filer);
    std::fs::remove_dir_all(&root).unwrap();
}

/// THE HOLD-BACK PAST ITS CAP names the first key it could not hold and the
/// count it would have reached, once: a later key past the cap changes
/// neither.
#[test]
fn the_hold_back_past_its_cap_names_its_first_key_once() {
    with_filer(true, |filer| {
        let month = filer.day.year_month().unwrap();
        let first = ("OVER-ONE".to_owned(), None, month);
        filer.hold_back(first.clone());
        filer.hold_back(("OVER-TWO".to_owned(), None, month));
        assert_eq!(
            filer.held_back_cap,
            Some((held_key_text(&first), super::super::HELD_BACK_CAP + 1))
        );
        assert_eq!(
            filer.ctx.held_back.borrow().len(),
            super::super::HELD_BACK_CAP
        );
    });
}

/// THE FIRST FILE THAT MET A FULL DISK, A LOST STORE OR A STORE THAT TAKES NO
/// WRITES is the one the day keeps: a later file's words for the same fault
/// never replace it.
#[test]
fn the_first_fault_of_each_kind_is_the_one_the_day_keeps() {
    with_filer(false, |filer| {
        let said = |word: &str| Filed {
            disk_full: Some(format!("{word}: disk")),
            store_lost: Some(format!("{word}: lost")),
            not_writable: Some(format!("{word}: not writable")),
            ..Filed::default()
        };
        filer.fold(said("first"));
        filer.fold(said("second"));
        assert_eq!(filer.disk_full.as_deref(), Some("first: disk"));
        assert_eq!(filer.store_lost.as_deref(), Some("first: lost"));
        assert_eq!(filer.not_writable.as_deref(), Some("first: not writable"));
    });
}

/// THE TEST DOOR'S OWN UNTAKEN ANSWERS (`inject`, D-7336): a file with no
/// swap or answer planted under its root is left alone, whatever another
/// root holds in the same lists; and a swap planted for a root that is not
/// there moves nothing and makes nothing.
#[test]
fn the_inject_door_leaves_an_unplanted_file_and_a_missing_root_alone() {
    let root = scratch("INJECT-NONE");
    let missing = root.join("not-there");
    let file = crate::gdfl_import::TickFile {
        symbol: "INVENTED".to_owned(),
        contract: None,
        name: "none.zip".to_owned(),
        ticks: Vec::new(),
    };
    // ANOTHER ROOT'S PLANTINGS, passed over by this root's asks.
    inject::plant_swap_after(&missing, "none.zip");
    inject::plant(
        &missing,
        "none.zip",
        crate::ingest::FailKind::DiskFull,
        false,
    );
    inject::after_landed(&root, "none.zip");
    assert!(root.is_dir(), "no swap planted for this root: it stays");
    assert!(
        inject::answer(&root, &file).is_none(),
        "no answer planted for it"
    );
    // TAKEN BY THEIR OWN ROOT: the answer once, and a swap with no root to move.
    let answered = inject::answer(&missing, &file).expect("planted for it");
    assert_eq!(answered.door_kind, Some(crate::ingest::FailKind::DiskFull));
    assert!(inject::answer(&missing, &file).is_none(), "taken once");
    inject::after_landed(&missing, "none.zip");
    assert!(!missing.exists(), "a missing root is not made");
    assert!(!root.join("not-there.gone").exists(), "nor moved");
    std::fs::remove_dir_all(&root).unwrap();
}

/// PS2 (the (U) audit: a direct call is an input): A BACK-FILL PASS HANDED NO
/// `begin` LINE refuses each file it would file at its `begin`, in the
/// journal's words, and files none of its bars. The day hands the pass the
/// line its files wrote before they reached a door, so only a direct call
/// reaches this. Every tick is invented.
#[test]
fn a_back_fill_pass_with_no_begin_line_refuses_its_file_at_the_journal() {
    with_filer(false, |filer| {
        let filing = Filing {
            run: filer.ctx.run,
            day: filer.day,
            request: filer.request,
            session: None,
            origin: None,
            source: None,
            confirm: None,
            id: filer.ctx.id,
            repair_reason: None,
            retries: std::sync::atomic::AtomicUsize::new(0),
            store_id: None,
        };
        let handed = super::super::Handed {
            file: crate::gdfl_import::TickFile {
                symbol: "INVENTED".to_owned(),
                contract: None,
                name: "INVENTED.csv".to_owned(),
                ticks: vec![crate::gdfl_import::Tick {
                    sod: 33_300,
                    ltp: 500_000,
                    ltq: 1,
                    oi: None,
                }],
            },
            facts: super::super::FileFacts::default(),
        };
        let done = back_fill(
            &filing,
            None,
            vec![JobDone::BackFill(Box::new(handed), None)],
        );
        let [Ok((filed, None))] = done.as_slice() else {
            panic!("one filed job");
        };
        assert!(
            filed.failures.iter().any(|staged| staged
                .failure
                .why
                .contains("the day's begin line was not written")),
            "{:?}",
            filed
                .failures
                .iter()
                .map(|s| &s.failure.why)
                .collect::<Vec<_>>()
        );
        assert_eq!(filed.committed, 0, "nothing filed");
    });
}

// ═══ The rows once called "G1's harness": a Filing and a DayFiler by hand ═══

/// The day's filing as `file_jobs` makes it, with no session, origin, source,
/// batch or store identity.
fn filing_of<'a>(filer: &DayFiler<'a>) -> Filing<'a> {
    Filing {
        run: filer.ctx.run,
        day: filer.day,
        request: filer.request,
        session: None,
        origin: None,
        source: None,
        confirm: None,
        id: filer.ctx.id,
        repair_reason: None,
        retries: std::sync::atomic::AtomicUsize::new(0),
        store_id: None,
    }
}

/// One invented file of one traded second at 09:15 IST.
fn invented_handed() -> super::super::Handed {
    super::super::Handed {
        file: crate::gdfl_import::TickFile {
            symbol: "INVENTED".to_owned(),
            contract: None,
            name: "INVENTED.csv".to_owned(),
            ticks: vec![crate::gdfl_import::Tick {
                sod: 33_300,
                ltp: 500_000,
                ltq: 1,
                oi: None,
            }],
        },
        facts: super::super::FileFacts::default(),
    }
}

/// The records `sink` holds from this module's target.
fn said(home: &std::path::Path, sink: &telemetry::Sink) -> Vec<telemetry::Record> {
    telemetry::tail(
        home,
        sink.keep_files(),
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(TARGET),
    )
    .records
}

/// A DAY WHOSE POOL THE HOST STARTS NO WORKER FOR files on one thread and
/// says so at `warn`, with the day and the host's words; every job is still
/// answered, in listing order. Run in a child, which installs its own sink.
#[test]
fn a_day_whose_pool_starts_no_worker_files_on_one_thread_and_says_so() {
    crate::support::where_permission_binds(
        "gdfl_import::filing::ps2_tests::a_day_whose_pool_starts_no_worker_files_on_one_thread_and_says_so",
        one_thread_day,
    );
}

fn one_thread_day() {
    let home = scratch("ONE-THREAD-SINK");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .unwrap();
    with_filer(false, |filer| {
        filer.workers = 2;
        inject::refuse_spawn(0);
        let (kind, day) = (filer.ctx.run.kind, filer.day);
        let jobs = [&1_u8, &2_u8];
        let read = move |_: &u8| -> ReadJob { Err(not_taken(kind, day, 1)) };
        let answered = filer.file_jobs(&jobs, &read);
        assert_eq!(answered.len(), 2, "every job answered");
        assert!(
            answered
                .iter()
                .all(|one| matches!(one, JobRead::Refused(_)))
        );
    });
    let records = said(&home, sink);
    let line = records
        .iter()
        .find(|record| {
            record.message == "no worker thread started; the day's files run on one thread"
        })
        .expect("the fallback is said");
    assert_eq!(line.level, telemetry::Level::Warn);
    assert_eq!(
        line.field("why"),
        Some(&telemetry::OwnedValue::Str(
            "invented: the host refused worker 0 a thread".to_owned()
        ))
    );
    std::fs::remove_dir_all(&home).unwrap();
}

/// A FILE THE STORE REFUSES AS NOT WRITABLE ends the day's taking: the door's
/// kind, never its words, is read (IA-16), the day keeps what failed and the
/// fix, and the next file is not taken, by that reason.
#[test]
fn a_file_the_store_refuses_not_writable_ends_the_days_taking() {
    with_filer(false, |filer| {
        filer.workers = 1;
        let root = filer.ctx.run.store_root.to_path_buf();
        inject::plant(
            &root,
            "INVENTED.csv",
            crate::ingest::FailKind::Io {
                kind: std::io::ErrorKind::PermissionDenied,
                publication_may_be_visible: false,
            },
            false,
        );
        let jobs = [&0_u8, &1_u8];
        let read = |_: &u8| -> ReadJob { Ok((invented_handed(), None)) };
        let answered = filer.file_jobs(&jobs, &read);
        let [JobRead::Read(None), JobRead::Refused(second)] = answered.as_slice() else {
            panic!("the first read, the second not taken");
        };
        assert!(
            second.failure.why.contains("store_not_writable"),
            "{}",
            second.failure.why
        );
        let kept = filer.not_writable.as_deref().unwrap();
        assert!(kept.contains("planted by the test"), "{kept}");
    });
}

/// A FILE WHOSE `begin` IS REFUSED AFTER THE STORE'S ROOT WENT AWAY names
/// both, the refused begin and the loss probe's words, so the run ends by the
/// lost store; nothing reaches a door.
#[test]
fn a_refused_begin_with_the_root_gone_names_the_lost_store() {
    with_filer(false, |filer| {
        let filing = filing_of(filer);
        let root = filer.ctx.run.store_root.to_path_buf();
        let aside = root.with_extension("aside");
        let begin = || -> Result<u64, ImportRefusal> {
            std::fs::rename(&root, &aside).unwrap();
            Err(ImportRefusal::Journal {
                why: "invented: the begin line was refused".to_owned(),
            })
        };
        let (filed, back_fill) = filing.file(&invented_handed(), &begin, Door::Append);
        std::fs::rename(&aside, &root).unwrap();
        assert!(!back_fill, "never handed to the back-fill door");
        let lost = filed.store_lost.as_deref().unwrap();
        assert!(
            lost.contains("invented: the begin line was refused"),
            "{lost}"
        );
        assert!(lost.contains("went away"), "{lost}");
        assert_eq!(filed.committed, 0);
    });
}

/// A NEW GENERATION OF A FILE WHOSE SOURCE NAMED NO MEMBER is said at `info`,
/// not `warn`: a run naming its origin and source misses a `.prv` record only
/// for a file that names its member. Run in a child, which installs its own
/// sink.
#[test]
fn a_new_generation_of_a_file_with_no_member_is_said_at_info() {
    crate::support::where_permission_binds(
        "gdfl_import::filing::ps2_tests::a_new_generation_of_a_file_with_no_member_is_said_at_info",
        no_member_generation,
    );
}

fn no_member_generation() {
    let home = scratch("NO-MEMBER-SINK");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .unwrap();
    with_filer(false, |filer| {
        let origin = super::super::Origin {
            source: super::super::SourceKind::Zip,
            path: std::path::PathBuf::from("invented.zip"),
            archive_bytes: 0,
            digest: None,
            commit: None,
            started: 0,
            fault: None,
        };
        let source: Result<DaySource, String> = Err("invented: no day source".to_owned());
        let filing = Filing {
            origin: Some(&origin),
            source: Some(&source),
            ..filing_of(filer)
        };
        let month = filer.day.year_month().unwrap();
        let held = crate::manifest::Held::unknown(crate::manifest::Entry {
            key: crate::manifest::EntryKey {
                contract: None,
                exchange: brutex_core::instrument::Exchange::Nse,
                segment: brutex_core::instrument::Segment::Fno,
                symbol: brutex_core::symbol::Symbol::new("INVENTED").unwrap(),
                timeframe: store::path::Timeframe::SECOND_1,
                month,
            },
            rows: 5,
            first_ts_micros: 0,
            last_ts_micros: 0,
        });
        filing.repaired(&invented_handed(), &held, 3);
    });
    let records = said(&home, sink);
    let line = records
        .iter()
        .find(|record| record.message == "day back-filled as a new generation of its month")
        .expect("the new generation is said");
    assert_eq!(line.level, telemetry::Level::Info);
    assert_eq!(
        line.field("file"),
        Some(&telemetry::OwnedValue::Str("INVENTED.csv".to_owned()))
    );
    std::fs::remove_dir_all(&home).unwrap();
}

/// A FILE WHOSE SYMBOL NAMES NO INSTRUMENT (a byte no symbol holds) is the
/// filing's own refusal, `filing`, never a store's: no door kind ends the
/// day's taking, the day keeps it once, and nothing is committed.
#[test]
fn a_file_whose_symbol_names_no_instrument_is_the_filings_own_refusal() {
    with_filer(false, |filer| {
        let filing = filing_of(filer);
        let mut handed = invented_handed();
        handed.file.symbol = "IN.VENTED".to_owned();
        let (filed, back_fill) = filing.file(&handed, &|| Ok(1), Door::Append);
        assert!(!back_fill, "never handed to the back-fill door");
        assert_eq!(filed.committed, 0, "nothing committed");
        let [one] = filed.failures.as_slice() else {
            panic!("{:?}", filed.failures);
        };
        assert_eq!(one.cause.code, "filing", "{one:?}");
        assert!(one.failure.why.contains("IN.VENTED"), "{one:?}");
        assert_eq!(
            (&filed.disk_full, &filed.store_lost, &filed.not_writable),
            (&None, &None, &None),
            "no store fault ends the day"
        );
    });
}

/// A MONTH WHOSE FOLDER ANOTHER WRITER REPLACED WITH A FILE after the file's
/// bars were found present and before the day's batch confirms them (D-7336,
/// the door's step between the two) keeps the day open by the file's name:
/// the confirm's refusal is the filing's own, `filing`, naming the month's
/// folder, and nothing is confirmed. The bars an earlier run landed stand.
#[test]
fn a_month_whose_folder_was_replaced_before_its_confirm_keeps_the_day_open() {
    with_filer(false, |filer| {
        let root = filer.ctx.run.store_root.to_path_buf();
        let (landed, _) = filing_of(filer).file(&invented_handed(), &|| Ok(1), Door::Append);
        assert_eq!(landed.committed, 1, "an earlier run landed the bar");
        let held = landed.held.expect("its month is held");
        let bars = month_path(&held).unwrap().to_path_buf(&root);
        let folder = bars.parent().unwrap().to_path_buf();
        let replaced = folder.clone();
        inject::plant_before_confirm(&root, "INVENTED.csv", move || {
            std::fs::rename(&replaced, replaced.with_extension("aside")).unwrap();
            std::fs::write(&replaced, b"invented").unwrap();
        });
        let batch = store::file::DurableBatch::new(store::file::BatchFlush::EveryFile);
        let again = Filing {
            confirm: Some(&batch),
            ..filing_of(filer)
        };
        let (filed, back_fill) = again.file(&invented_handed(), &|| Ok(1), Door::Append);
        assert!(!back_fill, "never handed to the back-fill door");
        assert!(!filed.confirmed, "nothing confirmed");
        let [one] = filed.failures.as_slice() else {
            panic!("{:?}", filed.failures);
        };
        assert_eq!(one.cause.code, "filing", "{one:?}");
        assert!(
            one.failure.why.contains(&folder.display().to_string()),
            "{one:?}"
        );
        assert!(
            folder
                .with_extension("aside")
                .join(bars.file_name().unwrap())
                .is_file(),
            "the landed bars stand"
        );
    });
}

/// THE TEST DOOR'S STEP BEFORE A CONFIRM (D-7336) is taken only by its own root
/// and file, once: a step planted for another file under the same root is
/// passed over by this file's confirm, and runs at its own.
#[test]
fn the_door_s_step_before_a_confirm_waits_for_its_own_file() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let root = scratch("CONFIRM-STEP");
    let ran = std::sync::Arc::new(AtomicU32::new(0));
    let counted = std::sync::Arc::clone(&ran);
    inject::plant_before_confirm(&root, "OTHER.csv", move || {
        counted.fetch_add(1, Ordering::SeqCst);
    });
    inject::before_confirm(&root, "INVENTED.csv");
    assert_eq!(
        ran.load(Ordering::SeqCst),
        0,
        "another file's step is passed over"
    );
    inject::before_confirm(&root, "OTHER.csv");
    inject::before_confirm(&root, "OTHER.csv");
    assert_eq!(ran.load(Ordering::SeqCst), 1, "its own file takes it, once");
    std::fs::remove_dir_all(&root).unwrap();
}

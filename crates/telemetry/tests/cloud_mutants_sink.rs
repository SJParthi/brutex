//! Mutation-survivor tests for `telemetry::sink`'s public surface.
//!
//! Each test here kills a mutant `cargo mutants` left alive in
//! `crates/telemetry/src/sink.rs` (cloud session 2026-10-16, report
//! `reports/cloud-mut-store-20261016.md`). Its doc comment names the mutant
//! and the behaviour that mutant breaks; the assertion is that behaviour.
//! Private items are proved in `sink.rs`'s own `cloud_mutant_tests` module.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::Path;

use telemetry::{FileTarget, Target as _};

/// **A FILE TARGET'S `sync` IS THE HOST'S FSYNC, ERRORS INCLUDED.**
///
/// Kills `replace <impl Target for FileTarget>::sync -> std::io::Result<()>
/// with Ok(())`: a sync that answers `Ok` without asking the host reports
/// every event durable when nothing was made so. No regular file on a working
/// disk refuses `fsync`, so the refusal is the host's own: Linux answers
/// `fsync` on a character device with `EINVAL` ("fd is bound to a special
/// file which does not support synchronization", fsync(2)), and `/dev/null`
/// opens for appending without being created or changed. The control beside
/// it is a regular file, whose sync succeeds, so the refusal is the device's
/// and not the target's.
#[cfg(target_os = "linux")]
#[test]
fn a_file_target_passes_the_hosts_fsync_refusal_through() {
    const EINVAL: i32 = 22;
    let device = FileTarget::open(Path::new("/dev/null")).expect("/dev/null opens to append");
    let refused = device
        .sync()
        .expect_err("fsync of a character device is refused by the host");
    assert_eq!(refused.raw_os_error(), Some(EINVAL), "{refused}");

    let dir = std::env::temp_dir().join(format!(
        "brutex-telemetry-cloud-sync-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let mut regular = FileTarget::open(&dir.join("events.ndjson")).expect("a regular file");
    regular.append(b"{}\n").expect("one line lands");
    regular.sync().expect("a regular file syncs");
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **THE AUDIT STREAM ROLLS AT THE DECIDED 64 MIB AND HOLDS AT MOST 1 GIB.**
///
/// Kills both `replace * with +` mutants of
/// `AUDIT_MAX_FILE_BYTES = 64 * 1024 * 1024` (to 1,048,640 and to 66,560
/// bytes): either would roll the durable
/// stream thousands of times sooner than D-3341 item 3 decided and shrink the
/// history its 16 files hold from 1 GiB to under 17 MiB, with nothing saying
/// so. The bound and the ceiling the constants' own documentation states are
/// asserted as numbers.
#[test]
fn the_audit_stream_is_bounded_at_the_decided_64_mib_and_1_gib() {
    assert_eq!(telemetry::AUDIT_MAX_FILE_BYTES, 67_108_864, "64 MiB");
    assert_eq!(telemetry::AUDIT_KEEP_FILES, 16);
    assert_eq!(
        telemetry::AUDIT_MAX_FILE_BYTES * u64::from(telemetry::AUDIT_KEEP_FILES),
        1_073_741_824,
        "1 GiB while rotation works"
    );
}

/// **A VALUE OF EXACTLY `AUDIT_MAX_POSTING_NAME` BYTES IS NAMED; ONE MORE IS
/// NOT.**
///
/// Kills `replace > with >=` and `replace > with ==` in `posting_name`'s
/// length guard: both refuse a value at the ceiling, so a run or instrument
/// whose name is exactly 120 safe bytes would go unindexed and a probe for it
/// would find nothing. The byte past the ceiling, and a value that only
/// passes it once encoded, are refused either way.
#[test]
fn a_posting_name_at_the_ceiling_is_named_and_one_past_it_is_not() {
    let dir = Path::new("/nonexistent/cloud");
    let at_ceiling = "A".repeat(telemetry::AUDIT_MAX_POSTING_NAME);
    let path = telemetry::audit_posting_path(dir, 7, "instrument", &at_ceiling)
        .expect("a value of exactly the ceiling is named");
    assert_eq!(
        path,
        telemetry::audit_generation_path(dir, 7)
            .join("instrument")
            .join(format!("{at_ceiling}.idx"))
    );
    let past = "A".repeat(telemetry::AUDIT_MAX_POSTING_NAME + 1);
    assert_eq!(
        telemetry::audit_posting_path(dir, 7, "instrument", &past),
        None
    );
    // 119 bytes, one of them encoded to three: 121 once named.
    let encoded_past = format!("{}.", "A".repeat(telemetry::AUDIT_MAX_POSTING_NAME - 2));
    assert_eq!(
        telemetry::audit_posting_path(dir, 7, "instrument", &encoded_past),
        None
    );
    assert_eq!(
        telemetry::audit_posting_path(dir, 7, "instrument", ""),
        None
    );
}

/// **EVERY SIGNAL OF `AuditHealth::is_loud` IS LOUD ON ITS OWN.**
///
/// Kills `replace > with <` on `dropped`, `rotation_failures` and
/// `index_failures`, and `replace || with &&` before `rotation_failures` and
/// before `index_failures`: each makes one failure, alone, quiet, so a stream
/// losing lines or index records would read healthy on the operator's page.
/// Each count is set to one with every other signal clear; a clear health is
/// quiet; the reaper is loud only past [`telemetry::AUDIT_KEEP_FILES`]
/// generations waiting.
#[test]
fn every_audit_failure_alone_makes_the_stream_loud() {
    let quiet = telemetry::AuditHealth::default();
    assert!(!quiet.is_loud(), "nothing failed");
    let alone: [(&str, fn(&mut telemetry::AuditHealth)); 4] = [
        ("dropped", |h| h.dropped = 1),
        ("rotation_failures", |h| h.rotation_failures = 1),
        ("index_failures", |h| h.index_failures = 1),
        ("reap_failures", |h| h.reap_failures = 1),
    ];
    for (signal, set) in alone {
        let mut health = telemetry::AuditHealth::default();
        set(&mut health);
        assert!(health.is_loud(), "{signal} alone is loud: {health:?}");
    }
    let keep = u64::from(telemetry::AUDIT_KEEP_FILES);
    let waiting = |gone_waiting| telemetry::AuditHealth {
        gone_waiting,
        ..telemetry::AuditHealth::default()
    };
    assert!(!waiting(keep).is_loud(), "as many as the stream keeps");
    assert!(waiting(keep + 1).is_loud(), "one more is the reaper behind");
}

/// A directory of this test's own, emptied first.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-telemetry-cloud-{name}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// **A SERIAL THAT IS THERE AND CANNOT BE READ IS AN ERROR, NOT SERIAL 0.**
///
/// Kills `replace match guard e.kind() == NotFound with true` in
/// `audit_serial`: it answers `Ok(0)` for every refusal, so a stream whose
/// serial file is a directory (or unreadable) would read as never rolled and
/// every pointer into its rolled files would resolve to the wrong file.
#[test]
fn an_unreadable_audit_serial_is_an_error_and_an_absent_one_is_zero() {
    let dir = scratch("serial-unreadable");
    assert_eq!(telemetry::audit_serial(&dir), Ok(0), "never written");
    std::fs::create_dir_all(telemetry::audit_serial_path(&dir)).expect("a directory there");
    let said = telemetry::audit_serial(&dir).expect_err("a directory is no serial");
    assert!(
        said.contains(&telemetry::audit_serial_path(&dir).display().to_string()),
        "{said}"
    );
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **A POINTER RESOLVES INTO THE KEPT FILES ONLY: `AUDIT_KEEP_FILES - 1` BACK
/// AT MOST.**
///
/// Kills `replace < with <=` in `audit_path_for`: the stream keeps 16 files,
/// the current one and `audit.1` to `audit.15`, so a pointer 16 generations
/// back names `audit.16.ndjson`, a file the stream never keeps, rather than
/// saying the line rolled off.
#[test]
fn a_pointer_past_the_kept_files_resolves_to_nothing() {
    let dir = Path::new("/nonexistent/cloud");
    let keep = u64::from(telemetry::AUDIT_KEEP_FILES);
    assert_eq!(
        telemetry::audit_path_for(dir, 40, 40),
        Some(telemetry::audit_current_path(dir))
    );
    assert_eq!(
        telemetry::audit_path_for(dir, 40, 40 - (keep - 1)),
        Some(telemetry::audit_rolled_path(
            dir,
            telemetry::AUDIT_KEEP_FILES - 1
        ))
    );
    assert_eq!(telemetry::audit_path_for(dir, 40, 40 - keep), None);
    assert_eq!(telemetry::audit_path_for(dir, 40, 41), None, "the future");
}

/// **A LINE WHOSE FILE IS GONE OR SHORT HAS ROLLED OFF; ONE WHOSE FILE CANNOT
/// BE READ IS UNREADABLE.**
///
/// Kills both mutants of the `NotFound | UnexpectedEof` guard in
/// `audit_read`. `with false` turns a file that rolled away (`NotFound`) or
/// no longer holds the line (`UnexpectedEof`) into `Unreadable`, an error
/// the page shows for an ordinary roll; `with true` turns a real read
/// failure (here a directory where the file should be: `EISDIR`) into
/// `RolledOff`, hiding it.
#[test]
fn audit_read_tells_a_rolled_off_line_from_an_unreadable_file() {
    let dir = scratch("audit-read");
    let pointer = telemetry::AuditPointer {
        serial: 3,
        offset: 0,
        seq: 1,
        len: 40,
        level: telemetry::Level::Error,
    };
    assert_eq!(
        telemetry::audit_read(&dir, 3, &pointer),
        telemetry::AuditLine::RolledOff,
        "no file at all"
    );
    std::fs::write(telemetry::audit_current_path(&dir), b"short\n").expect("a short file");
    assert_eq!(
        telemetry::audit_read(&dir, 3, &pointer),
        telemetry::AuditLine::RolledOff,
        "a file that ends before the line"
    );
    std::fs::remove_file(telemetry::audit_current_path(&dir)).expect("removed");
    std::fs::create_dir_all(telemetry::audit_current_path(&dir)).expect("a directory there");
    match telemetry::audit_read(&dir, 3, &pointer) {
        telemetry::AuditLine::Unreadable(said) => assert!(
            said.contains(&telemetry::audit_current_path(&dir).display().to_string()),
            "{said}"
        ),
        other => panic!("a directory is not a rolled-off line: {other:?}"),
    }
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **A POSTING THAT CANNOT BE LOOKED AT IS AN ERROR; ONLY AN ABSENT ONE IS
/// EMPTY.**
///
/// Kills `replace match guard e.kind() == NotFound with true` in
/// `audit_postings_before`: it passes over every generation whose posting
/// cannot be asked about, so a probe through a generation that is a file
/// (`ENOTDIR`) answers "nothing audited carried that value" where it must
/// say it could not look.
#[test]
fn a_posting_that_cannot_be_looked_at_is_an_error() {
    let dir = scratch("postings-unlookable");
    let empty = telemetry::audit_postings_before(&dir, "instrument", "VBL", 5, None)
        .expect("nothing written is an empty answer");
    assert_eq!((empty.total, empty.pointers.len()), (0, 0));
    std::fs::create_dir_all(dir.join(telemetry::AUDIT_INDEX_DIR)).expect("the index");
    std::fs::write(
        telemetry::audit_generation_path(&dir, 0),
        b"not a directory",
    )
    .expect("a file where generation 0 belongs");
    let said = telemetry::audit_postings_before(&dir, "instrument", "VBL", 5, None)
        .expect_err("a generation that is a file cannot be looked in");
    assert!(said.contains("VBL.idx"), "{said}");
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// Writes `seqs` as posting `instrument = VBL` of generation `serial`.
fn posting(dir: &Path, serial: u64, seqs: &[u64]) {
    let path = telemetry::audit_posting_path(dir, serial, "instrument", "VBL").expect("a posting");
    std::fs::create_dir_all(path.parent().expect("a dimension directory")).expect("mkdir");
    let mut bytes = Vec::new();
    for &seq in seqs {
        bytes.extend_from_slice(
            &telemetry::AuditPointer {
                serial,
                offset: seq * 100,
                seq,
                len: 100,
                level: telemetry::Level::Error,
            }
            .image(),
        );
    }
    std::fs::write(path, bytes).expect("the posting");
}

/// **A PAGE FROM A PLACE HOLDS ONLY WHAT IS STRICTLY OLDER: NEWER GENERATIONS
/// WHOLE ARE PASSED OVER, ITS OWN IS READ BELOW THE INDEX.**
///
/// Kills the four mutants of the `before` guards in `audit_postings_before`
/// (`serial > at.serial` to `false` or `==`, `serial == at.serial` to `false`
/// or `!=`), which repeat or skip pointers between pages of a keyed
/// `/logs.json` walk. These were caught only by timeout before (a paging test
/// that never reached its end); each now fails on the page's contents.
#[test]
fn a_page_from_a_place_holds_exactly_the_strictly_older_pointers() {
    let dir = scratch("postings-paging");
    std::fs::create_dir_all(dir.join(telemetry::AUDIT_INDEX_DIR)).expect("the index");
    std::fs::write(telemetry::audit_serial_path(&dir), 1u64.to_le_bytes()).expect("serial 1");
    posting(&dir, 0, &[1, 2]);
    posting(&dir, 1, &[11, 12, 13]);
    let page = |before| {
        let page = telemetry::audit_postings_before(&dir, "instrument", "VBL", 10, before)
            .expect("a page");
        assert_eq!(page.total, 5, "every kept record is counted");
        page.pointers
            .iter()
            .map(|(at, pointer)| (at.serial, at.index, pointer.seq))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        page(None),
        [(1, 2, 13), (1, 1, 12), (1, 0, 11), (0, 1, 2), (0, 0, 1)]
    );
    assert_eq!(
        page(Some(telemetry::PostingAt {
            serial: 1,
            index: 1
        })),
        [(1, 0, 11), (0, 1, 2), (0, 0, 1)],
        "below index 1 of generation 1, then all of generation 0"
    );
    assert_eq!(
        page(Some(telemetry::PostingAt {
            serial: 0,
            index: 1
        })),
        [(0, 0, 1)],
        "generation 1 passed over whole"
    );
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **AN OPEN STREAM'S HEALTH NAMES ITS FILE.**
///
/// Kills `delete field path from struct AuditHealth expression in
/// Audit::open`: the health would carry an empty path, and every "cannot
/// append an audited line" it reports would name no file.
#[test]
fn an_open_audit_streams_health_names_its_file() {
    let dir = scratch("audit-health-path");
    let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("opens");
    let health = sink.audit_health().expect("a stream");
    assert_eq!(health.path, telemetry::audit_current_path(&dir));
    drop(sink);
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **A STREAM THAT ALREADY HOLDS LINES BUT NO INDEX IS COVERED BY THE NEWER
/// DIMENSIONS ONLY FROM WHERE IT STANDS.**
///
/// Kills `replace > with ==`, `replace > with <` and `replace || with &&` in
/// `older = self.bytes > 0 || generation.exists()` of `Audit::mark_coverage`:
/// each takes a file with bytes and no generation for a new one, and records
/// `run` and `code` as covering it from byte 0, so a probe by run would say
/// "nothing carried it" for lines that were never indexed by run.
/// (`replace > with >=` there is equivalent: with no bytes both arms record
/// 0.)
#[test]
fn an_unindexed_stream_is_covered_by_the_newer_dimensions_from_its_end() {
    let dir = scratch("audit-coverage-older");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let held = b"{\"an\":\"earlier line\"}\n";
    std::fs::write(telemetry::audit_current_path(&dir), held).expect("an earlier stream");
    let len = u64::try_from(held.len()).expect("small");
    let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("opens");
    for dimension in telemetry::AUDIT_DIMENSIONS {
        let from = telemetry::audit_coverage(&dir, 0, dimension);
        if telemetry::AUDIT_FIRST_DIMENSIONS.contains(&dimension) {
            assert_eq!(from, Some(0), "{dimension}: kept since the first build");
        } else {
            assert_eq!(from, Some(len), "{dimension}: from the stream's end");
        }
    }
    drop(sink);
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **A REOPEN RECORDS NO DIMENSION TWICE.**
///
/// Kills `replace match guard held.iter().any(..) with false` in
/// `Audit::mark_coverage`: every open would append every dimension again, so
/// the coverage file grows by eight lines at each start, for ever.
#[test]
fn a_reopened_stream_records_no_dimension_twice() {
    let dir = scratch("audit-coverage-reopen");
    let dims = telemetry::audit_generation_path(&dir, 0).join(telemetry::AUDIT_DIMS_FILE);
    let first = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("opens");
    drop(first);
    let before = std::fs::read_to_string(&dims).expect("coverage written at the first open");
    assert_eq!(before.lines().count(), telemetry::AUDIT_DIMENSIONS.len());
    let second = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("reopens");
    drop(second);
    assert_eq!(
        std::fs::read_to_string(&dims).expect("still there"),
        before,
        "nothing appended"
    );
    let _ignored = std::fs::remove_dir_all(&dir);
}

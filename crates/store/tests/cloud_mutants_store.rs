//! Mutation-survivor tests for `store`'s public surface.
//!
//! Each test here kills a mutant `cargo mutants` left alive in one of the
//! store's source files (cloud session 2026-10-16, report
//! `reports/cloud-mut-store-20261016.md`). Its doc comment names the mutant
//! and the behaviour that mutant breaks; the assertion is that behaviour.
//! Private items are proved in a `cloud_mutant_tests` module at the end of
//! their own source file.
//!
//! Every directory is the test's own under the temporary directory, and
//! every byte written is invented: no vendor row is read or copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail; a table test is as long as \
              its table."
)]

mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory of this test's own.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-store-cloud-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ignored = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// **A STAGING NAME THE HOST REFUSES TO CREATE IS REFUSED AS A CREATE, NOT
/// LOOKED AT.**
///
/// Kills `replace match guard why.kind() == ErrorKind::AlreadyExists with
/// true` in `staging::create`: every refusal of the first `open` would be
/// taken for "something is there", so a name below a regular file
/// (`ENOTDIR`) is answered by the `lstat` that follows, as a `stat` failure,
/// with a wasted host call, where the host's refusal was the create's.
#[test]
fn a_staging_create_the_host_refuses_is_named_a_create() {
    let dir = scratch("staging-enotdir");
    let file = dir.join("a-file");
    fs::write(&file, b"synthetic").unwrap();
    let refusal = store::staging::create(&file.join("a.tmp")).unwrap_err();
    assert!(
        matches!(
            &refusal,
            store::staging::StagingError::Io {
                action: "create",
                ..
            }
        ),
        "{refusal}"
    );
    assert_eq!(fs::read(&file).unwrap(), b"synthetic", "nothing touched");
    let _ignored = fs::remove_dir_all(&dir);
}

/// **EVERY KIND A READER REFUSES HAS ITS OWN WORD.**
///
/// Kills `replace Found::word -> &'static str with ""` and `with "xyzzy"`:
/// the word is what a refusal's line and its `found=` field carry, so a
/// blank or invented word would leave an operator unable to tell a planted
/// pipe from a link. Each word is pinned, they are distinct, and a refusal's
/// text carries the word of what it found.
#[test]
fn every_found_kind_has_its_own_word_in_the_refusal() {
    use store::safe_open::Found;
    let table = [
        (Found::Symlink, "symlink"),
        (Found::Pipe, "pipe"),
        (Found::Socket, "socket"),
        (Found::CharDevice, "char_device"),
        (Found::BlockDevice, "block_device"),
        (Found::Directory, "directory"),
        (Found::Unknown, "unknown"),
    ];
    for (found, word) in table {
        assert_eq!(found.word(), word, "{found:?}");
    }
    let dir = scratch("found-word");
    let refusal = store::safe_open::open_read(&dir).unwrap_err();
    assert_eq!(refusal.found(), Some(Found::Directory));
    assert!(
        refusal
            .to_string()
            .starts_with("read_not_regular (directory): "),
        "{refusal}"
    );
    let _ignored = fs::remove_dir_all(&dir);
}

/// **A LEFTOVER THAT GOES BETWEEN THE CREATE AND THE LOOK LEAVES THE NAME
/// FREE, AND THE CREATE GOES ON.**
///
/// Kills the mutants of `staging::create` that only a race between its host
/// calls reaches: the `lstat` guard `why.kind() == ErrorKind::NotFound` to
/// `false` and `==` to `!=` (a name refused by `O_EXCL` and gone by the
/// `lstat` is refused as a `stat` failure, though nothing stands in the
/// writer's way); the `remove` guard `why.kind() != ErrorKind::NotFound` to
/// `true` (a leftover gone by the `unlink` is refused as a `remove`
/// failure); and the second create's `AlreadyExists` guard to `false` (a
/// name taken again after its removal is an `io` failure, not
/// `staging_contended`). One thread makes and removes a file at the name as
/// fast as it can while this one creates at it, until the vanish branch
/// (three host calls, nothing cleared) and `staging_contended` have each been
/// seen twenty times. Every answer on the way must be a new file,
/// `staging_contended`, or the zero-link refusal a look mid-unlink sees.
#[test]
fn a_leftover_that_vanishes_before_the_look_leaves_the_create_free() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    let dir = scratch("staging-vanish");
    let path = dir.join("raced.tmp");
    let stop = Arc::new(AtomicBool::new(false));
    let racer = {
        let (path, stop) = (path.clone(), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let _made = fs::File::create_new(&path);
                let _gone = fs::remove_file(&path);
            }
        })
    };
    let deadline = Instant::now() + Duration::from_mins(1);
    let (mut vanished, mut contended) = (0u32, 0u32);
    while (vanished < 20 || contended < 20) && Instant::now() < deadline {
        match store::staging::create(&path) {
            Ok(made) => {
                if (made.calls, made.cleared) == (3, None) {
                    vanished += 1;
                }
                drop(made);
                let _gone = fs::remove_file(&path);
            }
            // A look that lands while the racer's unlink is in flight sees
            // the file with no name left (`nlink` 0): refused as linked,
            // nothing removed. Safe, and recorded in the report as a
            // misnamed refusal; it is not this test's mutant's answer.
            Err(store::staging::StagingError::Linked { links: 0, .. }) => {}
            Err(refusal) => {
                assert_eq!(refusal.word(), "staging_contended", "{refusal}");
                contended += 1;
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    racer.join().unwrap();
    assert!(
        vanished >= 20 && contended >= 20,
        "in 60 s of racing: {vanished} vanished, {contended} contended"
    );
    let _ignored = fs::remove_dir_all(&dir);
}

#[test]
fn a_leftover_whose_name_cannot_be_removed_is_refused_as_a_remove() {
    support::where_permission_binds(
        "a_leftover_whose_name_cannot_be_removed_is_refused_as_a_remove",
        a_leftover_whose_name_cannot_be_removed_is_refused_as_a_remove_body,
    );
}

/// **A LEFTOVER WHOSE NAME THE HOST WILL NOT REMOVE IS REFUSED AS A REMOVE,
/// AND LEFT WHOLE.**
///
/// Kills `replace match guard why.kind() != ErrorKind::NotFound with false`
/// and `replace != with ==` in `staging::create`: both swallow the `unlink`
/// refusal of a read-only directory (`EACCES`), and the create that follows
/// meets the leftover still there and says `staging_contended`, "another
/// writer is using the name", when the truth is that this one cannot clear
/// it. Run where the mode bits bind (D-0995).
fn a_leftover_whose_name_cannot_be_removed_is_refused_as_a_remove_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = scratch("staging-readonly");
    let path = dir.join("a.tmp");
    fs::write(&path, b"a dead writer's half").unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let answer = store::staging::create(&path);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let refusal = answer.unwrap_err();
    assert!(
        matches!(
            &refusal,
            store::staging::StagingError::Io {
                action: "remove",
                source,
                ..
            } if source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{refusal}"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        b"a dead writer's half",
        "left whole"
    );
    let _ignored = fs::remove_dir_all(&dir);
}

/// **EVERY STAGING REFUSAL SAYS WHAT IT IS, NAMES ITS PATH, AND CARRIES THE
/// HOST'S ERROR ONLY WHEN THERE IS ONE.**
///
/// Covers the refusal text, `path` and `source` of the two refusals the other
/// staging tests meet only through a race or a host error: the operator
/// reads these words in the caller's event.
#[test]
fn every_staging_refusal_names_itself_its_path_and_its_cause() {
    use std::error::Error as _;
    use store::staging::StagingError;
    let path = PathBuf::from("/nonexistent/cloud/a.tmp");
    let contended = StagingError::Contended { path: path.clone() };
    assert_eq!(contended.word(), "staging_contended");
    assert_eq!(contended.path(), path);
    assert_eq!(
        contended.to_string(),
        "staging_contended: /nonexistent/cloud/a.tmp existed again after it was cleared; \
         another writer is using the name"
    );
    assert!(contended.source().is_none());
    let io = StagingError::Io {
        path: path.clone(),
        action: "remove",
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    };
    assert_eq!(io.path(), path);
    assert_eq!(
        io.to_string(),
        "io: remove /nonexistent/cloud/a.tmp: permission denied"
    );
    assert_eq!(
        io.source().map(ToString::to_string).as_deref(),
        Some("permission denied")
    );
    let not_regular = StagingError::NotRegular {
        path: path.clone(),
        found: store::safe_open::Found::Pipe,
    };
    assert_eq!(not_regular.path(), path);
    assert!(not_regular.source().is_none());
    let linked = StagingError::Linked {
        path: path.clone(),
        links: 3,
    };
    assert_eq!(linked.path(), path);
    assert!(linked.source().is_none());
}

/// **EVERY KIND A READER REFUSES IS SAID IN WORDS AN OPERATOR CAN ACT ON.**
///
/// Covers [`store::safe_open::Found::sentence`] for every kind, `Unknown`
/// included: no host this crate builds for produces a file type outside the
/// other six, so only a direct call reaches that arm.
#[test]
fn every_found_kind_has_its_sentence() {
    use store::safe_open::Found;
    let table = [
        (Found::Symlink, "a symbolic link"),
        (Found::Pipe, "a pipe (FIFO)"),
        (Found::Socket, "a UNIX socket"),
        (Found::CharDevice, "a character device"),
        (Found::BlockDevice, "a block device"),
        (Found::Directory, "a directory"),
        (Found::Unknown, "a file of a kind this host does not name"),
    ];
    for (found, sentence) in table {
        assert_eq!(found.sentence(), sentence, "{found:?}");
    }
}

/// A one-minute month for the generation tests: invented `CLOUDIDX` bars in
/// 2024-06.
fn minute_month() -> store::path::StorePath<'static> {
    store::path::StorePath::new(store::path::PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "CLOUDIDX",
        contract: None,
        timeframe: store::path::Timeframe::MINUTE_1,
        month: store::path::YearMonth::new(2024, 6).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap()
}

/// Five invented minute bars after 09:15 IST on each of `days` of June 2024.
fn minute_sessions(days: &[i64]) -> Vec<store::format::Bar> {
    const JUNE_IST_MIDNIGHT: i64 = 1_717_180_200;
    days.iter()
        .flat_map(|&day| {
            (0..5).map(move |minute| {
                let secs = JUNE_IST_MIDNIGHT + (day - 1) * 86_400 + 555 * 60 + minute * 60;
                let price = 3_000_000 + day * 1_000 + minute;
                store::format::Bar {
                    ts_micros: secs * 1_000_000,
                    open: price,
                    high: price + 9,
                    low: price - 9,
                    close: price + 2,
                    volume: 0,
                    open_interest: store::format::OI_NULL,
                }
            })
        })
        .collect()
}

/// **A REVISION WRITTEN TO AFTER ITS PROMOTION IS REFUSED BY EVERY REBUILD
/// DOOR, BY NAME.**
///
/// Covers the D-3350 refusal in `BarFile::sort_index_existing` ("not
/// sorted"), `BarFile::index_existing` ("not indexed") and
/// `BarFile::open_existing_audited` ("not audited"): a promoted revision
/// appended to outside the repair protocol no longer matches the pointer its
/// promotion wrote, and no door sorts, indexes or audits it as if it did.
#[test]
fn every_rebuild_door_refuses_a_revision_that_moved_after_promotion() {
    use store::file::{BarFile, StoreError};
    const SYMBOL: u32 = 11;
    let root = scratch("revision-moved");
    let month = minute_month();
    BarFile::open_or_create(&root, month, SYMBOL)
        .unwrap()
        .append(&minute_sessions(&[17]))
        .unwrap();
    let policy = store::repair::Policy {
        open_interest_applies: false,
        basis_witness: false,
    };
    store::repair::repair_month(&root, month, SYMBOL, &minute_sessions(&[3, 17]), policy).unwrap();
    BarFile::open_or_create(&root.join("bar-revisions-v1/1"), month, SYMBOL)
        .unwrap()
        .append(&minute_sessions(&[20]))
        .unwrap();
    let pointer = store::generation::pointer_path(&root, month);
    for (door, answer) in [
        (
            "not sorted",
            BarFile::sort_index_existing(&root, month, SYMBOL).map(|_| ()),
        ),
        (
            "not indexed",
            BarFile::index_existing(&root, month, SYMBOL).map(|_| ()),
        ),
    ] {
        match answer {
            Err(StoreError::Generation { path, reason }) => {
                assert_eq!(path, pointer, "{door}");
                assert_eq!(
                    reason,
                    format!("revision 1 is not the one its promotion recorded; {door}")
                );
            }
            other => panic!("{door}: {other:?}"),
        }
    }
    let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
        panic!("a moved revision is audited");
    };
    assert!(
        audited.starts_with("repair revision 1 of ")
            && audited.ends_with(" is not the one its promotion recorded; not audited"),
        "{audited}"
    );
    let _ignored = fs::remove_dir_all(&root);
}

/// A one-second NIFTY month in 2031-02, the sort-index tests' shape.
fn second_month(file: store::path::FileKind) -> store::path::StorePath<'static> {
    store::path::StorePath::new(store::path::PathParts {
        vendor: brutex_core::vendor::Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: store::path::Timeframe::SECOND_1,
        month: store::path::YearMonth::new(2031, 2).unwrap(),
        file,
    })
    .unwrap()
}

/// Invented one-second bar `i` of 2031-02, from 09:15 IST on the 1st, its
/// stamp moved `nudge` microseconds.
fn second_bar(i: i64, nudge: i64) -> store::format::Bar {
    let origin = store::path::YearMonth::new(2031, 2)
        .unwrap()
        .ist_bounds_micros()
        .0;
    let close = 5_550_000 + (i * 7_919) % 613;
    store::format::Bar {
        ts_micros: origin + (555 * 60 + i) * 1_000_000 + nudge,
        open: close - 11,
        high: close + 40,
        low: close - 40,
        close,
        volume: (i * 31) % 19,
        open_interest: store::format::OI_NULL,
    }
}

fn second_bars(from: i64, to: i64) -> Vec<store::format::Bar> {
    (from..to).map(|i| second_bar(i, 0)).collect()
}

/// **THE REBUILD DOOR NAMES EVERY WAY AN INDEX FAILS TO DESCRIBE ITS MONTH,
/// AND WRITES IT WHOLE.**
///
/// Covers each reason `BarFile::sort_index_existing` gives before it writes a
/// month whole: runs that order only the first rows, bytes past the last run,
/// an index that will not walk, and runs that are another month's bars. Each
/// time the answer is one run over every row, and the bars are untouched.
#[test]
fn the_rebuild_door_names_each_way_an_index_fails_its_month() {
    use store::file::{BarFile, Resorted};
    use store::path::FileKind;
    const SYMBOL: u32 = 11;
    let root = scratch("srt-reasons");
    let srt = second_month(FileKind::SortIndex).to_path_buf(&root);
    let bars = second_month(FileKind::Bars);
    let mut writer = BarFile::open_or_create(&root, bars, SYMBOL).unwrap();
    writer.append(&second_bars(0, 1_100)).unwrap();
    let first_rows_only = fs::read(&srt).expect("an index after 1,100 rows");
    writer.append(&second_bars(1_100, 1_200)).unwrap();
    drop(writer);
    let bars_before = fs::read(bars.to_path_buf(&root)).unwrap();
    let rebuild = || BarFile::sort_index_existing(&root, bars, SYMBOL).unwrap();
    let built = |answer: store::file::SortIndexed| match answer.outcome {
        Resorted::Built(why) => {
            assert_eq!((answer.n_valid, answer.runs), (1_200, 1));
            why
        }
        other => panic!("{other:?}"),
    };

    fs::write(&srt, &first_rows_only).unwrap();
    assert_eq!(built(rebuild()), "its runs order rows 0..1100 of 1200");

    let whole = fs::read(&srt).unwrap();
    let mut trailing = whole.clone();
    trailing.extend_from_slice(b"not a run");
    fs::write(&srt, &trailing).unwrap();
    let why = built(rebuild());
    assert!(
        why.starts_with("bytes past its last run are not a run ("),
        "{why}"
    );
    assert_eq!(fs::read(&srt).unwrap(), whole, "rebuilt to the same bytes");

    fs::write(&srt, b"not an index").unwrap();
    let why = built(rebuild());
    assert!(!why.is_empty(), "the walk's refusal is named");

    // Another month's bars of the same shape: an index that walks and
    // describes nothing here.
    let other = scratch("srt-reasons-other");
    let mut writer = BarFile::open_or_create(&other, bars, SYMBOL).unwrap();
    writer
        .append(&(0..1_200).map(|i| second_bar(i + 7, 0)).collect::<Vec<_>>())
        .unwrap();
    drop(writer);
    fs::copy(second_month(FileKind::SortIndex).to_path_buf(&other), &srt).unwrap();
    let why = built(rebuild());
    assert!(!why.is_empty(), "the bind's refusal is named");

    assert_eq!(rebuild().outcome, Resorted::Kept, "and kept after");
    assert_eq!(fs::read(bars.to_path_buf(&root)).unwrap(), bars_before);
    let _ignored = fs::remove_dir_all(&root);
    let _ignored = fs::remove_dir_all(&other);
}

/// **A SMALL MONTH'S MISSING TIME INDEX IS SAID AS THE RULE, NOT AS DAMAGE.**
///
/// Covers `time_index::Why::BelowThreshold`'s text: the reason a reader
/// prints for a month under `INDEX_MIN_ROWS` bars names the threshold, the
/// rule and its bound, so an operator does not take it for a lost `.tix`.
#[test]
fn a_small_months_missing_time_index_is_said_as_the_rule() {
    assert_eq!(store::time_index::INDEX_MIN_ROWS, 1_024);
    assert_eq!(
        store::time_index::Why::BelowThreshold.to_string(),
        "the month holds fewer than 1024 bars, so by the small-month rule it keeps no .tix \
         time index and a lookup bisects in at most ten bar reads (D-4933)"
    );
}

/// **EVERY VERDICT REFUSAL HAS ITS WORD, ITS EXACT SENTENCE, AND ITS CAUSE
/// ONLY WHEN IT HAS ONE.**
///
/// The verdict store's refusals are what an operator reads in a `store.verdict`
/// event and on the CLI; each variant's word and text is pinned here,
/// exactly, with the variables it names. `source` is the host's error for
/// `io` and the staging refusal for a staging word, and nothing for the
/// rest.
#[test]
fn every_verdict_refusal_has_its_word_its_sentence_and_its_cause() {
    use std::error::Error as _;
    use store::verdict::VerdictError as V;
    let p = || PathBuf::from("/nonexistent/cloud/2031.vdi");
    let image = PathBuf::from("/nonexistent/cloud/2031-03-04.g0000000002.vdt");
    let table: Vec<(V, &str, String)> =
        vec![
        (
            V::KeyTooLong { len: 65 },
            "key_too_long",
            "key_too_long: an instrument key of 65 bytes does not fit the 64-byte key field, \
             and a key is never cut"
                .to_owned(),
        ),
        (
            V::KeyInvalid,
            "key_invalid",
            "key_invalid: a key is empty or holds a NUL byte".to_owned(),
        ),
        (
            V::DuplicateKey {
                key: "ZZCLOUD".to_owned(),
            },
            "duplicate_key",
            "duplicate_key: one generation names ZZCLOUD twice".to_owned(),
        ),
        (
            V::TableTooLarge { records: 9 },
            "table_too_large",
            "table_too_large: no table of at most 1048576 slots holds 9 keys within 8 slots of \
             home with no repeated tag (format version 1's record ceiling); nothing was written \
             and the day's verdict is pending"
                .to_owned(),
        ),
        (
            V::RunIdOverflow { run: 1 << 33 },
            "run_id_overflow",
            "run_id_overflow: run id 8589934592 does not fit a record's u32 run field".to_owned(),
        ),
        (
            V::GenerationCollision { path: image.clone() },
            "generation_collision",
            "generation_collision: 16 generations up to \
             /nonexistent/cloud/2031-03-04.g0000000002.vdt were all already linked; none is \
             overwritten"
                .to_owned(),
        ),
        (
            V::FlagUnknown { bits: 0x300 },
            "flag_unknown",
            "flag_unknown: flag bits 0x300 are not named by format version 1, and a writer \
             never writes a fact it cannot name"
                .to_owned(),
        ),
        (
            V::DetailInvalid { kind: 0, len: 3 },
            "detail_invalid",
            "detail_invalid: a detail of kind 0 and 3 bytes (kind 0 is unset; a payload holds at \
             most 65535 bytes)"
                .to_owned(),
        ),
        (
            V::DayOutOfRange {
                day: store::provenance::Day::new(1969, 12, 31).unwrap(),
            },
            "day_out_of_range",
            "day_out_of_range: 1969-12-31 is before 1970-01-01, which a day field cannot hold"
                .to_owned(),
        ),
        (
            V::FeedUnknown { feed: "cloudfeed" },
            "feed_unknown",
            "feed_unknown: vendor cloudfeed is not in the store's vendor table".to_owned(),
        ),
        (
            V::LockStalled {
                path: p(),
                waited_ms: 30_000,
                tries: 3_001,
            },
            "lock_stalled",
            "lock_stalled: /nonexistent/cloud/2031.vdi stayed held through 3001 tries over 30000 \
             ms; the commit was not made"
                .to_owned(),
        ),
        (
            V::Unknown {
                path: p(),
                what: "codec",
                value: 7,
            },
            "written_by_newer_build",
            "written_by_newer_build: /nonexistent/cloud/2031.vdi holds codec 7, which this build \
             does not know"
                .to_owned(),
        ),
        (
            V::IndexShort { path: p(), len: 9 },
            "index_short",
            "index_short: /nonexistent/cloud/2031.vdi is 9 bytes, shorter than the 32768 a year \
             index is born with"
                .to_owned(),
        ),
        (
            V::IndexUnreadable { path: p() },
            "index_unreadable",
            "index_unreadable: neither slot of /nonexistent/cloud/2031.vdi is a valid year-index \
             slot"
                .to_owned(),
        ),
        (
            V::IndexMissing {
                path: p(),
                image: Some(image.clone()),
                seen: 2,
            },
            "index_missing",
            "index_missing: /nonexistent/cloud/2031.vdi is absent, but \
             /nonexistent/cloud/2031-03-04.g0000000002.vdt is a day image of its year (2 name(s) \
             looked at); the year is not born again empty over it"
                .to_owned(),
        ),
        (
            V::IndexMissing {
                path: p(),
                image: None,
                seen: 1_024,
            },
            "index_missing",
            "index_missing: /nonexistent/cloud/2031.vdi is absent and its year directory holds \
             1024 or more names, none of the first 1024 a day image; no image could be ruled \
             out, so the year is not born again"
                .to_owned(),
        ),
        (
            V::OrdinalPastRecords {
                path: image.clone(),
                ordinal: 5,
                records: 2,
            },
            "ordinal_past_records",
            "ordinal_past_records: record 5 was asked of \
             /nonexistent/cloud/2031-03-04.g0000000002.vdt, which holds 2 record(s)"
                .to_owned(),
        ),
        (
            V::ImageMissing {
                path: image.clone(),
            },
            "image_missing",
            "image_missing: the year index commits \
             /nonexistent/cloud/2031-03-04.g0000000002.vdt, which does not exist"
                .to_owned(),
        ),
        (
            V::ImageCorrupt {
                path: image.clone(),
                why: "the header's magic".to_owned(),
            },
            "image_corrupt",
            "image_corrupt: /nonexistent/cloud/2031-03-04.g0000000002.vdt: the header's magic"
                .to_owned(),
        ),
        (
            V::TrailerMismatch {
                path: image.clone(),
            },
            "trailer_mismatch",
            "trailer_mismatch: /nonexistent/cloud/2031-03-04.g0000000002.vdt does not hash to \
             its BLAKE3 trailer; a byte changed after it was linked"
                .to_owned(),
        ),
        (
            V::RecordCorrupt {
                path: image.clone(),
                ordinal: 4,
            },
            "record_corrupt",
            "record_corrupt: record 4 of /nonexistent/cloud/2031-03-04.g0000000002.vdt fails its \
             CRC-32C"
                .to_owned(),
        ),
        (
            V::Symlinked { path: p() },
            "symlinked",
            "symlinked: /nonexistent/cloud/2031.vdi is a symbolic link, which the verdict store \
             never follows"
                .to_owned(),
        ),
        (
            V::NotRegular {
                path: p(),
                found: store::safe_open::Found::Pipe,
            },
            "not_regular",
            "not_regular (pipe): /nonexistent/cloud/2031.vdi is a pipe (FIFO), not a regular \
             file; the verdict store reads and writes nothing else, and nothing waited on it"
                .to_owned(),
        ),
    ];
    for (refusal, word, text) in &table {
        assert_eq!(refusal.word(), *word, "{refusal:?}");
        assert_eq!(&refusal.to_string(), text, "{refusal:?}");
        assert!(refusal.source().is_none(), "{refusal:?}");
    }
    let io = V::Io {
        path: p(),
        action: "read",
        source: std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
    };
    assert_eq!(io.word(), "io");
    assert_eq!(
        io.to_string(),
        "io: read /nonexistent/cloud/2031.vdi: unexpected end of file"
    );
    assert_eq!(
        io.source().map(ToString::to_string).as_deref(),
        Some("unexpected end of file")
    );
    let staging = V::Staging(store::staging::StagingError::Contended {
        path: PathBuf::from("/nonexistent/cloud/2031.vdi.1-1.tmp"),
    });
    assert_eq!(staging.word(), "staging_contended");
    assert_eq!(
        staging.to_string(),
        "staging_contended: /nonexistent/cloud/2031.vdi.1-1.tmp existed again after it was \
         cleared; another writer is using the name"
    );
    assert_eq!(
        staging.source().map(ToString::to_string),
        Some(staging.to_string())
    );
}

/// Each value of a one-byte enumeration is filed under its byte, read back as
/// itself and says its word; a byte past the table is `None`.
fn round<T: Copy + PartialEq + std::fmt::Debug>(
    all: &[T],
    code: fn(T) -> u8,
    from: fn(u8) -> Option<T>,
    word: fn(T) -> &'static str,
    words: &[(u8, &str)],
) {
    assert_eq!(all.len(), words.len());
    for (value, (byte, said)) in all.iter().zip(words) {
        assert_eq!(code(*value), *byte, "{value:?}");
        assert_eq!(from(*byte), Some(*value));
        assert_eq!(word(*value), *said);
    }
    assert_eq!(from(200), None, "a byte this build does not know");
}

/// **EVERY VERDICT CODE AND ONE-BYTE ENUMERATION ROUND-TRIPS THROUGH ITS BYTE,
/// AND SAYS ITS WORD.**
///
/// The words are what the routes and documents print (D-4809: numbered
/// append-only, never renamed); a byte this build does not know is `None`,
/// never another value. Each table is the format's own, in code order.
#[test]
fn every_verdict_code_and_byte_enumeration_round_trips_and_says_its_word() {
    use store::verdict::{Code, DayState, JournalOutcome, Mode, SourceHeld, Unit, V3Manifest};
    let codes = [
        "match",
        "value_mismatch",
        "missing_in_store",
        "extra_in_store",
        "count_mismatch",
        "source_refused",
        "store_without_source",
        "absent_from_store",
        "store_unreadable",
        "no_response_kept",
        "not_compared",
        "source_lacks_day",
        "vendor_hole",
        "withheld_by_rule",
        "built_under_earlier_rule",
        "day_partial",
        "no_ledger",
        "owed",
    ];
    assert_eq!(Code::ALL.len(), codes.len());
    for (at, (code, word)) in Code::ALL.iter().zip(codes).enumerate() {
        assert_eq!(usize::from(code.code()), at + 1);
        assert_eq!(Code::from_code(code.code()), Some(*code));
        assert_eq!(code.word(), word);
        assert_eq!(code.to_string(), word);
    }
    assert_eq!(Code::from_code(0), None, "unset");
    assert_eq!(Code::from_code(19), None, "a later build's");

    round(
        Mode::ALL,
        Mode::code,
        Mode::from_code,
        Mode::word,
        &[
            (1, "inline"),
            (2, "cold"),
            (3, "completeness"),
            (4, "deferred"),
            (5, "unverified_import"),
            (6, "zerodha_merge"),
            (7, "zerodha_cold"),
        ],
    );
    round(
        Unit::ALL,
        Unit::code,
        Unit::from_code,
        Unit::word,
        &[(0, "second"), (1, "minute"), (2, "day")],
    );
    round(
        DayState::ALL,
        DayState::code,
        DayState::from_code,
        DayState::word,
        &[
            (0, "complete"),
            (1, "stopped"),
            (2, "source_lost"),
            (3, "disk_full"),
            (4, "census"),
            (5, "held_back_cap"),
            (6, "not_imported"),
            (7, "verify_off"),
        ],
    );
    round(
        JournalOutcome::ALL,
        JournalOutcome::code,
        JournalOutcome::from_code,
        JournalOutcome::word,
        &[
            (0, "none"),
            (1, "done"),
            (2, "done_with_refusals"),
            (3, "incomplete"),
            (4, "refused"),
        ],
    );
    round(
        SourceHeld::ALL,
        SourceHeld::code,
        SourceHeld::from_code,
        SourceHeld::word,
        &[(0, "no"), (1, "yes"), (2, "unknown")],
    );
    round(
        V3Manifest::ALL,
        V3Manifest::code,
        V3Manifest::from_code,
        V3Manifest::word,
        &[
            (0, "absent"),
            (1, "agrees"),
            (2, "disagrees"),
            (3, "unverified"),
        ],
    );
}

/// **A FLAG BIT THIS BUILD DOES NOT NAME IS KEPT, AND SAID BY NUMBER.**
///
/// Covers `flags::unnamed`: a reader keeps every bit a later build appended
/// (D-4844) and names it `flag_<n>` beside the named ones.
#[test]
fn a_flag_bit_this_build_does_not_name_is_kept_and_said_by_number() {
    use store::verdict::flags;
    let later = 1u64 << 22 | 1 << 40;
    let bits = flags::TIX_ABSENT | flags::SOURCE_CRC_UNRECORDED | later;
    assert_eq!(flags::unnamed(bits), later);
    assert_eq!(flags::unnamed(flags::KNOWN), 0);
    assert_eq!(
        flags::words(bits),
        ["tix_absent", "source_crc_unrecorded", "flag_22", "flag_40"]
    );
}

/// **EVERY FILING BYTE ROUND-TRIPS, AND A BYTE NO TABLE HOLDS NAMES
/// NOTHING.**
///
/// Covers the exchange, segment and feed bytes of a verdict image's series:
/// each value is filed under its own byte and read back as itself, and 0 or
/// a byte past the table is `None`, never another exchange, segment or
/// vendor.
#[test]
fn every_verdict_filing_byte_round_trips_and_an_unknown_one_names_nothing() {
    use brutex_core::instrument::{Exchange, Segment};
    use brutex_core::vendor::Vendor;
    use store::verdict::{
        exchange_code, exchange_of, feed_code, feed_of, segment_code, segment_of,
    };
    for (exchange, byte) in [(Exchange::Nse, 1), (Exchange::Bse, 2)] {
        assert_eq!(exchange_code(exchange), byte);
        assert_eq!(exchange_of(byte), Some(exchange));
    }
    assert_eq!(exchange_of(0), None);
    assert_eq!(exchange_of(3), None);
    for (segment, byte) in [(Segment::Index, 1), (Segment::Cash, 2), (Segment::Fno, 3)] {
        assert_eq!(segment_code(segment), byte);
        assert_eq!(segment_of(byte), Some(segment));
    }
    assert_eq!(segment_of(0), None);
    assert_eq!(segment_of(4), None);
    for (at, vendor) in Vendor::ALL.iter().enumerate() {
        let byte = u8::try_from(at + 1).unwrap();
        assert_eq!(feed_code(*vendor), Some(byte));
        assert_eq!(feed_of(byte), Some(*vendor));
    }
    assert_eq!(feed_of(0), None);
    assert_eq!(feed_of(u8::try_from(Vendor::ALL.len() + 1).unwrap()), None);
}

/// **A STEM LONGER THAN THE KEY FIELD IS CUT AT A CHARACTER BOUNDARY, KEEPS
/// ITS WHOLE NAME, AND HASHES AS THE WHOLE.**
///
/// Covers `Key::hash` and the stem's cut: the kept text never splits a
/// character, `whole` and `Display` give the name as written, and the hash a
/// probe uses is the whole name's, so two stems sharing their first 64 bytes
/// are two keys.
#[test]
fn a_long_stem_is_cut_at_a_character_and_hashed_whole() {
    use store::verdict::{KEY_LEN, Key, key_hash};
    let whole = format!("{}é-TAIL", "Z".repeat(KEY_LEN - 1));
    let key = Key::stem(&whole).unwrap();
    assert_eq!(key.text(), "Z".repeat(KEY_LEN - 1), "é is not split");
    assert_eq!(key.whole(), Some(whole.as_str()));
    assert_eq!(key.to_string(), whole);
    assert_eq!(key.hash(), key_hash(whole.as_bytes()));
    let other = Key::stem(&format!("{}é-OTHER", "Z".repeat(KEY_LEN - 1))).unwrap();
    assert_eq!(other.text(), key.text());
    assert_ne!(other.hash(), key.hash());
    let short = Key::instrument("ZZCLOUD").unwrap();
    assert_eq!(
        (short.whole(), short.to_string()),
        (None, "ZZCLOUD".to_owned())
    );
    assert_eq!(short.hash(), key_hash(b"ZZCLOUD"));
}

/// A verdict series of this test's own root: GDFL, NSE, F&O, one-second.
fn verdict_series(root: &std::path::Path) -> store::verdict::Series<'_> {
    store::verdict::Series {
        root,
        feed: brutex_core::vendor::Vendor::Gdfl,
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Fno,
        timeframe: store::path::Timeframe::SECOND_1,
    }
}

fn verdict_day() -> store::provenance::Day {
    store::provenance::Day::new(2031, 3, 4).unwrap()
}

/// One invented generation of `entries`, each `(key, code, details)`.
fn verdict_generation(
    entries: &[(&str, store::verdict::Code, Vec<store::verdict::Detail>)],
) -> store::verdict::Generation {
    use store::verdict::{Carry, Code, DayBlock, Entry, Facts, Generation, Key, Mode};
    Generation {
        day: verdict_day(),
        run: 1,
        written_at: 1_900_000_000,
        verifier: 1,
        mode: Mode::Cold,
        rule_set: 0,
        origin: [7; 16],
        day_block: DayBlock::of(Code::Match),
        day_details: Vec::new(),
        entries: entries
            .iter()
            .map(|(name, code, details)| Entry {
                key: Key::instrument(name).unwrap(),
                facts: Facts::of(*code),
                details: details.clone(),
            })
            .collect(),
        carry: Carry::WholeDay,
    }
}

/// **A READER ANSWERS WHAT IS NOT THERE AS ABSENT, AND SAYS WHY A DAY HOLDS
/// NO RECORD.**
///
/// Covers the reader's empty answers: a page of a day never committed is
/// `None`, a generation with no detail section has no details, a key the day
/// does not hold is answered by the day's `NoRecord` and its code, and a new
/// reader holds no handle. The staleness words are pinned beside them.
#[test]
fn a_verdict_reader_answers_what_is_not_there_as_absent() {
    use store::verdict::{Code, Found, Key, NoRecord, Reader, Stale, Unknown, commit};
    let root = scratch("verdict-absent");
    let series = verdict_series(&root);
    let mut reader = Reader::default();
    assert_eq!(reader.cached(), 0);
    assert_eq!(
        reader
            .page(&series, verdict_day(), Code::Match, 0, 10)
            .unwrap(),
        None,
        "a day never committed"
    );
    let committed = commit(
        &series,
        &verdict_generation(&[("ZZA", Code::Match, Vec::new())]),
    )
    .unwrap();
    assert_eq!(
        reader
            .details(&series, verdict_day(), committed.generation, None)
            .unwrap(),
        Vec::new(),
        "no detail section"
    );
    let lookup = reader
        .lookup(
            &series,
            verdict_day(),
            &Key::instrument("ZZNOTHERE").unwrap(),
        )
        .unwrap();
    let answer = lookup.answer.expect("the day is committed");
    assert_eq!(answer.found, Found::NoRecord(NoRecord::NotInDay));
    assert_eq!(answer.code(), None);
    assert!(reader.cached() > 0, "the image's handle is held");
    for (stale, word) in [
        (
            Stale::Verifier {
                image: 1,
                running: 2,
            },
            "verifier",
        ),
        (Stale::CrossKindPending, "cross_kind_pending"),
        (
            Stale::StoreChanged {
                verified: (1, 0, 9),
                now: (2, 0, 9),
            },
            "store_changed",
        ),
        (Stale::StoreUnknown(Unknown::NoTix), "unknown"),
    ] {
        assert_eq!(stale.word(), word);
    }
    let _ignored = fs::remove_dir_all(&root);
}

/// **A READER REFUSES A DAMAGED YEAR INDEX OR DAY IMAGE BY NAME.**
///
/// Covers the reader's own checks, which run without the BLAKE3 trailer a
/// lookup never reads: a year index cut short, a slot naming a record past
/// the records, an image longer than its header implies, and a detail entry
/// that runs past its section.
#[test]
fn a_verdict_reader_refuses_a_damaged_index_or_image_by_name() {
    use store::verdict::{
        Code, Detail, Found, Key, Reader, SLOT_LEN, TABLE_OFFSET, VerdictError, commit,
    };
    let corrupt_why = |answer: Result<(), VerdictError>| match answer {
        Err(VerdictError::ImageCorrupt { why, .. }) => why,
        other => panic!("not image_corrupt: {other:?}"),
    };
    let root = scratch("verdict-damaged");
    let series = verdict_series(&root);
    let detail = Detail {
        kind: 2,
        payload: b"cloud".to_vec(),
    };
    let committed = commit(
        &series,
        &verdict_generation(&[
            ("ZZA", Code::Match, Vec::new()),
            ("ZZB", Code::ValueMismatch, vec![detail.clone()]),
        ]),
    )
    .unwrap();
    let image = committed.path.clone();
    let good = fs::read(&image).unwrap();
    let zzb = Key::instrument("ZZB").unwrap();
    let lookup = |reader: &mut Reader| {
        reader
            .lookup(&series, verdict_day(), &zzb)
            .map(|found| found.answer.map(|answer| answer.found))
    };
    let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
        panic!("ZZB is in the day");
    };
    assert_eq!(
        Reader::new()
            .details(
                &series,
                verdict_day(),
                committed.generation,
                Some((ordinal, &record))
            )
            .unwrap(),
        vec![detail]
    );

    // A detail entry whose size runs past the section.
    let mut bytes = good.clone();
    let size_at = bytes
        .windows(5)
        .position(|w| w == b"cloud")
        .expect("the payload")
        - 2;
    bytes[size_at..size_at + 2].copy_from_slice(&900u16.to_le_bytes());
    fs::write(&image, &bytes).unwrap();
    assert_eq!(
        corrupt_why(
            Reader::new()
                .details(
                    &series,
                    verdict_day(),
                    committed.generation,
                    Some((ordinal, &record))
                )
                .map(|_| ())
        ),
        "a detail entry runs past the section"
    );

    // A slot naming a record past the records: ZZB's own slot, its mark
    // raised.
    let mut bytes = good.clone();
    let tag = u32::try_from(zzb.hash() >> 32).unwrap();
    let slot = (TABLE_OFFSET..)
        .step_by(SLOT_LEN)
        .take(64)
        .find(|&at| bytes[at + 4..at + 8] == tag.to_le_bytes())
        .expect("ZZB's slot");
    bytes[slot..slot + 4].copy_from_slice(&99u32.to_le_bytes());
    fs::write(&image, &bytes).unwrap();
    assert_eq!(
        corrupt_why(lookup(&mut Reader::new()).map(|_| ())),
        "a slot names record 98, past the records"
    );

    // An image longer than its header implies.
    let mut bytes = good.clone();
    bytes.extend_from_slice(&[0; 8]);
    fs::write(&image, &bytes).unwrap();
    assert_eq!(
        corrupt_why(lookup(&mut Reader::new()).map(|_| ())),
        format!(
            "{} bytes where the header implies {}",
            good.len() + 8,
            good.len()
        )
    );
    fs::write(&image, &good).unwrap();

    // A year index cut short.
    let index = series.year_index(2031);
    let file = fs::OpenOptions::new().write(true).open(&index).unwrap();
    file.set_len(9).unwrap();
    drop(file);
    assert!(
        matches!(
            Reader::new().year(&series, 2031),
            Err(VerdictError::IndexShort { len: 9, .. })
        ),
        "a year index of 9 bytes"
    );
    let _ignored = fs::remove_dir_all(&root);
}

/// **A TIME INDEX DAMAGED UNDER A LIVE WRITER IS REBUILT BEFORE THE NEXT
/// COMMIT, AND THE COMMIT LANDS.**
///
/// Covers the writer's mid-life reindex (`BarFile::index_batch` meeting an
/// entry that fails its checksum, then `reindex`): the `.tix` entries around
/// the month's last committed bar are damaged on disk while the writer holds
/// the month, the next append rebuilds the index from the bars and commits,
/// and the index afterwards describes the bars exactly.
#[test]
fn a_time_index_damaged_under_a_live_writer_is_rebuilt_before_the_next_commit() {
    use store::file::{Appended, BarFile, Reindexed};
    use store::path::FileKind;
    const SYMBOL: u32 = 11;
    let root = scratch("tix-damaged-live");
    let bars = second_month(FileKind::Bars);
    let tix = second_month(FileKind::TimeIndex).to_path_buf(&root);
    let mut writer = BarFile::open_or_create(&root, bars, SYMBOL).unwrap();
    writer.append(&second_bars(0, 1_100)).unwrap();
    // The entries of buckets 515 to the last: every bucket the month's bars
    // touch (09:15 IST is slot 33,300, bucket 520; the last bar's is 537).
    let mut bytes = fs::read(&tix).unwrap();
    let from = usize::try_from(store::time_index::HEADER_LEN).unwrap() + 515 * 16;
    for byte in &mut bytes[from..] {
        *byte ^= 0x5A;
    }
    fs::write(&tix, &bytes).unwrap();
    assert!(matches!(
        writer.append(&second_bars(1_100, 1_150)),
        Ok(Appended::Committed { .. })
    ));
    drop(writer);
    let indexed = BarFile::index_existing(&root, bars, SYMBOL).unwrap();
    assert_eq!(indexed.n_valid, 1_150);
    assert_eq!(
        indexed.outcome,
        Reindexed::Kept,
        "the rebuilt index describes the bars"
    );
    let _ignored = fs::remove_dir_all(&root);
}

/// **A SMALL MONTH WHOSE COMMITTED BAR WILL NOT READ CROSSES THE THRESHOLD
/// UNINDEXED, AND THE COMMIT LANDS.**
///
/// Covers `BarFile::index_at_threshold` when the committed bars cannot be
/// indexed: a block of committed bars is damaged on disk, the commit that
/// brings the month to 1,024 bars commits its own bars, and no `.tix` is
/// written that would claim bars nothing read.
#[test]
fn a_small_month_whose_committed_bar_will_not_read_crosses_unindexed() {
    use store::file::{Appended, BarFile};
    use store::path::FileKind;
    const SYMBOL: u32 = 11;
    let root = scratch("tix-threshold-unreadable");
    let bars = second_month(FileKind::Bars);
    let tix = second_month(FileKind::TimeIndex).to_path_buf(&root);
    let mut writer = BarFile::open_or_create(&root, bars, SYMBOL).unwrap();
    writer.append(&second_bars(0, 1_000)).unwrap();
    assert!(!tix.exists(), "a small month has no .tix");
    // One byte of the first committed bar (the header is 32 KiB), in a block
    // the commit's own tail check never reads.
    let bar_path = bars.to_path_buf(&root);
    let mut bytes = fs::read(&bar_path).unwrap();
    let header = 32_768;
    bytes[header + 3] ^= 0xFF;
    fs::write(&bar_path, &bytes).unwrap();
    assert!(matches!(
        writer.append(&second_bars(1_000, 1_030)),
        Ok(Appended::Committed { .. })
    ));
    assert!(!tix.exists(), "no index claims bars that did not read");
    drop(writer);
    let _ignored = fs::remove_dir_all(&root);
}

/// **A NAME THAT APPEARS AT THE TIME INDEX BEFORE THE CROSSING COMMIT REFUSES
/// THAT COMMIT, AND NO BAR IS WRITTEN.**
///
/// Covers the create of `BarFile::index_at_threshold`: the `.tix` is born
/// with `create_new`, so a file put at its name after the writer opened is
/// never written through; the commit that would have crossed is refused and
/// the month still holds what it held.
#[test]
fn a_name_at_the_time_index_before_the_crossing_commit_refuses_it() {
    use store::file::BarFile;
    use store::path::FileKind;
    const SYMBOL: u32 = 11;
    let root = scratch("tix-threshold-taken");
    let bars = second_month(FileKind::Bars);
    let tix = second_month(FileKind::TimeIndex).to_path_buf(&root);
    let mut writer = BarFile::open_or_create(&root, bars, SYMBOL).unwrap();
    writer.append(&second_bars(0, 1_000)).unwrap();
    fs::write(&tix, b"put here by something else").unwrap();
    let refused = writer.append(&second_bars(1_000, 1_030));
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(writer.header().n_valid, 1_000, "no bar written");
    assert_eq!(
        fs::read(&tix).unwrap(),
        b"put here by something else",
        "never written through"
    );
    drop(writer);
    let _ignored = fs::remove_dir_all(&root);
}

/// **THE STORE'S REFUSAL SENTENCES NAME THE FILE AND THE REASON.**
///
/// Covers the text of `StoreError::TimeIndex`, `StoreError::SortIndex` and
/// `StoreError::Generation`, and of every `Conflict` an overlap can name.
#[test]
fn the_store_refusal_sentences_name_the_file_and_the_reason() {
    use store::file::{Conflict, StoreError};
    let path = PathBuf::from("/nonexistent/cloud/2031-02.tix");
    assert_eq!(
        StoreError::TimeIndex {
            path: path.clone(),
            reason: "a reason".to_owned()
        }
        .to_string(),
        "/nonexistent/cloud/2031-02.tix: time index refused: a reason"
    );
    assert_eq!(
        StoreError::SortIndex {
            path: path.clone(),
            reason: "a reason".to_owned()
        }
        .to_string(),
        "/nonexistent/cloud/2031-02.tix: sort index not written: a reason"
    );
    let generation = StoreError::Generation {
        path,
        reason: "revision 1 is not the one its promotion recorded; not sorted".to_owned(),
    }
    .to_string();
    assert!(
        generation.contains("/nonexistent/cloud/2031-02.tix")
            && generation.contains("revision 1 is not the one its promotion recorded"),
        "{generation}"
    );
    for (conflict, text) in [
        (Conflict::Restated, "restates the record held at"),
        (
            Conflict::NotHeld,
            "is inside the held range at a stamp the month never held:",
        ),
        (Conflict::Skipped, "skips the record held at"),
    ] {
        assert_eq!(conflict.to_string(), text);
    }
}

/// **A BATCH THAT RESTATES THE START OF THE HELD RANGE, SKIPS THE REST AND
/// RUNS PAST IT IS REFUSED AT THE FIRST HELD BAR IT SKIPPED.**
///
/// Covers `BarFile::diagnose_overlap`'s walk past the held range: the
/// overlap's first two bars match what the month holds, the batch then
/// jumps beyond the last held bar, and the refusal names the third held bar
/// as skipped, at the batch position where it should have been. Nothing is
/// written.
#[test]
fn a_batch_that_skips_held_bars_and_runs_past_them_is_refused_where_it_skipped() {
    use store::file::{BarFile, Conflict, StoreError};
    use store::path::FileKind;
    const SYMBOL: u32 = 11;
    let root = scratch("overlap-skipped");
    let bars = second_month(FileKind::Bars);
    let mut writer = BarFile::open_or_create(&root, bars, SYMBOL).unwrap();
    writer.append(&second_bars(0, 10)).unwrap();
    let batch = [second_bar(0, 0), second_bar(1, 0), second_bar(20, 0)];
    match writer.append(&batch) {
        Err(StoreError::OverlapDisagrees {
            at,
            ts_micros,
            conflict,
            ..
        }) => {
            assert_eq!(
                (at, ts_micros, conflict),
                (2, second_bar(2, 0).ts_micros, Conflict::Skipped)
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(writer.header().n_valid, 10, "nothing written");
    drop(writer);
    let _ignored = fs::remove_dir_all(&root);
}

/// **A DURABLE BATCH HOLDS EACH DIRECTORY ONCE, AND A SEALED ONE HOLDS
/// NOTHING MORE.**
///
/// Covers `DurableBatch::defer_directory`: a directory deferred twice is one
/// flush when the batch is made durable, and after that the batch is sealed,
/// so a later name's flush is the writer's own (`false`) rather than one a
/// finished batch would never issue.
#[test]
fn a_durable_batch_holds_a_directory_once_and_a_sealed_one_nothing() {
    use store::file::{BatchFlush, DurableBatch};
    let dir = scratch("batch-directory");
    let file_path = dir.join("held.bin");
    fs::write(&file_path, b"synthetic").unwrap();
    let batch = DurableBatch::new(BatchFlush::EveryFile);
    assert!(batch.defer_directory(&dir));
    assert!(batch.defer_directory(&dir), "the same directory again");
    let made = batch.make_durable(1).unwrap();
    assert_eq!((made.directories, made.files), (1, 0));
    assert!(!batch.defer_directory(&dir), "sealed");
    let file = fs::File::open(&file_path).unwrap();
    assert!(!batch.defer_file(&file, &file_path).unwrap(), "sealed");
    let _ignored = fs::remove_dir_all(&dir);
}

/// **A TIME INDEX WHOSE LAST SLOT IS OCCUPIED AT ANOTHER RANK IS STALE, AND
/// IS REBUILT.**
///
/// Covers the second half of `time_index::resume`'s check: the `.tix` of a
/// month with every second from 09:15 is laid beside a month with the same
/// last second and one second missing, so the last bar's slot is occupied
/// but at a rank one past this month's last row. The index door names it
/// stale and writes it from the bars.
#[test]
fn a_time_index_occupied_at_another_rank_is_stale_and_rebuilt() {
    use store::file::{BarFile, Reindexed};
    use store::path::FileKind;
    use store::time_index::Why;
    const SYMBOL: u32 = 11;
    let whole = scratch("tix-rank-whole");
    let gap = scratch("tix-rank-gap");
    let bars = second_month(FileKind::Bars);
    let mut writer = BarFile::open_or_create(&whole, bars, SYMBOL).unwrap();
    writer.append(&second_bars(0, 1_100)).unwrap();
    drop(writer);
    let mut writer = BarFile::open_or_create(&gap, bars, SYMBOL).unwrap();
    let missing_one: Vec<_> = (0..1_100)
        .filter(|&i| i != 5)
        .map(|i| second_bar(i, 0))
        .collect();
    writer.append(&missing_one).unwrap();
    drop(writer);
    let tix = second_month(FileKind::TimeIndex);
    fs::copy(tix.to_path_buf(&whole), tix.to_path_buf(&gap)).unwrap();
    let indexed = BarFile::index_existing(&gap, bars, SYMBOL).unwrap();
    assert_eq!(indexed.n_valid, 1_099);
    assert_eq!(indexed.outcome, Reindexed::Built(Why::Stale));
    assert_eq!(
        BarFile::index_existing(&gap, bars, SYMBOL).unwrap().outcome,
        Reindexed::Kept
    );
    let _ignored = fs::remove_dir_all(&whole);
    let _ignored = fs::remove_dir_all(&gap);
}

/// **THE NSE OPEN IS 09:15 IST: 555 MINUTES PAST MIDNIGHT, AND THE RUNGS THAT
/// DIVIDE IT ALIGN WITH IT.**
///
/// Kills `replace * with /` in `OPEN_MINUTES_PAST_IST_MIDNIGHT = 9 * 60 +
/// 15`, which makes the open 00:15: the rungs that divide 555 and 15 are the
/// same, so no alignment answer tells them apart, but every session offset
/// computed from the constant moves by nine hours.
#[test]
fn the_open_is_555_minutes_past_ist_midnight_and_its_divisors_align() {
    use store::path::Timeframe;
    assert_eq!(Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT, 555);
    for (rung, aligns) in [
        (Timeframe::MINUTE_1, true),
        (Timeframe::MINUTE_2, false),
        (Timeframe::MINUTE_3, true),
        (Timeframe::MINUTE_5, true),
        (Timeframe::MINUTE_10, false),
        (Timeframe::MINUTE_15, true),
        (Timeframe::MINUTE_30, false),
        (Timeframe::MINUTE_60, false),
    ] {
        assert_eq!(rung.aligns_with_the_open(), aligns, "{rung:?}");
    }
}

/// **A COMMIT IS NAMED NEAR THE CEILING AT HALF THE MEASURED PLACEMENT:
/// 68,950 RECORDS.**
///
/// Kills `replace / with *` in `RECORDS_WATCH = RECORDS_PLACED / 2`, which
/// would name nothing until a day held twice the most records ever placed,
/// so the day that halves the margin (RA-261, D-4925) would pass unsaid. The
/// tests that use the constant compare a commit against itself, so the
/// number is pinned here as the decision states it.
#[test]
fn a_commit_is_named_near_the_ceiling_at_half_the_measured_placement() {
    assert_eq!(store::verdict::RECORDS_PLACED, 137_900);
    assert_eq!(store::verdict::RECORDS_WATCH, 68_950);
}

/// **A STEM CUT INSIDE A CHARACTER STEPS BACK TO ITS START, IN BOUNDED
/// TIME.**
///
/// Kills `replace -= with /=` in `Key::stem`'s step back to a character
/// boundary (the cut never moves, so the step never ends; caught before only
/// by a timeout): the stem is made on a worker with a deadline, and the cut
/// lands before the three-byte character the field's last byte falls in.
#[test]
fn a_stem_cut_inside_a_character_steps_back_in_bounded_time() {
    use store::verdict::{KEY_LEN, Key};
    // 62 ASCII bytes, then "€" (3 bytes) across the 64-byte field's end.
    let whole = format!("{}€TAIL", "Z".repeat(KEY_LEN - 2));
    let (send, answer) = std::sync::mpsc::channel();
    let text = whole.clone();
    std::thread::spawn(move || {
        let _receiver_gone = send.send(Key::stem(&text).map(|key| key.text().to_owned()));
    });
    let kept = answer
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("the stem returned")
        .unwrap();
    assert_eq!(kept, "Z".repeat(KEY_LEN - 2));
}

/// **A DAY ALREADY FLAGGED `DETAILS_CAPPED` WHOSE DETAILS ARE CAPPED AGAIN
/// KEEPS THE FLAG.**
///
/// Kills `replace | with ^` in the day flags of a capped build: a toggle
/// would clear a fact the caller had already stated. The day's details are
/// one byte past `DETAIL_CAP`, so none are kept, and the committed day block
/// says so either way.
#[test]
fn a_capped_day_keeps_its_capped_flag() {
    use store::verdict::{Code, Detail, Reader, commit, flags};
    let root = scratch("verdict-capped-day");
    let series = verdict_series(&root);
    let mut generation = verdict_generation(&[("ZZA", Code::Match, Vec::new())]);
    generation.day_block.flags = flags::DETAILS_CAPPED;
    // 64 entries of 65,528 bytes fill the cap; one byte more is past it.
    generation.day_details = vec![
        Detail {
            kind: 3,
            payload: vec![0x5A; 65_528],
        };
        64
    ];
    generation.day_details[0].payload.push(0);
    let committed = commit(&series, &generation).unwrap();
    assert!(committed.details_capped);
    let lookup = Reader::new()
        .lookup(
            &series,
            verdict_day(),
            &store::verdict::Key::instrument("ZZA").unwrap(),
        )
        .unwrap();
    let day = lookup.answer.expect("committed").day;
    assert_ne!(day.flags & flags::DETAILS_CAPPED, 0, "still flagged");
    let _ignored = fs::remove_dir_all(&root);
}

/// **A FILE OF EXACTLY THE LONGEST DAY IMAGE IS READ; ONE BYTE MORE IS
/// REFUSED UNREAD.**
///
/// Kills `replace > with >=` in `read_image`'s size gate. The extreme case
/// a file at the maximum size: a sparse file of `MAX_IMAGE_LEN` bytes (about
/// 180 MB, no block of it written) is read whole and refused by its BLAKE3
/// trailer, as any image whose bytes changed is; one byte longer is refused
/// as too long before a byte is read or allocated for.
#[test]
fn a_file_of_exactly_the_longest_day_image_is_read_and_one_byte_more_is_not() {
    use store::verdict::{MAX_IMAGE_LEN, VerdictError, read_image};
    let dir = scratch("verdict-max-image");
    let path = dir.join("2031-03-04.g0000000001.vdt");
    let file = fs::File::create(&path).unwrap();
    let max = u64::try_from(MAX_IMAGE_LEN).unwrap();
    file.set_len(max).unwrap();
    assert!(
        matches!(read_image(&path), Err(VerdictError::TrailerMismatch { .. })),
        "read whole and refused by its trailer"
    );
    file.set_len(max + 1).unwrap();
    match read_image(&path) {
        Err(VerdictError::ImageCorrupt { why, .. }) => assert_eq!(
            why,
            format!(
                "the file is {} bytes, longer than any day image can be ({MAX_IMAGE_LEN}); not read",
                max + 1
            )
        ),
        other => panic!("{other:?}"),
    }
    drop(file);
    let _ignored = fs::remove_dir_all(&dir);
}

/// **TWO VALID YEAR-INDEX SLOTS AT THE SAME COMMIT ARE READ FROM SLOT 0.**
///
/// Kills `replace > with >=` in `read_year`, which would prefer slot 1 on a
/// tie. The committed slot is copied over the other, so both are valid at
/// one commit; the reader keeps slot 0, as the function's contract says, and
/// the answer is the same commit either way.
#[test]
fn two_year_index_slots_at_one_commit_are_read_from_slot_zero() {
    use store::verdict::{Code, INDEX_SLOT_LEN, INDEX_SLOT_STRIDE, Reader, commit};
    let root = scratch("verdict-slot-tie");
    let series = verdict_series(&root);
    let committed = commit(
        &series,
        &verdict_generation(&[("ZZA", Code::Match, Vec::new())]),
    )
    .unwrap();
    let before = Reader::new().year(&series, 2031).unwrap().unwrap();
    assert_eq!(before.commit, committed.commit);
    let path = series.year_index(2031);
    let mut bytes = fs::read(&path).unwrap();
    let from = usize::from(before.slot) * INDEX_SLOT_STRIDE;
    let to = (1 - usize::from(before.slot)) * INDEX_SLOT_STRIDE;
    let slot = bytes[from..from + INDEX_SLOT_LEN].to_vec();
    bytes[to..to + INDEX_SLOT_LEN].copy_from_slice(&slot);
    fs::write(&path, &bytes).unwrap();
    let tied = Reader::new().year(&series, 2031).unwrap().unwrap();
    assert_eq!((tied.slot, tied.commit), (0, committed.commit));
    let _ignored = fs::remove_dir_all(&root);
}

#[test]
fn a_store_root_that_cannot_be_listed_refuses_the_first_commit_of_a_series() {
    support::where_permission_binds(
        "a_store_root_that_cannot_be_listed_refuses_the_first_commit_of_a_series",
        a_store_root_that_cannot_be_listed_refuses_the_first_commit_of_a_series_body,
    );
}

/// **THE FIRST COMMIT OF A SERIES MAKES EVERY DIRECTORY IT MADE DURABLE, UP
/// TO AND INCLUDING THE STORE ROOT, OR IS REFUSED BEFORE ANYTHING IS
/// PUBLISHED.**
///
/// Kills `replace sync_dir -> Result<(), VerdictError> with Ok(())` and
/// `replace == with != in make_dirs`: the first answers every directory sync
/// "done" without one, and the second stops after the year directory's own
/// parent, so the entries the commit made in `verify/` and in the root are
/// never made durable. The root here may be written and searched but not
/// read (mode 0300), so its sync cannot open it: the commit is refused
/// `io` naming the root, with no lock, index or image made. Run where the
/// mode bits bind (D-0995).
fn a_store_root_that_cannot_be_listed_refuses_the_first_commit_of_a_series_body() {
    use std::os::unix::fs::PermissionsExt as _;
    use store::verdict::{Code, VerdictError, commit};
    let dir = scratch("verdict-root-unlisted");
    let root = dir.join("store");
    fs::create_dir(&root).unwrap();
    let series = verdict_series(&root);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o300)).unwrap();
    let answer = commit(
        &series,
        &verdict_generation(&[("ZZA", Code::Match, Vec::new())]),
    );
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let refusal = answer.unwrap_err();
    assert!(
        matches!(
            &refusal,
            VerdictError::Io { path, action: "sync directory", source }
                if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{refusal:?}"
    );
    assert!(
        series.dir().join("2031").is_dir(),
        "the year directory made"
    );
    assert!(!series.lock_path(2031).exists(), "no lock");
    assert!(!series.year_index(2031).exists(), "no index");
    let _ignored = fs::remove_dir_all(&dir);
}

#[test]
fn a_year_directory_that_cannot_be_listed_is_refused_never_absent() {
    support::where_permission_binds(
        "a_year_directory_that_cannot_be_listed_is_refused_never_absent",
        a_year_directory_that_cannot_be_listed_is_refused_never_absent_body,
    );
}

/// **A YEAR WITH NO INDEX WHOSE DIRECTORY CANNOT BE LISTED IS THE HOST'S
/// REFUSAL, NEVER "NO GENERATION".**
///
/// Kills `replace match guard why.kind() == ErrorKind::NotFound with true` in
/// `lost_index`: every failure to list the year directory would be taken for
/// "no directory", so a reader would answer a year whose images it cannot see
/// as a year never written, the silent absence the lost-index rule exists to
/// refuse. Run where the mode bits bind (D-0995).
fn a_year_directory_that_cannot_be_listed_is_refused_never_absent_body() {
    use std::os::unix::fs::PermissionsExt as _;
    use store::verdict::{Reader, VerdictError};
    let root = scratch("verdict-year-unlisted");
    let series = verdict_series(&root);
    let year = series.dir().join("2031");
    fs::create_dir_all(&year).unwrap();
    fs::set_permissions(&year, fs::Permissions::from_mode(0o300)).unwrap();
    let answer = Reader::new().year(&series, 2031);
    fs::set_permissions(&year, fs::Permissions::from_mode(0o755)).unwrap();
    let refusal = answer.unwrap_err();
    assert!(
        matches!(
            &refusal,
            VerdictError::Io { path, action: "list", source }
                if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{refusal:?}"
    );
    let _ignored = fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_index_or_image_is_refused_never_absent() {
    support::where_permission_binds(
        "an_unreadable_index_or_image_is_refused_never_absent",
        an_unreadable_index_or_image_is_refused_never_absent_body,
    );
}

/// **AN INDEX OR AN IMAGE THAT IS THERE BUT CANNOT BE OPENED IS THE HOST'S
/// REFUSAL, TO THE WRITER AND TO THE READER: NEVER "ABSENT".**
///
/// Kills three guards that take any `io` refusal for `ENOENT`:
/// `commit_within`'s open of the year index (the writer would call a present
/// index lost, `index_missing`), `Reader::handle` (the reader would say
/// `index_missing` or `image_missing` of files that are there), and `plan`'s
/// read of the image a carrying generation copies from (`image_missing`).
/// Each file is made unreadable in turn (mode 000) and the refusal must be
/// `io` naming it, with the host's `PermissionDenied`. Run where the mode
/// bits bind (D-0995).
fn an_unreadable_index_or_image_is_refused_never_absent_body() {
    use std::os::unix::fs::PermissionsExt as _;
    use store::verdict::{Carry, Code, Key, Reader, VerdictError, commit};
    let denied = |answer: VerdictError, at: &std::path::Path| {
        assert!(
            matches!(
                &answer,
                VerdictError::Io { path, source, .. }
                    if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
            ),
            "{answer:?}"
        );
    };
    let root = scratch("verdict-unreadable");
    let series = verdict_series(&root);
    let committed = commit(
        &series,
        &verdict_generation(&[("ZZA", Code::Match, Vec::new())]),
    )
    .unwrap();
    let key = Key::instrument("ZZA").unwrap();
    let mut carrying = verdict_generation(&[("ZZB", Code::Match, Vec::new())]);
    carrying.run = 2;
    carrying.carry = Carry::Others;

    let index = series.year_index(2031);
    fs::set_permissions(&index, fs::Permissions::from_mode(0o000)).unwrap();
    let written = commit(&series, &carrying);
    let read = Reader::new().lookup(&series, verdict_day(), &key);
    fs::set_permissions(&index, fs::Permissions::from_mode(0o644)).unwrap();
    denied(written.unwrap_err(), &index);
    denied(read.unwrap_err(), &index);

    let image = committed.path;
    fs::set_permissions(&image, fs::Permissions::from_mode(0o000)).unwrap();
    let written = commit(&series, &carrying);
    let read = Reader::new().lookup(&series, verdict_day(), &key);
    fs::set_permissions(&image, fs::Permissions::from_mode(0o644)).unwrap();
    denied(written.unwrap_err(), &image);
    denied(read.unwrap_err(), &image);

    // Readable again, the same carrying commit lands over generation 1.
    let landed = commit(&series, &carrying).unwrap();
    assert_eq!((landed.previous, landed.carried), (1, 1));
    let _ignored = fs::remove_dir_all(&root);
}

/// **A YEAR DIRECTORY WITH NO INDEX AND NO IMAGE COSTS ITS ONE LOOK.**
///
/// Kills `replace += with *= in lost_index`: the `opendir` that tells a year
/// never written from a lost index is counted as an open, as the function's
/// cost line says, so a lookup of a year whose directory exists empty costs
/// exactly one open and no read; with no directory at all, nothing.
#[test]
fn a_year_directory_with_no_index_and_no_image_costs_one_open() {
    use store::verdict::{Cost, Key, Reader};
    let root = scratch("verdict-empty-year");
    let series = verdict_series(&root);
    let key = Key::instrument("ZZA").unwrap();
    let never = Reader::new().lookup(&series, verdict_day(), &key).unwrap();
    assert_eq!((never.cost, never.answer), (Cost::default(), None));
    fs::create_dir_all(series.dir().join("2031")).unwrap();
    let empty = Reader::new().lookup(&series, verdict_day(), &key).unwrap();
    assert_eq!(
        (empty.cost, empty.answer),
        (Cost { opens: 1, reads: 0 }, None)
    );
    let _ignored = fs::remove_dir_all(&root);
}

/// **A COMMIT LOCK HELD FOR THE WHOLE WAIT IS REFUSED `lock_stalled` WHEN
/// THE WAIT ENDS, AFTER ONE TRY PER STEP, AND NOTHING IS PUBLISHED.**
///
/// Kills `replace match guard step < steps with true` and `replace += with
/// *=` in `lock_within` (caught before only by a timeout: both wait for
/// ever). The lock is held by this test through its own handle on the lock
/// file; the commit runs on a worker with a deadline, and must answer within
/// it, having tried the lock once and once more per `LOCK_STEP` of its 50 ms.
#[test]
fn a_held_commit_lock_is_refused_when_the_wait_ends() {
    use std::time::Duration;
    use store::verdict::{Code, LOCK_STEP, VerdictError, commit, commit_within};
    let root = scratch("verdict-lock-held");
    let series = verdict_series(&root);
    fs::create_dir_all(series.dir()).unwrap();
    let lock_path = series.lock_path(2031);
    let holder = fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .unwrap();
    holder.try_lock().unwrap();
    let wait = Duration::from_millis(50);
    let (send, answer) = std::sync::mpsc::channel();
    let worker_root = root.clone();
    std::thread::spawn(move || {
        let series = verdict_series(&worker_root);
        let generation = verdict_generation(&[("ZZA", Code::Match, Vec::new())]);
        let _receiver_gone = send.send(commit_within(&series, &generation, wait).map(|_| ()));
    });
    let refusal = answer
        .recv_timeout(Duration::from_secs(20))
        .expect("the commit answered")
        .unwrap_err();
    let steps = u32::try_from(wait.as_millis() / LOCK_STEP.as_millis()).unwrap();
    assert!(
        matches!(
            &refusal,
            VerdictError::LockStalled { path, waited_ms: 50, tries }
                if path == &lock_path && *tries == steps + 1
        ),
        "{refusal:?}"
    );
    assert!(!series.year_index(2031).exists(), "no index");
    holder.unlock().unwrap();
    commit(
        &series,
        &verdict_generation(&[("ZZA", Code::Match, Vec::new())]),
    )
    .unwrap();
    let _ignored = fs::remove_dir_all(&root);
}

/// **A PAGE NAMES A DAMAGED RECORD BY ITS OWN ORDINAL.**
///
/// Kills `replace + with *` in `Reader::page`'s ordinal of each record read:
/// the refusal must name the record that failed its CRC, not the page's
/// first. Two records of one code; the one at ordinal 1 has a byte of its key
/// changed on disk; a page of both, and a page starting at it, each name
/// record 1.
#[test]
fn a_page_names_a_damaged_record_by_its_own_ordinal() {
    use store::verdict::{Code, Found, Key, Reader, VerdictError, commit};
    let root = scratch("verdict-page-ordinal");
    let series = verdict_series(&root);
    let committed = commit(
        &series,
        &verdict_generation(&[
            ("ZZA", Code::Match, Vec::new()),
            ("ZZB", Code::Match, Vec::new()),
        ]),
    )
    .unwrap();
    let second = ["ZZA", "ZZB"]
        .into_iter()
        .find(|name| {
            let lookup = Reader::new()
                .lookup(&series, verdict_day(), &Key::instrument(name).unwrap())
                .unwrap();
            matches!(
                lookup.answer.map(|answer| answer.found),
                Some(Found::Record { ordinal: 1, .. })
            )
        })
        .expect("one key is record 1");
    let mut field = second.as_bytes().to_vec();
    field.resize(store::verdict::KEY_LEN, 0);
    let mut bytes = fs::read(&committed.path).unwrap();
    let at = bytes
        .windows(field.len())
        .position(|w| w == field.as_slice())
        .expect("record 1's key field");
    bytes[at + 2] ^= 0x01;
    fs::write(&committed.path, &bytes).unwrap();
    for (start, len) in [(0, 2), (1, 1)] {
        let refusal = Reader::new()
            .page(&series, verdict_day(), Code::Match, start, len)
            .unwrap_err();
        assert!(
            matches!(refusal, VerdictError::RecordCorrupt { ordinal: 1, .. }),
            "page ({start}, {len}): {refusal:?}"
        );
    }
    let _ignored = fs::remove_dir_all(&root);
}

/// **A RECORD'S DETAILS ARE READ ENTRY BY ENTRY FROM ITS OWN OFFSET.**
///
/// Kills `replace += with *= in Reader::details` (caught before only by a
/// timeout, as a walk from offset 0 never moves): from a record whose entries
/// follow another record's, the walk must step one entry at a time, so both
/// of its entries come back in order. Only that record is asked for, so this
/// test cannot hang under the mutant: its walk jumps past the section.
#[test]
fn a_records_details_after_another_records_are_read_entry_by_entry() {
    use store::verdict::{Code, Detail, Found, Key, Reader, commit};
    let detail = |kind: u16, payload: &[u8]| Detail {
        kind,
        payload: payload.to_vec(),
    };
    let own = |name: &str| {
        let low = name.to_ascii_lowercase();
        vec![detail(2, name.as_bytes()), detail(3, low.as_bytes())]
    };
    let root = scratch("verdict-details-walk");
    let series = verdict_series(&root);
    let committed = commit(
        &series,
        &verdict_generation(&[
            ("ZZA", Code::ValueMismatch, own("ZZA")),
            ("ZZB", Code::ValueMismatch, own("ZZB")),
        ]),
    )
    .unwrap();
    let mut walked = 0;
    for name in ["ZZA", "ZZB"] {
        let lookup = Reader::new()
            .lookup(&series, verdict_day(), &Key::instrument(name).unwrap())
            .unwrap();
        let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
        else {
            panic!("{name} is in the day");
        };
        assert!(record.detail_at > 0, "{name} owns details");
        if record.detail_at == 1 {
            continue;
        }
        walked += 1;
        assert_eq!(
            Reader::new()
                .details(
                    &series,
                    verdict_day(),
                    committed.generation,
                    Some((ordinal, &record))
                )
                .unwrap(),
            own(name),
            "{name}"
        );
    }
    assert_eq!(walked, 1, "exactly one record's entries follow another's");
    let _ignored = fs::remove_dir_all(&root);
}

#[test]
fn an_image_whose_temporary_cannot_be_created_is_refused_and_nothing_moves() {
    support::where_permission_binds(
        "an_image_whose_temporary_cannot_be_created_is_refused_and_nothing_moves",
        an_image_whose_temporary_cannot_be_created_is_refused_and_nothing_moves_body,
    );
}

/// **A NEXT GENERATION WHOSE TEMPORARY THE HOST WILL NOT CREATE IS REFUSED
/// AS THE CREATE, AND THE DAY STAYS AT ITS COMMITTED GENERATION.**
///
/// Covers `write_durable`'s own reading of `staging::create`'s host refusal
/// (`io`, never `staging`): the year directory may be read and searched but
/// not written (mode 0555), so the image temporary cannot be born. The
/// refusal names the temporary and the host's `PermissionDenied`, and a
/// reader still finds generation 1. Run where the mode bits bind (D-0995).
fn an_image_whose_temporary_cannot_be_created_is_refused_and_nothing_moves_body() {
    use std::os::unix::fs::PermissionsExt as _;
    use store::verdict::{Code, Reader, VerdictError, commit};
    let root = scratch("verdict-tmp-refused");
    let series = verdict_series(&root);
    let first = commit(
        &series,
        &verdict_generation(&[("ZZA", Code::Match, Vec::new())]),
    )
    .unwrap();
    let year = series.dir().join("2031");
    let mut second = verdict_generation(&[("ZZB", Code::Match, Vec::new())]);
    second.run = 2;
    fs::set_permissions(&year, fs::Permissions::from_mode(0o555)).unwrap();
    let answer = commit(&series, &second);
    fs::set_permissions(&year, fs::Permissions::from_mode(0o755)).unwrap();
    let refusal = answer.unwrap_err();
    assert!(
        matches!(
            &refusal,
            VerdictError::Io { path, action: "create", source }
                if path.parent() == Some(year.as_path())
                    && path.extension().is_some_and(|ext| ext == "tmp")
                    && source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{refusal:?}"
    );
    let index = Reader::new().year(&series, 2031).unwrap().unwrap();
    assert_eq!(
        index.entry(verdict_day()).generation,
        first.generation,
        "still generation 1"
    );
    let _ignored = fs::remove_dir_all(&root);
}

/// A sealed one-second NIFTY month of June 2024 (GDFL, NSE INDEX), symbol 4242.
fn sealed_path() -> store::path::StorePath<'static> {
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
    StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// The committed header of a month of no bar, as a sealed head copies it.
fn empty_month_header() -> store::header::Header {
    store::header::Header {
        format_version: 3,
        record_stride: 56,
        flags: store::format::FLAG_CHECKSUMS,
        generation: 7,
        n_valid: 0,
        first_ts_micros: 5,
        last_ts_micros: 9,
        symbol_id: 4_242,
        timeframe_secs: 1,
    }
}

/// **A PROVENANCE IMAGE THAT PACKS TO MORE THAN ITS LENGTH AND TWO, AND AT
/// MOST TWICE ITS LENGTH, IS SEALED AND READ BACK.**
///
/// Kills `replace * with +` in `Head::check_geometry`'s bound on the packed
/// provenance (`packed <= 2 * raw`, the most `pack_provenance` can write):
/// `raw + 2` would refuse a valid head. Every isolated zero byte packs to two
/// bytes, so eight bytes alternating one and zero pack to twelve: more than
/// ten, less than sixteen. An image shorter than a provenance header is
/// stored as given (only a whole header is checked for its owner).
#[test]
fn a_provenance_image_that_packs_past_its_length_is_sealed_and_read() {
    use store::sealed::{SealedMonth, build, pack_provenance};
    let path = sealed_path();
    let image = [1u8, 0, 1, 0, 1, 0, 1, 0];
    assert_eq!(pack_provenance(&image).len(), 12);
    let built = build(path, &empty_month_header(), &[], Some(&image), None).unwrap();
    let month = SealedMonth::open(built.to_bytes(), path, 4_242).unwrap();
    assert_eq!(month.provenance().unwrap(), Some(image.to_vec()));
}

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
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
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

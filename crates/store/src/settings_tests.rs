//! Unit tests of `store::settings` (R3-05). Every path is a fresh temporary
//! store; no value names a real folder of the operator's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    Change, Key, MAX_ALWAYS_SHOWN, MAX_FILE_BYTES, Reader, Refusal, Settings, check, dir, history,
    path_of, save,
};

/// A new, empty store root for one test.
fn root(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "brutex-settings-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// A change setting `set` against `expected`.
fn change(expected: u64, set: &[(Key, &str)]) -> Change {
    Change {
        expected,
        set: set.iter().map(|(k, v)| (*k, (*v).to_owned())).collect(),
        unset: Vec::new(),
        changed_by: "test".to_owned(),
        reason: "a test change".to_owned(),
        at_micros: 1_700_000_000_000_000,
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the shape `save` takes for the caller's own check"
)]
fn no_extra(_: &Settings) -> Result<(), Refusal> {
    Ok(())
}

/// Every settings file under `root`, temporary names included.
fn files(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir(root))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// **A SAVED SETTING IS READ BACK, AND THE ONE BEFORE IT IS STILL ON DISK,
/// BYTE FOR BYTE** — the generations are the audit.
#[test]
fn a_setting_round_trips_and_every_generation_is_kept() {
    let root = root("round");
    let mut reader = Reader::new(&root);
    reader.refresh().unwrap();
    assert_eq!(reader.highest(), 0);
    assert!(reader.newest().is_none());

    let first = save(
        &mut reader,
        &change(
            0,
            &[
                (Key::WindowFrom, "2018-01-01"),
                (Key::WindowTo, "2026-09-30"),
            ],
        ),
        no_extra,
    )
    .unwrap();
    assert_eq!(first.number, 1);
    assert_eq!(first.changed, vec![Key::WindowFrom, Key::WindowTo]);
    let first_bytes = std::fs::read(path_of(&root, 1)).unwrap();

    let second = save(
        &mut reader,
        &change(1, &[(Key::WindowTo, "2026-10-31")]),
        no_extra,
    )
    .unwrap();
    assert_eq!(second.number, 2);
    assert_eq!(second.changed, vec![Key::WindowTo]);
    assert_eq!(second.settings.get(Key::WindowFrom), Some("2018-01-01"));
    assert_eq!(second.settings.get(Key::WindowTo), Some("2026-10-31"));
    assert_eq!(std::fs::read(path_of(&root, 1)).unwrap(), first_bytes);

    // A NEW READER — a restarted server — reads the same newest generation.
    let mut fresh = Reader::new(&root);
    fresh.refresh().unwrap();
    assert_eq!(fresh.newest().unwrap().as_ref(), second.as_ref());

    let past = history(&root, 2, 10);
    assert_eq!(past.len(), 2);
    assert_eq!(past[0].as_ref().unwrap().number, 2);
    assert_eq!(
        past[1].as_ref().unwrap().settings.get(Key::WindowTo),
        Some("2026-09-30")
    );

    // UNSET RESTORES THE FALLBACK, and is itself a recorded change.
    let mut unset = change(2, &[]);
    unset.unset = vec![Key::WindowTo];
    let third = save(&mut reader, &unset, no_extra).unwrap();
    assert_eq!(third.settings.get(Key::WindowTo), None);
    assert_eq!(third.changed, vec![Key::WindowTo]);
}

/// **A BAD PATH IS REFUSED BEFORE IT IS SAVED**: nothing is written for any
/// of them, not even a temporary file.
#[test]
fn a_bad_path_is_refused_before_it_is_saved() {
    let root = root("badpath");
    let plain_file = root.join("a-file");
    std::fs::write(&plain_file, b"x").unwrap();
    let long = format!("/{}", "a".repeat(super::MAX_PATH_BYTES + 1));
    let climbing = format!("{}/../{}", root.display(), "x");
    let missing = root.join("no-such-dir").display().to_string();
    let file_text = plain_file.display().to_string();
    let control = format!("{}\nset window.to 2099-01-01", root.display());
    for bad in [
        "relative/dir",
        "",
        missing.as_str(),
        file_text.as_str(),
        climbing.as_str(),
        long.as_str(),
        control.as_str(),
    ] {
        for key in [Key::SourceGdfl, Key::SourceMasters] {
            let mut reader = Reader::new(&root);
            let got = save(&mut reader, &change(0, &[(key, bad)]), no_extra);
            match got {
                Err(Refusal::Invalid { key: Some(at), .. }) => assert_eq!(at, key, "{bad:?}"),
                other => panic!("{bad:?} for {key:?}: {other:?}"),
            }
        }
    }
    assert!(files(&root).is_empty(), "{:?}", files(&root));

    // A GOOD PATH IS SAVED.
    let good = root.display().to_string();
    let mut reader = Reader::new(&root);
    let saved = save(
        &mut reader,
        &change(0, &[(Key::SourceGdfl, &good)]),
        no_extra,
    )
    .unwrap();
    assert_eq!(saved.settings.get(Key::SourceGdfl), Some(good.as_str()));
}

/// **EVERY VALUE IS CHECKED**: garbage, edges and cross-key rules.
#[test]
fn every_value_is_checked_before_it_is_saved() {
    for (key, value, ok) in [
        (Key::WindowFrom, "2024-02-29", true),
        (Key::WindowFrom, "2026-02-29", false),
        (Key::WindowFrom, "2026-13-01", false),
        (Key::WindowFrom, "2026-1-01", false),
        (Key::WindowFrom, "+026-01-01", false),
        (Key::WindowFrom, "1989-12-31", false),
        (Key::WindowTo, "2100-12-31", true),
        (Key::WindowTo, "2026-09-31", false),
        (Key::OpenFeed, "gdfl", true),
        (Key::OpenFeed, "GDFL", false),
        (Key::OpenInstrument, "NIFTY", true),
        (Key::OpenInstrument, "nifty 50 ☃", false),
        (Key::OpenSegment, "FNO", true),
        (Key::OpenSegment, "OPT", false),
        (Key::OpenTimeframe, "finest", true),
        (Key::OpenTimeframe, "1s", true),
        (Key::OpenTimeframe, "7s", false),
        (Key::OpenOrder, "newest", true),
        (Key::OpenOrder, "latest", false),
        (Key::AlwaysShown, "NIFTY, BANKNIFTY", true),
        (Key::AlwaysShown, "NIFTY,NIFTY", false),
        (Key::AlwaysShown, "NIFTY,,BANKNIFTY", false),
    ] {
        assert_eq!(check(key, value).is_ok(), ok, "{key:?} = {value:?}");
    }
    assert_eq!(
        check(Key::AlwaysShown, " NIFTY , BANKNIFTY ").unwrap(),
        "NIFTY,BANKNIFTY"
    );
    let many: Vec<String> = (0..=MAX_ALWAYS_SHOWN).map(|n| format!("ZZ{n}")).collect();
    assert!(check(Key::AlwaysShown, &many.join(",")).is_err());
}

/// **A CHANGE IS CHECKED AS A WHOLE BEFORE ANY BYTE**: the window across
/// keys, one key both set and unset, the reason, the caller's own check, and
/// a change that changes nothing.
#[test]
fn a_change_is_checked_as_a_whole_before_any_byte() {
    let root = root("checks");
    let mut reader = Reader::new(&root);
    // A WINDOW THAT ENDS BEFORE IT BEGINS.
    let got = save(
        &mut reader,
        &change(
            0,
            &[
                (Key::WindowFrom, "2026-10-01"),
                (Key::WindowTo, "2026-09-30"),
            ],
        ),
        no_extra,
    );
    assert!(
        matches!(
            got,
            Err(Refusal::Invalid {
                key: Some(Key::WindowTo),
                ..
            })
        ),
        "{got:?}"
    );
    // SET AND UNSET OF ONE KEY.
    let mut both = change(0, &[(Key::OpenOrder, "newest")]);
    both.unset = vec![Key::OpenOrder];
    assert!(matches!(
        save(&mut reader, &both, no_extra),
        Err(Refusal::Invalid { .. })
    ));
    // AN EMPTY OR MULTI-LINE REASON.
    for reason in ["", "two\nlines"] {
        let mut bad = change(0, &[(Key::OpenOrder, "newest")]);
        bad.reason = reason.to_owned();
        assert!(matches!(
            save(&mut reader, &bad, no_extra),
            Err(Refusal::Invalid { key: None, .. })
        ));
    }
    // THE CALLER'S OWN CHECK RUNS BEFORE ANY BYTE.
    let refused = save(&mut reader, &change(0, &[(Key::OpenFeed, "dhan")]), |s| {
        if s.get(Key::OpenFeed) == Some("dhan") {
            Err(Refusal::Invalid {
                key: Some(Key::OpenFeed),
                why: "not offered".to_owned(),
            })
        } else {
            Ok(())
        }
    });
    assert!(matches!(
        refused,
        Err(Refusal::Invalid {
            key: Some(Key::OpenFeed),
            ..
        })
    ));
    assert!(files(&root).is_empty(), "{:?}", files(&root));

    // A CHANGE THAT CHANGES NOTHING.
    save(
        &mut reader,
        &change(0, &[(Key::OpenOrder, "newest")]),
        no_extra,
    )
    .unwrap();
    assert_eq!(
        save(
            &mut reader,
            &change(1, &[(Key::OpenOrder, "newest")]),
            no_extra
        ),
        Err(Refusal::Unchanged)
    );
}

/// **A CHANGE MADE AGAINST AN OLD GENERATION IS STALE**, and of eight
/// concurrent savers that all saw generation 0 exactly one lands.
#[test]
fn a_stale_or_racing_change_is_refused_and_one_racer_lands() {
    let root = root("race");
    let mut reader = Reader::new(&root);
    save(
        &mut reader,
        &change(0, &[(Key::OpenOrder, "newest")]),
        no_extra,
    )
    .unwrap();
    assert_eq!(
        save(
            &mut reader,
            &change(0, &[(Key::OpenOrder, "oldest")]),
            no_extra
        ),
        Err(Refusal::Stale {
            expected: 0,
            newest: 1
        })
    );

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|n| {
            let root = root.clone();
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut mine = Reader::new(&root);
                mine.refresh().unwrap();
                barrier.wait();
                let day = format!("2026-10-{:02}", n + 1);
                save(&mut mine, &change(1, &[(Key::WindowTo, &day)]), no_extra)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let landed = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(landed, 1, "{results:?}");
    assert!(
        results
            .iter()
            .filter_map(|r| r.as_ref().err())
            .all(|r| matches!(r, Refusal::Stale { expected: 1, .. })),
        "{results:?}"
    );
    // NO TEMPORARY FILE IS LEFT AND EXACTLY TWO GENERATIONS EXIST.
    assert_eq!(files(&root).len(), 2, "{:?}", files(&root));
}

/// **A DAMAGED, TRUNCATED, OVERSIZED OR MISNAMED GENERATION IS A NAMED
/// FAULT**, and the newest one that reads stays in force — loudly.
#[test]
fn a_damaged_generation_is_a_named_fault_and_never_read_as_settings() {
    let root = root("damaged");
    let mut writer = Reader::new(&root);
    save(
        &mut writer,
        &change(0, &[(Key::OpenOrder, "newest")]),
        no_extra,
    )
    .unwrap();
    save(
        &mut writer,
        &change(1, &[(Key::OpenOrder, "oldest")]),
        no_extra,
    )
    .unwrap();
    let second = path_of(&root, 2);
    let good = std::fs::read(&second).unwrap();

    let mut flipped = good.clone();
    let at = flipped.iter().position(|b| *b == b'o').unwrap();
    flipped[at] = b'O';
    let truncated = good[..good.len() / 2].to_vec();
    let oversized = vec![b'x'; usize::try_from(MAX_FILE_BYTES).unwrap() + 1];
    let other_version = String::from_utf8(good.clone())
        .unwrap()
        .replacen("brutex-settings 1", "brutex-settings 2", 1)
        .into_bytes();
    for (what, bytes) in [
        ("flipped", flipped),
        ("truncated", truncated),
        ("oversized", oversized),
        ("version", other_version),
        ("empty", Vec::new()),
    ] {
        // A FRESH ROOT PER CASE: the store is append-only, so a damaged file
        // is made by copying, never by rewriting one in place.
        let case = root.join(what);
        std::fs::create_dir_all(dir(&case)).unwrap();
        std::fs::copy(path_of(&root, 1), path_of(&case, 1)).unwrap();
        std::fs::write(path_of(&case, 2), &bytes).unwrap();
        let mut reader = Reader::new(&case);
        reader.refresh().unwrap();
        assert_eq!(reader.highest(), 2, "{what}");
        assert_eq!(reader.newest().unwrap().number, 1, "{what}");
        assert_eq!(reader.faults().len(), 1, "{what}");
        assert_eq!(reader.faults()[0].generation, 2, "{what}");
        // A SAVE AFTER A FAULT TAKES THE NEXT NUMBER, never the damaged one's.
        let saved = save(
            &mut reader,
            &change(2, &[(Key::OpenOrder, "oldest")]),
            no_extra,
        )
        .unwrap();
        assert_eq!(saved.number, 3, "{what}");
    }

    // A FILE WHOSE NAME AND CONTENT DISAGREE.
    let misnamed = root.join("misnamed");
    std::fs::create_dir_all(dir(&misnamed)).unwrap();
    std::fs::copy(path_of(&root, 2), path_of(&misnamed, 5)).unwrap();
    let mut reader = Reader::new(&misnamed);
    reader.refresh().unwrap();
    assert!(reader.newest().is_none());
    assert!(
        reader.faults()[0]
            .why
            .contains("generation 2 under the name of 5"),
        "{:?}",
        reader.faults()
    );
}

/// **A WRITER KILLED BEFORE ITS LINK LEAVES A TEMPORARY FILE NO READER
/// OPENS**, and the next save is unaffected.
#[test]
fn a_writer_killed_mid_write_leaves_nothing_a_reader_opens() {
    let root = root("killed");
    std::fs::create_dir_all(dir(&root)).unwrap();
    std::fs::write(
        dir(&root).join(".00000000000000000001.999-1.tmp"),
        b"brutex-settings 1\ngener",
    )
    .unwrap();
    let mut reader = Reader::new(&root);
    reader.refresh().unwrap();
    assert_eq!(reader.highest(), 0);
    assert!(reader.faults().is_empty());
    let saved = save(
        &mut reader,
        &change(0, &[(Key::OpenOrder, "newest")]),
        no_extra,
    )
    .unwrap();
    assert_eq!(saved.number, 1);
}

/// **A SETTING SAVED BY ANOTHER PROCESS IS SEEN WITHOUT A RESTART**: a
/// reader that has listed once sees the next generation on its next refresh.
#[test]
fn a_reader_sees_another_writers_generation_on_its_next_refresh() {
    let root = root("other");
    let mut server = Reader::new(&root);
    server.refresh().unwrap();
    assert!(server.newest().is_none());
    let mut other = Reader::new(&root);
    save(
        &mut other,
        &change(0, &[(Key::OpenInstrument, "BANKNIFTY")]),
        no_extra,
    )
    .unwrap();
    server.refresh().unwrap();
    assert_eq!(
        server.newest().unwrap().settings.get(Key::OpenInstrument),
        Some("BANKNIFTY")
    );
    save(
        &mut other,
        &change(1, &[(Key::OpenInstrument, "NIFTY")]),
        no_extra,
    )
    .unwrap();
    save(
        &mut other,
        &change(2, &[(Key::OpenInstrument, "RELIANCE")]),
        no_extra,
    )
    .unwrap();
    server.refresh().unwrap();
    assert_eq!(server.highest(), 3);
    assert_eq!(
        server.newest().unwrap().settings.get(Key::OpenInstrument),
        Some("RELIANCE")
    );
}

/// **A DISK THAT REFUSES THE WRITE IS AN `Io` REFUSAL WITH NOTHING VISIBLE.**
/// A read-only directory stands in for a full disk: both fail the first
/// write, which is the step that matters.
#[cfg(unix)]
#[test]
fn a_disk_that_refuses_the_write_is_refused_with_nothing_visible() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = root("readonly");
    std::fs::create_dir_all(dir(&root)).unwrap();
    std::fs::set_permissions(dir(&root), std::fs::Permissions::from_mode(0o555)).unwrap();
    let mut reader = Reader::new(&root);
    let got = save(
        &mut reader,
        &change(0, &[(Key::OpenOrder, "newest")]),
        no_extra,
    );
    std::fs::set_permissions(dir(&root), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(got, Err(Refusal::Io { .. })), "{got:?}");
    assert!(files(&root).is_empty());
}

/// Every key has a distinct name and slot, and its name reads back.
#[test]
fn every_key_has_one_name_and_one_slot() {
    let mut slots: Vec<usize> = Key::ALL.iter().map(|k| k.slot()).collect();
    slots.sort_unstable();
    slots.dedup();
    assert_eq!(slots.len(), Key::ALL.len());
    for key in Key::ALL {
        assert_eq!(Key::of_name(key.name()), Some(key));
    }
    assert_eq!(Key::of_name("window"), None);
    assert_eq!(Key::SourceMasters.env(), Some("BRUTEX_MASTERS"));
    assert_eq!(Key::WindowTo.env(), None);
}

/// **A GENERATION WHOSE CHECKSUM HOLDS BUT WHOSE VALUE ITS CHECK REFUSES IS
/// A FAULT, NEVER SETTINGS** — a hand-edited or foreign file is not read as
/// the operator's settings; a source folder that has MOVED since its save is
/// still read, because that is the route's to report, not damage.
///
/// Before: `decode` checked the grammar and the checksum and took every value
/// as written, so a `window.to` of 2026-02-30 with a valid checksum was the
/// window in force.
#[test]
fn a_value_its_check_refuses_is_a_fault_even_under_a_good_checksum() {
    let write = |case: &Path, number: u64, set: &[(Key, &str)]| {
        let mut settings = Settings::default();
        for (key, value) in set {
            settings.put(*key, Some((*value).to_owned()));
        }
        let keys: Vec<Key> = set.iter().map(|(key, _)| *key).collect();
        std::fs::create_dir_all(dir(case)).unwrap();
        let text = super::encode(number, &settings, &change(number - 1, &[]), &keys);
        std::fs::write(path_of(case, number), text).unwrap();
    };
    let root = root("refused-on-read");
    for (what, set) in [
        ("no such day", vec![(Key::WindowTo, "2026-02-30")]),
        (
            "a window ending before it begins",
            vec![
                (Key::WindowFrom, "2026-09-30"),
                (Key::WindowTo, "2018-01-01"),
            ],
        ),
        (
            "a relative folder",
            vec![(Key::SourceGdfl, "relative/gdfl")],
        ),
        ("a climbing folder", vec![(Key::SourceMasters, "/a/../b")]),
        (
            "not as a save writes it",
            vec![(Key::AlwaysShown, "NIFTY, BANKNIFTY")],
        ),
        ("an order no page knows", vec![(Key::OpenOrder, "sideways")]),
    ] {
        let case = root.join(what.replace(' ', "-"));
        write(&case, 1, &[(Key::OpenOrder, "newest")]);
        write(&case, 2, &set);
        let mut reader = Reader::new(&case);
        reader.refresh().unwrap();
        assert_eq!(reader.highest(), 2, "{what}");
        assert_eq!(
            reader.newest().map(|g| g.number),
            Some(1),
            "{what}: {:?}",
            reader.faults()
        );
        assert_eq!(reader.faults().len(), 1, "{what}");
        assert_eq!(reader.faults()[0].generation, 2, "{what}");
    }

    // A FOLDER THAT HAS MOVED SINCE ITS SAVE STILL READS: shape only.
    let moved = root.join("moved");
    write(
        &moved,
        1,
        &[(Key::SourceGdfl, "/nonexistent-brutex-test/gdfl")],
    );
    let mut reader = Reader::new(&moved);
    reader.refresh().unwrap();
    assert!(reader.faults().is_empty(), "{:?}", reader.faults());
    assert_eq!(
        reader.newest().unwrap().settings.get(Key::SourceGdfl),
        Some("/nonexistent-brutex-test/gdfl")
    );
}

/// **A DIRECTORY THAT STOPS BEING READABLE IS A FAULT, NEVER "NOTHING NEW"**
/// (R3-05 repair 2, the second agent's blocking 3).
///
/// Before: `refresh` stepped with `Path::exists`, which answers `false` for
/// a name it cannot stat, so a reader whose directory became unreadable
/// returned `Ok` with no word, and a generation saved meanwhile was simply
/// not served.
#[cfg(unix)]
#[test]
fn a_directory_that_stops_being_readable_is_a_fault_not_nothing_new() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = root("unreadable");
    let mut reader = Reader::new(&root);
    save(
        &mut reader,
        &change(0, &[(Key::WindowTo, "2026-10-31")]),
        no_extra,
    )
    .unwrap();
    let settings_dir = dir(&root);
    std::fs::set_permissions(&settings_dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    // A PROCESS RUNNING AS ROOT READS THROUGH 000: then there is nothing to
    // prove on this host, and the test says so rather than passing vacuously.
    let readable_anyway = std::fs::read_dir(&settings_dir).is_ok();
    let refreshed = reader.refresh();
    let fresh = Reader::new(&root).refresh();
    std::fs::set_permissions(&settings_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        !readable_anyway,
        "this host reads a 000 directory (root?): the property cannot be shown here"
    );
    let fault = refreshed.expect_err("an unreadable directory is a fault");
    assert_eq!(fault.generation, 2, "{fault:?}");
    assert!(
        reader
            .stopped()
            .is_some_and(|why| why.contains("generation 2")),
        "{:?}",
        reader.stopped()
    );
    // THE GENERATION READ BEFORE STAYS IN FORCE, beside the fault.
    assert_eq!(reader.newest().map(|g| g.number), Some(1));
    // A READER THAT HAS NEVER LISTED IT (a restart) is refused as well.
    assert!(fresh.is_err(), "{fresh:?}");
    // READABLE AGAIN, THE NEXT REFRESH IS CLEAN.
    reader.refresh().unwrap();
    assert!(reader.stopped().is_none());
}

// -- R3-05 repair 3 -----------------------------------------------------------

/// Plants something at a generation's name: `(case root, the name)`.
type Plant = fn(&Path, &Path);

/// A valid generation `number` written straight to `case` (a test seam: a
/// save writes only the next number).
fn good(case: &Path, number: u64) {
    let mut settings = Settings::default();
    let order = if number.is_multiple_of(2) {
        "oldest"
    } else {
        "newest"
    };
    settings.put(Key::OpenOrder, Some(order.to_owned()));
    std::fs::create_dir_all(dir(case)).unwrap();
    let text = super::encode(
        number,
        &settings,
        &change(number - 1, &[]),
        &[Key::OpenOrder],
    );
    std::fs::write(path_of(case, number), text).unwrap();
}

/// A real FIFO at `at`, made by `mkfifo` (test-only, as `tests/fifo.rs`).
#[cfg(unix)]
fn mkfifo(at: &Path) {
    let made = std::process::Command::new("mkfifo")
        .arg(at)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made {}", at.display());
}

/// Runs `read` on its own thread and fails the test if it has not answered
/// within ten seconds. Before repair 3 a FIFO at a generation's name made
/// `refresh` wait for a writer for ever; the pipe is opened for writing on
/// the failure path so the stuck thread is released.
fn answers_at_once<T: Send + 'static>(
    pipe: Option<PathBuf>,
    what: &str,
    read: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (send, answer) = std::sync::mpsc::channel();
    let _thread = std::thread::spawn(move || send.send(read()));
    let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
        if let Some(pipe) = pipe {
            let _release = std::fs::OpenOptions::new().write(true).open(pipe);
        }
        panic!("{what}: the settings read waited instead of naming a fault");
    };
    answered
}

/// **A NAME THAT IS NOT A REGULAR FILE IS A NAMED FAULT, NEVER A WAIT AND
/// NEVER AN UNBOUNDED READ** (R3-05 repair 3, the second agent's blocking 2;
/// D-4951).
///
/// Before: `read_one` `stat`ed the name with `metadata`, which follows a
/// link, so a FIFO and `/dev/zero` both showed length 0 and passed the
/// 64 KiB cap; `fs::read` then waited for a FIFO's writer for ever and read
/// `/dev/zero` without end. A dangling link was worse in the step: the
/// next name "did not exist", so every later save's hard link was refused at
/// it as stale for ever. The FIFO is the FIRST case, so on the old code the
/// test fails there, before it can reach `/dev/zero`.
#[cfg(unix)]
#[test]
fn a_generation_name_that_is_not_a_regular_file_is_a_named_fault_never_a_wait() {
    let root = root("not-regular");
    good(&root, 1);
    let cases: [(&str, &str, Plant); 5] = [
        ("fifo", "pipe", |_, at| mkfifo(at)),
        ("dev-zero", "symlink", |_, at| {
            std::os::unix::fs::symlink("/dev/zero", at).unwrap();
        }),
        ("dangling", "symlink", |_, at| {
            std::os::unix::fs::symlink("/nonexistent-brutex-test/generation", at).unwrap();
        }),
        ("link-to-good", "symlink", |case, at| {
            let other = case.join("elsewhere.settings");
            std::fs::copy(path_of(case, 1), &other).unwrap();
            std::os::unix::fs::symlink(&other, at).unwrap();
        }),
        ("directory", "directory", |_, at| {
            std::fs::create_dir(at).unwrap();
        }),
    ];
    for (what, word, plant) in cases {
        for stepped in [false, true] {
            let case = root.join(format!("{what}-{stepped}"));
            std::fs::create_dir_all(dir(&case)).unwrap();
            std::fs::copy(path_of(&root, 1), path_of(&case, 1)).unwrap();
            // STEPPED: a reader that listed generation 1 alone, then finds
            // the odd name at generation 2 by its one look; else a reader
            // that lists the directory with the odd name already there.
            let mut live = Reader::new(&case);
            if stepped {
                live.refresh().unwrap();
                assert_eq!(live.highest(), 1, "{what}");
            }
            let at = path_of(&case, 2);
            plant(&case, &at);
            let pipe = (what == "fifo").then(|| at.clone());
            let label = format!("{what} (stepped {stepped})");
            let mut reader = answers_at_once(pipe.clone(), &label, move || {
                let refreshed = live.refresh();
                (live, refreshed)
            });
            reader.1.unwrap();
            let reader = &mut reader.0;
            assert_eq!(reader.highest(), 2, "{label}");
            assert_eq!(reader.newest().map(|g| g.number), Some(1), "{label}");
            assert_eq!(reader.faults().len(), 1, "{label}: {:?}", reader.faults());
            let fault = &reader.faults()[0];
            assert_eq!(fault.generation, 2, "{label}");
            assert!(
                fault.why.contains(&format!("not a regular file: {word}")),
                "{label}: {fault:?}"
            );
            // `?history=` reads the same name through the same door.
            let history_root = case.clone();
            let listed = answers_at_once(pipe, &label, move || history(&history_root, 2, 2));
            assert!(
                matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
                "{label}: {listed:?}"
            );
            assert!(listed[1].is_ok(), "{label}: {listed:?}");
            // A SAVE STEPS PAST THE ODD NAME, never stuck as stale at it,
            // and leaves it as it is.
            let saved = save(reader, &change(2, &[(Key::OpenFeed, "gdfl")]), no_extra).unwrap();
            assert_eq!(saved.number, 3, "{label}");
            assert!(reader.last_cleared().is_none(), "{label}");
            assert!(
                std::fs::symlink_metadata(&at).is_ok(),
                "{label}: the odd name is left where it was"
            );
        }
    }
}

/// **A FAULT IS RE-READ, NEVER REMEMBERED: A LIVE READER AGREES WITH ONE
/// THAT HAS NEVER LISTED** (R3-05 repair 3, the second agent's follow-up 1).
///
/// Before: a live reader kept every fault it had ever recorded until
/// restart — one OLDER than the generation in force, and one the host had
/// stopped refusing — so `/feeds.json` refused the window as degraded in a
/// process that had seen it and served it in one that had not.
#[cfg(unix)]
#[test]
fn a_fault_is_reread_and_a_live_reader_agrees_with_a_fresh_one() {
    // A DAMAGED GENERATION BELOW THE ONE IN FORCE.
    let below = root("reread");
    good(&below, 1);
    let mut live = Reader::new(&below);
    live.refresh().unwrap();
    std::fs::write(path_of(&below, 2), b"brutex-settings 1\ntorn").unwrap();
    live.refresh().unwrap();
    assert_eq!(live.faults().len(), 1);
    let saved = save(
        &mut live,
        &change(2, &[(Key::OpenOrder, "oldest")]),
        no_extra,
    )
    .unwrap();
    assert_eq!(saved.number, 3);
    let mut fresh = Reader::new(&below);
    fresh.refresh().unwrap();
    assert_eq!(fresh.newest().map(|g| g.number), Some(3));
    assert!(fresh.faults().is_empty(), "{:?}", fresh.faults());
    assert_eq!(
        live.faults(),
        fresh.faults(),
        "a fault below the generation in force is not served as damage"
    );
    // The older damage is still named where every generation is read again.
    assert!(history(&below, 3, 3)[1].is_err());

    // A REFUSAL THE HOST STOPPED MAKING: the odd name is replaced by a good
    // generation (a test seam; a store never rewrites a generation).
    let transient = root("transient");
    good(&transient, 1);
    let mut live = Reader::new(&transient);
    live.refresh().unwrap();
    let at = path_of(&transient, 2);
    mkfifo(&at);
    let pipe = at.clone();
    let mut live = answers_at_once(Some(pipe), "transient fifo", move || {
        live.refresh().unwrap();
        live
    });
    assert_eq!(live.faults().len(), 1);
    std::fs::remove_file(&at).unwrap();
    good(&transient, 2);
    live.refresh().unwrap();
    let mut fresh = Reader::new(&transient);
    fresh.refresh().unwrap();
    assert_eq!(live.newest().map(|g| g.number), Some(2));
    assert_eq!(fresh.newest().map(|g| g.number), Some(2));
    assert!(live.faults().is_empty(), "{:?}", live.faults());
}

/// **A LISTED RECORD OF FAULTS IS NEWEST FIRST, AS A STEPPED ONE IS**
/// (R3-05 repair 3). Before, the listing walked newest first and inserted
/// each fault at the front, so the record came out oldest first and
/// `degraded` named the oldest of them.
#[test]
fn a_listed_record_of_faults_is_newest_first() {
    let root = root("fault-order");
    good(&root, 1);
    for number in 2..=4 {
        std::fs::write(path_of(&root, number), format!("damaged {number}")).unwrap();
    }
    let mut fresh = Reader::new(&root);
    fresh.refresh().unwrap();
    let listed: Vec<u64> = fresh.faults().iter().map(|f| f.generation).collect();
    assert_eq!(listed, vec![4, 3, 2]);
    let stepped_root = root.join("stepped");
    good(&stepped_root, 1);
    let mut live = Reader::new(&stepped_root);
    live.refresh().unwrap();
    for number in 2..=4 {
        std::fs::write(path_of(&stepped_root, number), format!("damaged {number}")).unwrap();
    }
    live.refresh().unwrap();
    let stepped: Vec<u64> = live.faults().iter().map(|f| f.generation).collect();
    assert_eq!(stepped, listed);
}

/// **A SETTINGS DIRECTORY THAT CANNOT BE LISTED IS THE STORE'S FAULT, NOT
/// THE CALLER'S**: a save is refused `Io` (the route's 500 `io`), never
/// `Invalid` (a 400 that blames the change). R3-05 repair 3, the second
/// agent's follow-up.
#[test]
fn a_settings_directory_that_is_a_file_refuses_a_save_as_io() {
    let root = root("dir-is-file");
    std::fs::create_dir_all(root.join(super::DIR)).unwrap();
    std::fs::write(dir(&root), b"not a directory").unwrap();
    let mut reader = Reader::new(&root);
    let refused = save(
        &mut reader,
        &change(0, &[(Key::OpenOrder, "newest")]),
        no_extra,
    );
    assert!(
        matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
        "{refused:?}"
    );
    assert_eq!(
        std::fs::read(dir(&root)).unwrap(),
        b"not a directory",
        "nothing written"
    );
}

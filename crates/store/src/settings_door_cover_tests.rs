#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the settings door's answers no other test reached: the
//! percent hint of a bare number under the disk floor, a settings directory
//! a save cannot list, a generation the host will not open, and a store root
//! a landed save cannot open for its directory flush. And (D-8561) the
//! races and the host refusal no input reaches: a file grown while read, a
//! name made at the save's temporary, a temporary the host will not write
//! (the host refusal seam), a directory replaced between two listings, a
//! link raced by another saver or a vanished temporary, and a read-back the
//! refresh missed (the race hook, making each change on disk). Every value
//! is invented.

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// A store root of this test's own, removed when it ends (its modes put
/// back first).
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let at = std::env::temp_dir().join(format!(
            "brutex-settings-door-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = std::fs::remove_dir_all(&at);
        std::fs::create_dir_all(&at).unwrap();
        Self(at)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let open = std::fs::Permissions::from_mode(0o755);
        let _ = std::fs::set_permissions(&self.0, open.clone());
        let _ = std::fs::set_permissions(dir(&self.0), open);
        let _cleanup = std::fs::remove_dir_all(&self.0);
    }
}

fn change(expected: u64, value: &str) -> Change {
    Change {
        expected,
        set: vec![(Key::WindowTo, value.to_owned())],
        unset: Vec::new(),
        changed_by: "ls1".to_owned(),
        reason: "an invented change for a cover test".to_owned(),
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

fn set_mode(at: &Path, mode: u32) {
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// **A BARE NUMBER UNDER THE FLOOR IS TOLD IT MAY HAVE MEANT PERCENT ONLY
/// WHERE IT COULD BE ONE.** `1` and `95` read as bytes are far under the
/// floor and get the hint; `96` is no percentage a floor takes, and gets
/// none; a number with a unit gets none either.
#[test]
fn a_bare_number_under_the_floor_is_told_of_percent_only_where_it_could_be_one() {
    for (text, hinted) in [("1", true), ("95", true), ("96", false), ("1000", false)] {
        let refused = disk_floor(text).expect_err(text);
        assert!(refused.contains("below the smallest floor"), "{refused}");
        assert_eq!(
            refused.contains(&format!("; for {text} percent write {text}%")),
            hinted,
            "{refused}"
        );
    }
    // WITH A UNIT NO AMOUNT IS UNDER THE FLOOR: the smallest unit, GB, is the
    // floor itself, so the percent clause is asked only of a bare number. MB is
    // no unit at all and is refused as one, never as a small floor.
    assert_eq!(disk_floor("1GB"), Ok(Floor::Bytes(MIN_FLOOR_BYTES)));
    let with_mb = disk_floor("95MB").expect_err("MB is no unit");
    assert!(
        with_mb.contains("is not a floor") && !with_mb.contains("percent"),
        "{with_mb}"
    );
}

/// **A SETTINGS DIRECTORY A SAVE CANNOT LIST REFUSES THE SAVE AS IO.** A save
/// lists before it lands, so a generation past a hole is never missed; a
/// listing the host refuses is the save's refusal, named, and nothing lands.
/// Root lists a mode-0300 directory, so the body runs where the mode bits
/// bind: as `nobody` when this process is root (D-0995).
#[test]
fn a_settings_directory_a_save_cannot_list_refuses_the_save_as_io() {
    crate::support::where_permission_binds(
        "settings::door_cover_tests::a_settings_directory_a_save_cannot_list_refuses_the_save_as_io",
        a_settings_directory_a_save_cannot_list_refuses_the_save_as_io_body,
    );
}

fn a_settings_directory_a_save_cannot_list_refuses_the_save_as_io_body() {
    let root = Root::new("unlisted");
    let mut reader = Reader::new(&root.0);
    save(&mut reader, &change(0, "2031-01-31"), no_extra).unwrap();
    set_mode(&dir(&root.0), 0o300);
    let refused = save(&mut reader, &change(1, "2031-02-28"), no_extra);
    set_mode(&dir(&root.0), 0o755);
    match refused {
        Err(Refusal::Io {
            kind: ErrorKind::Other,
            why,
            ..
        }) => assert!(why.contains("cannot be listed"), "{why}"),
        other => panic!("the listing refuses the save: {other:?}"),
    }
    assert!(!path_of(&root.0, 2).exists(), "nothing landed");
}

/// **A GENERATION THE HOST WILL NOT OPEN IS A FAULT NAMED BY THE HOST'S
/// WORDS.** A fresh reader lists generations 1 and 2; generation 2's open is
/// refused (it is no special file, so the refusal's own sentence is the
/// fault), and generation 1 stays in force beside the fault. Root opens a
/// mode-000 file, so the body runs where the mode bits bind: as `nobody`
/// when this process is root (D-0995).
#[test]
fn a_generation_the_host_will_not_open_is_a_fault_in_the_hosts_words() {
    crate::support::where_permission_binds(
        "settings::door_cover_tests::a_generation_the_host_will_not_open_is_a_fault_in_the_hosts_words",
        a_generation_the_host_will_not_open_is_a_fault_in_the_hosts_words_body,
    );
}

fn a_generation_the_host_will_not_open_is_a_fault_in_the_hosts_words_body() {
    let root = Root::new("closed-generation");
    let mut writer = Reader::new(&root.0);
    save(&mut writer, &change(0, "2031-01-31"), no_extra).unwrap();
    save(&mut writer, &change(1, "2031-02-28"), no_extra).unwrap();
    let second = path_of(&root.0, 2);
    set_mode(&second, 0o000);
    let mut fresh = Reader::new(&root.0);
    let refreshed = fresh.refresh();
    set_mode(&second, 0o644);
    assert!(refreshed.is_ok(), "{refreshed:?}");
    assert_eq!(fresh.newest().map(|g| g.number), Some(1));
    let fault = &fresh.faults()[0];
    assert_eq!(fault.generation, 2, "{fault:?}");
    assert!(
        fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
        "{fault:?}"
    );
}

/// **A STORE ROOT A LANDED SAVE CANNOT OPEN STILL LANDS THE SAVE.** The link
/// landed it; the directory flushes after it are tidiness, and a root the
/// host will not open for reading is passed over, never answered refused.
/// Root opens a mode-0300 directory, so the body runs where the mode bits
/// bind: as `nobody` when this process is root (D-0995).
#[test]
fn a_store_root_a_landed_save_cannot_open_still_lands_the_save() {
    crate::support::where_permission_binds(
        "settings::door_cover_tests::a_store_root_a_landed_save_cannot_open_still_lands_the_save",
        a_store_root_a_landed_save_cannot_open_still_lands_the_save_body,
    );
}

fn a_store_root_a_landed_save_cannot_open_still_lands_the_save_body() {
    let root = Root::new("closed-root");
    assert!(std::fs::File::open(&root.0).is_ok());
    set_mode(&root.0, 0o300);
    let opened = std::fs::File::open(&root.0);
    let mut reader = Reader::new(&root.0);
    let saved = save(&mut reader, &change(0, "2031-03-31"), no_extra);
    set_mode(&root.0, 0o755);
    assert!(opened.is_err(), "the premise: the root does not open");
    let saved = saved.expect("the link landed the save");
    assert_eq!(saved.generation.number, 1);
    assert_eq!(saved.read_back, None);
    let mut fresh = Reader::new(&root.0);
    fresh.refresh().unwrap();
    assert_eq!(
        fresh.newest().unwrap().settings.get(Key::WindowTo),
        Some("2031-03-31")
    );
}

/// The temporaries a save left in the settings directory.
fn temporaries(root: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir(root))
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "tmp"))
                .collect()
        })
        .unwrap_or_default()
}

/// **A SETTINGS FILE THAT GROWS PAST THE BOUND BETWEEN ITS `fstat` AND ITS
/// READ IS A FAULT, NEVER READ** (D-8561): a generation measured within
/// `MAX_FILE_BYTES` is grown past it by another actor before it is read (a
/// real change, the race hook's). The bounded read takes one byte more than
/// the bound, and the generation is a fault saying it grew while it was
/// read: no generation is in force.
#[test]
fn a_settings_file_grown_past_the_bound_while_read_is_a_fault() {
    let root = Root::new("grown");
    let mut saver = Reader::new(&root.0);
    save(&mut saver, &change(0, "2026-09-30"), no_extra).unwrap();
    crate::file::tests::arm_race("settings file measured", |path| {
        let mut bytes = std::fs::read(path).unwrap();
        bytes.resize(usize::try_from(MAX_FILE_BYTES).unwrap() + 4_096, b'#');
        std::fs::write(path, bytes).unwrap();
    });
    let mut reader = Reader::new(&root.0);
    reader.refresh().unwrap();
    assert!(!crate::file::tests::race_armed(), "the race ran");
    assert!(reader.newest().is_none(), "nothing grown is in force");
    let fault = &reader.faults()[0];
    assert_eq!(fault.generation, 1);
    assert!(
        fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),
        "{}",
        fault.why
    );
}

/// **A NAME MADE AT THE SAVE'S TEMPORARY BEFORE THE STAGING DOOR OPENS IT
/// REFUSES THE SAVE, AND THE NAME IS LEFT** (D-8561): another actor makes a
/// directory at the very name the save chose for its temporary (a real
/// change, the race hook's); the staging door refuses it as not a regular
/// file, and the save answers `Io` naming the temporary, in the kind for a
/// name already taken, with nothing at the generation's name and the other
/// actor's directory left as it was.
#[test]
fn a_name_made_at_the_temporary_refuses_the_save_and_is_left() {
    let root = Root::new("temp-taken");
    let mut reader = Reader::new(&root.0);
    crate::file::tests::arm_race("settings temporary named", |temp| {
        std::fs::create_dir(temp).unwrap();
    });
    match save(&mut reader, &change(0, "2026-09-30"), no_extra) {
        Err(Refusal::Io { path, kind, why }) => {
            assert_eq!(kind, std::io::ErrorKind::AlreadyExists);
            assert!(path.is_dir(), "the other actor's directory is left");
            assert!(why.starts_with("staging_not_regular"), "{why}");
        }
        other => panic!("the taken name refuses the save: {other:?}"),
    }
    assert!(!crate::file::tests::race_armed(), "the race ran");
    assert!(!path_of(&root.0, 1).exists(), "nothing is visible");
}

/// **A TEMPORARY WHOSE WRITE THE HOST REFUSES IS REMOVED, AND THE SAVE IS
/// REFUSED WITH NOTHING VISIBLE** (D-8561): the write and flush of the
/// staged temporary are refused (the host refusal seam, after the calls
/// ran): the save answers `Io` naming the temporary in the host's kind, the
/// temporary is gone and no generation is on disk.
#[test]
fn a_temporary_whose_write_is_refused_is_removed_with_nothing_visible() {
    let root = Root::new("temp-refused");
    let mut reader = Reader::new(&root.0);
    crate::file::tests::arm_refusal(
        "write the temporary",
        &dir(&root.0),
        0,
        std::io::ErrorKind::StorageFull,
    );
    match save(&mut reader, &change(0, "2026-09-30"), no_extra) {
        Err(Refusal::Io { path, kind, .. }) => {
            assert_eq!(kind, std::io::ErrorKind::StorageFull);
            assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("tmp"));
            assert!(!path.exists(), "the temporary is removed");
        }
        other => panic!("the refused write refuses the save: {other:?}"),
    }
    assert!(
        !crate::file::tests::refusal_armed(),
        "the write asked the seam"
    );
    assert!(temporaries(&root.0).is_empty());
    assert!(!path_of(&root.0, 1).exists(), "nothing is visible");
}

/// **A SETTINGS DIRECTORY THAT STOPS BEING A DIRECTORY BETWEEN THE SAVE'S
/// LISTING AND THE READER'S IS THE STORE'S FAULT** (D-8561): the save's
/// listing finds a generation past a hole (3 beside 1), so the reader lists
/// again before refusing; between the two another actor puts a FILE where
/// the directory was (a real change, the race hook's). The re-list is
/// refused, and the save answers `Io` "the settings directory cannot be
/// read", never `Stale` and never `Invalid`.
#[test]
fn a_directory_replaced_between_the_two_listings_is_the_stores_fault() {
    let root = Root::new("relist");
    let mut reader = Reader::new(&root.0);
    save(&mut reader, &change(0, "2026-09-30"), no_extra).unwrap();
    std::fs::copy(path_of(&root.0, 1), path_of(&root.0, 3)).unwrap();
    crate::file::tests::arm_race("settings listed", |root| {
        let at = dir(root);
        std::fs::rename(&at, at.with_extension("moved")).unwrap();
        std::fs::write(&at, b"not a directory").unwrap();
    });
    match save(&mut reader, &change(1, "2026-10-31"), no_extra) {
        Err(Refusal::Io { kind, why, .. }) => {
            assert_eq!(kind, std::io::ErrorKind::Other);
            assert!(
                why.starts_with("the settings directory cannot be read: "),
                "{why}"
            );
        }
        other => panic!("the refused re-list is the store's fault: {other:?}"),
    }
    assert!(!crate::file::tests::race_armed(), "the race ran");
    let at = dir(&root.0);
    std::fs::remove_file(&at).unwrap();
    std::fs::rename(at.with_extension("moved"), &at).unwrap();
}

/// **A SAVER THAT LANDS THE SAME NUMBER FIRST MAKES THE SECOND SAVE STALE,
/// AND A TEMPORARY TAKEN AWAY REFUSES THE LINK BY THE GENERATION'S NAME**
/// (D-8561). Between a save's listing and its hard link, another saver
/// lands the same number (a real save, the race hook's): the link is refused
/// as the name exists, and the save answers `Stale` against the generation
/// that landed, which is the other saver's. And when another actor removes
/// the save's temporary there instead, the link is refused otherwise: `Io`
/// naming the generation's path, in the host's kind, with nothing at it.
#[test]
fn a_link_raced_by_another_saver_or_a_vanished_temporary_is_refused() {
    let root = Root::new("link-race");
    let mut reader = Reader::new(&root.0);
    save(&mut reader, &change(0, "2026-09-30"), no_extra).unwrap();
    let other_root = root.0.clone();
    crate::file::tests::arm_race("settings temporary written", move |_temp| {
        let mut other = Reader::new(&other_root);
        let theirs = save(&mut other, &change(1, "2026-11-30"), no_extra).unwrap();
        assert_eq!(theirs.generation.number, 2);
    });
    assert_eq!(
        save(&mut reader, &change(1, "2026-10-31"), no_extra),
        Err(Refusal::Stale {
            expected: 1,
            newest: 2
        })
    );
    assert!(!crate::file::tests::race_armed(), "the race ran");
    let on_disk = history(&root.0, 2, 1);
    assert_eq!(
        on_disk[0].as_ref().unwrap().settings.get(Key::WindowTo),
        Some("2026-11-30"),
        "the other saver's generation is the one on disk"
    );
    assert!(
        temporaries(&root.0).is_empty(),
        "the loser's temporary is gone"
    );
    crate::file::tests::arm_race("settings temporary written", |temp| {
        std::fs::remove_file(temp).unwrap();
    });
    match save(&mut reader, &change(2, "2026-12-31"), no_extra) {
        Err(Refusal::Io { path, kind, .. }) => {
            assert_eq!(path, path_of(&root.0, 3));
            assert_eq!(kind, std::io::ErrorKind::NotFound);
        }
        other => panic!("the vanished temporary refuses the link: {other:?}"),
    }
    assert!(!crate::file::tests::race_armed(), "the race ran");
    assert!(!path_of(&root.0, 3).exists(), "nothing is visible");
}

/// **A LANDED SAVE THE REFRESH DID NOT STEP TO, THAT READS WHEN READ AGAIN,
/// SAYS SO BESIDE THE SAVE** (D-8561): after the link, another actor takes
/// the new generation's name away (the after-link step), so the refresh
/// finds nothing past the old one; before the read-back reads the generation
/// once more, the actor puts it back (a real change, the race hook's). The
/// save is answered `Saved` with the old generation in force and a read-back
/// that says the file reads when read again and the refresh did not step to
/// it.
#[test]
fn a_landed_save_the_refresh_missed_that_reads_again_says_so() {
    let root = Root::new("read-again");
    let mut reader = Reader::new(&root.0);
    save(&mut reader, &change(0, "2026-09-30"), no_extra).unwrap();
    super::AFTER_LINK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(|root: &Path, landed: u64| {
            let at = path_of(root, landed);
            std::fs::rename(&at, at.with_extension("aside")).unwrap();
        }));
    });
    crate::file::tests::arm_race("settings read back", |target| {
        std::fs::rename(target.with_extension("aside"), target).unwrap();
    });
    let saved = save(&mut reader, &change(1, "2026-10-31"), no_extra)
        .expect("the link succeeded: the save landed");
    assert!(!crate::file::tests::race_armed(), "the race ran");
    assert_eq!(saved.generation.number, 2);
    assert_eq!(saved.in_force, Some(1));
    let read_back = saved.read_back.expect("the missed read-back is said");
    assert!(
        read_back.contains("did not read back")
            && read_back.ends_with("it reads when read again, and the refresh did not step to it"),
        "{read_back}"
    );
    reader.refresh().unwrap();
    assert_eq!(
        reader.newest().unwrap().number,
        2,
        "the next refresh finds it"
    );
}

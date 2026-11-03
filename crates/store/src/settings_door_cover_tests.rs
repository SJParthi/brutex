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
//! a landed save cannot open for its directory flush. Every value is
//! invented.

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
    let with_unit = disk_floor("95MB").expect_err("95MB is under the floor");
    assert!(!with_unit.contains("percent"), "{with_unit}");
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

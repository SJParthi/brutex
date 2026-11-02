//! BASE-COV-PULL: what a GDFL run's locks and claims refuse, and each thing a
//! probe can find at a lock's path, which no test had reached (Route B, recipe
//! v4). Every claim, host and path is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::os::unix::fs::symlink;

use super::*;
use crate::gdfl_fixtures::scratch;

/// A whole claim line, valid in every field.
fn claim_line() -> String {
    Claim {
        run: 7,
        mode: "import".to_owned(),
        kind: "stocks".to_owned(),
        batch: "stocks:7,options:8".to_owned(),
        source: "tickstore".to_owned(),
        from: "2024-04-01".to_owned(),
        to: "-".to_owned(),
        filter: "*".to_owned(),
        verify: "off".to_owned(),
        restate: false,
        repair: "0".to_owned(),
        workers: 8,
        auto_resume: true,
        pid: 4242,
        host: "invented.local".to_owned(),
        exe: "1:2:-3:4".to_owned(),
        by: "cli".to_owned(),
        started: 1_700_000_000,
        resumes: 0,
        resume_day: "-".to_owned(),
        legacy_ack: false,
        unstamped: false,
        request: "-".to_owned(),
    }
    .line()
}

#[test]
fn a_layouts_words_and_its_lock_are_the_kinds_or_the_stores() {
    assert_eq!(layout_word(GdflLayout::V3), "v3");
    assert_eq!(layout_word(GdflLayout::V4), "v4");
    assert_eq!(claim_lock("stocks", GdflLayout::V4), kind_lock("stocks"));
    assert_eq!(claim_lock("census", GdflLayout::V4), STORE_LOCK);
    assert_eq!(claim_lock("stocks", GdflLayout::V3), STORE_LOCK);
    assert_eq!(stop_name("imports/x.lock"), "imports/x.stop");
    assert_eq!(stop_name("imports/x"), "imports/x.stop");
    assert_eq!(sidecar_name("imports/x.lock"), "imports/x.request");
    assert_eq!(sidecar_name("imports/x"), "imports/x.request");
}

#[test]
fn a_layout_changed_under_the_locks_names_both_layouts() {
    let fault = LockFault::LayoutChanged {
        locked: GdflLayout::V3,
        found: GdflLayout::V4,
    };
    assert_eq!(fault.code(), "census_layout_changed");
    let said = fault.to_string();
    assert!(
        said.contains("was v3 when") && said.contains("is v4 now"),
        "{said}"
    );
}

#[test]
fn a_probe_says_what_stands_at_a_lock_path() {
    let root = scratch("cov-what-is-at");
    let (file, dir, link) = (root.join("f"), root.join("d"), root.join("l"));
    std::fs::write(&file, b"x").unwrap();
    std::fs::create_dir(&dir).unwrap();
    symlink(&file, &link).unwrap();
    assert_eq!(what_is_at(&file), "a regular file");
    assert_eq!(what_is_at(&dir), "a directory");
    assert_eq!(what_is_at(&link), "a symbolic link (never followed)");
    assert!(what_is_at(&root.join("none")).starts_with("unmeasurable ("));
    assert_eq!(
        what_is_at(Path::new("/dev/null")),
        "a special file (a pipe, a socket or a device)"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A link at a lock's path is opened by no one and written through by no one;
/// a directory there is not one to lock; both are refused by what they are;
/// a device at the name is no regular file. A parent that is a file refuses
/// the folder. All of them refused, none created.
#[test]
fn a_lock_path_that_is_not_a_file_is_refused_by_what_it_is() {
    let root = scratch("cov-lock-path");
    let target = root.join("target");
    std::fs::write(&target, b"x").unwrap();
    let link = root.join("link.lock");
    symlink(&target, &link).unwrap();
    let refused = open_lock_path(&link, "link.lock").unwrap_err().to_string();
    assert!(
        refused.contains("a symbolic link (never followed)"),
        "{refused}"
    );
    let dir = root.join("dir.lock");
    std::fs::create_dir(&dir).unwrap();
    let refused = open_lock_path(&dir, "dir.lock").unwrap_err().to_string();
    assert!(refused.contains("a directory"), "{refused}");
    let special = open_lock_path(Path::new("/dev/null"), "null").unwrap_err();
    assert!(
        special.to_string().contains("not a regular file"),
        "{special}"
    );
    let lost = root.join("no-such-folder").join("a.lock");
    let refused = open_lock_path(&lost, "a.lock").unwrap_err().to_string();
    assert!(refused.contains("cannot be created"), "{refused}");
    let refused = open_lock_file(&target, "a.lock").unwrap_err().to_string();
    assert!(refused.contains("is not a directory"), "{refused}");
    let refused = open_lock_file(&root, "no/such/a.lock")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("its folder"), "{refused}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_file_is_its_path_only_while_the_path_stands() {
    let root = scratch("cov-same-file");
    let path = root.join("a");
    std::fs::write(&path, b"x").unwrap();
    let file = File::open(&path).unwrap();
    assert!(same_file(&file, &path));
    let other = root.join("b");
    std::fs::write(&other, b"x").unwrap();
    assert!(!same_file(&file, &other), "another inode");
    assert!(!same_file(&file, &root.join("none")), "no name at all");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_printable_line_escapes_all_that_would_break_one() {
    assert_eq!(
        printable(b"a\\b\tc\nd\re\xff\x7f z"),
        "a\\\\b\\tc\\nd\\re\\xff\\x7f z"
    );
    assert_eq!(printable(b""), "");
}

#[test]
fn a_host_is_encoded_cut_on_a_whole_escape_and_never_empty() {
    assert_eq!(host_field(""), "-");
    assert_eq!(host_field("-"), "-");
    assert_eq!(host_field("a.b-c9"), "a.b-c9");
    assert_eq!(host_field("a b"), "a%20b");
    let long = host_field(&" ".repeat(100));
    assert!(
        long.len() <= HOST_MAX && long.len().is_multiple_of(3),
        "{long}"
    );
}

#[test]
fn each_claim_field_takes_its_own_values_and_no_other() {
    for (name, good, bad) in [
        ("run", "7", ""),
        ("run", "7", "7a"),
        ("run", "7", "123456789012345678901"),
        ("mode", "import", "imported"),
        ("kind", "census", "bonds"),
        ("batch", "stocks:7", "stocks"),
        ("batch", "stocks:7", "bonds:7"),
        ("batch", "stocks:7", "stocks:x"),
        ("source", "dayzips", "ftp"),
        ("from", "-", "2024-4-1"),
        ("from", "2024-04-01", "2024-13-01"),
        ("filter", "*", "3"),
        ("filter", "3:0123456789abcdef", "x:0123456789abcdef"),
        ("filter", "3:0123456789abcdef", "3:0123456789ABCDEF"),
        ("filter", "3:0123456789abcdef", "3:0123"),
        ("verify", "inline", "on"),
        ("restate", "1", "2"),
        ("repair", "0", "1"),
        ("host", "a.b", "a b"),
        ("host", "a.b", ""),
        ("exe", "-", "1:2:3"),
        ("exe", "1:2:-3:4", "a:2:3:4"),
        ("by", "api", "web"),
        ("request", "-", "xyz"),
    ] {
        assert!(value_ok(name, good), "{name}={good}");
        assert!(!value_ok(name, bad), "{name}={bad}");
    }
    assert!(!value_ok("host", &"a".repeat(HOST_MAX + 1)));
    assert!(!value_ok("exe", &"1:".repeat(50)));
    assert!(!value_ok("batch", &"stocks:7,".repeat(20)));
    assert!(!value_ok("no-such-field", "x"));
}

#[test]
fn a_claim_line_of_any_other_shape_is_refused_by_name() {
    let line = claim_line();
    assert_eq!(Claim::parse(&line).unwrap().line(), line);
    let long = "x".repeat(CLAIM_MAX + 1);
    assert!(Claim::parse(&long).unwrap_err().contains("past"));
    let bad = line.replace("mode=import", "mode=imported");
    assert!(
        Claim::parse(&bad)
            .unwrap_err()
            .contains("is not a value of mode")
    );
    let extra = line.replace('\n', " extra=1\n");
    assert!(
        Claim::parse(&extra)
            .unwrap_err()
            .contains("an unknown field")
    );
}

#[test]
fn a_sidecar_past_its_bound_is_not_written_and_a_bad_one_is_stale() {
    let root = scratch("cov-sidecar");
    let path = root.join("a.request");
    let long = "x".repeat(SIDECAR_MAX + 1);
    assert!(write_sidecar(&path, &long).unwrap_err().contains("is past"));
    assert!(!path.exists());
    let digest = write_sidecar(&path, "a request line").unwrap();
    assert_eq!(
        read_sidecar(&path, &digest),
        SidecarState::Verified("a request line".to_owned())
    );
    assert_eq!(read_sidecar(&path, "-"), SidecarState::None);
    let stale = |bytes: &[u8]| {
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, bytes).unwrap();
        read_sidecar(&path, &digest)
    };
    assert_eq!(
        stale(&[0xFF, b'\n']),
        SidecarState::Stale("not text".to_owned())
    );
    assert_eq!(
        stale(b"no crc here\n"),
        SidecarState::Stale("no crc".to_owned())
    );
    assert_eq!(
        stale(&vec![b'x'; SIDECAR_MAX + 1]),
        SidecarState::Stale(format!("past {SIDECAR_MAX} bytes"))
    );
    let bare = root.join("no-such-sidecar");
    assert!(matches!(
        read_sidecar(&bare, &digest),
        SidecarState::Stale(_)
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_lock_state_names_its_lock_and_its_claim_when_it_holds_one() {
    let claim = Claim::parse(&claim_line()).unwrap();
    let held = LockState::Held {
        lock: "a.lock".to_owned(),
        claim: ClaimState::Claim(Box::new(claim.clone())),
    };
    assert_eq!(held.lock(), "a.lock");
    assert_eq!(held.claim(), Some(&claim));
    let dead = LockState::Interrupted {
        lock: "b.lock".to_owned(),
        claim: ClaimState::Claim(Box::new(claim.clone())),
        stop_requested: false,
    };
    assert_eq!(dead.claim(), Some(&claim));
    let free = LockState::Free {
        lock: "c.lock".to_owned(),
    };
    assert_eq!(free.lock(), "c.lock");
    assert_eq!(free.claim(), None);
    let unprobed = LockState::Unprobed {
        lock: "d.lock".to_owned(),
        why: "invented".to_owned(),
    };
    assert_eq!(unprobed.lock(), "d.lock");
    assert_eq!(unprobed.claim(), None);
}

/// A directory, a link and nothing at a lock's name each probe as unprobed,
/// by what they are; a kind lock's file on the disk joins the names.
#[test]
fn a_probe_of_what_is_no_lock_file_is_unprobed_by_name() {
    let root = scratch("cov-probe");
    std::fs::create_dir_all(root.join("imports")).unwrap();
    let name = kind_lock("stocks");
    assert_eq!(lock_names(&root), [STORE_LOCK]);
    std::fs::create_dir(root.join(&name)).unwrap();
    assert_eq!(lock_names(&root), [STORE_LOCK.to_owned(), name.clone()]);
    let state = probe_one(&root, &name, 0);
    assert!(
        matches!(&state, LockState::Unprobed { why, .. } if why == "is a directory"),
        "{state:?}"
    );
    let gone = probe_one(&root, "imports/none.lock", 0);
    assert!(matches!(gone, LockState::Unprobed { .. }), "{gone:?}");
    std::fs::remove_dir_all(root).unwrap();
}

#![cfg(test)]
//! BASE-COV-PULL-PS3 (G3): the tick store's day table as one walk finds it
//! (D-4866), at its edges: a folder whose name starts with a dot, more strays
//! than are named, a misfiled day asked as absent, a folder gone part way
//! through the walk, a walk past its cap and a listing that fails part way.
//! Every day file here is an empty file under an invented layout (`own`
//! below); the walk reads names only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};

use super::*;
use crate::gdfl_fixtures::scratch;

/// What a name is to every walk here: `D<dd>.bts` is day `dd` of April 2024
/// in tree 0, filed at `t/<dd>/D<dd>.bts`. An invented layout: the walk is
/// generic over what a name means.
fn own(name: &str) -> Option<(u8, Day, String)> {
    let dd = name.strip_prefix('D')?.strip_suffix(".bts")?;
    let day = Day::new(2024, 4, dd.parse().ok()?).ok()?;
    Some((0, day, format!("t/{dd}/{name}")))
}

/// Day `dd` of April 2024.
fn april(dd: u8) -> Day {
    Day::new(2024, 4, dd).unwrap()
}

/// An empty file at `root/rel`, its folders made.
fn touch(root: &Path, rel: &str) -> PathBuf {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"").unwrap();
    path
}

/// Every entry of a listing as it was read: what production passes.
fn as_read(entry: std::io::Result<std::fs::DirEntry>) -> std::io::Result<std::fs::DirEntry> {
    entry
}

/// A FOLDER WHOSE NAME STARTS WITH A DOT IS ONE STRAY AND IS NEVER DESCENDED:
/// a day file inside it is neither held nor misfiled, so its day is absent,
/// and a dot file beside it is a stray of its own.
#[test]
fn a_dot_folder_is_one_stray_and_its_day_files_are_never_reached() {
    let root = scratch("cov-walk-dot");
    touch(&root, "t/01/D01.bts");
    touch(&root, "t/.hidden/D02.bts");
    touch(&root, "t/.D03.bts");
    let table = DayTable::walk(&root, "t", &own);
    assert_eq!((table.held, table.misfiled_count), (1, 0));
    let mut named = table.stray_named.clone();
    named.sort();
    assert_eq!(named, ["t/.D03.bts", "t/.hidden"]);
    assert_eq!(table.stray, 2);
    assert_eq!(table.misfiled(0, april(2)), None);
    assert_eq!(table.absent(0, april(2)), Ok(()));
    assert_eq!(table.fault, None);
    std::fs::remove_dir_all(root).unwrap();
}

/// PAST [`TICKSTORE_NAMED`] STRAYS, EVERY ONE IS COUNTED AND THE FIRST SIXTEEN
/// ARE NAMED: at sixteen all are named; at seventeen the count goes on and
/// the names stop.
#[test]
fn past_sixteen_strays_each_is_counted_and_sixteen_are_named() {
    for strays in [TICKSTORE_NAMED, TICKSTORE_NAMED + 1, 40] {
        let root = scratch("cov-walk-strays");
        for n in 0..strays {
            touch(&root, &format!("t/stray-{n:02}"));
        }
        let table = DayTable::walk(&root, "t", &own);
        assert_eq!(table.stray, strays);
        assert_eq!(table.stray_named.len(), TICKSTORE_NAMED, "{strays}");
        let mut named = table.stray_named.clone();
        named.sort();
        named.dedup();
        assert_eq!(named.len(), TICKSTORE_NAMED, "each named once");
        assert!(named.iter().all(|name| name.starts_with("t/stray-")));
        assert_eq!((table.held, table.fault.clone()), (0, None));
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// A MISFILED DAY ASKED AS ABSENT IS REFUSED BY BOTH ITS NAMES (D-4866): where
/// it was found and where its name files it. Its tree's other days, and the
/// same day of the other tree, are absent.
#[test]
fn a_misfiled_day_asked_as_absent_is_refused_by_both_its_names() {
    let root = scratch("cov-walk-misfiled");
    touch(&root, "t/09/D08.bts");
    touch(&root, "t/D08.bts");
    let table = DayTable::walk(&root, "t", &own);
    assert_eq!(
        (table.held, table.misfiled_count),
        (0, 2),
        "each copy counted"
    );
    let (found, expected) = table.misfiled(0, april(8)).unwrap().clone();
    assert_eq!(expected, "t/08/D08.bts");
    assert_eq!(
        found, "t/D08.bts",
        "the top is listed before any folder in it"
    );
    assert_eq!(
        table.absent(0, april(8)),
        Err(CmRefusal::DayFileMisfiled { found, expected })
    );
    assert_eq!(table.absent(1, april(8)), Ok(()), "the other tree");
    assert_eq!(table.absent(0, april(9)), Ok(()), "another day");
    std::fs::remove_dir_all(root).unwrap();
}

/// A FOLDER GONE PART WAY THROUGH THE WALK IS A FAULT, NEVER AN EMPTY TREE.
/// Listed as a folder, then removed before its turn (an ingest moving the
/// tree, staged here by the name reader of the folder walked first, whose
/// one file names the other folder): only the tree's own top is read as
/// empty when it is missing, so every day the walk did not reach is refused
/// with the host's `NotFound`, never read as absent.
#[test]
fn a_folder_gone_part_way_through_the_walk_is_a_fault_never_an_empty_tree() {
    let root = scratch("cov-walk-gone");
    touch(&root, "t/a/Xb");
    touch(&root, "t/b/Xa");
    let tree = root.join("t");
    let removing = move |name: &str| {
        let _gone = std::fs::remove_dir_all(tree.join(name.trim_start_matches('X')));
        own(name)
    };
    let table = DayTable::walk(&root, "t", &removing);
    let fault = CmRefusal::TickStoreUnavailable {
        kind: std::io::ErrorKind::NotFound,
    };
    assert_eq!(table.fault, Some(fault.clone()));
    assert_eq!(table.stray, 1, "the one file of the folder walked first");
    assert_eq!(table.absent(0, april(1)), Err(fault));
    std::fs::remove_dir_all(root).unwrap();
}

/// COMBINED: a misfiled copy AND a folder gone part way. The misfiled day is
/// refused by its names, every other day with the walk's fault.
#[test]
fn a_misfiled_copy_and_a_folder_gone_each_refuse_their_own_days() {
    let root = scratch("cov-walk-misfiled-gone");
    touch(&root, "t/D08.bts");
    touch(&root, "t/a/Xb");
    touch(&root, "t/b/Xa");
    let tree = root.join("t");
    let removing = move |name: &str| {
        let _gone = std::fs::remove_dir_all(tree.join(name.trim_start_matches('X')));
        own(name)
    };
    let table = DayTable::walk(&root, "t", &removing);
    assert_eq!(
        table.absent(0, april(8)),
        Err(CmRefusal::DayFileMisfiled {
            found: "t/D08.bts".to_owned(),
            expected: "t/08/D08.bts".to_owned()
        })
    );
    assert_eq!(
        table.absent(0, april(9)),
        Err(CmRefusal::TickStoreUnavailable {
            kind: std::io::ErrorKind::NotFound
        })
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A WALK PAST ITS CAP STOPS AND SAYS SO, AND ONE THAT MEETS IT EXACTLY DOES
/// NOT: six entries (three folders, three day files) walk whole under a cap
/// of six; under five the walk is refused [`WALK_OVER_CAP`], and every day it
/// did not reach is refused with it. Production's cap is a million.
#[test]
fn a_walk_past_its_cap_is_refused_and_one_at_it_is_whole() {
    let root = scratch("cov-walk-cap");
    for dd in 1..=3 {
        touch(&root, &format!("t/{dd:02}/D{dd:02}.bts"));
    }
    let whole = DayTable::walk_within(&root, "t", &own, 6, &as_read);
    assert_eq!((whole.held, whole.fault.clone()), (3, None));
    assert_eq!(
        whole,
        DayTable::walk(&root, "t", &own),
        "the production walk"
    );
    let past = DayTable::walk_within(&root, "t", &own, 5, &as_read);
    let fault = CmRefusal::TickStoreMalformed {
        what: WALK_OVER_CAP,
    };
    assert_eq!(past.fault, Some(fault.clone()));
    assert!(past.held < 3, "{}", past.held);
    assert_eq!(past.absent(0, april(4)), Err(fault));
    assert_eq!(TICKSTORE_WALK_CAP, 1_000_000);
    std::fs::remove_dir_all(root).unwrap();
}

/// COMBINED: a cap reached after a misfiled copy was found. The two entries
/// of the top (the copy and a folder) fit a cap of two; the folder's file is
/// the third. The copy's day is refused by its names, every other day with
/// the cap.
#[test]
fn a_cap_reached_after_a_misfiled_copy_refuses_each_day_by_its_own_cause() {
    let root = scratch("cov-walk-cap-misfiled");
    touch(&root, "t/D08.bts");
    touch(&root, "t/09/D09.bts");
    let table = DayTable::walk_within(&root, "t", &own, 2, &as_read);
    assert_eq!(table.misfiled_count, 1);
    assert!(matches!(
        table.absent(0, april(8)),
        Err(CmRefusal::DayFileMisfiled { .. })
    ));
    assert_eq!(
        table.absent(0, april(9)),
        Err(CmRefusal::TickStoreMalformed {
            what: WALK_OVER_CAP
        })
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A LISTING THAT FAILS PART WAY IS A FAULT, NEVER A PARTIAL TREE: an entry the
/// host could not read (here refused as a permission fault the moment it is
/// read) stops the walk, and every day it did not reach is refused with the
/// host's own kind.
#[test]
fn a_listing_that_fails_part_way_is_a_fault_never_a_partial_tree() {
    let root = scratch("cov-walk-listing");
    touch(&root, "t/01/D01.bts");
    touch(&root, "t/02/D02.bts");
    let refusing = |entry: std::io::Result<std::fs::DirEntry>| {
        entry.and_then(|entry| {
            if entry.file_name() == "D02.bts" {
                Err(std::io::ErrorKind::PermissionDenied.into())
            } else {
                Ok(entry)
            }
        })
    };
    let table = DayTable::walk_within(&root, "t", &own, TICKSTORE_WALK_CAP, &refusing);
    let fault = CmRefusal::TickStoreUnavailable {
        kind: std::io::ErrorKind::PermissionDenied,
    };
    assert_eq!(table.fault, Some(fault.clone()));
    assert!(table.held <= 1, "{}", table.held);
    assert_eq!(table.absent(0, april(2)), Err(fault));
    std::fs::remove_dir_all(root).unwrap();
}

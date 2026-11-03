#![cfg(test)]
//! BASE-COV-PULL-PS2 (fork B): the loose day-zip census names at most its
//! bound of strays, invalid names and misfiled copies, and counts every one;
//! a hidden folder under the root (a name starting with a dot) is never
//! walked, so a day zip inside it files no day. Every file here is empty or
//! invented; no vendor row.

#![expect(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// A fresh, empty directory of this test's own (never reused).
fn fresh(tag: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "gdfl_dayzips_ps2_{}_{}_{tag}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An empty file at `rel` under `root`, its folders made.
fn touch(root: &Path, rel: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"").unwrap();
}

/// Past each bound at once: more strays than [`STRAY_NAMED`], more invalid
/// names and more misfiled copies than [`DAYZIPS_NAMED`]; every one is
/// counted and only the first of each are named.
#[test]
fn the_census_counts_every_name_and_names_only_its_bound() {
    let root = fresh("bounds");
    let strays = STRAY_NAMED + 3;
    for n in 0..strays {
        touch(&root, &format!("stray-{n:02}.txt"));
    }
    let prefix = LooseKind::Indices.prefix();
    let invalid = DAYZIPS_NAMED + 2;
    for n in 0..invalid {
        // A kind's prefix and `.zip` around a date that is no day.
        touch(&root, &format!("{prefix}99{n:02}0000.zip"));
    }
    let misfiled = DAYZIPS_NAMED + 1;
    let day = Day::new(2026, 9, 25).unwrap();
    for n in 0..misfiled {
        // The same day, each copy in a folder of its own: misfiled.
        let name = LooseKind::Indices.path_of(day);
        let file = name.rsplit_once('/').unwrap().1;
        touch(&root, &format!("elsewhere-{n:02}/{file}"));
    }
    let loose = DayZips::open(&root).unwrap();
    let census = loose.census();
    assert_eq!(census.stray, strays);
    assert_eq!(census.stray_named.len(), STRAY_NAMED);
    assert_eq!(census.invalid, invalid);
    assert_eq!(census.invalid_named.len(), DAYZIPS_NAMED);
    assert_eq!(census.misfiled, misfiled);
    assert_eq!(census.misfiled_named.len(), DAYZIPS_NAMED);
    assert_eq!(census.held, 0);
    // The day held only misfiled copies, many of them: refused, never read
    // as absent.
    assert_eq!(
        crate::gdfl_cm::CmSource::day(&loose, CmKind::Indices, day).err(),
        Some(CmRefusal::ArchiveDuplicateDay { day })
    );
}

/// A hidden folder under the root is passed over whole: the day zip inside
/// it, at the very path it would be held at below a visible root, files no
/// day, and the folder itself is a stray name. A hidden FILE is a stray too.
#[test]
fn a_hidden_folder_under_the_root_is_never_walked() {
    let root = fresh("hidden");
    let day = Day::new(2026, 9, 25).unwrap();
    let own = LooseKind::Indices.path_of(day);
    touch(&root, &format!(".snapshot/{own}"));
    touch(&root, ".hidden-file");
    let loose = DayZips::open(&root).unwrap();
    let census = loose.census();
    assert_eq!(census.held, 0);
    assert_eq!(census.misfiled, 0);
    assert_eq!(census.stray, 2);
    let mut named = census.stray_named.clone();
    named.sort();
    assert_eq!(named, [".hidden-file", ".snapshot"]);
    assert!(
        crate::gdfl_cm::CmSource::day(&loose, CmKind::Indices, day)
            .unwrap()
            .is_none()
    );
    // The same zip at its own place below the visible root is held.
    touch(&root, &own);
    let seen = DayZips::open(&root).unwrap();
    assert_eq!(seen.census().held, 1);
    assert_eq!(seen.census().stray, 2);
}

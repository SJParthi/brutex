#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

//! BASE-COV-PULL-PS2 (D-7336): the import's census room when the census
//! cannot be read for a reason other than its absence. Every fixture is
//! invented.

use super::*;

/// A CENSUS UNDER A `manifest` THAT IS A FILE CANNOT BE READ, and its room is
/// refused with the path and the host's words; an absent one is an empty
/// census with room for the batch.
#[test]
fn a_census_under_a_file_where_manifest_belongs_is_refused_by_its_path() {
    let root = std::env::temp_dir().join(format!(
        "brutex-import-census-ps2-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&root).unwrap();
    let empty = census_room(&root, &[]).expect("an absent census is empty");
    assert_eq!((empty.n_valid, empty.batch, empty.new), (0, 0, 0));
    std::fs::write(root.join("manifest"), b"invented: a file, not a directory").unwrap();
    let why = census_room(&root, &[]).expect_err("refused");
    let path = manifest_path(&root, Vendor::Gdfl);
    assert!(why.starts_with(&format!("{}: ", path.display())), "{why}");
    assert!(why.contains("ot a directory"), "the host's words: {why}");
    std::fs::remove_dir_all(&root).unwrap();
}

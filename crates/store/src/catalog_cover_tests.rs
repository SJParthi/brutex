#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BASE-COV-LIBS: the catalog walk's two remaining edges: a `bars/` root the
//! process may not list is refused, never an empty store; a path that is not
//! under `bars/` at all is counted at the wrong depth, never filed; and every
//! depth around the spot path's six components lands in its own bucket.

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use super::*;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let bars = self.0.join("bars");
        let _ = std::fs::set_permissions(&bars, std::fs::Permissions::from_mode(0o755));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let root = std::env::temp_dir().join(format!(
        "brutex-catalog-cover-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    Scratch(root)
}

#[test]
fn a_bars_root_that_cannot_be_listed_is_refused_not_empty() {
    let root = scratch("locked");
    let bars = root.0.join("bars");
    std::fs::create_dir_all(bars.join("zerodha")).unwrap();
    std::fs::set_permissions(&bars, std::fs::Permissions::from_mode(0o300)).unwrap();
    let walked = walk(&root.0);
    std::fs::set_permissions(&bars, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(walked, Err(CatalogError::BarsUnreadable { .. })),
        "{walked:?}"
    );
}

#[test]
fn every_depth_lands_in_its_own_bucket_and_a_foreign_path_is_never_filed() {
    let bars = Path::new("/brutex-cover/bars");
    let mut out = Holdings::default();
    classify(
        bars,
        Path::new("/elsewhere/zerodha/NSE/CASH/X/1min/2020-01.bin"),
        &mut out,
    );
    assert_eq!(out.census.wrong_depth, 1);
    let depths = [
        ("zerodha/NSE/CASH/X/2020-01.bin", "short"),
        ("zerodha/NSE/CASH/X/1min/2020-01.bin", "spot"),
        ("zerodha/NSE/FO/X/C1/1min/2020-01.bin", "contract"),
        ("zerodha/NSE/FO/X/C1/extra/1min/2020-01.bin", "deep"),
    ];
    for (rel, _) in depths {
        classify(bars, &bars.join(rel), &mut out);
    }
    assert_eq!(out.census.wrong_depth, 3, "{:?}", out.census);
    assert_eq!(out.census.with_contract, 1, "{:?}", out.census);
    assert!(out.held.len() <= 1, "only the spot path may be filed");
}

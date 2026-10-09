//! A directory chain whose every NEW entry is durable (xcut-3, ledgerv6-3;
//! D-2623).
//!
//! `fs::create_dir_all` creates up to N missing levels and syncs none of
//! them. A ledger or detail writer then syncs only its own leaf, so after a
//! power loss the leaf's ancestors — and with them the leaf and everything a
//! durable receipt elsewhere already names — can be gone. [`create_all`]
//! creates the same chain one level at a time and syncs each new level's
//! parent before the next level is made, so a level that exists after a crash
//! has a parent entry that names it.
//!
//! Levels that already existed are left as they are and not synced again: a
//! level made by an earlier `create_dir_all` that never reached the device
//! is outside what this module can repair. Like `create_dir_all`, an existing
//! level may be a symbolic link to a directory.
//!
//! The cost is one `mkdir` and one directory `fsync` per NEW level, at most
//! the depth of the chain, and one `stat` per existing level; nothing scales
//! with data. UNVERIFIED as a measured bound: no bench times it.

use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

/// Creates every missing directory of `path`, syncing each new level's
/// parent before the next level is made.
///
/// # Errors
///
/// An existing non-directory on the chain (`AlreadyExists`), a `..` or other
/// non-name component below the deepest existing level (`InvalidInput`), or
/// the host's refusal of a `mkdir`, an open or a sync.
pub(crate) fn create_all(path: &Path) -> io::Result<()> {
    let mut base = Path::new("");
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() || ancestor.is_dir() {
            base = ancestor;
            break;
        }
        if fs::symlink_metadata(ancestor).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} exists and is not a directory", ancestor.display()),
            ));
        }
    }
    let suffix = path.strip_prefix(base).map_err(io::Error::other)?;
    // Every component is checked before the first `mkdir`, so a refused
    // chain creates nothing.
    let mut parts = Vec::new();
    for component in suffix.components() {
        let Component::Normal(part) = component else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{} names a non-name component below {}",
                    path.display(),
                    base.display()
                ),
            ));
        };
        parts.push(part);
    }
    let mut current = base.to_path_buf();
    for part in parts {
        let parent = if current.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            current.clone()
        };
        current.push(part);
        match fs::create_dir(&current) {
            Ok(()) => File::open(&parent)?.sync_all()?,
            Err(why) if why.kind() == io::ErrorKind::AlreadyExists && current.is_dir() => {}
            Err(why) => return Err(why),
        }
    }
    Ok(())
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "scratch fixtures must fail loudly")]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "brutex-durable-dir-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch directory");
        dir
    }

    /// Depths 0 to 4 below an existing base, each created twice: every level
    /// exists afterwards and the second call is a no-op, never a refusal.
    #[test]
    fn every_depth_is_created_and_a_repeat_is_idempotent() {
        let base = scratch("depths");
        for depth in 0..=4_usize {
            let mut path = base.join(format!("d{depth}"));
            for level in 1..depth {
                path.push(format!("l{level}"));
            }
            let path = if depth == 0 { base.clone() } else { path };
            create_all(&path).expect("first creation");
            assert!(path.is_dir(), "{}", path.display());
            create_all(&path).expect("a repeat is a no-op");
            assert!(path.is_dir(), "{}", path.display());
        }
    }

    /// A file anywhere on the chain is refused, whether it is the leaf, an
    /// intermediate level or the deepest existing ancestor, and nothing is
    /// created beneath it.
    #[test]
    fn a_file_on_the_chain_is_refused_and_nothing_is_created_beneath_it() {
        let base = scratch("file");
        fs::write(base.join("leaf"), b"x").expect("leaf file");
        let leaf = create_all(&base.join("leaf")).expect_err("a file leaf");
        assert_eq!(leaf.kind(), io::ErrorKind::AlreadyExists);
        let below = create_all(&base.join("leaf").join("a").join("b")).expect_err("a file above");
        assert_eq!(below.kind(), io::ErrorKind::AlreadyExists);
        assert!(!base.join("leaf").join("a").exists());
    }

    /// `..` below the deepest existing level is refused rather than walked;
    /// `..` inside the existing prefix is the host's to resolve.
    #[test]
    fn a_parent_component_below_the_existing_prefix_is_refused() {
        let base = scratch("parent");
        let refused = create_all(&base.join("new").join("..").join("other")).expect_err("..");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(!base.join("new").exists());
        fs::create_dir(base.join("existing")).expect("existing level");
        create_all(&base.join("existing").join("..").join("sibling")).expect("existing ..");
        assert!(base.join("sibling").is_dir());
    }

    /// An existing level that is a symbolic link to a directory is followed,
    /// exactly as `create_dir_all` followed it.
    #[cfg(unix)]
    #[test]
    fn an_existing_link_to_a_directory_is_followed() {
        let base = scratch("link");
        fs::create_dir(base.join("target")).expect("target");
        std::os::unix::fs::symlink(base.join("target"), base.join("alias")).expect("alias");
        create_all(&base.join("alias").join("x").join("y")).expect("through the alias");
        assert!(base.join("target").join("x").join("y").is_dir());
    }
}

//! What a GDFL run files (D-3333), moved from `cli gdfl-import` into the one
//! run authority (IA-1, the design's D-4800): the stored scope of a
//! capital-market run, the filter `--stored` stands for, and the receipt's
//! sentence saying so. The cli and the api ask here; neither keeps a copy.
//!
//! WP-12f owns the scope RECORD (D-4878: each close names the digest of the
//! members it was asked to read, `gdfl_import::scope`); this module only
//! builds the [`CmScope`] a run hands its source, exactly as `cli` built it
//! before the move (`gdfl_import_command.rs:94`-`:207` at 50093fa3).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::gdfl_cm::CmScope;
use crate::gdfl_import::ImportKind;

/// The store's GDFL options tree, whose underlyings a share run files.
#[must_use]
pub fn options_tree(store: &Path) -> PathBuf {
    store.join("bars").join("gdfl").join("NSE").join("FNO")
}

/// Every underlying the options store holds, sorted: one read of the options
/// tree's directory, O(underlyings). A tree that does not exist yet holds
/// none.
///
/// # Errors
///
/// A tree that cannot be listed, or an entry whose name is not text, named.
pub fn option_underlyings(store: &Path) -> Result<Vec<String>, String> {
    let tree = options_tree(store);
    let entries = match std::fs::read_dir(&tree) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", tree.display())),
    };
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", tree.display()))?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|name| format!("{}: {} is not a symbol", tree.display(), name.display()))?;
        names.insert(name);
    }
    Ok(names.into_iter().collect())
}

/// The scope of a capital-market run of `kind` (D-3333), and how many
/// option underlyings it read.
///
/// # Errors
///
/// What [`option_underlyings`] and `CmScope::stored` refuse.
pub fn of(kind: ImportKind, store: &Path) -> Result<(CmScope, usize), String> {
    match kind {
        ImportKind::Stocks => {
            let underlyings = option_underlyings(store)?;
            let scope = CmScope::stored(&underlyings)?.with_masters(store.join("session-masters"));
            Ok((scope, underlyings.len()))
        }
        _ => Ok((CmScope::stored(std::iter::empty::<&str>())?, 0)),
    }
}

/// The filter `--stored` stands for: every store symbol the scope files that
/// the swept surface alone does not, sorted. An index row's store key; a
/// stored share's own symbol, and the newest symbol its chain of changes
/// reaches, which is where its old name's days are filed.
#[must_use]
pub fn stored_filter(kind: ImportKind, scope: &CmScope) -> Vec<String> {
    let mut names = BTreeSet::new();
    match kind {
        ImportKind::Indices => {
            for row in scope.indices() {
                names.insert(row.store_key.to_owned());
            }
        }
        _ => {
            for share in scope.shares() {
                names.insert(share.to_string());
                names.insert(brutex_core::alias::LINEAGE.current(share).to_owned());
            }
        }
    }
    names.into_iter().collect()
}

/// WHAT A RUN FILES (D-3333), and the receipt's sentence saying so: the
/// swept surface alone for an options run; for a capital-market run the
/// stored scope (the other index rows, the old names and every share
/// underlying the options store holds), `stored_only` narrowing `only` to
/// what it adds, under a filter of its own.
///
/// # Errors
///
/// What [`of`] refuses.
pub fn for_request(
    kind: ImportKind,
    store: &Path,
    stored_only: bool,
    only: &mut Vec<String>,
) -> Result<(Arc<CmScope>, Option<String>), String> {
    if kind == ImportKind::Options {
        return Ok((Arc::new(CmScope::swept()), None));
    }
    let (scope, underlyings) = of(kind, store)?;
    if stored_only {
        only.extend(stored_filter(kind, &scope));
    }
    let words = format!(
        "stored (D-3333): {} index rows and {} shares beyond the F&O list; {} option \
         underlyings read from {}",
        scope.indices().len(),
        scope.shares().len(),
        underlyings,
        options_tree(store).display()
    );
    Ok((Arc::new(scope), Some(words)))
}

#![cfg(test)]
//! BASE-COV-PULL-PS3 (G3): what `--stored` stands for (D-3333), over a stored
//! scope built from the store's own reference tables (a store that holds
//! none is served the compiled ones) and invented option underlyings
//! (`ZQ...`): every index row the scope adds, by its store key; every share
//! and the newest symbol its changes reach; and a request that narrows to it.
//! The compiled rows a test reads are read from their tables, never copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use brutex_core::alias::LINEAGE;

use super::*;
use crate::gdfl_fixtures::scratch;

/// A store whose options tree holds a folder for each of `names`.
fn store_with(tag: &str, names: &[&str]) -> PathBuf {
    let store = scratch(tag);
    for name in names {
        std::fs::create_dir_all(options_tree(&store).join(name)).unwrap();
    }
    store
}

/// Whether `names` is sorted with no name twice.
fn sorted_once(names: &[String]) -> bool {
    names.windows(2).all(|pair| pair[0] < pair[1])
}

/// The first F&O share of the swept surface that no symbol change names: an
/// underlying with a swept slot of its own and no older or newer name, read
/// from the lists rather than named here.
fn a_swept_share() -> &'static str {
    let reference = crate::reference_data::load(Path::new(""));
    let renames = reference.renames.table().unwrap();
    let named: std::collections::HashSet<&str> = renames
        .rows()
        .iter()
        .flat_map(|change| [change.old, change.new])
        .collect();
    brutex_core::universe::FNO_UNDERLYINGS
        .iter()
        .copied()
        .filter(|name| !brutex_core::universe::FNO_INDEX_UNDERLYINGS.contains(name))
        .find(|name| !named.contains(name))
        .unwrap()
}

/// An index run's filter is every index row the scope adds, by its store
/// key, sorted, once each; and nothing else: no share of the scope's.
#[test]
fn an_index_runs_filter_is_every_index_row_the_scope_adds() {
    let store = scratch("cov-scope-indices");
    let (scope, underlyings) = of(ImportKind::Indices, &store).unwrap();
    assert_eq!(underlyings, 0, "an index run reads no options tree");
    assert!(
        !scope.indices().is_empty(),
        "the premise: the compiled table holds index rows beyond the swept two"
    );
    let names = stored_filter(ImportKind::Indices, &scope);
    let mut keys: Vec<String> = scope
        .indices()
        .iter()
        .map(|row| row.store_key.to_owned())
        .collect();
    keys.sort();
    keys.dedup();
    assert_eq!(names, keys);
    assert!(sorted_once(&names));
    for share in scope.shares() {
        assert!(!names.contains(&share.to_string()), "{share}");
    }
    std::fs::remove_dir_all(store).unwrap();
}

/// COMBINED: a share run's filter over an options tree holding, at once, two
/// invented shares (out of order), an index underlying, a swept F&O share and
/// a stray file. The invented shares are named; the index is the index
/// import's and the swept share has its own slot, so neither is; every share
/// of the scope is named with the newest symbol its changes reach, even where
/// that symbol is a swept share the scope does not hold (where its old
/// name's days are filed); and nothing else is named.
#[test]
fn a_share_runs_filter_is_each_share_and_the_symbol_its_changes_reach() {
    let swept = a_swept_share();
    let store = store_with("cov-scope-shares", &["ZQB", "ZQA", "NIFTY", swept]);
    std::fs::write(options_tree(&store).join("a-stray-file"), b"x").unwrap();
    let (scope, underlyings) = of(ImportKind::Stocks, &store).unwrap();
    assert_eq!(underlyings, 4, "folders only");
    let names = stored_filter(ImportKind::Stocks, &scope);
    assert!(sorted_once(&names));
    for invented in ["ZQA", "ZQB"] {
        assert!(names.contains(&invented.to_owned()), "{invented}");
    }
    assert!(!names.contains(&"NIFTY".to_owned()));
    assert!(!names.contains(&swept.to_owned()));
    for share in scope.shares() {
        assert!(names.contains(&share.to_string()), "{share}");
        assert!(
            names.contains(&LINEAGE.current(share).to_owned()),
            "{share}"
        );
    }
    for name in &names {
        assert!(
            scope
                .shares()
                .iter()
                .any(|share| **share == **name || LINEAGE.current(share) == name),
            "{name} is neither a share of the scope nor the newest symbol of one"
        );
    }
    let reached: Vec<&str> = scope
        .shares()
        .iter()
        .map(|share| LINEAGE.current(share))
        .filter(|current| !scope.shares().iter().any(|share| &**share == *current))
        .collect();
    assert!(
        !reached.is_empty(),
        "the premise: a compiled change whose newest symbol has a swept slot"
    );
    for current in reached {
        assert!(names.contains(&current.to_owned()), "{current}");
    }
    std::fs::remove_dir_all(store).unwrap();
}

/// OUT OF THE BOX: the options tree holds the OLD symbol of an applied change
/// (read from the store's own table). The filter names the old symbol, and
/// the newest one its chain reaches, which is where its days are filed.
#[test]
fn an_old_symbols_options_folder_is_named_with_the_symbol_it_became() {
    let reference = crate::reference_data::load(Path::new(""));
    let renames = reference.renames.table().unwrap();
    let change = renames
        .rows()
        .iter()
        .find(|change| change.status.applies())
        .expect("the premise: an applied change");
    let store = store_with("cov-scope-old", &[change.old]);
    let (scope, underlyings) = of(ImportKind::Stocks, &store).unwrap();
    assert_eq!(underlyings, 1);
    let names = stored_filter(ImportKind::Stocks, &scope);
    let newest = LINEAGE.current(change.old);
    assert_ne!(newest, change.old, "the premise: the change renames it");
    assert!(names.contains(&change.old.to_owned()), "{names:?}");
    assert!(names.contains(&newest.to_owned()), "{names:?}");
    std::fs::remove_dir_all(store).unwrap();
}

/// A store with no options tree: a share run reads no underlying, and its
/// filter is the shares of the reference tables' changes alone, each with
/// the newest symbol it reaches.
#[test]
fn a_store_with_no_options_tree_files_the_changes_shares_alone() {
    let store = scratch("cov-scope-empty");
    let (scope, underlyings) = of(ImportKind::Stocks, &store).unwrap();
    assert_eq!(underlyings, 0);
    let names = stored_filter(ImportKind::Stocks, &scope);
    assert!(
        !names.is_empty(),
        "the premise: the compiled changes name shares"
    );
    assert!(sorted_once(&names));
    assert!(names.iter().all(|name| !name.starts_with("ZQ")));
    std::fs::remove_dir_all(store).unwrap();
}

/// `--stored` adds to a request's own names exactly the stored filter, after
/// them, and the receipt's sentence counts what the scope holds; without it
/// the names are the request's own and the sentence is the same; an index
/// run adds its index rows; and an options run is the swept surface with no
/// sentence, its names untouched even under `--stored`.
#[test]
fn a_stored_only_request_adds_exactly_the_stored_filter() {
    let store = store_with("cov-scope-request", &["ZQA"]);
    let mut only = vec!["ZQPRIOR".to_owned()];
    let (scope, words) = for_request(ImportKind::Stocks, &store, true, &mut only).unwrap();
    let filter = stored_filter(ImportKind::Stocks, &scope);
    assert!(!filter.is_empty());
    assert_eq!(only[0], "ZQPRIOR");
    assert_eq!(&only[1..], filter.as_slice());
    let words = words.unwrap();
    assert!(
        words.contains(&format!(
            "{} index rows and {} shares beyond the F&O list; 1 option underlyings read from {}",
            scope.indices().len(),
            scope.shares().len(),
            options_tree(&store).display()
        )),
        "{words}"
    );
    let mut own = vec!["ZQPRIOR".to_owned()];
    let (again, said) = for_request(ImportKind::Stocks, &store, false, &mut own).unwrap();
    assert_eq!(own, ["ZQPRIOR"]);
    assert_eq!(said.as_deref(), Some(words.as_str()));
    assert_eq!(again.shares(), scope.shares());
    let mut indices = Vec::new();
    let (index_scope, _) = for_request(ImportKind::Indices, &store, true, &mut indices).unwrap();
    assert_eq!(indices, stored_filter(ImportKind::Indices, &index_scope));
    assert!(!indices.is_empty());
    let mut options = vec!["ZQPRIOR".to_owned()];
    let (swept, none) = for_request(ImportKind::Options, &store, true, &mut options).unwrap();
    assert_eq!(none, None);
    assert!(!swept.is_stored());
    assert_eq!(options, ["ZQPRIOR"]);
    std::fs::remove_dir_all(store).unwrap();
}

/// COMBINED: `--stored` over an options tree that cannot be read (a file
/// where the tree goes) or that holds a folder whose name is no share symbol
/// (lower case): the request is refused by name and its names are left
/// exactly as they were.
#[test]
fn a_stored_request_over_a_tree_it_cannot_take_is_refused_and_adds_nothing() {
    let file = scratch("cov-scope-tree-file");
    std::fs::create_dir_all(options_tree(&file).parent().unwrap()).unwrap();
    std::fs::write(options_tree(&file), b"a file where the tree goes").unwrap();
    let mut only = vec!["ZQPRIOR".to_owned()];
    let why = for_request(ImportKind::Stocks, &file, true, &mut only).unwrap_err();
    assert!(
        why.contains(&options_tree(&file).display().to_string()),
        "{why}"
    );
    assert_eq!(only, ["ZQPRIOR"]);
    let lower = store_with("cov-scope-lower", &["zqa"]);
    let why = for_request(ImportKind::Stocks, &lower, true, &mut only).unwrap_err();
    assert!(
        why.contains("\"zqa\" is not a share symbol the store files under its own spelling"),
        "{why}"
    );
    assert_eq!(only, ["ZQPRIOR"]);
    std::fs::remove_dir_all(file).unwrap();
    std::fs::remove_dir_all(lower).unwrap();
}

//! `docs/01-architecture.md`'s dependency table, checked against the manifests it
//! describes.
//!
//! # Why this file exists
//!
//! That table said `vocab`, `indicators` and `engine` **did not exist** while all
//! three were workspace members with tests passing. It gave `indicators` two
//! dependencies it does not have and `engine` four. It omitted `lake` and
//! `telemetry` entirely — four of eleven members were absent or wrong.
//!
//! Every one of those statements was true when written. That is the failure mode: a
//! graph transcribed into prose cannot be wrong at the moment of transcription and
//! cannot stay right afterwards. `docs/05-decisions.md` D-0102 recorded the rule
//! that closes it — where a document states something the code also knows, a test
//! reads the document and compares — and this is that rule applied to the arrows.
//!
//! # What it checks, and what it deliberately does not
//!
//! It checks the **set of workspace members** and, for each one, the **set of
//! intra-workspace dependencies** the table claims. Both directions: a member the
//! table omits fails, and a table row naming a member that does not exist fails.
//!
//! It does not check external dependencies, which belong to
//! `crates/vocab/tests/workspace_is_rust.rs` and its package fingerprint. It does
//! not check the ASCII diagram above the table — a picture cannot be parsed
//! reliably and pretending otherwise would be a test that passes on a wrong
//! drawing. `docs/06-limits.md` is where that gap is recorded rather than here.
//!
//! # Why it lives in `core`
//!
//! `core` is the root of the graph and depends on nothing, so a test here cannot
//! itself perturb what it measures. `include_str!` takes a literal, so every
//! manifest is named below by hand and the hand-written list is closed against the
//! workspace `members` list by [`the_manifest_list_is_the_whole_workspace`].

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::{BTreeMap, BTreeSet};

/// The architecture document, whose §1 table is the claim under test.
const ARCHITECTURE: &str = include_str!("../../../docs/01-architecture.md");

/// The workspace manifest, whose `members` list is the set of real crates.
const WORKSPACE: &str = include_str!("../../../Cargo.toml");

/// Every member manifest, named by hand because `include_str!` takes a literal.
///
/// Closed against `WORKSPACE` by [`the_manifest_list_is_the_whole_workspace`], so a
/// crate added to the workspace and not to this list is a failing test rather than a
/// silently unchecked row.
const MANIFESTS: [(&str, &str); 11] = [
    ("api", include_str!("../../api/Cargo.toml")),
    ("core", include_str!("../Cargo.toml")),
    ("costs", include_str!("../../costs/Cargo.toml")),
    ("engine", include_str!("../../engine/Cargo.toml")),
    ("greeks", include_str!("../../greeks/Cargo.toml")),
    ("indicators", include_str!("../../indicators/Cargo.toml")),
    ("lake", include_str!("../../lake/Cargo.toml")),
    ("pull", include_str!("../../pull/Cargo.toml")),
    ("store", include_str!("../../store/Cargo.toml")),
    ("telemetry", include_str!("../../telemetry/Cargo.toml")),
    ("vocab", include_str!("../../vocab/Cargo.toml")),
];

/// The crate names, so a dependency line can be recognised as intra-workspace.
fn member_names() -> BTreeSet<&'static str> {
    MANIFESTS.iter().map(|(name, _)| *name).collect()
}

/// The intra-workspace dependencies a manifest actually declares.
///
/// Reads only the `[dependencies]` table, not `[dev-dependencies]`: the graph in
/// question is what a consumer links, and a dev-dependency is not that. The lib
/// target of `core` is renamed — `brutex_core = { path = "../core", package =
/// "core" }` — so the `package =` key is preferred over the dependency key when
/// present, which is why this reads the value and not just the name.
fn declared_deps(manifest: &str) -> BTreeSet<String> {
    let members = member_names();
    let Some(after) = manifest.split("\n[dependencies]").nth(1) else {
        return BTreeSet::new();
    };
    let table = after.split("\n[").next().unwrap_or(after);
    let mut found = BTreeSet::new();
    for line in table.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        // `package = "core"` renames; otherwise the key is the crate name.
        let renamed = value
            .split_once("package")
            .and_then(|(_, rest)| rest.split('"').nth(1));
        let named = renamed.map_or_else(|| key.trim().to_owned(), str::to_owned);
        if members.contains(named.as_str()) {
            found.insert(named);
        }
    }
    found
}

/// The table in §1, as `crate -> the dependencies its row claims`.
///
/// A row looks like:
///
/// ```text
/// | `costs` | Indian F&O transaction costs: ... | `core` | ✓ |
/// ```
///
/// The first cell is the crate, the third is the dependency claim. Backticked
/// names in that third cell are the arrows; the word "nothing" means no arrow, and
/// is required to be spelt rather than left as an empty cell — an empty cell is
/// indistinguishable from a row somebody forgot to finish.
fn documented_graph() -> BTreeMap<String, BTreeSet<String>> {
    let members = member_names();
    let section = ARCHITECTURE
        .split("## 1. The graph")
        .nth(1)
        .expect("§1 exists; if it was renamed this test must be updated with it");
    let section = section
        .split("\n## ")
        .next()
        .expect("splitting always yields a first element");

    let mut graph = BTreeMap::new();
    for line in section.lines() {
        if !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // ["", crate, owns, depends-on, exists, ""]
        if cells.len() < 5 {
            continue;
        }
        let name = cells[1].trim_matches('`');
        if !members.contains(name) {
            continue;
        }
        let claim = cells[3];
        let deps: BTreeSet<String> = claim
            .split('`')
            .skip(1)
            .step_by(2)
            .filter(|t| members.contains(*t))
            .map(str::to_owned)
            .collect();
        assert!(
            !deps.is_empty() || claim.to_lowercase().contains("nothing"),
            "§1's `{name}` row claims neither a dependency nor \"nothing\": {claim:?}. \
             An empty claim cannot be distinguished from an unfinished row."
        );
        graph.insert(name.to_owned(), deps);
    }
    graph
}

/// The hand-written manifest list is the whole workspace.
///
/// Without this, a twelfth crate could be added and every other test here would
/// still pass while never looking at it — the check would silently narrow.
#[test]
fn the_manifest_list_is_the_whole_workspace() {
    let members: BTreeSet<&str> = WORKSPACE
        .split("members = [")
        .nth(1)
        .expect("the workspace manifest has a members list")
        .split(']')
        .next()
        .expect("splitting always yields a first element")
        .split('"')
        .skip(1)
        .step_by(2)
        .filter_map(|p| p.strip_prefix("crates/"))
        .collect();

    assert_eq!(
        members,
        member_names(),
        "the workspace members and the hand-written MANIFESTS list have diverged. \
         Add the new crate to MANIFESTS and to `docs/01-architecture.md` §1."
    );
}

/// Every crate in the workspace has a row, and every row names a real crate.
///
/// This is the half that was wrong: three members were marked as not existing and
/// two were absent from the table altogether.
#[test]
fn the_documented_table_lists_every_crate_and_no_others() {
    let documented: BTreeSet<String> = documented_graph().into_keys().collect();
    let real: BTreeSet<String> = member_names().iter().map(|s| (*s).to_owned()).collect();

    let missing: Vec<&String> = real.difference(&documented).collect();
    assert!(
        missing.is_empty(),
        "`docs/01-architecture.md` §1 has no row for {missing:?}. A crate absent from \
         the table is a crate whose arrows nothing checks."
    );

    let invented: Vec<&String> = documented.difference(&real).collect();
    assert!(
        invented.is_empty(),
        "§1 has a row for {invented:?}, which is not a workspace member. `CLAUDE.md` \
         §5 names a `cli` crate that does not exist; §1 must not repeat that."
    );
}

/// Every arrow the document draws is an arrow the manifest declares, and vice versa.
///
/// Both directions matter and they fail differently. A **missing** arrow makes the
/// graph look more decoupled than it is, which is how `indicators` was believed to
/// be reusable while it still linked `store`. An **invented** arrow makes it look
/// more coupled, which is how `engine` was documented with four dependencies and
/// deters a consumer from taking it.
#[test]
fn every_documented_arrow_is_a_real_arrow() {
    let documented = documented_graph();
    for (name, manifest) in MANIFESTS {
        let real = declared_deps(manifest);
        let claimed = documented
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` has no row; the other test says so too"));
        assert_eq!(
            claimed, &real,
            "`docs/01-architecture.md` §1 says `{name}` depends on {claimed:?} and its \
             Cargo.toml declares {real:?}. `cargo tree -p {name} --edges normal \
             --depth 1` is the arbiter and the document is the stale copy."
        );
    }
}

/// The graph is acyclic, which is `CLAUDE.md` §5's actual requirement.
///
/// §5's *picture* is wrong in three ways D-0095 records, and this test deliberately
/// does not check the picture — it checks the property §5 exists to protect. A cycle
/// would make the build order undefined and is the one graph defect that cannot be
/// worked around downstream.
#[test]
fn the_dependency_graph_is_acyclic() {
    let edges: BTreeMap<&str, BTreeSet<String>> = MANIFESTS
        .iter()
        .map(|(name, manifest)| (*name, declared_deps(manifest)))
        .collect();

    // Kahn's algorithm: repeatedly remove a node with no remaining outgoing edge.
    // Whatever is left when nothing can be removed is a cycle.
    let mut remaining: BTreeSet<&str> = edges.keys().copied().collect();
    loop {
        let removable: Vec<&str> = remaining
            .iter()
            .copied()
            .filter(|n| {
                edges[n]
                    .iter()
                    .all(|d| !remaining.contains(d.as_str()) || d.as_str() == *n)
            })
            .collect();
        if removable.is_empty() {
            break;
        }
        for n in removable {
            remaining.remove(n);
        }
    }
    assert!(
        remaining.is_empty(),
        "these crates form a dependency cycle: {remaining:?}. `CLAUDE.md` §5 requires \
         the graph to be acyclic and a cycle has no build order."
    );
}

/// `core`, `vocab`, `greeks` and `telemetry` depend on no workspace crate.
///
/// The four roots. `core` and `greeks` each have their own CI gate for this (gate 9
/// and gate 9b); `vocab` and `telemetry` did not, and `vocab` being a root is what
/// D-0095's shareable set rests on — `indicators` and `engine` reach a consumer
/// through it, so an arrow added here would silently widen what they drag along.
#[test]
fn the_four_roots_depend_on_nothing_in_this_workspace() {
    for (name, manifest) in MANIFESTS {
        if !matches!(name, "core" | "vocab" | "greeks" | "telemetry") {
            continue;
        }
        let deps = declared_deps(manifest);
        assert!(
            deps.is_empty(),
            "`{name}` is a root of the graph and now depends on {deps:?}. If that arrow \
             is wanted it needs a `docs/05-decisions.md` entry, and `docs/10-shared-core.md` \
             needs re-reading: the six shareable crates are shareable because their \
             roots are empty."
        );
    }
}

/// The six crates `docs/10-shared-core.md` offers a consumer reach nothing else.
///
/// Transitively: `costs -> core` and `indicators -> vocab` are inside the set, so
/// the closure must stay inside it. An arrow from any of the six to `store`, `pull`,
/// `api`, `lake` or `telemetry` would make the boundary document false, and that
/// document is what another repository reads before depending on this one.
#[test]
fn the_shareable_six_close_over_themselves() {
    const SHAREABLE: [&str; 6] = ["core", "vocab", "greeks", "costs", "indicators", "engine"];
    let by_name: BTreeMap<&str, &str> = MANIFESTS.iter().copied().collect();

    for name in SHAREABLE {
        let manifest = by_name
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` is a workspace member"));
        for dep in declared_deps(manifest) {
            assert!(
                SHAREABLE.contains(&dep.as_str()),
                "`{name}` is in the shareable set and depends on `{dep}`, which is not. \
                 `docs/10-shared-core.md` promises a consumer these six and nothing \
                 else; this arrow breaks that promise."
            );
        }
    }
}

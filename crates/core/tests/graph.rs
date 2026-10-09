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
const MANIFESTS: [(&str, &str); 13] = [
    ("api", include_str!("../../api/Cargo.toml")),
    // The operator entry point for the sweep. D-0169; before it, nothing that
    // could be RUN reached `runner`, `engine`, `indicators`, `vocab` or `costs`.
    ("cli", include_str!("../../cli/Cargo.toml")),
    ("core", include_str!("../Cargo.toml")),
    ("costs", include_str!("../../costs/Cargo.toml")),
    ("engine", include_str!("../../engine/Cargo.toml")),
    ("greeks", include_str!("../../greeks/Cargo.toml")),
    ("indicators", include_str!("../../indicators/Cargo.toml")),
    ("lake", include_str!("../../lake/Cargo.toml")),
    ("pull", include_str!("../../pull/Cargo.toml")),
    ("runner", include_str!("../../runner/Cargo.toml")),
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
/// The table kinds whose keys are dependency names.
///
/// `dev-` and `build-` are here because they LINK. A dev-dependency is compiled into
/// every test and bench, and a build-dependency runs at compile time -- so a crate that
/// declares one can reach it, whatever the normal graph says. The previous version of this
/// parser read only `[dependencies]`.
///
/// The underscore spellings are here because Cargo still reads them as the same tables
/// (with a deprecation warning), so a parser that knows only the hyphen misses a link.
/// D-1107.
const DEPENDENCY_TABLES: [&str; 5] = [
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
    "dev_dependencies",
    "build_dependencies",
];

/// Every dependency a manifest declares, as `(key, the text after the `=`)`.
///
/// # Why this exists rather than a `split("\n[dependencies]")`
///
/// Cargo accepts four spellings of one dependency and the previous parser saw one:
///
/// ```text
/// [dependencies]
/// store = { path = "../store" }        the only form it read
/// store.path = "../store"             a dotted key -- read as a crate called "store.path"
///
/// [dependencies.store]                a table header -- invisible
/// path = "../store"
///
/// [dev-dependencies]                  links into every test and bench -- invisible
/// [target.'cfg(unix)'.dependencies]   links on that target -- invisible
/// ```
///
/// An adversarial audit confirmed by running it that one `[dependencies.store]` stanza
/// defeated NINE guarantees at once, four of which are the tests in this file: the
/// acyclicity proof, the four roots, the shareable six, and every documented arrow. The
/// other five were gate 9, gate 9b, gate 21 clause A, gate 22 clause A and
/// `engine::the_sweep_cannot_compute_a_condition_bit`. Nine mechanisms, one parser shape,
/// one bypass.
///
/// # The second repair: keys are read as TOML keys (D-1107)
///
/// The version above recognised a table only when its header was spelt exactly
/// `dependencies.NAME`, and read a key only inside such a table. TOML spells the same
/// key many ways, and each of these declared a dependency this parser did not see:
///
/// ```text
/// [ dependencies . store ]            spaces around the dot
/// ["dependencies"]                    a quoted segment
/// dependencies.store = { .. }         a dotted key at the ROOT, before any header
/// "store".path = ".."                 a quoted key inside the table
/// ```
///
/// Every header and every key is now split into its segments the way TOML splits
/// them -- quoted segments unquoted, whitespace around a dot dropped -- and joined to
/// the open table, so a declaration is recognised by its full path wherever it is
/// written. A `#` inside a quoted value is no longer a comment, and an `=` inside a
/// quoted key no longer ends the key.
///
/// The limit, stated: this is not a TOML parser. A multi-line array or string is read
/// line by line, so a continuation line holding a top-level `=` would be read as a key.
/// No manifest in this workspace has one; gates 9, 9b and 22 read manifests with
/// `.github/source_scan.rs`'s `deps`, which parses values rather than lines.
fn declarations(manifest: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut table: Vec<String> = Vec::new();
    // The `[dependencies.NAME]` currently open, carried so a `package = "x"` line inside
    // it can rename it the same way an inline table can.
    let mut pending: Option<(String, String)> = None;

    for raw in manifest.lines() {
        let line = without_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[') {
            if let Some(done) = pending.take() {
                out.push(done);
            }
            let inner = inner.trim_start_matches('[');
            let inner = inner.trim_end().trim_end_matches(']');
            table = key_segments(inner);
            pending = dependency_of(&table).map(|(name, _)| (name.to_owned(), String::new()));
            continue;
        }
        let Some(eq) = top_level_equals(line) else {
            continue;
        };
        let (key, value) = (&line[..eq], &line[eq + 1..]);
        let key = key_segments(key);
        if let Some((_, current)) = pending.as_mut() {
            if key == ["package"] {
                current.clear();
                current.push_str("package =");
                current.push_str(value);
            }
            continue;
        }
        let path: Vec<String> = table.iter().chain(key.iter()).cloned().collect();
        let Some((name, rest)) = dependency_of(&path) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        // `store.package = "x"` renames exactly as an inline `package` key does.
        let value = if rest == ["package"] {
            format!("package ={value}")
        } else {
            value.to_owned()
        };
        out.push((name.to_owned(), value));
    }
    if let Some(done) = pending.take() {
        out.push(done);
    }
    out
}

/// The dependency a full key path declares, and the path below its name.
///
/// `dependencies.NAME..`, `dev-dependencies.NAME..`, `build-dependencies.NAME..` and
/// `target.CFG.<any of the three>.NAME..`.
fn dependency_of(path: &[String]) -> Option<(&str, &[String])> {
    match path {
        [table, name, rest @ ..] if DEPENDENCY_TABLES.contains(&table.as_str()) => {
            Some((name.as_str(), rest))
        }
        [target, _, table, name, rest @ ..]
            if target == "target" && DEPENDENCY_TABLES.contains(&table.as_str()) =>
        {
            Some((name.as_str(), rest))
        }
        _ => None,
    }
}

/// A line with its comment removed; a `#` inside a quoted string is text.
fn without_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (at, c) in line.char_indices() {
        match quote {
            Some('"') if escaped => escaped = false,
            Some('"') if c == '\\' => escaped = true,
            Some(q) if c == q => quote = None,
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' => return &line[..at],
            _ => {}
        }
    }
    line
}

/// The byte offset of the first `=` outside a quoted string.
fn top_level_equals(line: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (at, c) in line.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '=' => return Some(at),
            _ => {}
        }
    }
    None
}

/// A TOML key split at its dots: quoted segments unquoted, bare ones trimmed.
fn key_segments(key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut segment = String::new();
    let mut quote: Option<char> = None;
    for c in key.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '.' => out.push(std::mem::take(&mut segment).trim().to_owned()),
            _ => segment.push(c),
        }
    }
    out.push(segment.trim().to_owned());
    out
}

/// The package a declaration's value renames it to, if it names one.
///
/// `package = "x"` (a header table's line, or a dotted key's) or an inline table holding
/// a `package` key at its top level. A path that merely CONTAINS the word `package` is
/// not a rename, which the previous `split_once("package")` read as one.
fn package_of(value: &str) -> Option<String> {
    let value = value.trim();
    let unquote = |text: &str| {
        let text = text.trim();
        let inner = text
            .strip_prefix('"')
            .and_then(|t| t.strip_suffix('"'))
            .or_else(|| text.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')))?;
        Some(inner.to_owned())
    };
    if let Some(inner) = value.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
        let mut depth = 0_u32;
        let mut quote: Option<char> = None;
        let mut start = 0;
        let mut parts = Vec::new();
        for (at, c) in inner.char_indices() {
            match quote {
                Some(q) if c == q => quote = None,
                None if c == '"' || c == '\'' => quote = Some(c),
                None if c == '[' || c == '{' => depth += 1,
                None if c == ']' || c == '}' => depth = depth.saturating_sub(1),
                None if c == ',' && depth == 0 => {
                    parts.push(&inner[start..at]);
                    start = at + 1;
                }
                _ => {}
            }
        }
        parts.push(&inner[start..]);
        return parts.into_iter().find_map(|part| {
            let eq = top_level_equals(part)?;
            (key_segments(&part[..eq]) == ["package"])
                .then(|| unquote(&part[eq + 1..]))
                .flatten()
        });
    }
    let eq = top_level_equals(value)?;
    (key_segments(&value[..eq]) == ["package"])
        .then(|| unquote(&value[eq + 1..]))
        .flatten()
}

fn declared_deps(manifest: &str) -> BTreeSet<String> {
    let members = member_names();
    // One dependency may be declared over several lines -- `store.path = ..` and
    // `store.package = "x"` -- so the rename is resolved per NAME, not per line: a
    // renamed dependency links the package, and its key names nothing.
    let mut by_name: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (key, value) in declarations(manifest) {
        let slot = by_name.entry(key).or_insert(None);
        if let Some(package) = package_of(&value) {
            *slot = Some(package);
        }
    }
    by_name
        .into_iter()
        .map(|(key, package)| package.unwrap_or(key))
        .filter(|named| members.contains(named.as_str()))
        .collect()
}

/// The parser is tested against the spellings it exists for.
///
/// A parser nothing tests is the previous version of this parser.
#[test]
fn every_spelling_of_a_dependency_is_seen() {
    let cases: [(&str, &[&str]); 7] = [
        (
            "[dependencies]\nstore = { path = \"../store\" }",
            &["store"],
        ),
        ("[dependencies]\nstore.path = \"../store\"", &["store"]),
        ("[dependencies.store]\npath = \"../store\"", &["store"]),
        ("[dev-dependencies]\nstore = \"1\"", &["store"]),
        (
            "[target.'cfg(unix)'.dependencies]\nstore = \"1\"",
            &["store"],
        ),
        ("[dependencies]\n# store = { path = \"../store\" }", &[]),
        // The rename, both ways round: an inline table and a header table.
        (
            "[dependencies]\nbc = { path = \"../core\", package = \"core\" }\n\n[dependencies.st]\npackage = \"store\"",
            &["core", "store"],
        ),
    ];
    for (manifest, want) in cases {
        let got = declared_deps(manifest);
        let want: BTreeSet<String> = want.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(
            got, want,
            "declared_deps read {got:?} from:\n{manifest}\nand must read {want:?}"
        );
    }
}

/// The spellings the first repair still missed, each one valid TOML that Cargo reads as
/// a dependency, and the false rename it used to invent. D-1107.
#[test]
fn every_toml_spelling_of_a_key_is_seen() {
    let cases: [(&str, &[&str]); 14] = [
        ("[dev_dependencies]\nstore = \"1\"", &["store"]),
        (
            "[build_dependencies.store]\npath = \"../store\"",
            &["store"],
        ),
        ("[ dependencies . store ]\npath = \"../store\"", &["store"]),
        ("[\"dependencies\"]\nstore = \"1\"", &["store"]),
        ("[dependencies.\"store\"]\npath = \"../store\"", &["store"]),
        ("dependencies.store = { path = \"../store\" }", &["store"]),
        ("[dependencies]\n\"store\".path = \"../store\"", &["store"]),
        ("[dependencies]\n'store' = \"1\"", &["store"]),
        (
            "[target.\"cfg(unix)\" . dev-dependencies]\nstore = \"1\"",
            &["store"],
        ),
        ("[[dependencies.store]]\npath = \"../store\"", &["store"]),
        // A `#` in a quoted value is text, so the key after it is not swallowed.
        (
            "[dependencies]\nvocab = { path = \"../#v\" }\nstore = \"1\"",
            &["store", "vocab"],
        ),
        // A dotted rename resolves per name, so the key names nothing.
        (
            "[dependencies]\nv.path = \"../store\"\nv.package = \"store\"",
            &["store"],
        ),
        (
            "[dependencies]\nv = { path = \"../store\", package = 'store' }",
            &["store"],
        ),
        // A path that merely contains the word is not a rename.
        (
            "[dependencies]\nstore = { path = \"../package/\\\"core\\\"\" }",
            &["store"],
        ),
    ];
    for (manifest, want) in cases {
        let got = declared_deps(manifest);
        let want: BTreeSet<String> = want.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(
            got, want,
            "declared_deps read {got:?} from:\n{manifest}\nand must read {want:?}"
        );
    }
    // Neither a comment nor a non-dependency table declares anything.
    for silent in [
        "# [dependencies]\n# store = \"1\"",
        "[package]\nstore = \"1\"",
        "[features]\ndependencies.store = []",
        "[dependencies]\n= \"1\"",
    ] {
        assert_eq!(declared_deps(silent), BTreeSet::new(), "{silent}");
    }
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
    documented_graph_from(ARCHITECTURE)
}

/// §1 of `document`, through to the next `## ` heading.
fn graph_section(document: &str) -> &str {
    let section = document
        .split("## 1. The graph")
        .nth(1)
        .expect("§1 exists; if it was renamed this test must be updated with it");
    section
        .split("\n## ")
        .next()
        .expect("splitting always yields a first element")
}

/// The first-cell name of EVERY crate-shaped row in §1, in document order,
/// before any filter against the workspace.
///
/// [`documented_graph_from`] drops a row whose name is not a member, which is
/// right for building the graph and is why "every row names a real crate"
/// could not fail against it (P1-14-02): the check compared a set the parser
/// had already narrowed to the members. This list is read before that filter,
/// and keeps duplicates so a second, contradictory row for one crate is seen.
fn documented_row_names_from(document: &str) -> Vec<String> {
    graph_section(document)
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            (cells.len() >= 5).then(|| cells[1].trim_matches('`').to_owned())
        })
        .collect()
}

/// What is wrong with §1's row names against `members`: rows for a crate that
/// is not a member, and crates with more than one row.
fn row_name_faults(names: &[String], members: &BTreeSet<&str>) -> (Vec<String>, Vec<String>) {
    let invented: Vec<String> = names
        .iter()
        .filter(|name| !members.contains(name.as_str()))
        .cloned()
        .collect();
    let mut seen = BTreeSet::new();
    let duplicated: Vec<String> = names
        .iter()
        .filter(|name| !seen.insert(name.as_str()))
        .cloned()
        .collect();
    (invented, duplicated)
}

/// The same document parser, with supplied text so its row alternatives can be tested.
fn documented_graph_from(document: &str) -> BTreeMap<String, BTreeSet<String>> {
    let members = member_names();
    let section = graph_section(document);

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

/// Every member the root manifest's `members = [ ... ]` names, as a crate
/// directory name under `crates/`.
///
/// # Panics
///
/// On a member outside `crates/` or a glob (P1-14-03). The parser used to keep
/// only `crates/` members and drop the rest, so `"tools/x"` or `"xtask"` never
/// reached any check here, and `"crates/*"` became a crate named `*`. Comments
/// are removed per line first, so a `]` or `"` inside one cannot truncate or
/// shift the list.
fn workspace_members(manifest: &str) -> BTreeSet<&str> {
    let list = manifest
        .split_once("members = [")
        .expect("the workspace manifest has a members list")
        .1;
    let mut members = BTreeSet::new();
    for line in list.lines() {
        let code = line.split_once('#').map_or(line, |(code, _)| code);
        for (index, piece) in code.split('"').enumerate() {
            if index % 2 == 1 {
                let name = piece.strip_prefix("crates/").unwrap_or_else(|| {
                    panic!(
                        "workspace member {piece:?} is not under crates/; no check here reads it"
                    )
                });
                assert!(
                    !name.contains('*') && !name.contains('/') && !name.is_empty(),
                    "workspace member {piece:?} is not one crate directory"
                );
                members.insert(name);
            }
        }
        if code.contains(']') {
            break;
        }
    }
    members
}

/// The members parser refuses what it used to drop.
#[test]
fn the_members_parser_refuses_a_member_it_cannot_check() {
    assert_eq!(
        workspace_members(
            "members = [\n    \"crates/core\", # a comment with ] and \" in it\n    \"crates/store\",\n]\n"
        ),
        BTreeSet::from(["core", "store"])
    );
    for bad in [
        "members = [\"crates/core\", \"tools/probe\"]",
        "members = [\"xtask\"]",
        "members = [\"crates/*\"]",
    ] {
        assert!(
            std::panic::catch_unwind(|| workspace_members(bad)).is_err(),
            "{bad} must be refused, not dropped"
        );
    }
}

/// The hand-written manifest list is the whole workspace.
///
/// Without this, a twelfth crate could be added and every other test here would
/// still pass while never looking at it — the check would silently narrow.
#[test]
fn the_manifest_list_is_the_whole_workspace() {
    let members = workspace_members(WORKSPACE);
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

    // READ BEFORE THE MEMBER FILTER (P1-14-02). `documented` cannot hold a
    // non-member by construction, so its difference with `real` was empty
    // whatever the document said; the row names are read unfiltered instead.
    let (invented, duplicated) =
        row_name_faults(&documented_row_names_from(ARCHITECTURE), &member_names());
    assert!(
        invented.is_empty(),
        "§1 has a row for {invented:?}, which is not a workspace member. `CLAUDE.md` \
         §5 once drew a `web` crate that does not exist; §1 must not repeat that."
    );
    assert!(
        duplicated.is_empty(),
        "§1 has more than one row for {duplicated:?}; only the last would be checked"
    );
}

/// The row-name check sees an invented row and a duplicated one.
#[test]
fn an_invented_or_duplicated_row_is_named() {
    let document = "## 1. The graph\n\
                    | `store` | wrong | `vocab` | ✓ |\n\
                    | `web` | browser | `core` | ✓ |\n\
                    | `store` | store | `core` | ✓ |\n";
    assert_eq!(
        row_name_faults(&documented_row_names_from(document), &member_names()),
        (vec!["web".to_owned()], vec!["store".to_owned()])
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

/// A later header flushes the named dependency, including its package rename.
#[test]
fn named_dependencies_survive_following_package_and_dependency_headers() {
    let manifest = r#"
[dependencies.bc]
package = "core"
path = "../core"
[package]
name = "graph-fixture"
[dependencies.store]
path = "../store"
[dev-dependencies.vocab]
path = "../vocab"
"#;
    assert_eq!(
        declarations(manifest),
        vec![
            ("bc".to_owned(), "package = \"core\"".to_owned()),
            ("store".to_owned(), String::new()),
            ("vocab".to_owned(), String::new()),
        ]
    );
    assert_eq!(
        declared_deps(manifest),
        BTreeSet::from(["core".to_owned(), "store".to_owned(), "vocab".to_owned()])
    );
}

/// A malformed line contributes no declaration and does not hide valid neighbors.
#[test]
fn a_dependency_line_without_equals_is_omitted() {
    let manifest = "[dependencies]\ncore = \"1\"\nstore\nvocab = \"1\"\n";
    assert_eq!(
        declarations(manifest),
        vec![
            ("core".to_owned(), " \"1\"".to_owned()),
            ("vocab".to_owned(), " \"1\"".to_owned()),
        ]
    );
    assert_eq!(
        declared_deps(manifest),
        BTreeSet::from(["core".to_owned(), "vocab".to_owned()])
    );
}

/// Even a package rename cannot turn an empty dependency name into a declaration.
#[test]
fn empty_dependency_names_are_omitted_before_package_resolution() {
    for malformed in ["= { package = \"store\" }", ".path = \"../store\""] {
        let manifest = format!("[dependencies]\ncore = \"1\"\n{malformed}\nvocab = \"1\"\n");
        assert_eq!(
            declarations(&manifest),
            vec![
                ("core".to_owned(), " \"1\"".to_owned()),
                ("vocab".to_owned(), " \"1\"".to_owned()),
            ],
            "malformed declaration: {malformed}"
        );
        assert_eq!(
            declared_deps(&manifest),
            BTreeSet::from(["core".to_owned(), "vocab".to_owned()]),
            "malformed declaration: {malformed}"
        );
    }
}

/// Short rows are omitted while complete rows retain both roots and real arrows.
#[test]
fn short_document_rows_do_not_hide_or_invent_graph_members() {
    for short in ["| `store`", "| `store` |", "| `store` | storage |"] {
        let document = format!(
            "## 1. The graph\n\
             | `core` | types | nothing | yes |\n\
             {short}\n\
             | `costs` | costs | `core` | yes |\n"
        );
        assert_eq!(
            documented_graph_from(&document),
            BTreeMap::from([
                ("core".to_owned(), BTreeSet::new()),
                ("costs".to_owned(), BTreeSet::from(["core".to_owned()])),
            ]),
            "short row: {short}"
        );
    }
}

/// A complete row for an unknown member is omitted without discarding valid rows.
#[test]
fn unknown_document_members_do_not_enter_the_graph() {
    let document = "## 1. The graph\n\
                    | `core` | types | nothing | yes |\n\
                    | `not-a-workspace-member` | unknown | `core` | yes |\n\
                    | `costs` | costs | `core` | yes |\n";
    assert_eq!(
        documented_graph_from(document),
        BTreeMap::from([
            ("core".to_owned(), BTreeSet::new()),
            ("costs".to_owned(), BTreeSet::from(["core".to_owned()])),
        ])
    );
}

/// `deny.toml`, whose `[bans] deny` list names every crate the build refuses.
const DENY: &str = include_str!("../../../deny.toml");

/// Every crate name in `deny.toml`'s `[bans] deny` list.
fn banned_crates(deny: &str) -> BTreeSet<String> {
    let Some((_, list)) = deny.split_once("\ndeny = [") else {
        return BTreeSet::new();
    };
    let list = list.split_once("\n]").map_or(list, |(head, _)| head);
    list.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_once("name = \"")?.1.split_once('"'))
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// Each run of consecutive comment lines in `manifest` that names a banned
/// crate in backticks and never says `deny.toml`: the comment's 1-based first
/// line and the crate it names.
fn unbanned_mentions(manifest: &str, banned: &BTreeSet<String>) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    let mut first = 0;
    for (index, line) in manifest.lines().chain(std::iter::once("")).enumerate() {
        if line.trim_start().starts_with('#') {
            if block.is_empty() {
                first = index + 1;
            }
            block.push(line);
            continue;
        }
        let text = block.join("\n");
        if !text.contains("deny.toml") {
            for name in banned {
                if text.contains(&format!("`{name}`")) {
                    out.push((first, name.clone()));
                }
            }
        }
        block.clear();
    }
    out
}

/// D-3501 (ONEAUTH-02). `pull`'s manifest said `ring` "arrives through the
/// `reqwest` line" and put 72 `ring_core` symbols in the shipped binary for
/// months after D-0211 removed it and D-0212 banned it. `deny.toml` is the one
/// authority on which native crates may build; a manifest comment that names
/// one of them must say it is banned, or it is a second, unchecked answer.
#[test]
fn a_manifest_comment_naming_a_banned_crate_says_it_is_banned() {
    let banned = banned_crates(DENY);
    for name in [
        "ring",
        "aws-lc-rs",
        "aws-lc-sys",
        "cc",
        "pyo3",
        "openssl-sys",
    ] {
        assert!(banned.contains(name), "deny.toml no longer bans {name}");
    }
    let mut stale = Vec::new();
    for (crate_name, manifest) in MANIFESTS.iter().chain([&("workspace", WORKSPACE)]) {
        for (line, name) in unbanned_mentions(manifest, &banned) {
            stale.push(format!("{crate_name}/Cargo.toml:{line} names `{name}`"));
        }
    }
    assert_eq!(stale, Vec::<String>::new());
}

/// The reader itself: a block is the whole comment run, a mention needs the
/// backticks, and `deny.toml` anywhere in the run clears it.
#[test]
fn the_banned_mention_reader_reads_whole_comment_runs() {
    let banned = BTreeSet::from(["ring".to_owned(), "cc".to_owned()]);
    let manifest = "# `ring` arrives here\n#\n# still the same run\nx = 1\n\
                    # ring without backticks\ny = 2\n\
                    # `cc` is named\n# and deny.toml bans it\nz = 3\n\
                    # `ring` at the end of the file";
    assert_eq!(
        unbanned_mentions(manifest, &banned),
        vec![(1, "ring".to_owned()), (10, "ring".to_owned())]
    );
    assert_eq!(
        banned_crates(
            "x\ndeny = [\n    # { name = \"no\" },\n    { name = \"a\" },\n]\nname = \"b\"\n"
        ),
        BTreeSet::from(["a".to_owned()])
    );
    assert_eq!(banned_crates("nothing"), BTreeSet::new());
}

/// `CLAUDE.md`, whose §5 block is one hand-drawn picture of the graph.
const LAW: &str = include_str!("../../../CLAUDE.md");

/// `AGENTS.md`, whose §5 block is the second.
const AGENTS: &str = include_str!("../../../AGENTS.md");

/// A crate name and the workspace crates it depends on, per crate.
type Graph = BTreeMap<String, BTreeSet<String>>;

/// The graph a `## 5.` picture draws: `depends on NOTHING  a · b` gives each of
/// `a` and `b` no arrows, and `x y <-- a · b` gives each of `a` and `b` the set
/// `{x, y}`. A consumer drawn twice is reported by its second line, so it cannot
/// hide behind the first; `None` when the section or its block is missing.
fn drawn_graph(document: &str) -> Option<(Graph, Vec<String>)> {
    let (_, section) = document.split_once("\n## 5.")?;
    let section = section
        .split_once("\n## ")
        .map_or(section, |(head, _)| head);
    let (_, block) = section.split_once("```\n")?;
    let (block, _) = block.split_once("```")?;
    let mut graph = BTreeMap::new();
    let mut twice = Vec::new();
    for line in block.lines() {
        let (deps, consumers) = if let Some(roots) = line.strip_prefix("depends on NOTHING") {
            (BTreeSet::new(), roots)
        } else if let Some((left, right)) = line.split_once("<--") {
            (left.split_whitespace().map(str::to_owned).collect(), right)
        } else {
            continue;
        };
        for consumer in consumers
            .split('·')
            .map(str::trim)
            .filter(|c| !c.is_empty())
        {
            if graph.insert(consumer.to_owned(), deps.clone()).is_some() {
                twice.push(consumer.to_owned());
            }
        }
    }
    Some((graph, twice))
}

/// D-3502 (ONEAUTH-03). `CLAUDE.md` §5 and `AGENTS.md` §5 are hand-drawn and
/// were checked by nothing: `docs/06-limits.md` recorded that `cli`'s `pull` and
/// `vocab` arrows were missing from them for three weeks while the checked table
/// carried both. The manifests are the one authority; each picture is now
/// compared against them both ways, every member and every arrow.
#[test]
fn the_law_pictures_of_the_graph_are_the_manifests() {
    let real: Graph = MANIFESTS
        .iter()
        .map(|(name, manifest)| ((*name).to_owned(), declared_deps(manifest)))
        .collect();
    for (file, document) in [("CLAUDE.md", LAW), ("AGENTS.md", AGENTS)] {
        let (drawn, twice) =
            drawn_graph(document).unwrap_or_else(|| panic!("{file} has no §5 picture"));
        assert_eq!(twice, Vec::<String>::new(), "{file} §5 draws a crate twice");
        assert_eq!(
            drawn, real,
            "{file} §5 draws a graph the manifests do not declare; the manifests win"
        );
    }
}

/// The picture reader: roots, a multi-consumer row, a consumer drawn twice, a
/// line with no arrow, and the block ending at its fence before §6.
#[test]
fn the_picture_reader_reads_roots_rows_and_repeats() {
    let document = "# x\n## 5. Graph\ntext <-- not in a block\n```\n\
                    depends on NOTHING   a · b\n\nprose without an arrow\n\
                    a b   <-- c · d\na <-- d\n```\nafter <-- fence\n## 6. Next\n```\nz <-- y\n```\n";
    let (graph, twice) = drawn_graph(document).expect("a picture");
    let set = |names: &[&str]| {
        names
            .iter()
            .map(|n| (*n).to_owned())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        graph,
        BTreeMap::from([
            ("a".to_owned(), set(&[])),
            ("b".to_owned(), set(&[])),
            ("c".to_owned(), set(&["a", "b"])),
            ("d".to_owned(), set(&["a"])),
        ])
    );
    assert_eq!(twice, vec!["d".to_owned()]);
    assert_eq!(drawn_graph("## 5. no block\n"), None);
    assert_eq!(
        drawn_graph("## 4. wrong section\n```\na <-- b\n```\n"),
        None
    );
}

//! Every bench module is mounted and run by its crate's gate-8 harness, and
//! CC-20's seven check-primitive benches exist among them (FM-615, D-6800).
//!
//! # Why this file exists
//!
//! A file under `crates/<name>/benches/ratio/` is not a bench target: cargo
//! builds it only when `benches/ratio.rs` mounts it with
//! `#[path = "ratio/<file>.rs"] mod <name>;`, and it measures something only
//! when the harness then calls into it. A module nobody mounts compiles in no
//! build and fails no gate, and a module that is mounted and never called is
//! dead code the compiler would warn about only if it were not `pub(crate)`.
//! Either way its cost claim reads as proven while no ratio of it is taken.
//! `cost_invariants.rs` cannot see this: it reads printed ids from the
//! harness files alone, by design, and a module's own prints never reach it.
//!
//! # What is checked, and why "called from `main`" is not read literally
//!
//! For every crate, every module file is mounted by exactly one `#[path]`
//! line, every `#[path]` line names a file that exists, and the module's name
//! appears as a `<module>::` path in the harness outside its own mount and
//! outside comments. That last rule is what "called" can soundly mean:
//! `api`'s harness reaches ten of its modules from `served_routes_are_flat`,
//! which `main` calls, and names `served` only as `served::stores`; reading
//! "called from `main`" literally would fail a harness that is right.
//!
//! CC-20's seven are held to the stronger rule, because they are new and
//! nothing else gates them: each is mounted, and its result is folded into
//! the harness's verdict with `ok &= <module>::`, so a breach exits 1.
//!
//! Everything else is discovered by walking `crates/`, never listed, so a
//! module added tomorrow is covered without editing this file.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The workspace root: two parents above `crates/core`.
fn workspace_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// One crate's gate-8 harness and the module files beside it.
struct Harness {
    /// `crates/<name>/benches/ratio.rs`, read.
    source: String,
    /// Every `*.rs` directly under `crates/<name>/benches/ratio/`, by stem.
    files: BTreeSet<String>,
}

/// Every crate that carries a `benches/ratio/` folder, by crate directory.
fn harnesses(crates: &Path) -> BTreeMap<String, Harness> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(crates).expect("crates/ is readable").flatten() {
        let folder = entry.path().join("benches/ratio");
        let Ok(listing) = std::fs::read_dir(&folder) else {
            continue;
        };
        let files: BTreeSet<String> = listing
            .flatten()
            .map(|file| file.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "rs"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_owned)
            })
            .collect();
        let harness = entry.path().join("benches/ratio.rs");
        let source = std::fs::read_to_string(&harness).unwrap_or_else(|why| {
            panic!(
                "{} has module files and no readable harness at {}: {why}",
                folder.display(),
                harness.display()
            )
        });
        let name = entry.file_name().to_string_lossy().into_owned();
        out.insert(name, Harness { source, files });
    }
    out
}

/// Every `#[path = "ratio/<file>.rs"]` mount of a harness: the file's stem
/// and the module name on the `mod <name>;` line after it.
fn mounts(source: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = source.lines().map(str::trim).collect();
    let mut out = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("#[path = \"ratio/") else {
            continue;
        };
        let stem = rest
            .strip_suffix(".rs\"]")
            .unwrap_or_else(|| panic!("a mount that is not `ratio/<file>.rs`: {line}"));
        let module = lines
            .iter()
            .skip(at + 1)
            .find(|next| !next.is_empty() && !next.starts_with("//") && !next.starts_with("#["))
            .and_then(|next| next.strip_prefix("mod "))
            .and_then(|next| next.strip_suffix(';'))
            .unwrap_or_else(|| panic!("`{line}` is not followed by a `mod <name>;` line"));
        out.push((stem.to_owned(), module.to_owned()));
    }
    out
}

/// The harness's code lines: comments and the mount lines themselves left
/// out, so a name in a doc comment or on its own `mod` line is no call.
fn code(source: &str) -> String {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && !line.starts_with("mod "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `code` names `module` as a path (`module::`), not as the tail of
/// a longer name (`served_routes::` is no use of `routes`).
fn names_module(code: &str, module: &str) -> bool {
    let needle = format!("{module}::");
    code.match_indices(&needle).any(|(at, _)| {
        code[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !(before.is_ascii_alphanumeric() || before == '_'))
    })
}

/// Every module file of every crate that has none of the three properties a
/// bench module needs, as one sentence each.
fn orphans(crates: &Path) -> Vec<String> {
    let mut faults = Vec::new();
    for (name, harness) in harnesses(crates) {
        let mounted = mounts(&harness.source);
        let body = code(&harness.source);
        let mut seen = BTreeSet::new();
        for (stem, module) in &mounted {
            if !harness.files.contains(stem) {
                faults.push(format!(
                    "{name}: `ratio/{stem}.rs` is mounted and is not a file"
                ));
            }
            if !seen.insert(stem.clone()) {
                faults.push(format!("{name}: `ratio/{stem}.rs` is mounted twice"));
            }
            if !names_module(&body, module) {
                faults.push(format!(
                    "{name}: module `{module}` (ratio/{stem}.rs) is mounted and the harness \
                     never calls into it"
                ));
            }
        }
        for stem in harness.files.difference(&seen) {
            faults.push(format!(
                "{name}: `benches/ratio/{stem}.rs` is mounted by no `#[path]` in \
                 benches/ratio.rs, so no build compiles it and no gate runs it"
            ));
        }
    }
    faults
}

/// CC-20's seven check-primitive benches (FM-615): crate, module file stem.
const CC20: [(&str, &str); 7] = [
    ("pull", "archive_from_source"),
    ("pull", "nfo_year"),
    ("pull", "census_load"),
    ("api", "nse_identity"),
    ("pull", "series_identity"),
    ("pull", "strike_implausible"),
    ("pull", "admit"),
];

#[test]
fn every_bench_module_is_registered() {
    let crates = workspace_root().join("crates");
    let faults = orphans(&crates);
    assert!(
        faults.is_empty(),
        "every file under crates/*/benches/ratio/ is a module of its crate's \
         gate-8 harness, mounted once and called: {faults:#?}"
    );
    let found = harnesses(&crates);
    for (krate, stem) in CC20 {
        let harness = found.get(krate).unwrap_or_else(|| {
            panic!("crates/{krate} carries no benches/ratio/ folder, so CC-20's `{stem}` bench is missing")
        });
        assert!(
            harness.files.contains(stem),
            "crates/{krate}/benches/ratio/{stem}.rs is missing: CC-20 (FM-615) owes one \
             bench per check primitive"
        );
        let module = mounts(&harness.source)
            .into_iter()
            .find(|(file, _)| file == stem)
            .map(|(_, module)| module)
            .unwrap_or_else(|| panic!("crates/{krate}: `ratio/{stem}.rs` is not mounted"));
        assert!(
            code(&harness.source).contains(&format!("ok &= {module}::")),
            "crates/{krate}: `{module}`'s result is not folded into the harness's verdict \
             with `ok &= {module}::`, so a breach of it would not fail gate 8"
        );
    }
}

#[test]
fn the_registration_rule_refuses_each_way_a_module_goes_missing() {
    // The rule above must be able to fail, or a green run says nothing.
    // Each case is a harness and its folder, invented, in a scratch tree.
    let root = std::env::temp_dir().join(format!(
        "brutex-bench-modules-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let case = |name: &str, harness: &str, files: &[&str]| -> Vec<String> {
        let crates = root.join(name).join("crates");
        let folder = crates.join("demo/benches/ratio");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(crates.join("demo/benches/ratio.rs"), harness).unwrap();
        for file in files {
            std::fs::write(folder.join(format!("{file}.rs")), "").unwrap();
        }
        orphans(&crates)
    };
    let good = "#[path = \"ratio/a.rs\"]\nmod a;\nfn main() { let mut ok = true; ok &= a::run(); }\n";
    assert!(case("good", good, &["a"]).is_empty());

    let unmounted = case("unmounted", good, &["a", "b"]);
    assert_eq!(unmounted.len(), 1, "{unmounted:?}");
    assert!(unmounted[0].contains("ratio/b.rs` is mounted by no"));

    let dangling = case("dangling", good, &[]);
    assert_eq!(dangling.len(), 1, "{dangling:?}");
    assert!(dangling[0].contains("is mounted and is not a file"));

    let uncalled = "#[path = \"ratio/a.rs\"]\nmod a;\n/// a::run is named in a comment only\nfn main() {}\n";
    let never = case("uncalled", uncalled, &["a"]);
    assert_eq!(never.len(), 1, "{never:?}");
    assert!(never[0].contains("never calls into it"));

    let twice = "#[path = \"ratio/a.rs\"]\nmod a;\n#[path = \"ratio/a.rs\"]\nmod b;\nfn main() { a::x(); b::x(); }\n";
    let doubled = case("twice", twice, &["a"]);
    assert_eq!(doubled.len(), 1, "{doubled:?}");
    assert!(doubled[0].contains("mounted twice"));

    // A longer name ending in the module's is no call of it.
    let tail = "#[path = \"ratio/routes.rs\"]\nmod routes;\nfn main() { served_routes::x(); }\n";
    let tailed = case("tail", tail, &["routes"]);
    assert_eq!(tailed.len(), 1, "{tailed:?}");
    assert!(tailed[0].contains("never calls into it"));

    // A doc comment between the mount and its `mod` line is skipped.
    let documented = "#[path = \"ratio/a.rs\"]\n/// the module\nmod a;\nfn main() { a::x(); }\n";
    assert!(case("documented", documented, &["a"]).is_empty());

    // The scratch tree is this test's own, under the system temp directory.
    let _ignored = std::fs::remove_dir_all(&root);
}

#[test]
fn the_walk_finds_the_harnesses_it_exists_for() {
    // A walk that found nothing passes the first test vacuously.
    let found = harnesses(&workspace_root().join("crates"));
    for krate in ["pull", "api", "store"] {
        let harness = found
            .get(krate)
            .unwrap_or_else(|| panic!("crates/{krate}/benches/ratio/ was not found"));
        assert!(
            !mounts(&harness.source).is_empty(),
            "crates/{krate}'s harness mounts no module: the mount parser stopped matching"
        );
    }
}

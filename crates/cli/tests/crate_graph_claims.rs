//! No source comment or lint reason in the workspace may deny an arrow
//! `CLAUDE.md` §5 draws for `cli` (R9-cli-law-1, D-1706; widened by G1-4,
//! D-4736).
//!
//! `cli` depends on `vocab` directly: `crates/cli/Cargo.toml` declares it and
//! §5 draws `... telemetry vocab <-- cli` (D-0683). Fifteen mask literals still
//! spelled `Default::default()` under `#[expect(clippy::default_trait_access)]`
//! whose reasons said the named path "would add a dependency arrow §5 does not
//! draw", that "`vocab` is not among `cli`'s dependencies", or that the mask
//! type "belongs to runner's private dependency graph". Each was false, and a
//! lint suppression kept for a false reason is the drift §5 warns about.
//!
//! D-1706 refused those exact sentences in the files directly under
//! `crates/cli/src`, and others said the same thing in other words: a comment
//! in `cli::append_condition_names` ("§5 does not give `cli` a `vocab` arrow"),
//! the doc of `runner::report::names_from_words` ("`vocab` is not among
//! them"), and the head of `crates/cli/Cargo.toml` ("NO ARROW TO `store`").
//! So this walks every `.rs` file and every `Cargo.toml` under `crates/`,
//! derives `cli`'s nine arrows from its own manifest, and refuses any comment
//! clause that names one of them beside a denial and beside `cli` (or, inside
//! `crates/cli`, with no crate named before the denial).
//!
//! Limits, stated: a denial written inside double quotes is read as a citation
//! of old text and skipped, and only whole-line `//` and `#` comment prose is
//! read by the general detector; the exact sentences are refused anywhere in a
//! file, string literals included.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::path::{Path, PathBuf};

/// The false sentences, each as it was written at 1087e544.
const FALSE_CLAIMS: [&str; 6] = [
    "the named path would add a dependency arrow §5 does not draw",
    "`vocab` is not among `cli`'s dependencies",
    "needs a `vocab` arrow that `CLAUDE.md` §5 does not",
    "needs a `vocab` arrow §5 does not draw for `cli`",
    "the named mask type belongs to runner's private dependency graph",
    "clippy::default_trait_access",
];

/// Words that deny an arrow, compared lowercase.
const DENIALS: [&str; 14] = [
    "not among",
    "no arrow",
    "does not give",
    "does not draw",
    "does not depend",
    "doesn't depend",
    "never depends",
    "has no `",
    "lacks",
    "is not a dependency",
    "not one of its",
    "no dependency on",
    "would add a dependency arrow",
    "would add an arrow",
];

/// What ends one clause and starts the next.
const BOUNDARIES: [&str; 11] = [
    ". ", "; ", "? ", "! ", ": ", " -- ", " — ", ", so ", ", but ", " (", ") ",
];

/// The repository root: `crates/cli` is two levels below it.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cli is two levels below the repository root")
        .to_owned()
}

/// Every `.rs` file and every `Cargo.toml` under `dir`, recursively.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|why| panic!("{}: {why}", dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs")
            || path.file_name().is_some_and(|n| n == "Cargo.toml")
        {
            out.push(path);
        }
    }
}

/// The crates `cli` depends on, as the `path = "../X"` keys of its
/// `[dependencies]` table name their directories.
fn cli_arrows() -> Vec<String> {
    let manifest = include_str!("../Cargo.toml");
    let deps = manifest
        .split("\n[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    deps.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split("path = \"../").nth(1))
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_owned)
        .collect()
}

/// The comment prose of a file: each whole-line comment's marker removed,
/// consecutive comment lines joined, any other line ending the block, `**`
/// dropped and every double-quoted span removed as a citation.
fn comment_prose(text: &str, toml: bool) -> String {
    let mut prose = String::with_capacity(text.len() / 2);
    for line in text.lines() {
        let line = line.trim_start();
        let comment = if toml {
            line.strip_prefix('#')
        } else {
            line.strip_prefix("//")
                .map(|rest| rest.trim_start_matches(['/', '!']))
        };
        if let Some(words) = comment {
            prose.push(' ');
            prose.push_str(words.trim());
        } else {
            prose.push_str(" . ");
        }
    }
    let mut kept = String::with_capacity(prose.len());
    let mut quoted = false;
    for c in prose.replace("**", "").chars() {
        if c == '"' {
            quoted = !quoted;
        } else if !quoted {
            kept.push(c);
        }
    }
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Where `clause` first names crate `name`, in any spelling comments use.
fn first_mention(clause: &str, name: &str) -> Option<usize> {
    let mut forms = vec![
        format!("`{name}`"),
        format!("`{name}::"),
        format!("`crates/{name}"),
    ];
    if name == "core" {
        forms.push("`brutex_core".to_owned());
    }
    forms
        .iter()
        .filter_map(|form| clause.find(form.as_str()))
        .min()
}

/// Each clause of `prose` that denies `cli` one of `arrows`. `crates` names
/// every workspace crate. `in_cli` says the file is `cli`'s own, where a
/// denial with no crate named before it has `cli` as its subject.
fn denials(prose: &str, in_cli: bool, arrows: &[String], crates: &[String]) -> Vec<String> {
    let mut clauses = vec![prose.to_owned()];
    for boundary in BOUNDARIES {
        clauses = clauses
            .iter()
            .flat_map(|clause| clause.split(boundary).map(str::to_owned))
            .collect();
    }
    let mut found = Vec::new();
    for clause in clauses {
        let lower = clause.to_lowercase();
        let Some(denial) = DENIALS.iter().filter_map(|d| lower.find(d)).min() else {
            continue;
        };
        if !arrows
            .iter()
            .any(|arrow| first_mention(&clause, arrow).is_some())
        {
            continue;
        }
        let before: Vec<&String> = crates
            .iter()
            .filter(|name| first_mention(&clause, name).is_some_and(|at| at < denial))
            .collect();
        let about_cli = if first_mention(&clause, "cli").is_some() {
            before
                .iter()
                .all(|name| *name == "cli" || arrows.contains(name))
        } else {
            in_cli && before.is_empty()
        };
        if about_cli {
            found.push(clause);
        }
    }
    found
}

/// The thirteen workspace crates, as `crates/` names their directories.
fn workspace_crates() -> Vec<String> {
    let mut crates: Vec<String> = std::fs::read_dir(repo().join("crates"))
        .expect("crates/ is readable")
        .map(|entry| entry.expect("a directory entry").file_name())
        .filter_map(|name| name.to_str().map(str::to_owned))
        .collect();
    crates.sort();
    crates
}

/// The detector flags the sentences that were false and passes the ones that
/// are true, so the walk below cannot pass by detecting nothing.
#[test]
fn the_arrow_denial_detector_tells_a_false_sentence_from_a_true_one() {
    let arrows = cli_arrows();
    let crates = workspace_crates();
    let flagged = |text: &str, in_cli: bool, toml: bool| {
        denials(&comment_prose(text, toml), in_cli, &arrows, &crates).len()
    };
    for false_sentence in [
        "// see its own\n// comment -- `CLAUDE.md` §5 does not give `cli` a `vocab` arrow.\n",
        "/// `CLAUDE.md` §5\n/// lists `cli`'s arrows and `vocab` is **not among them** -- adding\n",
        "//! `cli` does not depend on `pull`.\n",
        "// `telemetry` is not a dependency of `cli`.\n",
        "/// `cli` has no `costs` arrow.\n",
        "/// `crates/cli` never depends on `brutex_core` itself.\n",
    ] {
        assert_eq!(flagged(false_sentence, false, false), 1, "{false_sentence}");
    }
    assert_eq!(
        flagged(
            "# NO ARROW TO `store`, AND THAT IS THE POINT. The rule\n",
            true,
            true
        ),
        1,
        "inside cli's own manifest the subject is cli"
    );
    assert_eq!(
        flagged(
            "# NO ARROW TO `store`, AND THAT IS THE POINT. The rule\n",
            false,
            true
        ),
        0,
        "outside cli's own files a subjectless denial is about some other crate"
    );
    for true_sentence in [
        "// `CLAUDE.md` §5 says `cli` is what makes the sweep reachable at all — `api` does \
         not depend on `runner`, `engine` or `indicators`, so the binary\n",
        "/// Every crate that takes a lock — `store`, `pull`, `api` and `cli` — already \
         depends on this one, so the guard adds no arrow to the crate graph.\n",
        "// `cli` does hold the `vocab` arrow (D-0683); the call is not a workaround.\n",
        "// `runner` does not depend on `store`.\n",
        "// `cli` does not depend on `greeks`.\n",
    ] {
        assert_eq!(flagged(true_sentence, false, false), 0, "{true_sentence}");
    }
    for in_cli_history in [
        "# `crates/api` has `store` and no `runner`; `crates/cli` had `runner` and no\n# `store`.\n",
        "# That entry read: \"NO ARROW TO `store`: the operator's standing rule\n# forbids\"\n",
    ] {
        assert_eq!(flagged(in_cli_history, true, true), 0, "{in_cli_history}");
    }
    assert_eq!(
        flagged("let x = 1; // `cli` lacks `vocab`\n", false, false),
        0,
        "a trailing comment on a code line is not read"
    );
}

/// No workspace source repeats a sentence denying an arrow `cli` has, and no
/// mask literal keeps the lint suppression the oldest of them justified.
#[test]
fn no_cli_source_denies_the_vocab_arrow_section_5_draws() {
    let repo = repo();
    let arrows = cli_arrows();
    assert_eq!(
        arrows.len(),
        9,
        "premise: CLAUDE.md §5 draws nine arrows for cli: {arrows:?}"
    );
    assert!(arrows.iter().any(|a| a == "vocab") && arrows.iter().any(|a| a == "store"));
    let crates = workspace_crates();
    assert_eq!(crates.len(), 13, "premise: thirteen crates: {crates:?}");
    let mut files = Vec::new();
    sources(&repo.join("crates"), &mut files);
    assert!(
        files.len() > 300
            && files
                .iter()
                .any(|p| p.ends_with("crates/runner/src/report.rs"))
            && files.iter().any(|p| p.ends_with("crates/cli/Cargo.toml"))
            && files.iter().any(|p| p.ends_with("crates/cli/src/lib.rs")),
        "premise: the walk reached every crate ({} files)",
        files.len()
    );
    let this_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/crate_graph_claims.rs");
    let cli_dir = repo.join("crates/cli");
    let mut false_claims = Vec::new();
    for path in files.iter().filter(|path| **path != this_file) {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|why| panic!("{} is UTF-8: {why}", path.display()));
        for claim in FALSE_CLAIMS {
            // The lint is refused only in `cli`, whose mask literals it covered:
            // `api` truly has no `indicators` or `runner` arrow, so its reasons
            // for the same suppression are true.
            let scoped = claim == "clippy::default_trait_access" && !path.starts_with(&cli_dir);
            if !scoped && text.contains(claim) {
                false_claims.push(format!("{}: {claim:?}", path.display()));
            }
        }
        let toml = path.extension().is_some_and(|e| e == "toml");
        for clause in denials(
            &comment_prose(&text, toml),
            path.starts_with(&cli_dir),
            &arrows,
            &crates,
        ) {
            false_claims.push(format!("{}: {clause:?}", path.display()));
        }
    }
    assert!(
        false_claims.is_empty(),
        "{} sentence(s) deny an arrow cli has; cli depends on {arrows:?} (D-0683, \
         D-1706, D-4736):\n{}",
        false_claims.len(),
        false_claims.join("\n")
    );
}

/// The manifest fact the sentences denied, checked rather than asserted.
#[test]
fn cli_declares_vocab_as_a_direct_dependency() {
    let manifest = include_str!("../Cargo.toml");
    let deps = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    assert!(
        deps.lines()
            .any(|line| line.trim_start().starts_with("vocab ")
                || line.trim_start().starts_with("vocab=")),
        "crates/cli/Cargo.toml no longer declares vocab: {deps}"
    );
}

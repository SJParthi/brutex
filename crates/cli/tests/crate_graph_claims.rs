//! No source comment or lint reason in `cli` may deny an arrow `CLAUDE.md` §5
//! draws (R9-cli-law-1, D-1706).
//!
//! `cli` depends on `vocab` directly: `crates/cli/Cargo.toml` declares it and
//! §5 draws `... telemetry vocab <-- cli` (D-0683). Fifteen mask literals still
//! spelled `Default::default()` under `#[expect(clippy::default_trait_access)]`
//! whose reasons said the named path "would add a dependency arrow §5 does not
//! draw", that "`vocab` is not among `cli`'s dependencies", or that the mask
//! type "belongs to runner's private dependency graph". Each was false, and a
//! lint suppression kept for a false reason is the drift §5 warns about.
//!
//! This walks every `.rs` file under `crates/cli/src` at test time, so a new
//! file is covered without being listed, and refuses each false sentence.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::path::Path;

/// The false sentences, each as it was written at 1087e544.
const FALSE_CLAIMS: [&str; 6] = [
    "the named path would add a dependency arrow §5 does not draw",
    "`vocab` is not among `cli`'s dependencies",
    "needs a `vocab` arrow that `CLAUDE.md` §5 does not",
    "needs a `vocab` arrow §5 does not draw for `cli`",
    "the named mask type belongs to runner's private dependency graph",
    "clippy::default_trait_access",
];

/// Every `.rs` file directly under `crates/cli/src`, with its text.
fn sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("crates/cli/src is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("a source file is UTF-8");
            out.push((path.display().to_string(), text));
        }
    }
    out
}

/// No `cli` source repeats a sentence denying the `vocab` arrow, and no mask
/// literal keeps the lint suppression that sentence justified.
#[test]
fn no_cli_source_denies_the_vocab_arrow_section_5_draws() {
    let files = sources();
    // A walk that found nothing would pass vacuously; lib.rs must be there.
    assert!(
        files.iter().any(|(path, _)| path.ends_with("lib.rs")),
        "the walk did not reach crates/cli/src/lib.rs"
    );
    for (path, text) in &files {
        for claim in FALSE_CLAIMS {
            assert!(
                !text.contains(claim),
                "{path} still says {claim:?}; cli depends on vocab directly (D-0683, D-1706)"
            );
        }
    }
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

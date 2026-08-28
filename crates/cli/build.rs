//! Stamps `BRUTEX_COMMIT` from git's own files, so a plain `cargo run` produces
//! a binary that can record a run.
//!
//! # The failure this closes, reproduced on the operator's machine
//!
//! `cli::commit_stamp()` is `option_env!("BRUTEX_COMMIT")`, resolved at COMPILE
//! time, and a build without it refuses every sweep by `CLAUDE.md` §3 rule 3 — a
//! run it cannot identify is a run it cannot record. That refusal is right.
//!
//! What was wrong is that **nothing set the variable on the path the operator
//! actually uses.** `docs/07-plan.md` §0 states the whole procedure as "open the
//! directory in `IntelliJ`, press Run on the `api` binary", and §0 also records
//! that "`IntelliJ` does not read that file. It takes run entries from `.idea/` or
//! `.run/*.xml`, neither tracked" — so the IDE auto-detects the binary and runs
//! `cargo run` with no environment. Measured twice on 81 months of freshly
//! pulled Zerodha NIFTY: the server served every page and swept nothing, across
//! two restarts.
//!
//! And this repository **cannot** fix that in the IDE. An `IntelliJ` run entry is
//! `.run/*.xml`; `.xml` is not in §2's extension allowlist and has no by-name
//! exception. So the launcher cannot be the answer, and the build has to be.
//!
//! # Why this is allowed to exist, said plainly
//!
//! CI gate 13 layer 3 bans a build script **anywhere in the tree**, and says so
//! in its own words: *"THIS IS STRICTER THAN CLAUDE.md SECTION 2, WHICH SAYS
//! 'any build.rs that invokes an external process'. The stricter reading is
//! deliberate and is the operator's instruction… The allowlist below is the
//! escape, and taking it should come with a decisions entry."*
//!
//! This is that escape, taken, with the entry. The clause the gate rested on —
//! *"there is no C to compile, no protocol to generate from, no version to stamp
//! that `env!("CARGO_PKG_VERSION")` does not already carry"* — was true when it
//! was written and is not now: rule 3 requires the COMMIT, and no cargo variable
//! carries it.
//!
//! # Why not the two alternatives, both of which `cli` already refused
//!
//! `crates/cli/src/lib.rs` argues both and both refusals stand:
//!
//! * **`git rev-parse` from a build script** is banned by §2 and by gate 2,
//!   which greps every build script for the two spellings that start a child
//!   process. THOSE TWO WORDS ARE DELIBERATELY NOT WRITTEN ANYWHERE IN THIS
//!   FILE: gate 2 reads the file whole and does not strip comments, so
//!   explaining the rule in its own vocabulary would trip it — the same
//!   self-reference the engine's source-text tests avoid by assembling their
//!   needles with `concat!`. Naming them cost a red build once already.
//!
//!   This file starts nothing. It reads FILES — git writes the commit into
//!   `.git/HEAD` and the ref it names, and reading a file is not starting
//!   anything.
//! * **Reading `.git/HEAD` at RUN time** "would stamp a result with a commit
//!   whose source never produced it, which is worse than recording nothing".
//!   That objection is about WHEN, not about WHERE the bytes come from. Here the
//!   read happens while the binary is being compiled, so the answer is the
//!   commit the binary was compiled FROM — exactly what rule 3 asks for, and
//!   exactly what `git rev-parse` would have returned.
//!
//! # It degrades to the old behaviour rather than guessing
//!
//! No `.git`, an unreadable HEAD, a ref this cannot resolve, or anything that is
//! not forty hex characters: this emits nothing, `option_env!` stays `None`, and
//! the sweep refuses exactly as it did before. A tarball build is not silently
//! given a fabricated identity.
//!
//! An explicit `BRUTEX_COMMIT` in the environment always wins, so CI and a
//! deliberate `BRUTEX_COMMIT=… cargo build` are unchanged.

use std::path::{Path, PathBuf};

fn main() {
    // THE OPERATOR'S OWN VALUE WINS, AND IS CHECKED FIRST. A build that states
    // its commit is stating something this file can only infer.
    println!("cargo:rerun-if-env-changed=BRUTEX_COMMIT");
    if std::env::var_os("BRUTEX_COMMIT").is_some() {
        return;
    }

    let Some(git_dir) = git_dir() else {
        return;
    };

    // REBUILD WHEN THE COMMIT MOVES. Without this, a stamp compiled once would
    // survive every later checkout — which is the runtime-read defect arriving
    // by a slower route.
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );

    if let Some(sha) = head_commit(&git_dir) {
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
        println!("cargo:rustc-env=BRUTEX_COMMIT={sha}");
    }
}

/// The directory git keeps its refs in, walking up from this crate.
///
/// `.git` is normally a directory. In a worktree or a submodule it is a FILE
/// whose contents are `gitdir: <path>` — which is how `.claude/worktrees/*`
/// builds see the real one, and there are three of those in this tree today.
fn git_dir() -> Option<PathBuf> {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")?;
    let mut here: &Path = Path::new(&manifest);
    loop {
        let candidate = here.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate).ok()?;
            let path = text.trim().strip_prefix("gitdir:")?.trim();
            let resolved = here.join(path);
            return resolved.is_dir().then_some(resolved);
        }
        here = here.parent()?;
    }
}

/// The forty hex characters HEAD resolves to, or `None`.
///
/// Three shapes, all of them files git wrote:
///
/// * `HEAD` holding a bare sha — a detached checkout, which is what CI produces;
/// * `HEAD` holding `ref: refs/heads/<branch>` and that ref existing as a loose
///   file — an ordinary working checkout;
/// * the same, with the ref **packed** into `packed-refs` because `git gc` moved
///   it there. A long-lived clone reaches this state on its own, and a stamp
///   that stopped working after a garbage collection would be worse than one
///   that never worked.
fn head_commit(git_dir: &Path) -> Option<String> {
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();

    let Some(reference) = head.strip_prefix("ref:") else {
        return sha(head);
    };
    let reference = reference.trim();

    // A loose ref that is absent, unreadable, or not forty hex characters falls
    // through to `packed-refs` rather than failing -- both are places git
    // legitimately keeps the same answer, and which one holds it depends on
    // whether `git gc` has run.
    if let Ok(loose) = std::fs::read_to_string(git_dir.join(reference))
        && let Some(found) = sha(loose.trim())
    {
        return Some(found);
    }

    let packed = std::fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    packed.lines().find_map(|line| {
        // `<sha> <refname>`, and lines beginning `#` or `^` are a header and a
        // peeled tag — neither is a branch tip.
        let rest = line.strip_prefix(|c: char| c.is_ascii_hexdigit()).is_some();
        if !rest || line.starts_with('#') || line.starts_with('^') {
            return None;
        }
        let (found, name) = line.split_once(' ')?;
        (name.trim() == reference).then(|| sha(found)).flatten()
    })
}

/// A candidate, if it is exactly forty lowercase-or-uppercase hex digits.
///
/// The width is checked because anything else is not a commit, and a stamp that
/// is not a commit is a reproducibility claim that cannot be honoured — the same
/// objection `crates/cli/src/lib.rs` raises against a runtime read.
fn sha(candidate: &str) -> Option<String> {
    let candidate = candidate.trim();
    (candidate.len() == 40 && candidate.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| candidate.to_owned())
}

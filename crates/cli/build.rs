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
//! not forty lowercase hex characters: this emits an empty refusal sentinel,
//! `cli::commit_stamp` converts it to `None`, and the sweep refuses. A tarball
//! build is not silently given a fabricated identity.
//!
//! An explicit `BRUTEX_COMMIT` is an assertion, not authority. It is accepted
//! only when it is canonical, equals locally resolvable HEAD, and the index and
//! working tree both equal that commit. A dirty build is deliberately
//! unstamped: naming HEAD would claim that HEAD produced bytes it did not.

mod build_provenance;
mod commit_stamp;

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=BRUTEX_COMMIT");
    let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        println!("cargo:rustc-env=BRUTEX_COMMIT=");
        return;
    };
    let explicit = std::env::var_os("BRUTEX_COMMIT");
    let verified = build_provenance::verify(Path::new(&manifest), explicit.as_deref());
    for path in &verified.watched_files {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    for path in &verified.watched_directories {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    if let Some(commit) = verified.commit {
        println!("cargo:rustc-env=BRUTEX_COMMIT={commit}");
    } else {
        // This OVERRIDES an inherited explicit value. Merely declining to emit
        // `rustc-env` would leave that ambient value visible to `option_env!`
        // and silently bless the assertion we just rejected.
        println!("cargo:rustc-env=BRUTEX_COMMIT=");
        println!(
            "cargo:warning=cli run persistence disabled: {}",
            verified.reason
        );
    }
}

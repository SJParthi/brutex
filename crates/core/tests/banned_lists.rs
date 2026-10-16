//! The three lists that ban a foreign runtime, read from where they live and held
//! together. D-1108.
//!
//! CI gate 13's `banned` (in `.github/workflows/ci.yml`), the `FORBIDDEN` list in
//! `crates/vocab/tests/workspace_is_rust.rs` and `deny.toml`'s `deny` were written
//! apart and disagreed: gate 13, which runs first, missed seven runtimes the other
//! two named or should have, the vocab list missed eleven gate 13 named, and one
//! vocab entry named a crate that does not exist. Three lists for one fact is the
//! shape `CLAUDE.md` §5 refuses for the vocabulary; this test is the same rule.
//!
//! It lives in `core` because `core` is not a sweep crate: gate 22 clause D refuses a
//! sweep crate's compile-time include of anything but a document or a source file,
//! and the workflow is neither.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeSet;

const WORKFLOW: &str = include_str!("../../../.github/workflows/ci.yml");
const DENY: &str = include_str!("../../../deny.toml");
const VOCAB_TEST: &str = include_str!("../../vocab/tests/workspace_is_rust.rs");

/// Entries of `FORBIDDEN` that bind a C library rather than embed a runtime. Gate
/// 13's title is "no foreign runtime", so its list omits them.
const C_BINDINGS: &[&str] = &["openssl-sys"];

/// Entries of `deny.toml`'s `deny` that are C and assembly, not a runtime (D-0211).
const DENIED_NATIVE: &[&str] = &["ring", "aws-lc-rs", "aws-lc-sys", "cc"];

/// The banned word, which gate 15 refuses in every tracked file.
const WORD: &str = concat!("py", "thon");

/// The double-quoted literals on one line, joined: `"a"` is `a`, and
/// `concat!("a", "b")` is `ab`.
fn joined_literals(line: &str) -> String {
    line.split('"').skip(1).step_by(2).collect()
}

/// The names in `FORBIDDEN`, read from the vocab test's source.
fn forbidden() -> BTreeSet<String> {
    let body = VOCAB_TEST
        .split_once("const FORBIDDEN: &[&str] = &[")
        .and_then(|(_, rest)| rest.split_once("\n];"))
        .map(|(body, _)| body)
        .expect("the vocab test declares FORBIDDEN");
    body.lines()
        .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
        .map(joined_literals)
        .filter(|n| !n.is_empty())
        .collect()
}

/// The names gate 13 bans, read from the workflow, with the shell's spelling of the
/// banned word expanded.
fn gate_13_banned() -> BTreeSet<String> {
    workflow_list("      - name: Gate 13 ", "banned")
}

/// The names gate 13b refuses in the host build graph. D-1111.
fn gate_13b_banned() -> BTreeSet<String> {
    workflow_list("      - name: Gate 13b ", "banned_built")
}

/// The whitespace-separated `VARIABLE="..."` list in the named workflow step.
fn workflow_list(step_header: &str, variable: &str) -> BTreeSet<String> {
    let step = WORKFLOW
        .split(step_header)
        .nth(1)
        .expect("the workflow has the step");
    let list = step
        .split_once(&format!("\n          {variable}=\""))
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(list, _)| list)
        .expect("the step declares its list as VARIABLE=\"...\"");
    list.split_whitespace()
        .map(|n| n.replace("${snake}", WORD))
        .collect()
}

/// The names `deny.toml` denies, read from its `deny = [ ... ]` array.
fn deny_toml_denied() -> BTreeSet<String> {
    let array = DENY
        .split_once("\ndeny = [")
        .and_then(|(_, rest)| rest.split_once("\n]"))
        .map(|(array, _)| array)
        .expect("deny.toml has a deny = [ ... ] array");
    array
        .lines()
        .map(|l| l.split_once('#').map_or(l, |(code, _)| code))
        .filter_map(|l| {
            let (_, rest) = l.split_once("name = \"")?;
            rest.split_once('"').map(|(n, _)| n.to_owned())
        })
        .collect()
}

/// Gate 13 bans exactly `FORBIDDEN` less [`C_BINDINGS`]; gate 13b bans all of
/// `FORBIDDEN` and [`DENIED_NATIVE`] in the host build graph. `deny.toml` denies every
/// entry of `FORBIDDEN` except those that spell the banned word, which gate 15
/// refuses in a `.toml` and which gate 13 and the vocab test hold instead, and
/// beyond them only [`DENIED_NATIVE`].
#[test]
fn the_three_banned_lists_agree() {
    let forbidden = forbidden();
    assert!(
        forbidden.len() > 20 && forbidden.contains("pyo3"),
        "FORBIDDEN was not read: {forbidden:?}"
    );
    let runtimes: BTreeSet<String> = forbidden
        .iter()
        .filter(|n| !C_BINDINGS.contains(&n.as_str()))
        .cloned()
        .collect();
    assert_eq!(
        gate_13_banned(),
        runtimes,
        "gate 13's list and FORBIDDEN differ"
    );

    // Gate 13b refuses, in what cargo BUILDS, every runtime and every native crate.
    let built: BTreeSet<String> = forbidden
        .iter()
        .cloned()
        .chain(DENIED_NATIVE.iter().map(|n| (*n).to_owned()))
        .collect();
    assert_eq!(
        gate_13b_banned(),
        built,
        "gate 13b's list differs from FORBIDDEN and the denied native crates"
    );

    let want: BTreeSet<String> = forbidden
        .iter()
        .filter(|n| !n.contains(WORD))
        .cloned()
        .chain(DENIED_NATIVE.iter().map(|n| (*n).to_owned()))
        .collect();
    assert_eq!(
        deny_toml_denied(),
        want,
        "deny.toml's deny list and FORBIDDEN differ"
    );
}

/// The readers above see what they are for, and a list that loses an entry differs.
#[test]
fn the_readers_bite() {
    assert_eq!(joined_literals("    concat!(\"c\", \"py\"),"), "cpy");
    assert_eq!(joined_literals("    \"napi\","), "napi");
    assert_eq!(joined_literals("];"), "");
    assert!(gate_13_banned().contains(&format!("c{WORD}")));
    assert!(deny_toml_denied().contains("rhai"));
    assert!(!deny_toml_denied().contains("unknown-registry"));
}

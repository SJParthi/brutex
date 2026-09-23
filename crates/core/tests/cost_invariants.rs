//! `docs/04-invariants.md`'s cost rows, reconciled against the benches that
//! prove them — **discovered at runtime, nothing listed by hand.**
//!
//! # Why this file exists
//!
//! Seven defects were found and fixed in one session and every one was the same
//! shape: **a written claim that no longer matched the code.** A page that
//! tested one field of a contract carrying three. A gate sitting deeper than the
//! fact it checked. A reader walking one directory while its own doc said two.
//! And twice, a cost invariant whose row described a gap the bench beside it had
//! already closed.
//!
//! `docs/05-decisions.md` D-0102 named the rule that closes this class: *where a
//! document states something the code also knows, a test reads the document and
//! compares.* `graph.rs` is that rule applied to the crate graph. **It was never
//! applied to the cost rows**, and D-0298 is what that cost — five ids declared
//! with no bench and three benched with no declaration, found by hand because
//! nothing looked.
//!
//! # Everything here is discovered, not declared
//!
//! `graph.rs` names every manifest by hand because `include_str!` takes a
//! literal. A test does not have to: it runs, so it can WALK. This walks
//! `crates/` for `benches/ratio.rs` and reads the document, both at runtime, so
//! **a crate added tomorrow is covered without editing this file** — which is
//! the same failure mode one layer up, since a hand-written list is exactly the
//! kind of written claim that stops matching the tree.
//!
//! # And the exceptions are derived, not listed
//!
//! Four rows have no bench of their own and are correct: `C-02` and `C-04` were
//! superseded, `C-03`'s gap was closed under a new id, and `C-E-02b` is a
//! STRUCTURAL claim no ratio can prove. An allowlist naming those four would be
//! the very thing this file refuses — a hand-kept statement that rots the day a
//! row is renumbered.
//!
//! So the rule is a reachability one instead: **a row is proven if a bench
//! prints its id, or if it names another cost id that is proven.** Each of the
//! four already names its replacement in its own text, because a row that says
//! *"superseded by `C-E-11`"* has to say which — and `C-02` reaches a bench in
//! two hops, through `C-E-02b` to `C-E-09`. Nothing is excused; the chain is
//! either there or the row fails.
//!
//! # What it deliberately does not check
//!
//! The FIGURES. A row claiming 1.117× against a bench measuring 1.687× passes
//! here, because a number lives in a run and this is a static read — gate 8 is
//! what fails on a real breach, and duplicating its job against transcribed
//! prose would be a second answer that can disagree with the first. This checks
//! that a claim and a measurement exist for one another, which is what was
//! missing.
//!
//! # Why it lives in `core`
//!
//! Same reason `graph.rs` does: `core` is the root of the graph and depends on
//! nothing, so a test here cannot perturb what it measures.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The workspace root, derived from this crate's own manifest directory.
///
/// `CARGO_MANIFEST_DIR` is `<root>/crates/core`, so two parents up is the root
/// wherever the tree is checked out — no absolute path, and nothing that breaks
/// on another machine or in CI.
fn workspace_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// The invariants document, read at runtime.
fn invariants() -> String {
    let path = workspace_root().join("docs/04-invariants.md");
    std::fs::read_to_string(&path).unwrap_or_else(|why| {
        panic!(
            "docs/04-invariants.md must be readable at {}: {why}",
            path.display()
        )
    })
}

/// Every `crates/<name>/benches/ratio.rs` in the tree, discovered by walking.
///
/// **Walked, never listed.** A crate that gains a bench is covered the moment it
/// lands. A crate that loses one is caught by
/// [`every_crate_carries_a_ratio_bench`], which is gate 8's own premise checked
/// where it can be read.
fn bench_sources() -> BTreeMap<String, String> {
    let crates_dir = workspace_root().join("crates");
    let entries = std::fs::read_dir(&crates_dir).unwrap_or_else(|why| {
        panic!(
            "crates/ must be readable at {}: {why}",
            crates_dir.display()
        )
    });

    let mut out = BTreeMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let bench = path.join("benches/ratio.rs");
        if let Ok(source) = std::fs::read_to_string(&bench) {
            out.insert(name.to_owned(), source);
        }
    }
    assert!(
        !out.is_empty(),
        "no crate under {} carries benches/ratio.rs — either the walk is wrong \
         or gate 8 has nothing to run, and both are failures",
        crates_dir.display()
    );
    out
}

/// Whether a token is a cost-invariant id: `C-`, optional letters, digits,
/// optional trailing sub-row letter.
///
/// Hand-written rather than a regex because this crate depends on nothing and
/// `CLAUDE.md` §5 makes that arrow load-bearing — gate 9 fails on any dependency
/// here at all.
fn is_cost_id(token: &str) -> bool {
    let Some(rest) = token.strip_prefix("C-") else {
        return false;
    };
    let mut saw_digit = false;
    for byte in rest.bytes() {
        match byte {
            b'0'..=b'9' => saw_digit = true,
            b'a'..=b'z' if saw_digit => {}
            b'A'..=b'Z' | b'-' if !saw_digit => {}
            _ => return false,
        }
    }
    saw_digit
}

/// Every cost id appearing anywhere in a piece of text.
///
/// Punctuation-tolerant: the document writes ``C-E-11`,`` and `C-04.` and
/// `(C-E-09)`, and an id that only counted when bare would miss most citations —
/// which would turn the reachability check below into a much weaker one without
/// anybody noticing.
fn ids_in(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|token| is_cost_id(token))
        .map(str::to_owned)
        .collect()
}

/// Every id declared as a table ROW, mapped to the other ids its row cites.
///
/// A row, not a mention. The document refers to ids in prose constantly — one
/// row cites another, a paragraph explains a retirement — and treating a mention
/// as a declaration would let the check pass on a document that names an id and
/// claims nothing about it.
fn declared_rows(doc: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for line in doc.lines() {
        let Some(rest) = line.strip_prefix("| ") else {
            continue;
        };
        let Some(id) = rest.split(" |").next().map(str::trim) else {
            continue;
        };
        if !is_cost_id(id) {
            continue;
        }
        let mut cites = ids_in(rest);
        cites.remove(id);
        out.insert(id.to_owned(), cites);
    }
    out
}

/// Every id a bench prints, taken from the string literals it prints them in.
fn printed_ids(benches: &BTreeMap<String, String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for source in benches.values() {
        // Odd-indexed chunks of a split on `"` are the string literals. The id
        // is the first token of a printed label, so the head of the literal is
        // what makes `"C-E-10 dedup HIT: 1,000 -> 10,000 seen"` yield `C-E-10`
        // and not every number in it.
        for literal in source.split('"').skip(1).step_by(2) {
            if let Some(head) = literal.split_whitespace().next().filter(|h| is_cost_id(h)) {
                out.insert(head.to_owned());
            }
        }
    }
    out
}

/// Ids that reach a bench, directly or through the rows they cite.
///
/// A fixed-point walk: start from what a bench prints, then repeatedly admit any
/// row citing something already admitted, until nothing new is admitted.
/// `C-02` reaches a bench in two hops — through `C-E-02b` to `C-E-09` — and a
/// one-hop rule would fail it, so the closure is taken rather than a single
/// step.
///
/// Terminates because the admitted set only grows and is bounded by the number
/// of rows.
fn proven(
    rows: &BTreeMap<String, BTreeSet<String>>,
    printed: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut reached: BTreeSet<String> = printed.clone();
    loop {
        let mut grew = false;
        for (id, cites) in rows {
            if reached.contains(id) {
                continue;
            }
            if cites.iter().any(|cited| reached.contains(cited)) {
                reached.insert(id.clone());
                grew = true;
            }
        }
        if !grew {
            return reached;
        }
    }
}

#[test]
fn every_measured_cost_invariant_is_declared() {
    // THE DIRECTION THAT WAS ACTUALLY BROKEN. `C-I-05`, `C-I-06` and `C-V-05`
    // ran and passed and were declared nowhere, so deleting any one of their
    // assertions would have deleted the measurement with every static check
    // still green. `C-I-05`'s own doc said exactly that — "JUDGED HERE, PINNED
    // NOWHERE ELSE" — and nothing acted on it until D-0298.
    let doc = invariants();
    let rows = declared_rows(&doc);
    let undeclared: Vec<String> = printed_ids(&bench_sources())
        .into_iter()
        .filter(|id| !rows.contains_key(id))
        .collect();

    assert!(
        undeclared.is_empty(),
        "these ids are MEASURED by a bench and declared by no row in \
         docs/04-invariants.md, so nothing states what they prove and deleting \
         the assertion would leave every static check green: {undeclared:?}"
    );
}

#[test]
fn every_declared_cost_invariant_reaches_a_bench() {
    // THE OTHER DIRECTION. A row claiming a bound with no measurement behind it
    // is a promise nothing keeps — and gate 8 cannot see it, because gate 8 runs
    // the benches that EXIST.
    //
    // No allowlist: a row without its own bench passes only by NAMING the id
    // that measures it, which every such row already does because "superseded
    // by" has to say by what.
    let doc = invariants();
    let rows = declared_rows(&doc);
    let printed = printed_ids(&bench_sources());
    let reached = proven(&rows, &printed);

    let unproven: Vec<&String> = rows.keys().filter(|id| !reached.contains(*id)).collect();

    assert!(
        unproven.is_empty(),
        "these rows CLAIM a cost bound and neither a bench prints their id nor \
         does their text name another cost id that is proven. Either write the \
         bench, or say in the row which id measures it: {unproven:?}"
    );
}

#[test]
fn the_reconciliation_is_reading_something() {
    // A WALK THAT FOUND NOTHING PASSES EVERY ASSERTION ABOVE. Both checks are
    // satisfied by an empty document and an empty bench set, which is the shape
    // `CLAUDE.md` §4 bans and which this repository has already been bitten by:
    // gate 8 once "tested for a benches directory at the repository root, found
    // none, and exited zero".
    let doc = invariants();
    let rows = declared_rows(&doc);
    let benches = bench_sources();
    let printed = printed_ids(&benches);

    assert!(
        rows.len() > 50,
        "docs/04-invariants.md declares {} cost row(s). The document held 78 \
         when this was written; a number this low means the row parser stopped \
         matching the table, not that the rows were deleted",
        rows.len()
    );
    assert!(
        printed.len() > 50,
        "the benches print {} distinct cost id(s) across {} crate(s). Too few \
         means the literal parser stopped matching, and both checks above would \
         then pass vacuously",
        printed.len(),
        benches.len()
    );
}

#[test]
fn every_crate_carries_a_ratio_bench() {
    // GATE 8'S OWN PREMISE, CHECKED WHERE IT CAN BE READ. Its step refuses a
    // workspace where NO crate carries a bench; it cannot see a single crate
    // that quietly lost one, and that crate's cost claims would then be
    // unmeasured while the gate stayed green.
    let root = workspace_root();
    let members: BTreeSet<String> = std::fs::read_dir(root.join("crates"))
        .expect("crates/ is readable")
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect();

    let benched: BTreeSet<String> = bench_sources().into_keys().collect();
    let bare: Vec<&String> = members.difference(&benched).collect();

    assert!(
        bare.is_empty(),
        "these crates have a manifest and no benches/ratio.rs, so nothing \
         measures whether their per-operation cost grows: {bare:?}"
    );
}

#[test]
fn the_document_declares_the_five_operations_rule_4_names() {
    // `CLAUDE.md` §3 rule 4 names five operations that must be O(1): bar lookup,
    // condition lookup, mask evaluation, duplicate rejection and result append.
    // A document that stopped mentioning one would leave that rule with nothing
    // behind it, and no ratio would notice.
    let doc = invariants();
    for operation in [
        "ar lookup",
        "ondition lookup",
        "ask evaluation",
        "uplicate rejection",
        "esult append",
    ] {
        assert!(
            doc.contains(operation),
            "`{operation}` is one of the five operations CLAUDE.md §3 rule 4 \
             makes constant, and docs/04-invariants.md does not mention it"
        );
    }
}

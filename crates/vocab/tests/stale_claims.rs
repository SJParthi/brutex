//! Fourteen written claims an audit found false, each checked against the code
//! it describes so it cannot drift back — D-1448.
//!
//! # Why this file exists
//!
//! Every claim checked here was once true or was copied from something true,
//! and nothing read it again when the code moved: a live-bit count of 238 when
//! the table holds 328, a 1.4× ceiling when every harness asserts 3.0×, a
//! previous-bar mask field that does not exist, an invariant row whose proof was
//! a test nobody wrote. `cost_invariants.rs` in `core` applies D-0102's rule —
//! *where a document states something the code also knows, a test reads the
//! document and compares* — to the cost rows. This applies it to these.
//!
//! # Why it lives in `vocab`
//!
//! Two of the checks need `vocab::table::LIVE` and `NEXT_FREE` themselves, not
//! a copy of their values, and `vocab` is the crate that has them with no
//! dependency to add. The rest are static reads of tracked files, which any
//! crate's test can do; keeping them beside the two that need `vocab` keeps
//! the D-1448 checks in one place.
//!
//! # What it does not check
//!
//! The FIGURES that came from a run (bench ratios). Those live in a run and a
//! static read cannot reproduce them. What this does check is every number the
//! code itself determines: counts, binomials, a ceiling constant, the probed
//! positions, and that every test a corrected row cites exists.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};

use vocab::table::{LIVE, NEXT_FREE};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative))
        .unwrap_or_else(|why| panic!("{relative} must be readable: {why}"))
}

/// Whitespace collapsed to single spaces, so a phrase matches however the
/// paragraph around it is wrapped.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text from the line starting `start` up to the next line starting
/// `end`, or to the end of the document.
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let from = text
        .find(&format!("\n{start}"))
        .unwrap_or_else(|| panic!("no section starts with {start:?}"));
    let rest = &text[from + 1..];
    let to = rest[start.len()..]
        .find(&format!("\n{end}"))
        .map_or(rest.len(), |at| at + start.len());
    &rest[..to]
}

/// One table row of `docs/04-invariants.md`, by id.
fn row(invariants: &str, id: &str) -> String {
    let prefix = format!("| {id} |");
    let mut rows = invariants.lines().filter(|line| line.starts_with(&prefix));
    let found = rows
        .next()
        .unwrap_or_else(|| panic!("no invariant row {id}"))
        .to_owned();
    assert!(rows.next().is_none(), "invariant row {id} appears twice");
    found
}

fn with_commas(mut value: u128) -> String {
    let mut groups = Vec::new();
    while value >= 1000 {
        groups.push(format!("{:03}", value % 1000));
        value /= 1000;
    }
    groups.push(value.to_string());
    groups.reverse();
    groups.join(",")
}

fn choose(n: u128, k: u128) -> u128 {
    (0..k).fold(1, |acc, i| acc * (n - i) / (i + 1))
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .expect("a readable directory")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// GAP5-50. The paper forms all `C(S,S/2)` training sets; this repository's
/// CSCV keeps segment 0 on the test side and visits `C(S-1,S/2)`. The charter
/// said the enumeration was "carried" from the paper. If the code ever moves to
/// the full enumeration, the second assertion fails and the charter must move
/// with it.
#[test]
fn the_charter_names_the_cscv_half_enumeration_as_a_deviation() {
    let charter = read("docs/00-charter.md");
    let cscv = charter
        .lines()
        .find(|line| line.starts_with("| CSCV begins"))
        .expect("the charter's CSCV row");
    assert!(
        !cscv.contains("equal-block complementary enumeration and rank event are carried"),
        "the charter must not say the paper's enumeration is carried"
    );
    for needed in ["`C(S-1,S/2)`", "`C(S,S/2)`", "deviation", "D-1448"] {
        assert!(cscv.contains(needed), "the CSCV row must name {needed}");
    }
    let code = read("crates/cli/src/population_statistics_v2.rs");
    assert!(
        code.contains("choose_u64(segment_count - 1, segment_count / 2)"),
        "the code no longer enumerates C(S-1,S/2): the charter row describes \
         something else now"
    );
}

/// ET-o1-proof-coverage-8. `docs/06-limits.md` §5 tabulated `C(238, k)` while
/// the table had 328 live positions. Every count below is computed here from
/// `LIVE`, so the next append that moves the live count fails this until the
/// table is redone.
#[test]
fn limits_section_5_tabulates_the_live_vocabulary() {
    let limits = read("docs/06-limits.md");
    let five = section(&limits, "## 5.", "## 6.");
    let live = u128::from(LIVE.popcount());
    let text = flat(five);
    assert!(
        text.contains(&format!("**{live} live positions**")),
        "§5 must state the live count {live}"
    );
    assert!(
        text.contains(&format!("| k | C({live}, k) |")),
        "§5's table must be C({live}, k)"
    );
    for k in 2..=8_u128 {
        let count = choose(live, k);
        assert!(
            text.contains(&format!("| {k} | {} |", with_commas(count))),
            "§5's k={k} row must read C({live}, {k}) = {}",
            with_commas(count)
        );
    }
    assert!(
        !text.contains("`seen` keys"),
        "§5 must not count memory for the deleted dedup set"
    );
}

/// AC-whp-o1-1. §5 said only ranked entry points take the streamed result and
/// that the ranked path no longer returns survivor vectors. The stored doors
/// rank a retained, checkpointed `Sweep`.
#[test]
fn limits_section_5_names_the_retaining_stored_door() {
    let lib = read("crates/cli/src/lib.rs");
    let kernel = section(&lib, "fn stored_month_kernel(", "fn ");
    assert!(
        kernel.contains("and_checkpoint::run("),
        "the stored kernel no longer ranks through the retaining checkpoint; \
         revisit docs/06-limits.md §5"
    );
    let limits = read("docs/06-limits.md");
    let five = flat(section(&limits, "## 5.", "## 6."));
    assert!(five.contains("`cli::and_checkpoint::run`"));
    assert!(five.contains("`runner::rank_checkpointed_sweep`"));
    assert!(!five.contains("only ranked entry points take the streamed result"));
    let resume = flat(&read("docs/20-sweep-resume.md"));
    assert!(!resume.contains("Encoding uses fixed scratch space"));
    assert!(resume.contains("`BoundedBytes`"));
}

/// ET-o1-proof-coverage-7, UC-17. Layer 8 named a 1.4× ceiling; the engine's
/// harness asserts the number in its own constant.
#[test]
fn layer_8_names_the_ceiling_its_harness_asserts() {
    let bench = read("crates/engine/benches/ratio.rs");
    let permille: u32 = bench
        .lines()
        .find_map(|line| line.trim().strip_prefix("const CEILING_PERMILLE: u128 = "))
        .and_then(|rest| rest.trim_end_matches(';').replace('_', "").parse().ok())
        .expect("the engine bench declares CEILING_PERMILLE");
    let ceiling = format!("**{}.{}×**", permille / 1000, permille % 1000 / 100);
    let doc = read("docs/07-o1-architecture.md");
    let layer = flat(section(&doc, "- Layer 8's flatness", "- Layer 3"));
    assert!(
        layer.contains(&ceiling),
        "Layer 8 must name the harness's {ceiling} ceiling: {layer}"
    );
    assert!(!layer.contains("asserted against the 1.4× ceiling"));
}

/// ET-vocabulary-conditions-bits-3. C-V-03 said 234 live bits and C-V-06 said
/// position 279; the bench uses `LIVE` and `NEXT_FREE - 1`.
#[test]
fn the_vocab_bench_rows_name_what_the_bench_probes() {
    let invariants = read("docs/04-invariants.md");
    let width = row(&invariants, "C-V-03");
    let live = LIVE.popcount();
    assert!(width.contains(&format!("1 bit(s) -> {live} bit(s)")));
    assert!(!width.contains("all 234 live bits"));
    let lookup = row(&invariants, "C-V-06");
    let last = NEXT_FREE - 1;
    let past = NEXT_FREE + 64;
    assert!(lookup.contains(&format!("`NEXT_FREE − 1` = {last}")));
    assert!(lookup.contains(&format!("`NEXT_FREE + 64` = {past}")));
    assert!(!lookup.contains("position 279, the midpoint"));
    assert!(lookup.contains("an early-exit scan does not"));
}

/// AC-whp-tb-3. The crossing rows described a previous-bar mask field. It does
/// not exist; the state is `last_side` and `crossings_seen`.
#[test]
fn no_document_or_source_names_the_crossing_field_that_does_not_exist() {
    let name = concat!("previous", "_mask");
    let evaluator = read("crates/indicators/src/evaluator.rs");
    assert!(evaluator.contains("last_side: [Side; vocab::table::CROSSINGS.len()],"));
    assert!(evaluator.contains("crossings_seen: [u8; vocab::table::CROSSINGS.len()],"));
    let mut sources = Vec::new();
    rust_sources(&root().join("crates"), &mut sources);
    for path in sources {
        let text = std::fs::read_to_string(&path).expect("a readable source");
        assert!(!text.contains(name), "{} names {name}", path.display());
    }
    // `docs/05-decisions.md` is append-only and keeps its historical mention.
    for doc in [
        "docs/04-invariants.md",
        "docs/03-vocabulary.md",
        "CLAUDE.md",
    ] {
        assert!(!read(doc).contains(name), "{doc} names {name}");
    }
}

/// AC-whp-tb-3, AC-whp-tb-6, AC-whp-tb-7, ET-o1-proof-coverage-9. Every test a
/// corrected row cites as its proof exists as a `fn` in the crate it names.
/// CX-04 cited no test, and `table::CROSSINGS`' doc named one nobody wrote.
#[test]
fn every_test_a_corrected_row_cites_exists() {
    let invariants = read("docs/04-invariants.md");
    let crates = root().join("crates");
    for id in [
        "CX-01", "CX-02", "CX-03", "CX-04", "S-06", "S-06b", "R-03", "C-03", "C-04", "C-E-10",
        "C-E-11",
    ] {
        let line = row(&invariants, id);
        let cells: Vec<&str> = line.split(" | ").collect();
        let proof = cells
            .get(cells.len().saturating_sub(2))
            .expect("a proof column");
        let cited: Vec<&str> = proof
            .split('`')
            .skip(1)
            .step_by(2)
            .filter(|token| {
                token.split("::").count() >= 3
                    && token
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
            })
            .collect();
        assert!(
            cited
                .iter()
                .any(|path| crates.join(path.split("::").next().unwrap_or("")).is_dir()),
            "{id} cites no test path: {proof}"
        );
        for path in cited {
            let krate = path.split("::").next().expect("a crate");
            let function = path.rsplit("::").next().expect("a function");
            if !crates.join(krate).is_dir() {
                continue;
            }
            let mut sources = Vec::new();
            rust_sources(&crates.join(krate), &mut sources);
            let declared = format!("fn {function}(");
            assert!(
                sources.iter().any(|source| std::fs::read_to_string(source)
                    .is_ok_and(|text| text.contains(&declared))),
                "{id} cites {path}, and no `{declared}` exists under crates/{krate}"
            );
        }
    }
}

/// UC-6, AC-gates-o1-4 (its `grid_entered_event` half too, D-1486), AC-whp-tb-6, AC-whp-tb-7, ET-o1-proof-coverage-8,
/// ET-strategies-trades-ranking-costs-9, R9-csr-o1-0: the false sentences, as
/// they were written, do not come back.
#[test]
fn the_corrected_sentences_do_not_return() {
    for (file, stale) in [
        (
            "CLAUDE.md",
            "`cli` holds no loop over bars and none over candidates",
        ),
        (
            "AGENTS.md",
            "`cli` holds no loop over bars and none over candidates",
        ),
        (
            "crates/cli/src/lib.rs",
            "this crate holds no loop over bars and none over candidates",
        ),
        (
            "crates/cli/src/lib.rs",
            "holding no loop over bars and none over candidates",
        ),
        (
            "docs/06-limits.md",
            "The one non-Rust file left anywhere in the graph is",
        ),
        (
            "docs/06-limits.md",
            "`peak_favourable` are each `for i in from..=to` and are called for every",
        ),
        (
            "docs/06-limits.md",
            "The tail block's first verification per handle adds one `fstat`",
        ),
        ("docs/06-limits.md", "It warns twice, `store.header`"),
        (
            "docs/06-limits.md",
            "there is no bar reader in `crates/store` at all",
        ),
        (
            "docs/06-limits.md",
            "**Absolute speed.** Every row but C-08 is a *ratio*",
        ),
        (
            "docs/06-limits.md",
            "the device's, and no bench in this repository times a syscall",
        ),
        (
            "docs/04-invariants.md",
            "**no bench in this repository times a syscall.**",
        ),
        (
            "docs/04-invariants.md",
            "`FLAG_CHECKSUMS` is set nowhere in production",
        ),
        (
            "docs/04-invariants.md",
            "the flag is clear, so `block::verify` returns `ChecksumsAbsent`",
        ),
        (
            "docs/04-invariants.md",
            "**The short direction is exercised, and differs from the long one.**",
        ),
        ("docs/04-invariants.md", "CURRENT MEASUREMENT PENDING"),
        (
            "docs/04-invariants.md",
            "pending rerun after production-shape correction",
        ),
    ] {
        assert!(
            !flat(&read(file)).contains(stale),
            "{file} says again: {stale}"
        );
    }
    let initialise = read("crates/store/src/file.rs");
    assert!(
        initialise.contains("crate::format::FLAG_CHECKSUMS,\n    );"),
        "store::file::initialise no longer passes FLAG_CHECKSUMS: S-06 is stale"
    );
}

/// R9-csr-cx-4: the weekday bits' comments said NSE does not trade on a
/// weekend, beside a paragraph and a charter (`docs/00-charter.md` §3) that
/// record six weekend sessions and 1,710 bars. The behaviour (a weekend bar sets
/// no weekday bit) is right and stays; the sentences that called the exchange
/// weekday-only are refused here so they cannot return. D-1667.
#[test]
fn no_weekday_comment_says_nse_never_trades_on_a_weekend() {
    for file in ["crates/indicators/src/lib.rs", "crates/vocab/src/table.rs"] {
        let text = flat(&read(file));
        for stale in [
            "NSE trades Monday to Friday",
            "NSE does not trade them",
            "NSE DOES NOT TRADE THESE",
            "an exchange that trades Monday to Friday",
            "A weekend bar in an equity series is a store defect",
        ] {
            assert!(!text.contains(stale), "{file} still says {stale:?}");
        }
    }
    let lib = flat(&read("crates/indicators/src/lib.rs"));
    assert!(
        lib.contains("NSE ordinarily trades Monday to Friday"),
        "the corrected statement is the one weekday_bit carries"
    );
    assert!(
        lib.contains("1,710 real trading bars"),
        "the charter's count stays"
    );
}

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

/// `text` with each line's leading comment marker (`//`, `///` or `//!`) dropped
/// and its whitespace collapsed, so a sentence a comment wraps across lines
/// reads as the one sentence it is. `pull::fold` wrapped "trades Monday // to
/// Friday", and collapsing whitespace alone could not see it (G5-3, D-4734).
fn prose(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix("///")
                .or_else(|| line.strip_prefix("//!"))
                .or_else(|| line.strip_prefix("//"))
                .unwrap_or(line)
        })
        .collect();
    flat(&lines.join("\n"))
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

/// AHB-01 (h-eng-1, D-1860). `indicators::column` described alignment as "the
/// first execution bar stamped at or after" a signal's close and cited an
/// `align` test asserting `[Some(10), Some(10)]`. Since D-0401 `align` maps
/// only to the bar at EXACTLY the close instant, and no such test exists.
#[test]
fn the_column_docs_state_the_exact_close_alignment_rule() {
    let align = read("crates/runner/src/align.rs");
    assert!(
        align.contains("c.ts_micros == deadline"),
        "align no longer maps to the exact close instant: AHB-01 must be re-derived"
    );
    let column = flat(&read("crates/indicators/src/column.rs"));
    for stale in [
        "first execution bar stamped at or after",
        "execution bar stamped at or after each signal's close",
        "Some(10), Some(10)",
        "A hole of one bar is enough to collide",
        "A hole in the execution series: rows 1",
    ] {
        assert!(!column.contains(stale), "column.rs says again: {stale}");
    }
    assert!(column.contains("returns the execution bar stamped EXACTLY at the"));
    assert!(column.contains("execution hole therefore DROPS a signal"));
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
///
/// G5-3 (D-4734): the list held only the two files the finding named, and
/// `pull::fold` and its unit test still said "an exchange that trades Monday to
/// Friday". Every `.rs` file under `crates/` is read now, so a sibling cannot
/// keep the sentence by not being listed.
#[test]
fn no_weekday_comment_says_nse_never_trades_on_a_weekend() {
    let mut files = Vec::new();
    rust_sources(&root().join("crates"), &mut files);
    assert!(
        files
            .iter()
            .any(|path| path.ends_with("crates/pull/src/fold.rs"))
            && files.len() > 300,
        "premise: the walk reached every crate ({} files)",
        files.len()
    );
    let this_file = root().join("crates/vocab/tests/stale_claims.rs");
    for path in files.iter().filter(|path| **path != this_file) {
        let text = prose(
            &std::fs::read_to_string(path)
                .unwrap_or_else(|why| panic!("{} must be readable: {why}", path.display())),
        );
        for stale in [
            "NSE trades Monday to Friday",
            "NSE does not trade them",
            "NSE DOES NOT TRADE THESE",
            "an exchange that trades Monday to Friday",
            "A weekend bar in an equity series is a store defect",
        ] {
            assert!(
                !text.contains(stale),
                "{} still says {stale:?}",
                path.display()
            );
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

/// XPERM-05 (D-3405). CX-01 and the D-0244 amendment lock the crossing rule as "this
/// bar's side differs from the last DEFINITE side earlier in the session", and
/// `a_touch_is_known_false_but_cannot_erase_the_side_or_count_across_it` pins it. The
/// bit table — the document with authority over what a bit means — still defined every
/// crossing row by the previous bar, so a reader decoding a mask got the rule that was
/// rejected. Every `CROSSINGS` row, both edges, and the `LevelCrossing` doc are held to
/// the locked rule here.
#[test]
fn every_crossing_row_states_the_last_definite_side_rule() {
    let doc = read("docs/03-vocabulary.md");
    let stale = concat!("on the previous", " bar");
    let mut rows = 0_u32;
    for crossing in vocab::table::CROSSINGS {
        for position in [crossing.up, crossing.down] {
            let prefix = format!("| {position} |");
            let row = doc
                .lines()
                .find(|line| line.starts_with(&prefix))
                .unwrap_or_else(|| panic!("03-vocabulary.md has no row {position}"));
            assert!(!row.contains(stale), "row {position} still says: {row}");
            assert!(
                row.contains("last definite side"),
                "row {position} must state the CX-01 rule: {row}"
            );
            rows += 1;
        }
    }
    assert_eq!(rows, 34, "seventeen levels, two edges each");
    for source in [
        "crates/vocab/src/table.rs",
        "crates/indicators/src/evaluator.rs",
    ] {
        let flat = read(source)
            .split_whitespace()
            .filter(|word| !matches!(*word, "//" | "///" | "//!"))
            .collect::<Vec<_>>()
            .join(" ");
        for stale in [
            concat!("clear on the previous", " bar"),
            concat!("CLEAR on the previous", " bar"),
            concat!("clear on the", " previous bar and is set"),
        ] {
            assert!(!flat.contains(stale), "{source} still documents: {stale}");
        }
    }
}

/// XPERM-06 (D-3406). Every count and kind the documents state about the table is the
/// table's own, computed here rather than restated: a number written down beside a
/// table that is appended to is stale the day after the append.
#[test]
fn every_count_and_kind_the_documents_state_is_the_tables() {
    use vocab::table::{BitStatus, COUNT, Kind, TABLE};
    let doc = read("docs/03-vocabulary.md");

    // (1) The "Needs a tolerance" column says `yes` exactly for a `Near` row, void
    // and retired rows included — the 13 void `near_forming_pivot_*` rows said `—`.
    let mut kinds = 0_u32;
    for line in doc.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // `| n | name | tolerance | status |` splits into six cells.
        let [_, index, _, tolerance, _, _] = cells.as_slice() else {
            continue;
        };
        let (Ok(index), true) = (index.parse::<usize>(), ["yes", "—"].contains(tolerance)) else {
            continue;
        };
        let row = TABLE.get(index).expect("a documented row is in the table");
        assert_eq!(
            *tolerance == "yes",
            row.kind == Kind::Near,
            "row {index} `{}` says `{tolerance}` for a {:?} position",
            row.name,
            row.kind
        );
        kinds += 1;
    }
    assert!(kinds >= 200, "the four-column rows were found: {kinds}");

    // (2) How many `close_above_` names D-0080 voided — CX-04 and the crossings
    // section said sixteen.
    let void_above = TABLE
        .iter()
        .filter(|row| {
            row.name.starts_with("close_above_") && matches!(row.status, BitStatus::Void { .. })
        })
        .count();
    assert_eq!(void_above, 13, "re-word CX-04 and §7 if this moves");
    let invariants = read("docs/04-invariants.md");
    for (name, text) in [
        ("03-vocabulary.md", &doc),
        ("04-invariants.md", &invariants),
    ] {
        assert!(
            !text.contains(concat!("Sixteen `close_above_` names", " are `void`")),
            "{name} still says sixteen"
        );
        assert!(
            text.contains("Thirteen `close_above_` names are `void`"),
            "{name} must state the table's thirteen"
        );
    }
    // The crossings table has its header row: the line before bit 280 is a separator.
    let lines: Vec<&str> = doc.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with("| 280 |"))
        .expect("bit 280 is documented");
    assert!(
        lines
            .get(at.wrapping_sub(1))
            .is_some_and(|line| line.starts_with("|---")),
        "the crossings table has no header: {:?}",
        lines.get(at.wrapping_sub(1))
    );

    // (3) CX-05's remaining positions, and the lib test's own comment.
    let remaining = vocab::ConditionMask::BITS as usize - COUNT;
    assert!(
        invariants.contains(&format!("the {remaining} remaining positions")),
        "CX-05 must state {remaining} remaining"
    );
    let lib = read("crates/vocab/src/lib.rs");
    assert!(
        !lib.contains(concat!("NINETEEN", " LEFT")),
        "lib.rs still says nineteen"
    );

    // (4) The module's "What is here" table tiles 0..COUNT with no gap.
    let table = read("crates/vocab/src/table.rs");
    let mut next = 0_usize;
    for line in table.lines().filter(|l| l.starts_with("//! | ")) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let Some((from, to)) = cells.get(1).and_then(|r| r.split_once('–')) else {
            continue;
        };
        let (Ok(from), Ok(to)) = (from.parse::<usize>(), to.parse::<usize>()) else {
            continue;
        };
        let count: usize = cells.get(2).and_then(|c| c.parse().ok()).expect("a count");
        assert_eq!(from, next, "the group table skips to {from} from {next}");
        assert_eq!(count, to + 1 - from, "{line}");
        next = to + 1;
    }
    assert_eq!(next, COUNT, "the group table stops at {next} of {COUNT}");
}


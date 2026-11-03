//! FNO-LAW: the law of the F&O strategy path, read back from the documents
//! that state it (`docs/05-decisions.md` D-8100 to D-8108).
//!
//! # Why this file exists
//!
//! On 30 Oct 2026 the operator fixed the F&O strategy backtest's universe, its
//! one fill model, its two cost clocks and its rules, and none of it was law:
//! `CLAUDE.md` §1 named the 208 shares of `core::universe::FNO_UNDERLYINGS` and
//! nothing else, and §3 rule 2 forbids a surface that widens without an entry.
//! FNO-LAW wrote the law and this file reads it back, so an edit that drops the
//! survivorship label, the universe-file rule, an entry of the block or a source
//! row of the charter fails `cargo test` instead of passing by being unread
//! (D-0102's rule: where a document states something the code also knows, a
//! test reads the document and compares).
//!
//! # What it checks
//!
//! * `CLAUDE.md` §1 and `AGENTS.md` §1 hold the third surface in the same
//!   words, and the label byte for byte; §3 rule 3 of both names the path's own
//!   identity domain.
//! * `docs/05-decisions.md` heads D-8100 to D-8108 once each, in order, under
//!   FNO-LAW's slot line; no other number of FNO-LAW's block of ten heads an
//!   entry anywhere; D-8108 holds one line for each of the 46 open points.
//! * The 24 slot lines stand in all four ledger documents, in the design's part
//!   order, under the section heading.
//! * `docs/00-charter.md` names the 25 rows of the cost table and the eight
//!   execution facts, each once and in order, and its scope lock names D-8100.
//! * FNO-LAW's invariant rows cite only tests this file declares, and its plan
//!   row stands under its slot.
//!
//! Every reader is also run on invented text, one change at a time, so each
//! refusal is seen to fire and to name what it refuses.
//!
//! # What it does not check
//!
//! Whether the words are true of the code that will carry them: the path's
//! parts (FNO-DAYMASK, FNO-COSTS, FNO-SEC-EXEC, FNO-SEC-WIRE and the rest)
//! prove their rules beside their own code. The documents are read as
//! `include_str!` sees them, so an edit of any of them rebuilds this test.
//! Numbers of FNO-LAW's block are written here as numbers, never as citations:
//! `decision_citations.rs` requires every number cited under `crates/` to head
//! an entry, and the block's tenth is the spare that heads none.

use std::fmt::Write as _;
use std::ops::RangeInclusive;

/// `CLAUDE.md`, the law.
const CLAUDE: &str = include_str!("../../../CLAUDE.md");
/// `AGENTS.md`, the law's mirror.
const AGENTS: &str = include_str!("../../../AGENTS.md");
/// `docs/00-charter.md`, where every source is recorded.
const CHARTER: &str = include_str!("../../../docs/00-charter.md");
/// `docs/04-invariants.md`.
const INVARIANTS: &str = include_str!("../../../docs/04-invariants.md");
/// `docs/05-decisions.md`, the ledger.
const DECISIONS: &str = include_str!("../../../docs/05-decisions.md");
/// `docs/06-limits.md`, read only for its slot lines.
const LIMITS: &str = include_str!("../../../docs/06-limits.md");
/// `docs/07-plan.md`.
const PLAN: &str = include_str!("../../../docs/07-plan.md");
/// This file, so a row's cited test is looked up where it would be declared.
const THIS_FILE: &str = include_str!("fno_law.rs");

/// The label every result and page of the path carries, word for word.
const LABEL: &str = "current F&O members only (survivorship bias not removed)";

/// What §1 says of the third surface, in the same words in `CLAUDE.md` and
/// `AGENTS.md`, read with whitespace folded so a rewrap changes nothing.
const SURFACE: [&str; 11] = [
    "Widened by D-8100",
    "The list is held as a data file, never as a list in code",
    "the blake3 digest of its bytes enters every run identity of the path",
    "Stocks that left F&O since 2018 are not added: not to the traded set and not to the rank pool",
    "a stock is traded only on a day it was a point-in-time F&O member",
    "a day the membership cannot answer is not traded",
    "ICICIPRULI is joined to ICICILIFE as one stock at read time, by a join map that is data",
    "Signals come from Zerodha candles",
    "the worst price printed in the second: a buy at that second's high, a sell at its low",
    "Every trade is charged on two cost clocks, its own day's dated rates and today's, and the worse of the two is the headline",
    "that stack is FNO-COSTS's dated cash-equity intraday clock",
];

/// What §3 rule 3 says of the path's identity, in both files.
const IDENTITY: [&str; 2] = [
    "The F&O strategy path has its own identity domain",
    "built by FNO-SEC-WIRE as the sole owner of `runner::identity`",
];

/// The decision numbers FNO-LAW uses.
const USED: RangeInclusive<u16> = 8100..=8108;
/// FNO-LAW's block of ten: the numbers it uses and its spare.
const BLOCK: RangeInclusive<u16> = 8100..=8109;
/// The last number of the last part's block, which closes the slot section's
/// heading.
const SECTION_END: u16 = 8339;
/// The open points D-8108 holds, one line each.
const OPEN_POINTS: RangeInclusive<u16> = 1..=46;

/// The slot lines of the four ledger documents, in the design's part order.
const SLOTS: [&str; 24] = [
    "FNO-LAW",
    "FNO-COSTS",
    "FNO-SEC-EXEC",
    "FNO-LADDER",
    "FNO-DAYMASK",
    "FNO-LEVERAGE",
    "FNO-RANK",
    "FNO-PAUSE",
    "FNO-PORTFOLIO",
    "FNO-WFT",
    "FNO-DAILY",
    "FNO-BAND",
    "FNO-CLOCK",
    "FNO-NET",
    "FNO-EXITS",
    "FNO-SEC-WIRE",
    "FNO-STRAT",
    "FNO-RECORDS",
    "FNO-FILTER",
    "FNO-API",
    "FNO-RUN",
    "FNO-LEAN",
    "FNO-SWEEP-WIRE",
    "FNO-BAND-FILES",
];

/// The 25 rows of the cost table `costs-table/2026-10-30/r1`, in its order.
const COST_ROWS: [&str; 25] = [
    "BRK-1", "BRK-SQ", "BRK-DP", "BRK-CT", "EX-2009", "EX-2020C", "EX-2021", "EX-2023", "EX-2024A",
    "EX-2024B", "EX-2026", "IP-OLD", "IP-2023", "IP-2026", "SF-2017", "SF-2019", "SF-2020",
    "SF-2021", "STT-ND", "SD-STATE", "SD-ND", "GST-1", "GST-SEBI", "OT-CLR", "OT-INT",
];

/// The eight exchange and broker facts, in the design's order.
const FACT_ROWS: [&str; 8] = [
    "T02-02", "T03-01", "T03-05", "T03-04", "T09-07", "T10-05", "T06-03", "T11-09",
];

/// FNO-LAW's invariant rows, in order.
const LAW_ROWS: [&str; 4] = ["FNO-LAW-I01", "FNO-LAW-I02", "FNO-LAW-I03", "FNO-LAW-I04"];

/// The charter section's heading, up to its date.
const CHARTER_SECTION: &str =
    "\n## F&O strategy path: dated cash-equity charges and execution facts";

/// NSE's daily margin file, as row T02-02 names it.
const MARGIN_FILE: &str = "`C_VAR1_DDMMYYYY_1.DAT`";

/// `text` with every run of whitespace made one space.
fn fold(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text after the first `start` in `doc` up to the next `end`, or `None`
/// when either is missing. Both splits fall on a char boundary, because each
/// is the end of a match or the start of one.
fn section<'a>(doc: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let (_, body) = doc.split_at(doc.find(start)? + start.len());
    let (body, _) = body.split_at(body.find(end)?);
    Some(body)
}

/// The heading of the slot section, built from its two numbers.
fn slot_heading() -> String {
    format!(
        "\n## F&O strategy path: part blocks (D-{}..D-{SECTION_END})\n",
        USED.start()
    )
}

/// Every way `doc`, called `name`, falls short of the law of D-8100: §1 must
/// hold the label byte for byte and every phrase of [`SURFACE`], and §3 rule 3
/// every phrase of [`IDENTITY`].
fn law_problems(name: &str, doc: &str) -> Vec<String> {
    let Some(surface) = section(doc, "\n## 1. ", "\n## 2. ") else {
        return vec![format!("{name} has no §1")];
    };
    let mut problems = Vec::new();
    if !surface.contains(LABEL) {
        problems.push(format!("{name} §1 does not hold the label byte for byte"));
    }
    let folded = fold(surface);
    for phrase in SURFACE {
        if !folded.contains(phrase) {
            problems.push(format!("{name} §1 does not say {phrase:?}"));
        }
    }
    let Some(rule) = section(doc, "\n3. **Reproducibility.**", "\n4. **") else {
        problems.push(format!("{name} has no §3 rule 3"));
        return problems;
    };
    let folded = fold(rule);
    for phrase in IDENTITY {
        if !folded.contains(phrase) {
            problems.push(format!("{name} §3 rule 3 does not say {phrase:?}"));
        }
    }
    problems
}

/// A fence line's character, its run and the text after the run: three or
/// more backticks or tildes after any indentation.
fn fence(line: &str) -> Option<(u8, usize, &str)> {
    let text = line.trim_start();
    let mark = *text.as_bytes().first()?;
    if mark != b'`' && mark != b'~' {
        return None;
    }
    let run = text.bytes().take_while(|&b| b == mark).count();
    if run < 3 {
        return None;
    }
    text.get(run..).map(|after| (mark, run, after))
}

/// The decision number `line` heads: two to four `#`, at least one space, `D-`
/// and four ASCII digits with no letter, digit or `_` after them. That is the
/// reading `decision_citations.rs` gives an entry: gate 27b's pattern with the
/// citation's trailing boundary.
fn heading(line: &str) -> Option<u16> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if !(2..=4).contains(&hashes) {
        return None;
    }
    // The split falls on a char boundary: the hashes before it are ASCII.
    let (_, rest) = line.split_at(hashes);
    let text = rest.trim_start_matches(' ');
    if text.len() == rest.len() {
        return None;
    }
    let digits = text.strip_prefix("D-")?.get(..4)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if text
        .as_bytes()
        .get(6)
        .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return None;
    }
    digits.parse().ok()
}

/// Every entry heading of `doc` outside a code fence, as (number, 0-based
/// line). A fence closes only on a run of its own character at least as long,
/// with nothing after it but spaces, so a heading inside one heads nothing.
fn entries(doc: &str) -> Vec<(u16, usize)> {
    let mut out = Vec::new();
    let mut open: Option<(u8, usize)> = None;
    for (index, line) in doc.lines().enumerate() {
        let marker = fence(line);
        if let Some((mark, run)) = open {
            if let Some((close, length, after)) = marker
                && close == mark
                && length >= run
                && after.trim().is_empty()
            {
                open = None;
            }
            continue;
        }
        if let Some((mark, run, _)) = marker {
            open = Some((mark, run));
        } else if let Some(number) = heading(line) {
            out.push((number, index));
        }
    }
    out
}

/// The slot names `doc` holds, in its order.
fn slots(doc: &str) -> Vec<&str> {
    doc.lines()
        .filter_map(|line| line.strip_prefix("<!-- slot "))
        .filter_map(|rest| rest.strip_suffix(" -->"))
        .collect()
}

/// Every way `doc`, called `name`, falls short of the slot section FNO-SEAM
/// placed: its heading, and the 24 slot lines once each, in order.
fn slot_problems(name: &str, doc: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !doc.contains(&slot_heading()) {
        problems.push(format!("{name} has no slot section heading"));
    }
    let got = slots(doc);
    if got != SLOTS {
        problems.push(format!("{name}'s slot lines are {got:?}"));
    }
    problems
}

/// The numbers of the `- OP-NN: ` lines of the entry whose heading is
/// `lines[at]`, up to the next heading or slot line; a malformed line reads 0.
fn op_numbers(lines: &[&str], at: usize) -> Vec<u16> {
    lines
        .iter()
        .skip(at + 1)
        .take_while(|line| !line.starts_with('#') && !line.starts_with("<!-- slot "))
        .filter_map(|line| line.strip_prefix("- OP-"))
        .map(|rest| match (rest.get(..2), rest.get(2..4)) {
            (Some(digits), Some(": ")) => digits.parse().unwrap_or(0),
            _ => 0,
        })
        .collect()
}

/// The refusal for a block whose headings read `got`.
fn block_problem(got: &[u16]) -> String {
    format!("the block heads {got:?}, not D-8100 to D-8108 once each in order")
}

/// The refusal for an entry of the block outside FNO-LAW's slot.
fn outside_problem(number: u16) -> String {
    format!("D-{number} heads an entry outside FNO-LAW's slot")
}

/// The refusal for D-8108's open-point lines reading `got`.
fn ops_problem(got: &[u16]) -> String {
    format!("D-8108 holds the OP lines {got:?}, not OP-01 to OP-46 once each in order")
}

/// Every way the ledger `doc` falls short of D-8107: the slot section, the
/// block's headings and where they stand, and D-8108's 46 lines.
fn ledger_problems(doc: &str) -> Vec<String> {
    let mut problems = slot_problems("docs/05-decisions.md", doc);
    let lines: Vec<&str> = doc.lines().collect();
    let law = lines
        .iter()
        .position(|line| *line == "<!-- slot FNO-LAW -->");
    let costs = lines
        .iter()
        .position(|line| *line == "<!-- slot FNO-COSTS -->");
    let block: Vec<(u16, usize)> = entries(doc)
        .into_iter()
        .filter(|(number, _)| BLOCK.contains(number))
        .collect();
    let got: Vec<u16> = block.iter().map(|&(number, _)| number).collect();
    if got != USED.collect::<Vec<_>>() {
        problems.push(block_problem(&got));
    }
    for &(number, line) in &block {
        if !(law.is_some_and(|at| line > at) && costs.is_some_and(|at| line < at)) {
            problems.push(outside_problem(number));
        }
    }
    let ops = block
        .iter()
        .find(|&&(number, _)| number == *USED.end())
        .map(|&(_, at)| op_numbers(&lines, at))
        .unwrap_or_default();
    if ops != OPEN_POINTS.collect::<Vec<_>>() {
        problems.push(ops_problem(&ops));
    }
    problems
}

/// The first cell of a table row and the row, unless it is a header row.
fn row(line: &str) -> Option<(&str, &str)> {
    let (id, _) = line.strip_prefix("| ")?.split_once(" |")?;
    (id != "Id").then_some((id, line))
}

/// Every way the charter `doc` falls short of FNO-LAW's sources: the scope
/// lock's fourth row, and the F&O section's 25 cost rows and eight facts.
fn charter_problems(doc: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let scope = section(doc, "\n## 1. Scope lock", "\n## 2. ").unwrap_or_default();
    if !scope.contains("| D-8100 |") {
        problems.push("the scope lock has no row since D-8100".to_owned());
    }
    let Some(body) = section(doc, CHARTER_SECTION, "\n## ") else {
        problems.push("the charter has no F&O strategy path section".to_owned());
        return problems;
    };
    for needed in [
        "`costs-table/2026-10-30/r1`",
        "`state/fib-strategy-20261030/costs-evidence/sha256-official.txt`",
    ] {
        if !body.contains(needed) {
            problems.push(format!("the F&O section does not name {needed}"));
        }
    }
    let rows: Vec<(&str, &str)> = body.lines().filter_map(row).collect();
    let got: Vec<&str> = rows.iter().map(|&(id, _)| id).collect();
    let want: Vec<&str> = COST_ROWS.iter().chain(&FACT_ROWS).copied().collect();
    if got != want {
        problems.push(format!("the F&O section's rows are {got:?}"));
    }
    for &(id, line) in &rows {
        if COST_ROWS.contains(&id)
            && !(line.contains("`https://") && line.contains("| 2026-10-30 |"))
        {
            problems.push(format!("cost row {id} has no https source or no read date"));
        }
    }
    if !rows
        .iter()
        .any(|&(id, line)| id == "T02-02" && line.contains(MARGIN_FILE))
    {
        problems.push("row T02-02 does not name NSE's margin file".to_owned());
    }
    problems
}

/// The text between FNO-LAW's slot line and FNO-COSTS's in `doc`.
fn law_slot(doc: &str) -> Option<&str> {
    section(doc, "<!-- slot FNO-LAW -->", "<!-- slot FNO-COSTS -->")
}

/// Every way FNO-LAW's own rows fall short: the four invariant rows under its
/// slot of `invariants`, each citing a `core::fno_law` test that `declared`
/// (this file's source) declares, and a plan row under its slot of `plan`.
fn rows_problems(invariants: &str, plan: &str, declared: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let rows: Vec<(&str, &str)> = law_slot(invariants)
        .unwrap_or_default()
        .lines()
        .filter_map(row)
        .collect();
    let ids: Vec<&str> = rows.iter().map(|&(id, _)| id).collect();
    if ids != LAW_ROWS {
        problems.push(format!("FNO-LAW's slot of the invariants holds {ids:?}"));
    }
    for &(id, line) in &rows {
        let cited: Vec<&str> = line
            .split('`')
            .skip(1)
            .step_by(2)
            .filter_map(|token| token.strip_prefix("core::fno_law::"))
            .collect();
        if cited.is_empty() {
            problems.push(format!("{id} cites no test of this file"));
        }
        for name in cited {
            if !declared.contains(&format!("fn {name}(")) {
                problems.push(format!(
                    "{id} cites {name}, which this file does not declare"
                ));
            }
        }
    }
    if !law_slot(plan).unwrap_or_default().contains("\n| FNO-LAW: ") {
        problems.push("FNO-LAW's slot of the plan holds no plan row".to_owned());
    }
    problems
}

// ---------------------------------------------------------------- fixtures --

/// A law text holding `label` and `surface` in §1 and `identity` in rule 3.
fn law_text(label: &str, surface: &[&str], identity: &[&str]) -> String {
    format!(
        "# law\n\n## 1. What this is\n\n{label}\n{}\n\n## 2. Next\n\n\
         3. **Reproducibility.** {}\n4. **Constant per-operation cost.**\n",
        surface.join("\n"),
        identity.join("\n   ")
    )
}

/// `all` less its `index`th item.
fn without<'a>(all: &[&'a str], index: usize) -> Vec<&'a str> {
    all.iter()
        .enumerate()
        .filter(|&(at, _)| at != index)
        .map(|(_, item)| *item)
        .collect()
}

/// A ledger whose slot section holds `slots`, with one entry per `numbers`
/// under FNO-LAW's slot and `ops` lines under D-8108's heading.
fn ledger_text(numbers: &[u16], ops: &[u16], slots: &[&str]) -> String {
    let mut text = format!(
        "# ledger\n\n### D-0001 — an older entry\n{}\n",
        slot_heading()
    );
    for slot in slots {
        let _ = write!(text, "<!-- slot {slot} -->\n\n");
        if *slot == "FNO-LAW" {
            for number in numbers {
                let _ = write!(text, "### D-{number} — an entry\n\nText.\n\n");
                if number == USED.end() {
                    for op in ops {
                        let _ = writeln!(text, "- OP-{op:02}: a choice.");
                    }
                    text.push('\n');
                }
            }
        }
    }
    text
}

/// A document holding only the slot section with `slots`.
fn slot_text(slots: &[&str]) -> String {
    let mut text = format!("# document\n{}\n", slot_heading());
    for slot in slots {
        let _ = write!(text, "<!-- slot {slot} -->\n\n");
    }
    text
}

/// A charter whose F&O section holds `costs` and `facts` as rows.
fn charter_text(costs: &[&str], facts: &[&str]) -> String {
    let mut text = String::from(
        "# charter\n\n## 1. Scope lock\n\n| the path's shares | NSE | CASH | D-8100 |\n\n\
         ## 2. Prohibitions\n\
         \n## F&O strategy path: dated cash-equity charges and execution facts — read\n\n\
         `costs-table/2026-10-30/r1`, \
         `state/fib-strategy-20261030/costs-evidence/sha256-official.txt`.\n\n\
         | Id | Charge | Source | Read | Status |\n|---|---|---|---|---|\n",
    );
    for id in costs {
        let _ = writeln!(
            text,
            "| {id} | a charge | `https://nsearchives.nseindia.com/{id}.pdf` | 2026-10-30 | PRIMARY |"
        );
    }
    // A line that opens a row and never closes its first cell is not a row.
    text.push_str("| a line with no closing bar\n");
    text.push_str("\n| Id | Fact | Source |\n|---|---|---|\n");
    for id in facts {
        let _ = writeln!(
            text,
            "| {id} | a fact read from {MARGIN_FILE} | as recorded |"
        );
    }
    text.push_str("\n## The next section\n");
    text
}

/// FNO-LAW's invariant slot holding `rows`, each citing `cites`, and its plan
/// slot holding `plan`.
fn rows_texts(rows: &[&str], cites: &str, plan: &str) -> (String, String) {
    let mut invariants = String::from("<!-- slot FNO-LAW -->\n\n| Id | Invariant | Proof | |\n");
    for id in rows {
        let _ = writeln!(invariants, "| {id} | **a rule** | {cites} | ✓ |");
    }
    invariants.push_str("\n<!-- slot FNO-COSTS -->\n");
    let plan = format!("<!-- slot FNO-LAW -->\n\n{plan}\n<!-- slot FNO-COSTS -->\n");
    (invariants, plan)
}

// ------------------------------------------------------- the documents --

/// **`CLAUDE.md` §1 and `AGENTS.md` §1 hold the F&O strategy path as a third
/// surface, in the same words, and the label byte for byte** (D-8100, D-8101,
/// D-8102); §3 rule 3 of both names the path's identity domain.
#[test]
fn the_law_holds_the_third_surface_the_label_and_the_universe_file_rule() {
    assert_eq!(law_problems("CLAUDE.md", CLAUDE), Vec::<String>::new());
    assert_eq!(law_problems("AGENTS.md", AGENTS), Vec::<String>::new());
    // NOT VACUOUS: §1 of the law is read from its heading to its last
    // paragraph, not an empty or a partial match.
    let surface = section(CLAUDE, "\n## 1. ", "\n## 2. ").unwrap_or_default();
    assert!(surface.starts_with("What this is\n"));
    assert!(surface.contains("the 208 F&O underlyings that are shares"));
    assert!(surface.contains("validated out of sample."));
}

/// **`docs/05-decisions.md` heads D-8100 to D-8108 once each, in order, under
/// FNO-LAW's slot, and no other number of the block heads an entry**; D-8108
/// holds the 46 open points; every ledger holds the 24 slot lines (D-8107).
#[test]
fn the_ledger_heads_d_8100_to_d_8108_under_the_law_slot_and_no_other_number_of_its_block() {
    assert_eq!(ledger_problems(DECISIONS), Vec::<String>::new());
    for (name, doc) in [
        ("docs/04-invariants.md", INVARIANTS),
        ("docs/06-limits.md", LIMITS),
        ("docs/07-plan.md", PLAN),
    ] {
        assert_eq!(slot_problems(name, doc), Vec::<String>::new());
    }
    // NOT VACUOUS: the reader finds the ledger's first entry and well over a
    // thousand others, so a reader that matched nothing could not pass.
    let all = entries(DECISIONS);
    assert_eq!(all.first().map(|&(number, _)| number), Some(1));
    assert!(all.len() > 1_000);
}

/// **The charter names each of the 25 cost rows and the eight execution facts
/// once, in order**, each cost row with its source link and read date, row
/// T02-02 with NSE's margin file, and the scope lock names D-8100.
#[test]
fn the_charter_names_every_cost_row_and_every_execution_source_once_in_order() {
    assert_eq!(charter_problems(CHARTER), Vec::<String>::new());
}

/// **FNO-LAW's rows sit under its own slot lines and cite only tests this file
/// declares** (D-8107).
#[test]
fn the_law_rows_sit_under_their_slots_and_cite_only_tests_this_file_declares() {
    assert_eq!(
        rows_problems(INVARIANTS, PLAN, THIS_FILE),
        Vec::<String>::new()
    );
}

// --------------------------------------------- the readers on invented text --

/// Each phrase of §1 and of rule 3 missing alone is named alone; a rewrapped
/// phrase still passes; the label in another case, broken over two lines or
/// only after §1 is refused; a text with no §1 or no rule 3 is refused.
#[test]
fn a_law_text_missing_any_one_phrase_or_misplacing_the_label_is_refused_by_name() {
    let none: Vec<String> = Vec::new();
    assert_eq!(
        law_problems("x", &law_text(LABEL, &SURFACE, &IDENTITY)),
        none
    );
    let wrapped: Vec<String> = SURFACE
        .iter()
        .map(|p| p.replacen(' ', "\n   ", 1))
        .collect();
    let wrapped: Vec<&str> = wrapped.iter().map(String::as_str).collect();
    assert_eq!(
        law_problems("x", &law_text(LABEL, &wrapped, &IDENTITY)),
        none
    );
    for (index, phrase) in SURFACE.iter().enumerate() {
        let text = law_text(LABEL, &without(&SURFACE, index), &IDENTITY);
        assert_eq!(
            law_problems("x", &text),
            [format!("x §1 does not say {phrase:?}")]
        );
    }
    for (index, phrase) in IDENTITY.iter().enumerate() {
        let text = law_text(LABEL, &SURFACE, &without(&IDENTITY, index));
        assert_eq!(
            law_problems("x", &text),
            [format!("x §3 rule 3 does not say {phrase:?}")]
        );
    }
    let label = ["x §1 does not hold the label byte for byte".to_owned()];
    for misplaced in [
        LABEL.replacen('c', "C", 1),
        LABEL.to_uppercase(),
        LABEL.replace(" bias", "\nbias"),
        String::new(),
    ] {
        let text = law_text(&misplaced, &SURFACE, &IDENTITY)
            .replace("## 2. Next", &format!("## 2. Next\n\n{LABEL}"));
        assert_eq!(law_problems("x", &text), label);
    }
    assert_eq!(law_problems("x", "# law\n\n## 2. Next\n"), ["x has no §1"]);
    assert_eq!(
        law_problems("x", "# law\n\n## 1. What this is\n\nno second section\n"),
        ["x has no §1"]
    );
    let text = law_text(LABEL, &SURFACE, &IDENTITY).replace("3. **Reproducibility.**", "3. Other.");
    assert_eq!(law_problems("x", &text), ["x has no §3 rule 3"]);
}

/// The ledger reader on invented text: every entry missing, each neighbouring
/// pair swapped, one doubled, the spare headed, an entry before the section or
/// under the next part's slot, FNO-LAW's slot line lost, and D-8108's lines
/// missing, doubled, swapped or malformed, each refused as itself.
#[test]
fn a_ledger_missing_moving_doubling_or_fencing_an_entry_is_refused() {
    let used: Vec<u16> = USED.collect();
    let ops: Vec<u16> = OPEN_POINTS.collect();
    let whole = ledger_text(&used, &ops, &SLOTS);
    assert_eq!(ledger_problems(&whole), Vec::<String>::new());
    for index in 0..used.len() {
        let mut fewer = used.clone();
        let gone = fewer.remove(index);
        let mut want = vec![block_problem(&fewer)];
        if gone == *USED.end() {
            want.push(ops_problem(&[]));
        }
        assert_eq!(ledger_problems(&ledger_text(&fewer, &ops, &SLOTS)), want);
    }
    for index in 1..used.len() {
        let mut swapped = used.clone();
        swapped.swap(index - 1, index);
        let text = ledger_text(&swapped, &ops, &SLOTS);
        assert_eq!(ledger_problems(&text), [block_problem(&swapped)]);
    }
    for extra in [8104, *BLOCK.end()] {
        let mut more = used.clone();
        more.push(extra);
        let text = ledger_text(&more, &ops, &SLOTS);
        assert_eq!(ledger_problems(&text), [block_problem(&more)]);
    }
    let early = whole.replacen("# ledger\n", "# ledger\n\n### D-8100 — too early\n", 1);
    let mut doubled = vec![8100];
    doubled.extend(&used);
    assert_eq!(
        ledger_problems(&early),
        [block_problem(&doubled), outside_problem(8100)]
    );
    let entry = "### D-8104 — an entry\n\nText.\n\n";
    let moved = whole.replacen(entry, "", 1).replacen(
        "<!-- slot FNO-COSTS -->\n\n",
        &format!("<!-- slot FNO-COSTS -->\n\n{entry}"),
        1,
    );
    let mut late: Vec<u16> = used.iter().copied().filter(|&n| n != 8104).collect();
    late.push(8104);
    assert_eq!(
        ledger_problems(&moved),
        [block_problem(&late), outside_problem(8104)]
    );
    let lost = whole.replacen("<!-- slot FNO-LAW -->", "<!-- not a slot -->", 1);
    let rest: Vec<&str> = SLOTS.iter().skip(1).copied().collect();
    let mut want = vec![format!("docs/05-decisions.md's slot lines are {rest:?}")];
    want.extend(used.iter().map(|&n| outside_problem(n)));
    assert_eq!(ledger_problems(&lost), want);
    for changed in [
        without_op(&ops, 16),
        [ops.as_slice(), &[46]].concat(),
        swap_op(&ops, 16),
    ] {
        let text = ledger_text(&used, &changed, &SLOTS);
        assert_eq!(ledger_problems(&text), [ops_problem(&changed)]);
    }
    let mut misread: Vec<u16> = (1..=6).collect();
    misread.push(0);
    misread.extend(8..=46);
    for bad in [
        "- OP-07 a choice.",
        "- OP-7: a choice.",
        "- OP-ab: a choice.",
    ] {
        let text = whole.replacen("- OP-07: a choice.", bad, 1);
        assert_eq!(ledger_problems(&text), [ops_problem(&misread)]);
    }
}

/// `ops` less its `index`th item.
fn without_op(ops: &[u16], index: usize) -> Vec<u16> {
    let mut out = ops.to_vec();
    out.remove(index);
    out
}

/// `ops` with its `index`th item and the one before swapped.
fn swap_op(ops: &[u16], index: usize) -> Vec<u16> {
    let mut out = ops.to_vec();
    out.swap(index - 1, index);
    out
}

/// The heading and fence readers on invented text: only an unfenced heading of
/// two to four `#`, a space, `D-` and exactly four digits heads an entry, and a
/// fence closes only on a run of its own character at least as long with
/// nothing after it.
#[test]
fn a_heading_inside_a_fence_or_of_the_wrong_shape_heads_nothing() {
    let text = format!(
        "<!-- slot FNO-LAW -->\n\n### D-{a} — read\n``not a fence\n\
         ````markdown\n```\n### D-{b} — a shorter run closes no longer fence\n````\n\
         ~~~\n### D-{c} — inside a tilde fence\n```\n~~~ text after the run closes nothing\n\
         ### D-{d} — still inside\n~~~\n\
         ```text\n### D-{e} — inside a backtick fence\n```\n\
         ##### D-{f} — five hashes head nothing\n####D-{f} — no space heads nothing\n\
         # D-{f} — one hash heads nothing\n### D-{f}0 — five digits head nothing\n\
         ### D-{f}x — a letter after heads nothing\n### D-{f}_ — an underscore after heads nothing\n\
         ### D-12 — too short\n### D-81a5 — not digits\n### Cited, never written\n\
         ### D-{g} — after every fence closed, read\n<!-- slot FNO-COSTS -->\n",
        a = 8100,
        b = 8101,
        c = 8102,
        d = 8103,
        e = 8104,
        f = 8105,
        g = 8106
    );
    let got: Vec<(u16, usize)> = entries(&text);
    assert_eq!(got, [(8100, 2), (8106, 26)]);
    assert_eq!(fence("   ~~~~ x"), Some((b'~', 4, " x")));
    assert_eq!(fence(""), None);
}

/// The slot reader on invented text: a missing heading, and a slot missing,
/// swapped or doubled, each refused.
#[test]
fn a_slot_section_missing_its_heading_or_a_slot_is_refused() {
    let name = "x";
    assert_eq!(
        slot_problems(name, &slot_text(&SLOTS)),
        Vec::<String>::new()
    );
    let headless = slot_text(&SLOTS).replace(&slot_heading(), "\n## Another heading\n");
    assert_eq!(
        slot_problems(name, &headless),
        ["x has no slot section heading"]
    );
    let mut swapped = SLOTS.to_vec();
    swapped.swap(4, 5);
    let doubled = [SLOTS.as_slice(), &["FNO-LAW"]].concat();
    for changed in [without(&SLOTS, 23), swapped, doubled] {
        assert_eq!(
            slot_problems(name, &slot_text(&changed)),
            [format!("x's slot lines are {changed:?}")]
        );
    }
}

/// The charter reader on invented text: each cost row and each fact missing
/// alone, a doubled, a swapped and an added row, a cost row with no link or
/// no read date, row T02-02 without the margin file, a missing version or
/// digest file, no scope-lock row and no section, each refused by id.
#[test]
fn a_charter_missing_doubling_or_reordering_a_row_is_refused_by_id() {
    let none: Vec<String> = Vec::new();
    let whole = charter_text(&COST_ROWS, &FACT_ROWS);
    assert_eq!(charter_problems(&whole), none);
    let rows = |costs: &[&str], facts: &[&str]| {
        let all: Vec<&str> = costs.iter().chain(facts).copied().collect();
        format!("the F&O section's rows are {all:?}")
    };
    for index in 0..COST_ROWS.len() {
        let costs = without(&COST_ROWS, index);
        let text = charter_text(&costs, &FACT_ROWS);
        assert_eq!(charter_problems(&text), [rows(&costs, &FACT_ROWS)]);
    }
    for index in 0..FACT_ROWS.len() {
        let facts = without(&FACT_ROWS, index);
        let mut want = vec![rows(&COST_ROWS, &facts)];
        if index == 0 {
            want.push("row T02-02 does not name NSE's margin file".to_owned());
        }
        assert_eq!(charter_problems(&charter_text(&COST_ROWS, &facts)), want);
    }
    let mut swapped = COST_ROWS.to_vec();
    swapped.swap(3, 4);
    let doubled = [COST_ROWS.as_slice(), &["OT-INT"]].concat();
    let added = [COST_ROWS.as_slice(), &["EX-2027"]].concat();
    for costs in [swapped, doubled, added] {
        let text = charter_text(&costs, &FACT_ROWS);
        assert_eq!(charter_problems(&text), [rows(&costs, &FACT_ROWS)]);
    }
    for (from, to, id) in [
        (
            "`https://nsearchives.nseindia.com/GST-1.pdf`",
            "NSE's site",
            "GST-1",
        ),
        (
            "| BRK-SQ | a charge | `https://nsearchives.nseindia.com/BRK-SQ.pdf` | 2026-10-30 |",
            "| BRK-SQ | a charge | `https://nsearchives.nseindia.com/BRK-SQ.pdf` | 2026-10-31 |",
            "BRK-SQ",
        ),
    ] {
        let text = whole.replacen(from, to, 1);
        assert_eq!(
            charter_problems(&text),
            [format!("cost row {id} has no https source or no read date")]
        );
    }
    let unnamed = whole.replacen(
        &format!("| T02-02 | a fact read from {MARGIN_FILE} |"),
        "| T02-02 | a fact |",
        1,
    );
    assert_eq!(
        charter_problems(&unnamed),
        ["row T02-02 does not name NSE's margin file"]
    );
    for needed in [
        "`costs-table/2026-10-30/r1`",
        "`state/fib-strategy-20261030/costs-evidence/sha256-official.txt`",
    ] {
        assert_eq!(
            charter_problems(&whole.replacen(needed, "", 1)),
            [format!("the F&O section does not name {needed}")]
        );
    }
    let unscoped = whole.replacen("| D-8100 |", "| D-0506 |", 1);
    assert_eq!(
        charter_problems(&unscoped),
        ["the scope lock has no row since D-8100"]
    );
    assert_eq!(
        charter_problems("# charter\n\n## 2. Prohibitions\n"),
        [
            "the scope lock has no row since D-8100",
            "the charter has no F&O strategy path section"
        ]
    );
}

/// The rows reader on invented text: a row citing a test this file does not
/// declare, a row citing none, a row missing, an empty slot and a plan slot
/// with no row, each refused by name.
#[test]
fn a_row_citing_a_test_this_file_lacks_or_an_empty_slot_is_refused() {
    let declared = "fn a_test_that_exists() {}";
    let cites = "`core::fno_law::a_test_that_exists`";
    let plan = "| FNO-LAW: the law | built |";
    let (invariants, plans) = rows_texts(&LAW_ROWS, cites, plan);
    assert_eq!(
        rows_problems(&invariants, &plans, declared),
        Vec::<String>::new()
    );
    let (invariants, plans) = rows_texts(&LAW_ROWS, "`core::fno_law::nobody_wrote_it`", plan);
    let want: Vec<String> = LAW_ROWS
        .iter()
        .map(|id| format!("{id} cites nobody_wrote_it, which this file does not declare"))
        .collect();
    assert_eq!(rows_problems(&invariants, &plans, declared), want);
    let (invariants, plans) = rows_texts(&LAW_ROWS, "`core::graph::a_test_that_exists`", plan);
    let want: Vec<String> = LAW_ROWS
        .iter()
        .map(|id| format!("{id} cites no test of this file"))
        .collect();
    assert_eq!(rows_problems(&invariants, &plans, declared), want);
    let fewer = without(&LAW_ROWS, 2);
    let (invariants, plans) = rows_texts(&fewer, cites, plan);
    assert_eq!(
        rows_problems(&invariants, &plans, declared),
        [format!("FNO-LAW's slot of the invariants holds {fewer:?}")]
    );
    let (invariants, plans) = rows_texts(&[], cites, "");
    assert_eq!(
        rows_problems(&invariants, &plans, declared),
        [
            "FNO-LAW's slot of the invariants holds []".to_owned(),
            "FNO-LAW's slot of the plan holds no plan row".to_owned()
        ]
    );
    assert_eq!(
        rows_problems("", "", declared),
        [
            "FNO-LAW's slot of the invariants holds []".to_owned(),
            "FNO-LAW's slot of the plan holds no plan row".to_owned()
        ]
    );
}

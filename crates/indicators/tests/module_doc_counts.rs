//! Every module header that quotes a position count is checked against the count.
//!
//! # The drift this closes, measured
//!
//! `crates/indicators/tests/shared_core_doc.rs` is a working doc-count gate and
//! it covers exactly one document: `docs/10-shared-core.md`. Every other quoted
//! count in this crate lives in a MODULE DOC COMMENT, and nothing checked any of
//! them. An adversarial audit measured two that had drifted:
//!
//! | Module | Header claimed | `positions()` returned |
//! |---|---:|---:|
//! | `daily.rs` | 41 | **44** |
//! | `fib.rs` | 26 | **27** |
//!
//! `fib.rs` was the clearer of the two: its header said 26 and **its own table,
//! two lines below, listed 9 + 7 + 11**. The document contradicted itself and
//! shipped that way. `daily.rs` conflated two real numbers — `plan()` has 41
//! entries and `positions()` returns those 41 plus three CPR-width states.
//!
//! # Why a test and not a careful edit
//!
//! Correcting the numbers fixes today. This fixes the class: the header is now
//! read from disk and compared against the array width the compiler enforces, so
//! a module that grows a position and does not say so fails the build.
//!
//! # What this does NOT cover
//!
//! Only the leading `**N vocabulary positions**` claim of each module listed
//! below. A count quoted mid-file, or in a different phrasing, is still
//! unchecked — and `crates/runner/src/rank.rs` quotes struct sizes that no test
//! reads. Those are named in `docs/11-findings.md` rather than silently left.

// The exception every test in this workspace takes, and here for the reason
// the crate's own test modules give at length: `panic!` expands to a panic
// written IN THIS CRATE, which `cargo llvm-cov` counts as a region no passing
// run can enter. `expect` panics inside core, which is not instrumented -- the
// same failure, the same message, and no dead region left behind.
#![allow(
    clippy::expect_used,
    reason = "a test that cannot fail cannot check anything, and `expect` leaves \
              no uncoverable region behind the way `panic!` does."
)]

use std::fs;

/// One module, the file to read, and the count its own `positions()` returns.
///
/// The right-hand number is NOT a literal repeated from the source. It is the
/// length of the array `positions()` returns, which the compiler enforces
/// against the ladders that build it — so this table cannot drift from the code
/// even if the header does.
fn modules() -> Vec<(&'static str, &'static str, usize)> {
    vec![
        (
            "daily",
            "src/daily.rs",
            indicators::daily::positions().len(),
        ),
        ("fib", "src/fib.rs", indicators::fib::positions().len()),
        ("orb", "src/orb.rs", indicators::orb::positions().len()),
        (
            "pattern",
            "src/pattern.rs",
            indicators::pattern::positions().len(),
        ),
        (
            "session",
            "src/session.rs",
            indicators::session::positions().len(),
        ),
        ("vwap", "src/vwap.rs", indicators::vwap::positions().len()),
        // `gap` and `trend` state their count in the checked form and were not
        // read until D-0947.
        (
            "gap",
            "src/gap.rs",
            indicators::gap::GapFib::positions().len(),
        ),
        (
            "trend",
            "src/trend.rs",
            indicators::trend::TrendState::positions().len(),
        ),
    ]
}

/// The number a module's header claims, if it makes the claim in the checked
/// form.
///
/// Reads the leading `//!` block only. A module that does not state a count is
/// reported as `None` and handled by the caller, rather than defaulted to zero —
/// "did not say" and "said zero" are different facts and the second would pass
/// silently.
fn claimed(source: &str) -> Option<usize> {
    for line in source.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//!") {
            // The header block has ended. Stop rather than scanning the whole
            // file: a count quoted inside a function's doc is a different claim
            // about a different thing.
            if trimmed.is_empty() {
                continue;
            }
            break;
        }
        let Some(at) = trimmed.find("**") else {
            continue;
        };
        let rest = trimmed.get(at.saturating_add(2)..)?;
        let Some(end) = rest.find(' ') else { continue };
        let word = rest.get(..end)?;
        if let Ok(n) = word.parse::<usize>()
            && rest.contains("vocabulary position")
        {
            return Some(n);
        }
    }
    None
}

#[test]
fn every_module_header_states_the_position_count_it_actually_emits() {
    let mut checked = 0_usize;
    for (name, path, actual) in modules() {
        let source = fs::read_to_string(path)
            .expect("every listed module must be readable from the crate root");
        let stated = claimed(&source);
        assert!(
            stated.is_some(),
            "{name}: the header of {path} no longer states a position count in the \
             form `**N vocabulary positions**`. Either restore the claim or remove \
             this module from the list -- a header that quietly stops making the \
             claim is how the check stops checking."
        );
        let stated = stated.expect("asserted to be Some on the line above");
        assert_eq!(
            stated, actual,
            "{name}: the header of {path} claims {stated} vocabulary positions and \
             `positions()` returns {actual}. This is the drift that shipped twice: \
             daily.rs said 41 against 44, and fib.rs said 26 against 27 while its \
             own table listed 9 + 7 + 11."
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 8,
        "eight modules state a count and all eight must be read; a module dropped from \
         the list is a module nobody checks"
    );
}

/// The crate header's leading `//!` block, one line per entry, with the `//!`
/// marker and one following space removed.
fn crate_header() -> Vec<String> {
    let source = fs::read_to_string("src/lib.rs").expect("the crate root is readable");
    source
        .lines()
        .map_while(|line| line.strip_prefix("//!"))
        .map(|rest| rest.strip_prefix(' ').unwrap_or(rest).to_owned())
        .collect()
}

/// `"7–18, 54–55, 60"` as the positions it names. `None` for anything that is
/// not a comma-separated list of numbers and en-dash (or hyphen) ranges, so a
/// cell this parser cannot read fails the test instead of reading as empty.
fn parse_ranges(cell: &str) -> Option<Vec<u16>> {
    let mut out = Vec::new();
    for part in cell.split(',') {
        let part = part.trim();
        let (lo, hi) = if let Some((lo, hi)) = part.split_once('–').or_else(|| part.split_once('-'))
        {
            (
                lo.trim().parse::<u16>().ok()?,
                hi.trim().parse::<u16>().ok()?,
            )
        } else {
            let one = part.parse::<u16>().ok()?;
            (one, one)
        };
        if lo > hi {
            return None;
        }
        out.extend(lo..=hi);
    }
    out.sort_unstable();
    Some(out)
}

/// Every row of the header's `| Source | Positions | Count |` table, as the
/// positions it lists and the count it states.
fn header_rows(header: &[String]) -> Vec<(String, Vec<u16>, usize)> {
    let mut rows = Vec::new();
    let mut inside = false;
    for line in header {
        if line.starts_with("| Source | Positions | Count |")
            || line.starts_with("| Module | Positions | Count |")
        {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if !line.starts_with('|') {
            break;
        }
        if line.starts_with("|---") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        assert_eq!(cells.len(), 3, "a header table row has three cells: {line}");
        let name = (*cells.first().expect("three cells")).to_owned();
        let positions = parse_ranges(cells.get(1).expect("three cells"))
            .expect("the Positions cell is a list of numbers and ranges");
        let count = cells
            .get(2)
            .expect("three cells")
            .parse::<usize>()
            .expect("the Count cell is a number");
        rows.push((name, positions, count));
    }
    rows
}

/// The number pair in `**N of the vocabulary's M live positions are computable`.
fn header_summary(header: &[String]) -> Option<(usize, usize)> {
    let joined = header.join(" ");
    let at = joined.find(" of the vocabulary's ")?;
    let before = joined.get(..at)?;
    let stated = before.rsplit("**").next()?.trim().parse::<usize>().ok()?;
    let after = joined.get(at.saturating_add(" of the vocabulary's ".len())..)?;
    let (live, rest) = after.split_once(' ')?;
    if !rest.starts_with("live positions are computable") {
        return None;
    }
    Some((stated, live.parse::<usize>().ok()?))
}

/// **The crate header's position table and its summary are the code's.**
///
/// # The drift this closes (cloud audits ET-indicators-7, D-0947)
///
/// `src/lib.rs` said **eleven** of 232 live positions were computable in its
/// first paragraph and **52** of 232 in its summary, while its own table summed
/// to 205, left out `gap`, `trend`, the crossings and the weekday rows, and
/// `Evaluator::positions()` returned every live position in the table. The test
/// above reads six module headers and never read this one.
///
/// It now checks, against the code: every table row's Count is the number of
/// positions it lists; no position is listed twice; the rows together are
/// exactly `Evaluator::positions()`; each module's own `positions()` (and the
/// current-day ladder, and `vocab::table::CROSSINGS`) is exactly one row, so a
/// family cannot be dropped or folded into another; and the summary's two
/// numbers are `Evaluator::positions().len()` and the live mask's population.
#[test]
fn the_crate_header_table_and_summary_are_the_code() {
    let header = crate_header();
    let rows = header_rows(&header);
    assert!(
        !rows.is_empty(),
        "src/lib.rs no longer carries the `| Source | Positions | Count |` table, so \
         nothing here is checked"
    );

    let mut listed: Vec<u16> = Vec::new();
    for (name, positions, count) in &rows {
        assert_eq!(
            positions.len(),
            *count,
            "lib.rs row {name}: Count says {count} and the Positions cell lists {}",
            positions.len()
        );
        listed.extend(positions.iter().copied());
    }
    let total = listed.len();
    listed.sort_unstable();
    listed.dedup();
    assert_eq!(
        listed.len(),
        total,
        "lib.rs lists some position in two rows"
    );

    let emitted = indicators::evaluator::Evaluator::positions();
    assert_eq!(
        listed, emitted,
        "lib.rs's table and `Evaluator::positions()` name different positions"
    );

    let curday_last = indicators::CURDAY_FIRST
        + u16::try_from(indicators::CURDAY_RUNGS.len()).expect("eleven rungs")
        - 1;
    let mut crossings: Vec<u16> = vocab::table::CROSSINGS
        .iter()
        .flat_map(|c| [c.up, c.down, c.first, c.second, c.later])
        .collect();
    crossings.sort_unstable();
    let sources: Vec<(&str, Vec<u16>)> = vec![
        (
            "CurDayFib",
            (indicators::CURDAY_FIRST..=curday_last).collect(),
        ),
        ("daily", indicators::daily::positions().to_vec()),
        ("fib", indicators::fib::positions().to_vec()),
        ("orb", indicators::orb::positions().to_vec()),
        ("pattern", indicators::pattern::positions().to_vec()),
        ("session", indicators::session::positions().to_vec()),
        ("vwap", indicators::vwap::positions().to_vec()),
        ("gap", indicators::gap::GapFib::positions().to_vec()),
        ("trend", indicators::trend::TrendState::positions().to_vec()),
        ("crossings", crossings),
    ];
    for (source, mut positions) in sources {
        positions.sort_unstable();
        let matching = rows.iter().filter(|(_, row, _)| *row == positions).count();
        assert_eq!(
            matching,
            1,
            "{source}'s own positions() ({} positions) is not exactly one row of \
             lib.rs's table",
            positions.len()
        );
    }

    let live = usize::try_from(vocab::table::LIVE.popcount()).expect("a small count");
    assert_eq!(
        header_summary(&header),
        Some((emitted.len(), live)),
        "lib.rs's summary `**N of the vocabulary's M live positions are computable` \
         must state N = Evaluator::positions().len() = {} and M = the live mask's \
         population = {live}",
        emitted.len()
    );
}

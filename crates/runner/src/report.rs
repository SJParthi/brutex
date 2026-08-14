//! One sweep, rendered so a human can audit it without reading the code.
//!
//! # Why a report, and why here
//!
//! CI gate 17 forbids `crates/vocab`, `crates/indicators` and `crates/engine`
//! from emitting anything at all: the sweep evaluates a mask per (bar,
//! combination), and a billion O(1) calls is still a billion calls. What it
//! prescribes instead is **plain integer counters, incremented in the loop with
//! no atomic and no allocation, emitted ONCE at a structural boundary**.
//!
//! `crates/runner` is not on gate 17's list, and this is that structural
//! boundary. Every number below was already counted by the sweep; nothing here
//! measures anything, it only renders what the walk carried out. So the audit
//! costs one string per run rather than anything per bar.
//!
//! # What it is for
//!
//! A [`Sweep`] read without its census cannot tell "no combination was frequent"
//! from "almost every bar was refused as corrupt", and a halted walk read
//! without [`engine::Halt`] looks like a complete one. `CLAUDE.md` §4 bans a
//! result that hides a failure behind a success, so the report puts the refusals,
//! the warm-up loss and the halt reason beside the answer rather than in a
//! different place a reader has to think to look.
//!
//! # Plain text on purpose
//!
//! No JSON, no HTML, no dependency. A sweep is audited from a terminal or a log
//! file, and both of those are text. `web/` is where a browser view belongs and
//! this crate cannot reach it.

use core::fmt::Write as _;

use crate::identity::RunId;
use crate::{Auto, Outcome};
use engine::Sweep;
use indicators::column::Census;

/// Column width for the label side of every row.
const LABEL: usize = 34;

/// One line: a label, a value, and an optional note.
fn row(out: &mut String, label: &str, value: &str, note: &str) {
    let _ = writeln!(out, "  {label:<LABEL$}{value:>14}  {note}");
}

/// A percentage in tenths, computed in integers.
///
/// `CLAUDE.md` §7 forbids a float here as firmly as it does in the engine: a
/// report that disagreed with the counters it renders would be worse than no
/// report. Tenths are enough to read and exact to compute.
fn permille(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "-".to_owned();
    }
    let tenths = part.saturating_mul(1_000) / whole;
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// Where every offered bar went, and what the ladder did with what survived.
///
/// The identity is optional because a caller may want the audit before it has
/// assembled one — but `CLAUDE.md` §3 rule 3 says no computation runs without an
/// identity recorded, so a report without one says so in place of the digest
/// rather than leaving the line out.
#[must_use]
pub fn render(outcome: &Outcome, id: Option<&RunId>) -> String {
    let mut out = String::with_capacity(2_048);

    let _ = writeln!(out, "SWEEP");
    let _ = writeln!(
        out,
        "  {:<LABEL$}{:>14}",
        "identity",
        id.map_or_else(|| "NOT RECORDED".to_owned(), RunId::hex)
    );
    let _ = writeln!(out);

    bars(&mut out, outcome.census);
    ladder(&mut out, &outcome.sweep);
    verdict(&mut out, outcome);
    out
}

/// Where every offered bar went.
fn bars(out: &mut String, census: Census) {
    let _ = writeln!(out, "BARS");
    row(out, "offered", &census.offered.to_string(), "");
    row(
        out,
        "swept",
        &census.swept.to_string(),
        &permille(census.swept, census.offered),
    );
    row(
        out,
        "warming (bits correct, not swept)",
        &census.warming.to_string(),
        &permille(census.warming, census.offered),
    );
    row(out, "refused", &census.refused().to_string(), "");
    // Every refusal reason, named. A total alone cannot tell a corrupt feed from
    // a single bad bar, which is the distinction §4 is about.
    for (name, n) in [
        ("  high below low", census.high_below_low),
        ("  range overflows", census.range_overflows),
        ("  price outside range", census.price_outside_range),
        (
            "  timestamp not increasing",
            census.timestamp_not_increasing,
        ),
        ("  negative volume", census.negative_volume),
        ("  accumulator too large", census.accumulator_too_large),
    ] {
        if n > 0 {
            row(out, name, &n.to_string(), "");
        }
    }
    row(
        out,
        "every bar accounted for",
        if census.reconciles() { "yes" } else { "NO" },
        if census.reconciles() {
            ""
        } else {
            "<- a bar was lost; this is a defect, not data"
        },
    );
    let _ = writeln!(out);
}

/// What the ladder did with the bars that survived.
fn ladder(out: &mut String, sweep: &Sweep) {
    let _ = writeln!(out, "LADDER");
    row(out, "min_hits applied", &sweep.min_hits.to_string(), "");
    row(
        out,
        "support that requires",
        &permille(sweep.min_hits, sweep.bars),
        "of swept bars",
    );
    row(out, "depth reached", &sweep.depth().to_string(), "");
    row(
        out,
        "combinations found",
        &sweep.all_frequent().count().to_string(),
        "",
    );
    row(
        out,
        "positions excluded before k=1",
        &sweep.excluded.len().to_string(),
        "always-true, always-false or not live",
    );
    let _ = writeln!(out);

    // ── per level ───────────────────────────────────────────────────────────
    // Every column `Frontier::reconciles` sums, in the order it sums them, so a
    // reader can add the row up on the page. Dropping `excluded` — which is only
    // ever non-zero at k=1 — would make that one row appear to lose 161
    // candidates, which is precisely the kind of unexplained gap this whole
    // report exists to prevent.
    let _ = writeln!(
        out,
        "  {:<6}{:>12}{:>12}{:>10}{:>10}{:>12}{:>10}",
        "level", "generated", "duplicate", "excluded", "pruned", "infrequent", "frequent"
    );
    for level in &sweep.levels {
        let _ = writeln!(
            out,
            "  k={:<4}{:>12}{:>12}{:>10}{:>10}{:>12}{:>10}{}",
            level.k,
            level.generated,
            level.duplicates,
            level.excluded,
            level.pruned,
            level.infrequent,
            level.frequent.len(),
            if level.reconciles() {
                ""
            } else {
                "  <- LOST A CANDIDATE"
            }
        );
    }
    let _ = writeln!(out);
}

/// Whether the answer may be believed as a whole.
fn verdict(out: &mut String, outcome: &Outcome) {
    let sweep = &outcome.sweep;
    let _ = writeln!(out, "VERDICT");
    match sweep.halted {
        None => row(
            out,
            "outcome",
            "complete",
            "the frontier went extinct, which is the answer",
        ),
        Some(halt) => {
            row(out, "outcome", "REFUSED", "the walk stopped short");
            row(
                out,
                "  budget spent",
                match halt.breach {
                    engine::Breach::Candidates => "candidates",
                    engine::Breach::Pairs => "pairs",
                },
                "",
            );
            row(out, "  at level", &format!("k={}", halt.k), "");
            row(
                out,
                "  candidates admitted",
                &halt.candidates.to_string(),
                &format!("of {}", halt.ceiling),
            );
            row(
                out,
                "  pairs walked",
                &halt.pairs.to_string(),
                &format!("of {}", halt.pair_budget),
            );
        }
    }
    row(
        out,
        "trustworthy as a whole answer",
        if outcome.is_complete() { "yes" } else { "NO" },
        "",
    );
}

/// The same, for a run the engine tuned for itself.
///
/// Adds what the search did — which thresholds it tried and which it refused —
/// because a threshold chosen by machine needs more explaining than one a caller
/// passed in, not less.
#[must_use]
pub fn render_auto(auto: &Auto, id: Option<&RunId>) -> String {
    let mut out = render(&auto.outcome, id);
    let _ = writeln!(out);
    let _ = writeln!(out, "SEARCH");
    row(
        &mut out,
        "threshold chosen",
        &auto
            .min_hits
            .map_or_else(|| "NONE".to_owned(), |m| m.to_string()),
        if auto.affordable {
            ""
        } else {
            "<- no rung measured anything"
        },
    );
    row(
        &mut out,
        "ladders walked",
        &auto.attempts.to_string(),
        "one per halving",
    );
    row(
        &mut out,
        "refused below",
        &auto
            .refused_below
            .map_or_else(|| "-".to_owned(), |m| m.to_string()),
        "this column cannot afford lower",
    );
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail, and `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind."
)]
mod tests {
    use super::{Outcome, permille, render, render_auto};
    use crate::identity::{Direction, Params, Run, data_digest, identity};
    use crate::{Sweeper, synthetic};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use engine::{Frontier, Ladder, Sweep};
    use indicators::column::Census;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use vocab::ConditionMask;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("both pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn bounded() -> Ladder {
        Ladder::with_min_hits(600).with_ceiling(50_000)
    }

    #[test]
    fn permille_is_integer_arithmetic_and_handles_the_empty_case() {
        assert_eq!(permille(0, 0), "-", "no denominator, no percentage");
        assert_eq!(permille(1, 2), "50.0%");
        assert_eq!(permille(1, 3), "33.3%");
        assert_eq!(permille(0, 100), "0.0%");
        assert_eq!(permille(100, 100), "100.0%");
    }

    #[test]
    fn a_complete_sweep_reports_every_section() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        for section in ["SWEEP", "BARS", "LADDER", "VERDICT"] {
            assert!(text.contains(section), "missing section {section}");
        }
        assert!(text.contains("NOT RECORDED"), "an absent identity is named");
        assert!(text.contains("complete"), "an extinct ladder is complete");
        assert!(text.contains("every bar accounted for"));
        // Every level is a row.
        for level in &out.sweep.levels {
            assert!(
                text.contains(&format!("k={}", level.k)),
                "level {} is missing from the table",
                level.k
            );
        }
    }

    #[test]
    fn every_level_row_adds_up_when_read_off_the_page() {
        // The report's claim is not "the counters reconcile" -- `reconciles()`
        // already says that inside the engine. The claim is that a HUMAN reading
        // the rendered table can verify it without trusting the code. So this
        // test parses the rendered text back, exactly as a reader would, and
        // adds the row up. A column silently dropped from the format string
        // passes every other test in this file and fails this one.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        let mut rows = 0_u32;
        for line in text.lines().filter(|l| l.trim_start().starts_with("k=")) {
            let n: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|f| f.parse().ok())
                .collect();
            assert_eq!(n.len(), 6, "a level row must print six counters: {line}");
            let (generated, rest) = n.split_first().expect("six is not zero");
            assert_eq!(
                *generated,
                rest.iter().sum::<u64>(),
                "this row does not add up on the page, so a reader auditing it \
                 would see candidates vanish: {line}"
            );
            rows = rows.saturating_add(1);
        }
        assert!(
            rows > 1,
            "the fixture must produce a real ladder, not one rung"
        );
    }

    #[test]
    fn a_refusal_names_the_budget_it_spent() {
        let bars = synthetic::sessions(8);
        let tight = Ladder::with_min_hits(2).with_pair_budget(50_000);
        let out = Sweeper::new(tight).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert!(text.contains("REFUSED"), "a halt must be loud");
        assert!(text.contains("pairs"), "and must name which budget");
        assert!(text.contains("trustworthy as a whole answer"));
        assert!(text.contains("NO"), "a halted sweep is not trustworthy");
    }

    #[test]
    fn a_refused_bar_is_named_by_its_reason() {
        let mut bars = synthetic::sessions(8);
        let last = bars.last().copied().expect("the run is not empty");
        bars.push(crate::candle(
            last.ts_micros + 60_000_000,
            last.close,
            last.close - 10,
            last.close + 10,
            last.close,
        ));
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert!(
            text.contains("high below low"),
            "a refusal reason must appear by name, not just in a total"
        );
    }

    /// A census that has lost a bar, built by hand.
    ///
    /// `Column::build` cannot produce one — every bar takes exactly one arm of an
    /// exhaustive match — which is why the branch is unreachable through the
    /// normal path and why it still has to be tested. The report's whole job is
    /// to be loud when an invariant breaks, and a detector nobody has ever seen
    /// fire is a detector nobody knows works.
    fn broken(census: Census, levels: Vec<Frontier>) -> Outcome {
        Outcome {
            census,
            first_swept: Some(0),
            sweep: Sweep {
                levels,
                excluded: Vec::new(),
                bars: census.swept,
                min_hits: 1,
                halted: None,
            },
        }
    }

    #[test]
    fn a_lost_bar_is_reported_as_a_defect_and_not_as_data() {
        let census = Census {
            offered: 10,
            swept: 4,
            warming: 1,
            ..Census::default()
        };
        assert!(!census.reconciles(), "the fixture must actually be broken");
        let text = render(&broken(census, Vec::new()), None);

        assert!(
            text.contains("a bar was lost; this is a defect, not data"),
            "a census that does not add up must say so in the report, not \
             silently render a smaller total"
        );
        assert!(
            text.contains("trustworthy as a whole answer") && text.contains("NO"),
            "and the verdict must carry it, not just the BARS section"
        );
    }

    #[test]
    fn a_lost_candidate_is_reported_on_the_level_that_lost_it() {
        // generated 99, accounted for 0.
        let level = Frontier {
            k: 1,
            generated: 99,
            ..Frontier::default()
        };
        assert!(!level.reconciles(), "the fixture must actually be broken");
        let text = render(&broken(Census::default(), vec![level]), None);

        assert!(
            text.contains("LOST A CANDIDATE"),
            "a level whose counters do not add up must be marked on its own row, \
             so a reader sees WHICH level rather than only that something is wrong"
        );
    }

    #[test]
    fn a_candidate_ceiling_breach_names_candidates_and_not_pairs() {
        let bars = synthetic::sessions(8);
        // Ten admitted candidates is far below the live position count, so the
        // ceiling binds at k=1 and the pair budget is never approached.
        let tight = Ladder::with_min_hits(2).with_ceiling(10);
        let out = Sweeper::new(tight).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert!(text.contains("REFUSED"));
        assert!(
            text.contains("candidates"),
            "the report must name the budget that was actually spent; the pair \
             budget was untouched and saying 'pairs' here would misdirect"
        );
    }

    #[test]
    fn the_identity_is_rendered_when_there_is_one() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("swept");
        let id = identity(&Run {
            mask: ConditionMask::default(),
            direction: Direction::Undirected,
            instrument: &key,
            timeframe: "1min",
            params: Params::of(bounded()),
            data_digest: data_digest(&bars),
            commit: "0123456789abcdef",
        });
        let text = render(&out, Some(&id));

        assert!(text.contains(&id.hex()), "the digest must be readable");
        assert!(!text.contains("NOT RECORDED"));
    }

    #[test]
    fn the_search_is_reported_for_a_tuned_run() {
        let bars = synthetic::sessions(8);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
        let text = render_auto(&auto, None);

        assert!(text.contains("SEARCH"));
        assert!(text.contains("ladders walked"));
        assert!(text.contains("threshold chosen"));
    }

    #[test]
    fn a_cold_run_reports_no_threshold_and_says_why() {
        let bars = synthetic::sessions(1);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
        let text = render_auto(&auto, None);

        assert!(text.contains("NONE"), "no threshold is stated as NONE");
        assert!(
            text.contains("no rung measured anything"),
            "and the reason is given rather than left to be inferred"
        );
        assert!(text.contains("warming"), "the warm-up loss is visible");
    }

    #[test]
    fn an_empty_run_renders_without_dividing_by_zero() {
        let auto = Sweeper::new(bounded()).auto(&[], &mut evaluator());
        let text = render_auto(&auto, None);
        assert!(text.contains('-'), "an empty denominator renders as a dash");
        assert!(!text.is_empty());
    }
}

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
//!
//! # Cost
//!
//! One `String` per run, sized once and written once. The render is linear in
//! the number of LEVELS — twelve on the fixture — and touches no bar and no
//! candidate, which is the whole reason it may exist outside gate 17's silence.
//!
//! Measured by `C-R-02` in `crates/runner/benches/ratio.rs`, with both ladders
//! neutered to a single level so the level count is held equal and only the
//! column varies: 1,124 against 10,124 swept bars, 0.914x. The row began as a
//! per-LEVEL cost and read 0.197x, which is the fixed prologue amortising over
//! a deeper ladder and says nothing whatever about bars.

use core::fmt::Write as _;

use crate::identity::RunId;
use crate::rank::Ranked;
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
    significance(&mut out, &outcome.sweep);
    verdict(&mut out, outcome);
    out
}

/// How large a t-statistic must be before it beats the best of luck.
///
/// Printed beside the answer rather than left for a reader to look up, because
/// the whole point of the number is that it is easy to forget: a sweep that
/// tested sixty-one million hypotheses has a noise floor near t = 6, and every
/// habit a reader brings from hand-built strategies says 2 or 3 is fine.
///
/// Derived entirely from this run's own counters — nobody is asked for a
/// number, the same standard depth and memory are held to.
fn significance(out: &mut String, sweep: &Sweep) {
    let raw = crate::significance::trials(sweep);
    // The EFFECTIVE count is what the bar is computed from. Two masks with
    // identical support are one hypothesis counted twice, and charging for a
    // trial nobody ran raises the bar against real findings.
    let n = crate::significance::effective_trials(sweep);
    let _ = writeln!(out, "SIGNIFICANCE");
    row(
        out,
        "hypotheses tested",
        &raw.to_string(),
        "support counted against bars",
    );
    row(
        out,
        "  distinct tests among them",
        &n.to_string(),
        "exact duplicates removed -- same support, same test",
    );
    if n < 2 {
        row(
            out,
            "threshold",
            "-",
            "too few hypotheses to have a noise floor",
        );
        let _ = writeln!(out);
        return;
    }
    row(
        out,
        "best t-stat by luck alone",
        &format!("{:.2}", crate::significance::expected_max_bailey(n)),
        "Bailey & Lopez de Prado, if every hypothesis were worthless",
    );
    row(
        out,
        "  the sqrt(2 ln N) approximation",
        &format!("{:.2}", crate::significance::expected_max_t(n)),
        "the figure usually quoted -- it OVERSTATES the floor",
    );
    row(
        out,
        "t required (Bonferroni 5%)",
        &format!("{:.2}", crate::significance::bonferroni_t(n)),
        "Harvey, Liu & Zhu: their floor is 3.0 for a HANDFUL of trials",
    );
    let _ = writeln!(out);
}

/// A statistical mean, rendered as the paisa integer §7 keeps money in.
///
/// `f64 as i64` truncates silently outside the integer range. No mean of paisa
/// closes can reach that, and printing a wrapped number rather than refusing is
/// the class of defect this repository keeps finding, so the conversion
/// saturates and says so.
#[allow(
    clippy::cast_possible_truncation,
    reason = "saturated at the i64 bounds immediately below; a paisa mean \
              outside them cannot arise from a price series."
)]
fn paisa(mean: f64) -> i64 {
    // 2^62 as a literal, comfortably inside both f64 exactness and i64 range,
    // so the comparison needs no lossy i64 -> f64 cast of its own. A paisa mean
    // beyond it is 46 quadrillion rupees and cannot arise from a price series.
    const UPPER: f64 = 4_611_686_018_427_387_904.0;
    // Written as a negative literal, not as `-UPPER`: negation is arithmetic,
    // and this module takes no float-arithmetic exception -- the one place that
    // rule bites is where a bound is expressed rather than computed.
    const LOWER: f64 = -4_611_686_018_427_387_904.0;
    let r = mean.round();
    if r >= UPPER {
        i64::MAX
    } else if r <= LOWER {
        i64::MIN
    } else {
        r as i64
    }
}

/// The kept combinations, each beside the bar it had to clear.
///
/// # Why the bar is printed on every row and not once at the top
///
/// The whole failure this guards against is a reader seeing `t = 4.1`, recalling
/// that three is the threshold they have always used, and stopping there. The
/// bar is a property of **how many hypotheses this run tested** — above six on a
/// sixty-one-million sweep — and it is printed against each row so the
/// comparison cannot be skipped.
///
/// A row that does not clear it is not a weak finding. It is **indistinguishable
/// from the best of pure noise**, and the verdict column says exactly that.
///
/// # What this table looks like on the synthetic fixture, and why that is a trap
///
/// `synthetic::sessions` generates a deterministic upward DRIFT. Every condition
/// that fires therefore "predicts" a rise, and the whole table clears the bar at
/// t values above twenty. That is the pipeline working -- it measures what it is
/// given -- and it is emphatically **not** an edge. On bars with no drift these
/// collapse toward zero, which is the entire reason the bar is printed.
///
/// A reader who sees t = 23 here and concludes the engine found gold has made
/// the mistake every part of this module exists to prevent.
#[must_use]
pub fn render_findings(ranked: &Ranked, sweep: &Sweep) -> String {
    let mut out = String::with_capacity(1_024);
    let n = crate::significance::trials(sweep);
    let bar = crate::significance::bonferroni_t(n);

    let _ = writeln!(out, "FINDINGS");
    row(
        &mut out,
        "combinations weighed",
        &ranked.considered.to_string(),
        "",
    );
    row(
        &mut out,
        "kept",
        &ranked.top.len().to_string(),
        "best by |t|",
    );
    row(
        &mut out,
        "bar every row must clear",
        &format!("{bar:.2}"),
        "Bonferroni 5% on this run's own trial count",
    );
    let _ = writeln!(out);

    if ranked.top.is_empty() {
        let _ = writeln!(out, "  nothing kept — the sweep produced no combination");
        let _ = writeln!(out);
        return out;
    }

    let _ = writeln!(
        out,
        "  {:<6}{:>10}{:>10}{:>14}{:>9}  verdict",
        "rank", "hits", "n", "mean paisa", "t"
    );
    for (index, s) in ranked.top.iter().enumerate() {
        let clears = s.edge.t.abs() >= bar;
        let _ = writeln!(
            out,
            "  {:<6}{:>10}{:>10}{:>14}{:>9.2}  {}",
            index.saturating_add(1),
            s.hits,
            s.edge.n,
            // The mean is a paisa figure and is printed as one: §7 keeps money
            // in integers, and a fractional paisa is not a price. `saturating`
            // rather than a bare cast: a mean outside i64 cannot arise from
            // paisa closes, and a truncating cast would print a wrapped number
            // rather than refuse.
            paisa(s.edge.mean_paisa),
            s.edge.t,
            if clears {
                "clears"
            } else {
                "BELOW THE BAR — indistinguishable from luck"
            }
        );
    }
    let _ = writeln!(out);
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
        // Extinction is a MEASUREMENT, and it may only be reported when one was
        // taken. `halted == None` is true for a sweep that finished and equally
        // true for one that never started: a cold column, a column where every
        // bar was refused, and the `Sweep::default()` that `Sweeper::auto`
        // substitutes when its search keeps no rung. All three used to print
        // "the frontier went extinct, which is the answer" -- an answer nobody
        // computed, which is the §4 breach this module's own header claims to
        // exist to prevent, and which no test here caught.
        None if sweep.levels.is_empty() || sweep.bars == 0 => row(
            out,
            "outcome",
            "NOTHING MEASURED",
            "no ladder was walked -- this is not extinction",
        ),
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
                    // Not a budget anyone set. The machine refused.
                    engine::Breach::Memory => "MEMORY",
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

    /// The VALUE cell of a labelled row — what a reader's eye lands on.
    ///
    /// `text.contains("NO")` is satisfied by "NOT RECORDED", which every report
    /// rendered with `id = None` prints at the top; `text.contains("pairs")` is
    /// satisfied by the "  pairs walked" row, which `verdict` prints for EVERY
    /// halt whatever budget was breached. Two assertions in this file were
    /// therefore tautologies and a third could not distinguish the two halt
    /// kinds — an adversarial audit found all three, and none of them could have
    /// failed on any input. Reading the one cell under test is the fix.
    fn cell(text: &str, label: &str) -> String {
        text.lines()
            .find(|l| l.contains(label))
            .and_then(|l| l.split_once(label))
            .map(|(_, rest)| rest.split_whitespace().next().unwrap_or("").to_owned())
            .unwrap_or_default()
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

        assert_eq!(cell(&text, "outcome"), "REFUSED", "a halt must be loud");
        assert_eq!(
            cell(&text, "budget spent"),
            "pairs",
            "and must name the budget that was actually spent -- the ceiling was \
             never approached here"
        );
        assert_eq!(
            cell(&text, "trustworthy as a whole answer"),
            "NO",
            "a halted sweep is not trustworthy"
        );
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
        assert_eq!(
            cell(&text, "every bar accounted for"),
            "NO",
            "the BARS cell must say NO -- the earlier form asserted \
             `text.contains(\"NO\")`, which \"NOT RECORDED\" satisfies on every \
             report rendered without an identity"
        );
        assert_eq!(
            cell(&text, "trustworthy as a whole answer"),
            "NO",
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

        assert_eq!(cell(&text, "outcome"), "REFUSED");
        assert_eq!(
            cell(&text, "budget spent"),
            "candidates",
            "the report must name the budget that was actually spent; the pair \
             budget was untouched and saying 'pairs' here would misdirect. The \
             earlier form asserted `text.contains(\"candidates\")`, which the \
             unconditional \"  candidates admitted\" row satisfies for a PAIR \
             breach too -- it could not fail either way"
        );
    }

    #[test]
    fn a_machine_refusal_is_reported_as_memory_and_not_as_a_budget() {
        // `Breach::Memory` is not a number anyone chose, and the report must not
        // dress it as one. Built by hand because a genuine allocation failure is
        // unreachable from a fixture -- the same reason `Ladder::exhausted` takes
        // a growth amount.
        let mut out = broken(Census::default(), Vec::new());
        out.sweep.bars = 1;
        out.sweep.halted = Some(engine::Halt {
            k: 7,
            candidates: 12_345,
            ceiling: engine::DEFAULT_CEILING,
            pairs: 99,
            pair_budget: engine::DEFAULT_PAIR_BUDGET,
            breach: engine::Breach::Memory,
        });
        let text = render(&out, None);

        assert_eq!(cell(&text, "outcome"), "REFUSED");
        assert_eq!(
            cell(&text, "budget spent"),
            "MEMORY",
            "the machine refusing an allocation is not a budget being spent, and \
             calling it `candidates` would name a ceiling that was never reached"
        );
        assert_eq!(cell(&text, "trustworthy as a whole answer"), "NO");
    }

    #[test]
    fn the_report_states_the_bar_a_result_must_clear() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert!(text.contains("SIGNIFICANCE"));
        let n: u64 = cell(&text, "hypotheses tested").parse().unwrap_or(0);
        assert!(
            n > 1_000,
            "this fixture tests thousands of hypotheses, got {n}"
        );
        // The bar must be well above the t>3 a reader would bring from a
        // hand-built strategy -- that is the entire reason it is printed.
        let required: f64 = cell(&text, "t required (Bonferroni 5%)")
            .parse()
            .unwrap_or(0.0);
        assert!(
            required > 3.0,
            "a sweep of {n} hypotheses cannot have a bar at or below the \
             literature's floor for a handful of trials; got {required}"
        );
        let luck: f64 = cell(&text, "best t-stat by luck alone")
            .parse()
            .unwrap_or(0.0);
        assert!(
            luck > 0.0 && luck < required,
            "the noise floor must be positive and sit BELOW the Bonferroni bar: \
             {luck} against {required}"
        );
        // THE DEFLATION IS ON THE PAGE. This fixture is 94% exact duplicates --
        // 3,591 of 3,798 are masks whose support is identical to a superset's --
        // so the distinct-test count must be far below the raw one. Asserting
        // the relationship rather than a fixed number, because the fixture's
        // redundancy is a property of the vocabulary and will move.
        let distinct: u64 = cell(&text, "distinct tests among them")
            .parse()
            .unwrap_or(0);
        assert!(
            distinct > 0 && distinct < n,
            "distinct tests must be positive and below the raw count: \
             {distinct} of {n}"
        );
    }

    #[test]
    fn a_run_with_no_hypotheses_states_that_rather_than_a_threshold() {
        // A cold column tested nothing, and "the noise floor of zero trials" is
        // not a quantity. It must say so rather than print 0.00, which reads
        // like an easy bar rather than an absent one.
        let auto = Sweeper::new(bounded()).auto(&synthetic::sessions(1), &mut evaluator());
        let text = render_auto(&auto, None);
        assert_eq!(cell(&text, "hypotheses tested"), "0");
        assert_eq!(cell(&text, "threshold"), "-");
    }

    #[test]
    fn every_finding_is_printed_beside_the_bar_it_had_to_clear() {
        // The failure this guards: a reader sees t = 4.1, remembers that three
        // is the threshold they have always used, and stops. The bar is a
        // property of how many hypotheses THIS run tested, so it goes on the
        // page beside the rows rather than in a paper the reader has not read.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let column = indicators::column::Column::build(&bars, &mut evaluator());
        let f = crate::outcome::forward(&bars, crate::outcome::Horizon::DEFAULT);
        let ranked = crate::rank::rank(&out.sweep, &column, &f, 10);
        let text = crate::report::render_findings(&ranked, &out.sweep);

        assert!(text.contains("FINDINGS"));
        assert_eq!(cell(&text, "kept"), "10");
        let bar: f64 = cell(&text, "bar every row must clear")
            .parse()
            .unwrap_or(0.0);
        assert!(bar > 3.0, "a sweep of thousands cannot have a bar at 3.0");

        // Every data row carries a verdict, and the verdict agrees with the bar.
        let mut rows = 0_u32;
        for line in text
            .lines()
            .filter(|l| l.trim_start().starts_with(char::is_numeric))
        {
            let clears = line.contains("clears");
            let below = line.contains("BELOW THE BAR");
            assert!(
                clears ^ below,
                "every row must carry exactly one verdict: {line}"
            );
            let t: f64 = line
                .split_whitespace()
                .nth(4)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            assert_eq!(
                t.abs() >= bar,
                clears,
                "the verdict disagrees with the bar on this row: {line}"
            );
            rows = rows.saturating_add(1);
        }
        assert_eq!(rows, 10, "ten kept means ten rows");
    }

    #[test]
    fn an_empty_ranking_says_so_rather_than_printing_an_empty_table() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let column = indicators::column::Column::build(&bars, &mut evaluator());
        let f = crate::outcome::forward(&bars, crate::outcome::Horizon::DEFAULT);
        let ranked = crate::rank::rank(&out.sweep, &column, &f, 0);
        let text = crate::report::render_findings(&ranked, &out.sweep);

        assert!(text.contains("nothing kept"));
        assert!(!text.contains("rank"), "no header for a table with no rows");
        // And the count of what was weighed is still stated: "kept none of four"
        // and "kept none of sixty-one million" are different facts.
        assert_ne!(cell(&text, "combinations weighed"), "0");
    }

    #[test]
    fn a_paisa_mean_saturates_rather_than_wrapping() {
        // `f64 as i64` truncates silently outside the range, printing a wrapped
        // number instead of refusing -- the class of defect this repository
        // keeps finding. No price series reaches these values; the arms exist
        // so that if one ever did the output would be visibly wrong rather than
        // quietly wrong.
        assert_eq!(super::paisa(0.0), 0);
        assert_eq!(super::paisa(47.4), 47, "rounds to the nearest paisa");
        assert_eq!(super::paisa(-47.6), -48);
        assert_eq!(super::paisa(f64::MAX), i64::MAX, "saturates, never wraps");
        assert_eq!(super::paisa(f64::MIN), i64::MIN);
        assert_eq!(super::paisa(f64::INFINITY), i64::MAX);
        assert_eq!(super::paisa(f64::NEG_INFINITY), i64::MIN);
    }

    #[test]
    fn a_row_below_the_bar_is_named_as_indistinguishable_from_luck() {
        // The synthetic fixture has an upward DRIFT, so every real row on it
        // clears -- which leaves the verdict that matters most untested. Built
        // by hand: one strong row and one weak one against a real sweep's bar.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let weak = crate::rank::Scored {
            mask: vocab::ConditionMask::default().with_bit(3),
            hits: 500,
            edge: crate::outcome::Edge {
                n: 500,
                mismatched: 0,
                mean_paisa: 1.0,
                t: 0.4,
            },
        };
        let ranked = crate::rank::Ranked {
            top: vec![weak],
            considered: 3_689,
        };
        let text = crate::report::render_findings(&ranked, &out.sweep);

        assert!(
            text.contains("BELOW THE BAR"),
            "a t of 0.4 against a bar above four must be named, not merely \
             ranked last"
        );
        assert!(
            text.contains("indistinguishable from luck"),
            "and named in words a reader cannot misread as 'weak but real'"
        );
        assert!(!text.contains("  clears"), "nothing here clears");
    }

    #[test]
    fn a_healthy_report_raises_neither_alarm() {
        // The negative direction, which nothing asserted. Both defect alarms
        // could have been wired permanently ON -- rendering "LOST A CANDIDATE"
        // on every row of every run -- and the two broken-fixture tests would
        // still pass, because they only prove an alarm CAN fire.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert_eq!(
            cell(&text, "every bar accounted for"),
            "yes",
            "a clean census must say so in the cell, not merely fail to complain"
        );
        assert!(
            !text.contains("LOST A CANDIDATE"),
            "no level of a healthy sweep may carry the candidate alarm"
        );
        assert!(!text.contains("a bar was lost"), "nor the census alarm");
        assert_eq!(cell(&text, "trustworthy as a whole answer"), "yes");
    }

    #[test]
    fn a_search_that_kept_no_rung_says_nothing_was_measured() {
        // `Sweeper::auto` substitutes `Sweep::default()` when no threshold was
        // affordable: bars 0, no levels, and `halted: None` because nothing ran
        // far enough to breach anything. `Sweep::completed()` reads that as a
        // clean finish, so the report printed "the frontier went extinct, which
        // is the answer" and "trustworthy: yes" for a search that walked nothing.
        // A fallback wearing a success's clothes -- `CLAUDE.md` §4.
        let bars = synthetic::sessions(1);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
        let text = render_auto(&auto, None);

        assert!(!auto.affordable, "the fixture must keep no rung");
        assert_eq!(
            cell(&text, "outcome"),
            "NOTHING",
            "an absent ladder must not be reported as an extinct one"
        );
        assert!(
            !text.contains("went extinct"),
            "extinction is a measurement, and none was taken"
        );
        assert_eq!(
            cell(&text, "trustworthy as a whole answer"),
            "NO",
            "and the verdict must carry it"
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
    fn a_grid_that_ends_vacuous_then_refused_still_finds_the_band() {
        // THE CASE THE FIRST BISECTION WAS DEAD CODE ON.
        //
        // The grid probes `swept - 1` first, and that rung is ALWAYS vacuous:
        // D-0080 excludes `support == bars` as AlwaysTrue, so it completes with
        // depth 0 and never sets `best`. When the next rung down REFUSES, a
        // bracket keyed on `best` has `hi == lo` and the loop never runs --
        // `auto` then reports "nothing affordable" with the answer sitting
        // between the two probes it already took.
        //
        // Measured before the fix, this exact fixture: affordable=false,
        // min_hits=None, attempts=2, 0 combinations -- while t=592 completes
        // with 4,722 frequent sets at the same ceiling. Found by an adversarial
        // audit; the existing bracket test below opens with `.expect()` on
        // `min_hits`, so it can only ever exercise the path that already worked.
        let bars = synthetic::sessions(8);
        let tight = Ladder::with_min_hits(1).with_ceiling(5_000);
        let auto = Sweeper::new(tight).auto(&bars, &mut evaluator());

        assert!(
            auto.affordable,
            "a band exists at this ceiling and the search must reach it"
        );
        assert!(
            auto.attempts > 2,
            "attempts == 2 is the signature of the defect: two grid probes and \
             zero bisection probes. got {}",
            auto.attempts
        );
        assert!(
            auto.outcome.sweep.all_frequent().count() > 0,
            "and the kept sweep must actually contain combinations"
        );

        // A TIGHTER CEILING PUSHES THE BAND HIGHER, so the bisection's first
        // midpoint lands in the VACUOUS region above it. That probe completes
        // and finds nothing: it must narrow the bracket without being kept, and
        // it is the only way the `found == false` arm is ever reached -- the
        // wider fixture above takes the true arm on all 31 of its probes.
        let tighter = Ladder::with_min_hits(1).with_ceiling(200);
        let narrow = Sweeper::new(tighter).auto(&bars, &mut evaluator());
        assert!(narrow.affordable, "a band exists here too");
        let chosen = narrow.min_hits.unwrap_or(0);
        assert!(
            chosen > auto.min_hits.unwrap_or(0),
            "a tighter ceiling can only afford a HIGHER threshold: {chosen} vs {:?}",
            auto.min_hits
        );
    }

    #[test]
    fn the_search_closes_the_bracket_the_halving_grid_leaves_open() {
        // The halving grid can only ever bracket the answer to a factor of two:
        // it probes swept-1, /2, /4 ... and stops at the first refusal, so the
        // true crossover is somewhere in (refused_below, chosen). An audit
        // measured what that costs -- 6,631 combinations chosen where 51,778 were
        // affordable at the same budget, because the grid jumped 48,374 -> 24,187
        // and never tried 30,961.
        //
        // Bisection must close the bracket to a single threshold. Asserting the
        // WIDTH rather than a particular value is what makes this scale-free:
        // there is no fixture size to get wrong and no number to re-pin when the
        // column changes.
        let bars = synthetic::sessions(8);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());

        let chosen = auto.min_hits.expect("this fixture must find a threshold");
        let refused = auto
            .refused_below
            .expect("and must find an edge, or the bracket below is vacuous");
        assert!(
            chosen > refused,
            "the chosen threshold must sit above the refused one: {chosen} vs {refused}"
        );
        assert!(
            chosen.saturating_sub(refused) <= 1,
            "the bracket must be closed to one. chosen {chosen}, refused {refused} \
             -- a gap wider than one means bisection did not converge and the \
             search is still reporting a power-of-two grid point as the answer"
        );
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

//! BASE-COV-LIBS (D-7320): the streamed report's every section, from a real
//! ranked run, and each verdict word from that run with one field moved to
//! the state it names. The renderer has no other reader in this crate, so
//! `cli` was the only thing that ever ran it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::{names_from_words, render, render_ranked, render_ranked_findings};
use crate::identity::{Direction, Params, Run, data_digest, identity};
use crate::rank::Lens;
use crate::{RankedRun, Sweeper, synthetic};
use brutex_core::instrument::{Exchange, InstrumentKey};
use engine::{Breach, Excluded, Ladder, Why};
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use vocab::ConditionMask;

fn ladder() -> Ladder {
    Ladder::with_min_hits(600)
        .with_ceiling(50_000)
        .with_support_lanes(1)
}

fn run() -> RankedRun {
    let mut evaluator = Evaluator::new(
        Widths::pinned().expect("both pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    );
    Sweeper::new(ladder()).run_ranked(
        &synthetic::sessions(8),
        &mut evaluator,
        crate::outcome::Horizon::DEFAULT,
        10,
    )
}

/// The value cell of the row labelled `label`.
fn cell<'a>(text: &'a str, label: &str) -> &'a str {
    text.lines()
        .find(|line| line.trim_start().starts_with(label))
        .unwrap_or_else(|| panic!("no row {label}: {text}"))
        .trim_start()
        .trim_start_matches(label)
        .trim_start()
}

/// EVERY SECTION OF A REAL RUN IS THERE, and the run says it is whole.
#[test]
fn a_ranked_run_renders_every_section_with_no_identity_said_so() {
    let ran = run();
    let text = render_ranked(&ran.outcome, None);
    for section in ["SWEEP", "LADDER", "SIGNIFICANCE", "VERDICT"] {
        assert!(text.contains(section), "{section}: {text}");
    }
    assert!(
        cell(&text, "identity").starts_with("NOT RECORDED"),
        "{text}"
    );
    assert!(cell(&text, "outcome").starts_with("complete"), "{text}");
    assert!(ran.outcome.is_complete());
    assert!(cell(&text, "trustworthy as a whole answer").starts_with("yes"));
    let excluded = ran.outcome.sweep.excluded.len().to_string();
    assert!(text.contains(&excluded), "{text}");
}

/// A RECORDED RUN PRINTS ITS OWN IDENTITY, in hex.
#[test]
fn a_recorded_identity_is_printed_where_the_refusal_word_was() {
    let ran = run();
    let bars = synthetic::sessions(8);
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("swept");
    let id = identity(&Run {
        mask: ConditionMask::default(),
        direction: Direction::Undirected,
        instrument: &key,
        timeframe: "1min",
        params: Params::of(ladder()),
        data_digest: data_digest(&bars),
        commit: "0123456789abcdef",
        feed: "synthetic",
    });
    let text = render_ranked(&ran.outcome, Some(&id));
    assert!(text.contains(&id.hex()), "{text}");
    assert!(!text.contains("NOT RECORDED"), "{text}");
}

/// THE NOISE FLOOR NEEDS TWO HYPOTHESES: none, one, the first with a floor,
/// and the largest count the type holds.
#[test]
fn the_noise_floor_is_refused_below_two_and_holds_at_the_largest_count() {
    let ran = run();
    for (effective, floor) in [(0, false), (1, false), (2, true), (u64::MAX, true)] {
        let mut outcome = ran.outcome.clone();
        outcome.effective_trials = effective;
        let text = render_ranked(&outcome, None);
        assert_eq!(
            text.contains("too few hypotheses"),
            !floor,
            "{effective}: {text}"
        );
        assert_eq!(text.contains("best t-stat by luck alone"), floor);
    }
}

/// A LADDER THAT WAS NOT WALKED IS NOT EXTINCTION, whichever way it was not
/// walked: no levels, no bars, or both at once.
#[test]
fn a_walk_with_no_levels_or_no_bars_says_nothing_was_measured() {
    let ran = run();
    for (levels, bars) in [(false, true), (true, false), (false, false)] {
        let mut outcome = ran.outcome.clone();
        if !levels {
            outcome.sweep.levels.clear();
        }
        if !bars {
            outcome.sweep.bars = 0;
        }
        let text = render_ranked(&outcome, None);
        assert!(
            cell(&text, "outcome").starts_with("NOTHING MEASURED"),
            "{text}"
        );
    }
}

/// EVERY HALT NAMES ITS BUDGET, and the answer says it cannot be trusted.
#[test]
fn every_breach_is_named_and_the_answer_is_not_trustworthy() {
    let ran = run();
    let halt = ladder().refused_on_memory(1).halted.expect("a halt");
    for (breach, word) in [
        (Breach::Candidates, "candidates"),
        (Breach::Pairs, "pairs"),
        (Breach::Memory, "MEMORY"),
        (Breach::Workers, "WORKERS"),
    ] {
        let mut outcome = ran.outcome.clone();
        outcome.sweep.halted = Some(engine::Halt { breach, ..halt });
        outcome.closure_complete = false;
        let text = render_ranked(&outcome, None);
        assert!(cell(&text, "outcome").starts_with("REFUSED"), "{text}");
        assert!(cell(&text, "budget spent").starts_with(word), "{text}");
        let trust = cell(&text, "trustworthy as a whole answer");
        assert!(trust.starts_with("NO"), "{text}");
        assert!(trust.contains("closure is incomplete"), "{text}");
    }
}

/// A LEVEL THAT DOES NOT RECONCILE IS MARKED ON ITS OWN LINE.
#[test]
fn a_level_that_lost_a_candidate_is_marked() {
    let ran = run();
    assert!(!render_ranked(&ran.outcome, None).contains("LOST A CANDIDATE"));
    let mut outcome = ran.outcome;
    let level = outcome.sweep.levels.first_mut().expect("a level");
    level.generated = level.generated.saturating_add(1);
    assert!(render_ranked(&outcome, None).contains("LOST A CANDIDATE"));
}

/// THE FINDINGS NAME THE LENS THEY WERE CUT BY, each of the four.
#[test]
fn the_findings_name_their_lens() {
    let mut ran = run();
    for (lens, word) in [
        (Lens::Detectability, "best by |t|"),
        (Lens::Payoff, "best by payoff"),
        (Lens::Path, "best by favourable/adverse excursion"),
        (Lens::Asymmetry, "best by smallest win"),
    ] {
        ran.ranked.lens = lens;
        let text = render_ranked_findings(&ran.ranked, &ran.outcome);
        assert!(text.contains(word), "{word}: {text}");
    }
}

/// COMBINED: one row mispaired and one with refused exits at once. Both are
/// said for the run, in their own lines, under the same table.
#[test]
fn a_mispaired_row_and_refused_exits_are_each_said_for_the_run() {
    let mut ran = run();
    assert!(!ran.ranked.top.is_empty(), "the fixture keeps something");
    let clean = render_ranked_findings(&ran.ranked, &ran.outcome);
    assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
    let row = &mut ran.ranked.top[0];
    row.edge.mismatched = 1;
    row.edge.refused = u64::MAX;
    let text = render_ranked_findings(&ran.ranked, &ran.outcome);
    assert!(text.contains("MISPAIRED: 1 of"), "{text}");
    assert!(
        text.contains(&format!("REFUSED EXITS: {}", u64::MAX)),
        "{text}"
    );
}

/// NOTHING KEPT IS SAID, and no table is drawn for it.
#[test]
fn a_run_that_kept_nothing_says_so() {
    let mut ran = run();
    ran.ranked.top.clear();
    let text = render_ranked_findings(&ran.ranked, &ran.outcome);
    assert!(text.contains("nothing kept"), "{text}");
}

/// THE WORDS OF A STORED MASK NAME WHAT THE MASK DOES, and an empty one
/// names nothing.
#[test]
fn stored_words_read_back_as_names() {
    assert!(names_from_words(ConditionMask::ZERO.words()).is_empty());
    let position = (0..ConditionMask::BITS)
        .find(|&p| u16::try_from(p).is_ok_and(vocab::table::is_live))
        .expect("a live position");
    let one = ConditionMask::ZERO.with_bit(position);
    let names = names_from_words(one.words());
    let want = u16::try_from(position).ok().and_then(vocab::table::name);
    assert_eq!(names.first().map(String::as_str), want);
}

/// THE PREPARED STREAMED WALK IS THE SAME WALK: same census, same depth, the
/// run whole; and a memory refusal of the search names itself and attempted
/// nothing.
#[test]
fn a_prepared_streamed_run_and_a_refused_search_read_what_they_did() {
    let ran = run();
    let sweeper = Sweeper::new(ladder());
    let streamed = sweeper.run_prepared_streamed(&ran.column);
    assert!(streamed.is_complete());
    assert_eq!(streamed.census, ran.outcome.census);
    assert_eq!(streamed.first_swept, ran.outcome.first_swept);
    assert_eq!(streamed.sweep.depth(), ran.outcome.sweep.depth());
    let refused = sweeper.auto_memory_refusal(&ran.column);
    assert!(!refused.affordable);
    assert_eq!(refused.attempts, 0);
    assert_eq!(refused.min_hits, None);
    let breach = refused.outcome.sweep.halted.map(|halt| halt.breach);
    assert_eq!(breach, Some(Breach::Memory));
}

/// A plain (unranked) run of the same fixture.
fn plain() -> crate::Outcome {
    let mut evaluator = Evaluator::new(
        Widths::pinned().expect("both pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    );
    Sweeper::new(ladder()).run(&synthetic::sessions(8), &mut evaluator)
}

/// BASE-COV-LIBS-LS2: A POSITION THAT WAS NEVER MEASURED SAYS SO, and names
/// why: a retired or void position beside the measured exclusions of a real
/// run. A generated column measures everything it excludes, so the row is
/// added.
#[test]
fn an_unmeasured_exclusion_says_not_measured_and_names_its_reason() {
    let mut out = plain();
    assert!(!render(&out, None).contains("not measured"));
    out.sweep.excluded.push(Excluded {
        position: 9_999,
        support: None,
        reason: Why::NotLive,
    });
    let text = render(&out, None);
    let line = text
        .lines()
        .find(|line| line.contains("retired, void, or outside the table"))
        .unwrap_or_else(|| panic!("no NotLive row: {text}"));
    assert!(line.contains("not measured"), "{line}");
    assert!(line.contains("9999") && line.contains(" ? "), "{line}");
}

/// A PLAIN RUN HALTED BY ITS WORKERS SAYS WORKERS, the one budget no caller
/// set besides memory, and is not trusted as a whole answer.
#[test]
fn a_plain_run_halted_by_its_workers_names_them() {
    let mut out = plain();
    let halt = ladder().refused_on_memory(1).halted.expect("a halt");
    out.sweep.halted = Some(engine::Halt {
        breach: Breach::Workers,
        ..halt
    });
    let text = render(&out, None);
    assert!(cell(&text, "outcome").starts_with("REFUSED"), "{text}");
    assert!(cell(&text, "budget spent").starts_with("WORKERS"), "{text}");
    assert!(cell(&text, "trustworthy as a whole answer").starts_with("NO"));
}

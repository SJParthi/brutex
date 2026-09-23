//! Boundary tests for durable admission, execution identity and command status.
#![allow(clippy::expect_used, reason = "a failed fixture must fail its test")]

use super::{AuditOptions, Recording, Rules, audit_bars, bind_execution_digest, identity, stored};
use runner::identity::{Direction, Params, Run, RunId};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

fn root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "brutex-sweep-wiring-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn id() -> RunId {
    let key = stored::swept_index("NIFTY").expect("known fixture key");
    identity(&Run {
        #[expect(
            clippy::default_trait_access,
            reason = "the identity API supplies the mask type"
        )]
        mask: Default::default(),
        direction: Direction::Undirected,
        instrument: &key,
        timeframe: "1min",
        params: Params::of(engine::Ladder::with_min_hits(1)),
        data_digest: [7; 32],
        commit: "boundary-test-fixture",
        feed: "zerodha",
    })
}

fn options(root: &Path) -> AuditOptions<'_> {
    AuditOptions {
        prepared_column: None,
        replay: None,
        execution: None,
        native_minute_execution: true,
        recording: Some(Recording {
            root,
            feed: "zerodha",
            underlying: "NIFTY",
            timeframe: "1min",
            from: (2024, 1),
            to: (2024, 1),
            attempt: None,
            months_asked: 1,
            months_found: 1,
        }),
        rules: Rules::BASELINE,
        lens: runner::rank::Lens::Detectability,
        ceiling: Some(64),
        validate: false,
        cost: runner::audit::CostScope::IndexSpot,
    }
}

#[test]
fn identity_reservation_refusal_precedes_evaluator_preparation() {
    let root = root();
    std::fs::create_dir_all(&root).expect("fixture root");
    std::fs::write(root.join("results"), b"directory deliberately obstructed").expect("fault");
    let report = audit_bars(
        &Err("EVALUATOR_WAS_REACHED"),
        Vec::new(),
        "fixture",
        1,
        Some(&id()),
        options(&root),
    );
    assert!(report.contains("audit did not start"), "{report}");
    assert!(!report.contains("EVALUATOR_WAS_REACHED"), "{report}");
    assert_eq!(
        std::fs::read(root.join("results")).expect("preserved obstruction"),
        b"directory deliberately obstructed"
    );
}

#[test]
fn evaluator_refusal_has_its_exact_preceding_durable_identity() {
    let root = root();
    let id = id();
    let report = audit_bars(
        &Err("EVALUATOR_WAS_REACHED"),
        Vec::new(),
        "fixture",
        1,
        Some(&id),
        options(&root),
    );
    assert!(report.contains("EVALUATOR_WAS_REACHED"), "{report}");
    let evidence = super::sweep_evidence::read(&root, id.bytes(), u64::MAX)
        .expect("valid evidence")
        .expect("the attempt preceded preparation");
    assert_eq!(evidence.identity, id.bytes());
    assert_eq!(evidence.operation, super::sweep_evidence::Operation::Audit);
    assert_eq!(evidence.validation_requested, Some(false));
    assert_eq!(
        evidence.completion,
        super::sweep_evidence::Completion::Refused
    );
    assert_eq!(evidence.depth_rows, 0);
    assert_eq!(evidence.ranked_rows, 0);
    assert!(!super::results::Results::path(&root).exists());
}

#[test]
fn a_recording_target_without_identity_never_prepares_an_evaluator() {
    let root = root();
    let report = audit_bars(
        &Err("EVALUATOR_WAS_REACHED"),
        Vec::new(),
        "fixture",
        1,
        None,
        options(&root),
    );
    assert!(report.contains("no run identity"), "{report}");
    assert!(!report.contains("EVALUATOR_WAS_REACHED"));
    assert!(!root.exists());
}

#[test]
fn an_interior_execution_change_rekeys_the_complete_stored_input() {
    let mut execution = vec![
        runner::candle(60_000_000, 100, 110, 90, 101),
        runner::candle(120_000_000, 101, 111, 91, 102),
        runner::candle(180_000_000, 102, 112, 92, 103),
    ];
    let anchored = [31; 32];
    let first = bind_execution_digest(anchored, runner::identity::data_digest(&execution));
    assert_eq!(
        first,
        bind_execution_digest(anchored, runner::identity::data_digest(&execution))
    );
    *execution.get_mut(1).expect("interior candle") =
        runner::candle(120_000_000, 101, 113, 91, 104);
    let changed = bind_execution_digest(anchored, runner::identity::data_digest(&execution));
    assert_ne!(first, changed);
    assert_ne!(
        first,
        bind_execution_digest(runner::identity::data_digest(&execution), anchored)
    );
}

#[test]
fn only_actual_computation_verbs_can_establish_sweep_status() {
    for word in super::COMMANDS {
        let non_sweep = matches!(
            word,
            "checksum-audit-stored"
                | "fold-audit"
                | "research-plan"
                | "policy-check"
                | "results"
                | "top"
                | "verify"
        );
        assert_eq!(super::is_sweep_command(word), !non_sweep, "{word}");
    }
    for word in ["", "typo", "top.json", "sweepy"] {
        assert!(!super::is_sweep_command(word), "{word}");
    }
}

#[test]
fn refused_unadmitted_children_never_publish_a_parent_summary() {
    let id = id();
    let scored = runner::rank::Scored {
        #[expect(
            clippy::default_trait_access,
            reason = "the public scored type supplies the mask type"
        )]
        mask: Default::default(),
        hits: 1,
        edge: runner::outcome::Edge::default(),
    };
    let sweep = engine::keep::Streamed::default();
    let priced = std::collections::HashMap::new();
    let retained = [&scored];
    let what = super::Unadmitted {
        sweep: &sweep,
        bars: 1,
        min_hits: 1,
        by_evidence: &retained,
        rules: Rules::BASELINE,
        priced: &priced,
    };
    for receipt_fault in [false, true] {
        let root = root();
        let obstruction = if receipt_fault {
            super::result_set::Receipts::path(&root)
        } else {
            super::frontier::Frontier::path(&root)
        };
        std::fs::create_dir_all(&obstruction).expect("directory blocks exact child file");
        let into = options(&root).recording.expect("fixture recording target");
        let why =
            super::record_unadmitted(into, &id, &what).expect_err("child publication refused");
        assert!(!why.is_empty());
        assert!(
            !super::results::Results::path(&root).exists(),
            "a failed child cannot acquire a parent"
        );
        assert!(
            obstruction.is_dir(),
            "existing obstruction is never replaced"
        );
        if receipt_fault {
            assert!(
                super::frontier::Frontier::path(&root).is_file(),
                "frontier prepared before receipt"
            );
        }
    }
}

fn classified(
    census: indicators::column::Census,
    first_swept: Option<usize>,
    sweep: engine::keep::Streamed,
    closure_complete: bool,
) -> (
    super::sweep_evidence::Completion,
    super::sweep_evidence::Completion,
) {
    let streamed = runner::StreamedOutcome {
        census,
        first_swept,
        sweep: sweep.clone(),
    };
    let ranked = runner::RankedOutcome {
        census,
        first_swept,
        sweep,
        trials: 0,
        effective_trials: 0,
        closure_complete,
    };
    (
        super::sweep_completion(streamed.is_complete(), streamed.sweep.halted.as_ref()),
        super::sweep_completion(ranked.is_complete(), ranked.sweep.halted.as_ref()),
    )
}

#[test]
fn empty_cold_refused_and_inconsistent_samples_cannot_be_completed_or_resource_halted() {
    use super::sweep_evidence::Completion;
    use indicators::column::Census;
    for census in [
        Census::default(),
        Census {
            offered: 2,
            warming: 2,
            ..Census::default()
        },
        Census {
            offered: 2,
            high_below_low: 2,
            ..Census::default()
        },
    ] {
        assert_eq!(
            classified(census, None, engine::keep::Streamed::default(), true),
            (Completion::Refused, Completion::Refused)
        );
    }
    let sweep = engine::keep::Streamed {
        bars: 3,
        ..engine::keep::Streamed::default()
    };
    assert_eq!(
        classified(
            Census::default(),
            Some(0),
            engine::keep::Streamed::default(),
            true
        ),
        (Completion::Refused, Completion::Refused),
        "a fabricated first index cannot turn zero samples into a complete outcome"
    );
    let census = Census {
        offered: 4,
        swept: 3,
        ..Census::default()
    };
    assert_eq!(
        classified(census, Some(0), sweep, true),
        (Completion::Refused, Completion::Refused)
    );
}

#[test]
fn a_real_candidate_budget_halt_and_certified_completion_keep_opposite_terminal_states() {
    use super::sweep_evidence::Completion;
    let empty = runner::rank::Scored {
        #[expect(
            clippy::default_trait_access,
            reason = "the scored type supplies the mask type"
        )]
        mask: Default::default(),
        hits: 0,
        edge: runner::outcome::Edge::default(),
    }
    .mask;
    let live: Vec<u32> = (0..8).collect();
    let masks: Vec<_> = (0..64_u32)
        .map(|bar| {
            let mut mask = empty;
            for bit in (0..8_u32).filter(|bit| bar % (bit + 2) != 0) {
                mask = mask.with_bit(bit);
            }
            mask
        })
        .collect();
    let census = indicators::column::Census {
        offered: 64,
        swept: 64,
        ..indicators::column::Census::default()
    };
    let halted = engine::Ladder::with_min_hits(1)
        .with_ceiling(1)
        .walk_streamed(&masks, &live, &mut |_, _, _| {});
    assert_eq!(
        halted.halted.expect("actual budget breach").breach,
        engine::Breach::Candidates
    );
    assert_eq!(
        classified(census, Some(0), halted, true),
        (Completion::Halted, Completion::Halted)
    );
    let finished = engine::Ladder::with_min_hits(1)
        .with_ceiling(10_000)
        .walk_streamed(&masks, &live, &mut |_, _, _| {});
    assert!(finished.completed());
    assert_eq!(
        classified(census, Some(0), finished.clone(), true),
        (Completion::Completed, Completion::Completed)
    );
    assert_eq!(
        classified(census, Some(0), finished, false),
        (Completion::Completed, Completion::Refused)
    );
}

/// THE AUDIT HEADER FOLLOWS THE CALLER'S COST SCOPE THROUGH THE WHOLE AUDIT.
///
/// D-0681. `render_selected` used to print the index header for every run,
/// so a stored stock audit told its reader there was no brokerage, STT, stamp
/// or GST. `runner::audit` proves each header renders; this proves the scope a
/// caller puts in `AuditOptions` is the one the final render receives, rather
/// than a constant somewhere between the two. Generated bars and no recording:
/// the header is the only thing under test, and nothing is written anywhere.
#[test]
fn the_audit_header_is_the_scope_the_caller_supplied() {
    use runner::audit::CostScope;

    let run = |cost: CostScope| {
        audit_bars(
            &super::evaluator(),
            runner::synthetic::sessions(12),
            "GENERATED TEST FIXTURE",
            1_400,
            None,
            AuditOptions {
                prepared_column: None,
                replay: None,
                execution: None,
                native_minute_execution: true,
                recording: None,
                rules: Rules::BASELINE,
                lens: runner::rank::Lens::Detectability,
                ceiling: Some(50_000),
                validate: false,
                cost,
            },
        )
    };

    let index = run(CostScope::IndexSpot);
    assert!(
        index.contains("\nAUDIT\n  INDEX SPOT run. There is no brokerage"),
        "an index audit keeps the header it has always printed:\n{index}"
    );
    assert!(!index.contains("GROSS OF EVERY CHARGE"), "{index}");

    let equity = run(CostScope::CashEquity);
    assert!(
        equity
            .contains("\nAUDIT\n  CASH EQUITY run. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE.\n"),
        "an equity audit must be labelled gross of every charge:\n{equity}"
    );
    for index_claim in ["INDEX SPOT run", "no brokerage"] {
        assert!(
            !equity.contains(index_claim),
            "{index_claim:?} is false of a share trade and must not reach an \
             equity audit:\n{equity}"
        );
    }
}

/// One generated-bar audit at `cost`, unrecorded, on the bounded fixture the
/// header test above uses. Nothing is written anywhere.
fn generated_audit(
    cost: runner::audit::CostScope,
    ceiling: usize,
    min_hits: u64,
    validate: bool,
) -> String {
    audit_bars(
        &super::evaluator(),
        runner::synthetic::sessions(12),
        "GENERATED TEST FIXTURE",
        min_hits,
        None,
        AuditOptions {
            prepared_column: None,
            replay: None,
            execution: None,
            native_minute_execution: true,
            recording: None,
            rules: Rules::BASELINE,
            lens: runner::rank::Lens::Detectability,
            ceiling: Some(ceiling),
            validate,
            cost,
        },
    )
}

/// **A halted equity ranking says it is gross before it refuses to trade.**
///
/// D-0681 put the charge statement in the AUDIT block, and a halted or
/// uncertified ladder returns before that block is rendered -- after printing
/// a ranked FINDINGS table of shares (`rank, hits, mean paisa, t, clears`).
/// Measured before this: at ceilings 8, 64 and 512 the equity page carried no
/// "gross" anywhere. The label is the ONLY difference from the index page.
#[test]
fn a_halted_equity_ranking_is_labelled_gross_before_it_refuses() {
    use runner::audit::CostScope;
    let _knobs = super::knobs::serially();
    super::knobs::clear_all();
    for ceiling in [8, 64, 512] {
        let equity = generated_audit(CostScope::CashEquity, ceiling, 1_400, false);
        assert!(
            super::carries_refusal(&equity) && equity.contains("NOT TRADED"),
            "premise: ceiling {ceiling} halts:\n{equity}"
        );
        let findings = super::section_note(&equity, "FINDINGS").expect("a ranking");
        assert!(
            findings.contains("  rank "),
            "premise: ranked rows:\n{findings}"
        );
        assert!(
            findings.contains(super::EQUITY_RANKING_GROSS.trim_end()),
            "ceiling {ceiling}: the ranking must say it is gross:\n{findings}"
        );
        let label = equity.find("GROSS OF EVERY CHARGE").expect("labelled");
        let refusal = equity.find("NOT TRADED").expect("refused");
        assert!(
            label < refusal,
            "the label qualifies the ranking above the refusal"
        );
        let index = generated_audit(CostScope::IndexSpot, ceiling, 1_400, false);
        assert_eq!(
            equity.replacen(&super::equity_ranking_statement(), "", 1),
            index,
            "ceiling {ceiling}: the scope may change the charge statement and nothing else"
        );
    }
}

/// **A completed equity audit is labelled in FINDINGS and in AUDIT, carries no
/// refusal, and reruns byte for byte.** §3 rule 5, on the scope D-0681 added.
///
/// The rung note `range-all` and `pool` keep when they discard the report
/// lifts the AUDIT block whole: the header's paragraphs are separated by blank
/// lines, and every line is indented, so none of them ends the lift.
#[test]
fn a_completed_equity_audit_is_labelled_twice_and_reruns_byte_for_byte() {
    use runner::audit::CostScope;
    let _knobs = super::knobs::serially();
    super::knobs::clear_all();
    let first = generated_audit(CostScope::CashEquity, 50_000, 1_400, false);
    assert_eq!(
        generated_audit(CostScope::CashEquity, 50_000, 1_400, false),
        first,
        "the same equity audit must render byte for byte"
    );
    assert!(!super::carries_refusal(&first), "{first}");
    let findings = super::section_note(&first, "FINDINGS").expect("a ranking");
    assert!(
        findings.contains(super::EQUITY_RANKING_GROSS.trim_end()),
        "{findings}"
    );
    let lifted = super::validation_note(&first).expect("the rung note");
    for paragraph in [
        "AUDIT\n  CASH EQUITY run. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE.",
        "chose this combination is on GROSS returns.",
        "COST-EXCLUDED RESEARCH, NOT A NET RESULT",
        "No equity result carries Selection V6",
        "GROSS OF THE SPREAD",
        "CASH EQUITY: EVERY FIGURE IN THIS RANKING IS GROSS OF EVERY CHARGE.",
    ] {
        assert!(
            lifted.contains(paragraph),
            "missing {paragraph:?}:\n{lifted}"
        );
    }
    for index_claim in ["INDEX SPOT run", "no brokerage"] {
        assert!(!lifted.contains(index_claim), "{lifted}");
    }
}

/// **An extinct equity audit prints no ranking, so nothing to label.**
#[test]
fn an_extinct_equity_audit_is_the_index_page_byte_for_byte() {
    use runner::audit::CostScope;
    let _knobs = super::knobs::serially();
    super::knobs::clear_all();
    let equity = generated_audit(CostScope::CashEquity, 50_000, u64::MAX, false);
    assert!(
        equity.contains("nothing kept — the sweep produced no combination"),
        "premise: extinct:\n{equity}"
    );
    assert!(!equity.contains("GROSS OF EVERY CHARGE"), "{equity}");
    assert_eq!(
        equity,
        generated_audit(CostScope::IndexSpot, 50_000, u64::MAX, false)
    );
}

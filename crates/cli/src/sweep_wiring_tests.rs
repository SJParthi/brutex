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

/// **An equity ranking that kept rows but traded none says it is gross before
/// it refuses.** D-0696.
///
/// The third page `EQUITY_RANKING_GROSS` was written for, and the one no test
/// reached: stripping the label on this exit alone left every other test
/// green. No generated store reaches the exit -- `NoneClosedFault`'s own
/// documentation gives the reason, read from the ranker's ordering -- so the
/// guard makes `retained_to_trade` answer `None` on a real completed audit and
/// the real `nothing_to_trade` branch prints the page. The AUDIT block there is
/// the refusal, so the charge header is never rendered and the FINDINGS label
/// is the page's only charge statement.
#[test]
fn an_untraded_equity_ranking_is_labelled_gross_before_it_refuses() {
    use runner::audit::CostScope;
    let _knobs = super::knobs::serially();
    super::knobs::clear_all();
    let (equity, index) = {
        let _none_closed = super::NoneClosedFault::install();
        (
            generated_audit(CostScope::CashEquity, 50_000, 1_400, false),
            generated_audit(CostScope::IndexSpot, 50_000, 1_400, false),
        )
    };
    let refused = "\nAUDIT\n  REFUSED. The streamed sweep offered ";
    assert!(
        equity.contains(refused) && !equity.contains("NOT TRADED"),
        "premise: a completed ladder refused at the untraded exit:\n{equity}"
    );
    assert!(super::carries_refusal(&equity), "{equity}");
    let findings = super::section_note(&equity, "FINDINGS").expect("a ranking");
    assert!(
        findings.contains("  rank "),
        "premise: ranked rows were kept:\n{findings}"
    );
    assert!(
        findings.contains(super::EQUITY_RANKING_GROSS.trim_end()),
        "the untraded ranking must say it is gross:\n{findings}"
    );
    // The equality below strips whatever `equity_ranking_statement` returns,
    // so a statement that lost the sentence would pass it. D-0694.
    assert!(
        findings.ends_with(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
        "the untraded ranking must say corporate actions are unchecked, after its label:\n{findings}"
    );
    let label = equity.find("GROSS OF EVERY CHARGE").expect("labelled");
    assert!(
        label < equity.find(refused).expect("refused"),
        "the label qualifies the ranking above the refusal"
    );
    assert!(
        !equity.contains("CASH EQUITY run."),
        "premise: this exit renders no AUDIT header:\n{equity}"
    );
    // THE WHOLE FINDINGS STATEMENT, the label and D-0694's corporate-action
    // sentence after it, as the halted exit's test strips it. Stripping the
    // label alone left the sentence behind, so this failed on the tree that
    // carries both. AF-19.
    assert!(
        findings.ends_with(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
        "the sentence closes the untraded ranking's FINDINGS:\n{findings}"
    );
    assert_eq!(
        equity.replacen(&super::equity_ranking_statement(), "", 1),
        index,
        "the scope may change the charge statement and nothing else"
    );
    let whole = generated_audit(CostScope::CashEquity, 50_000, 1_400, false);
    assert!(
        !super::carries_refusal(&whole) && whole.contains("CASH EQUITY run."),
        "the seam is gone with its guard:\n{whole}"
    );
}

/// **A nested untraded guard puts back what it displaced.** D-0696.
///
/// The guard cleared the flag when it dropped, so an inner guard dropping
/// switched off an outer one still in scope, and the outer's test then drove
/// the ordinary exit without knowing it.
#[test]
fn a_nested_none_closed_guard_restores_the_outer_one_when_it_drops() {
    let held = || super::NONE_CLOSED.with(std::cell::Cell::get);
    assert!(!held(), "premise: no guard on this thread");
    {
        let _outer = super::NoneClosedFault::install();
        assert!(held(), "the outer guard sets the flag");
        {
            let _inner = super::NoneClosedFault::install();
            assert!(held(), "the inner guard sets it too");
        }
        assert!(
            held(),
            "the inner guard dropping must leave the outer one in force"
        );
    }
    assert!(
        !held(),
        "the outer guard dropping leaves the flag as it found it"
    );
}

/// **Three doc comments this piece corrected say no more than holds.** D-0696.
///
/// `Rules::elite` said every admitted row's figures are "per ONE unit of the
/// index, gross of the statutory charge stack" after D-0506 let a row be a
/// share, which `cli top` tells is per ONE share and GROSS OF EVERY CHARGE.
/// The stored-sweep share test said "An index's sweep is unchanged" and
/// compared no index page with a baseline. `EQUITY_RANKING_GROSS` called itself
/// one of three copies in this crate beside the header, after
/// `runner::audit::CASH_EQUITY_GROSS` became the fourth copy the binding test
/// reads, and named neither that copy nor the "none is subtracted" check
/// (measured by the review). Read from the source, because a doc comment
/// reaches no other test.
#[test]
fn the_corrected_doc_comments_say_no_more_than_holds() {
    let doc_before = |source: &'static str, item: &str| -> String {
        let at = source.find(item).expect("the item is still there");
        let head = source.get(..at).expect("a prefix");
        let from = head.rfind("\n\n").map_or(0, |gap| gap + 2);
        let doc = head.get(from..).expect("its doc");
        assert!(doc.len() > 200, "the doc of {item} is {} bytes", doc.len());
        words(&doc.replace("///", " "))
    };
    let elite = doc_before(include_str!("lib.rs"), "    pub const fn elite(");
    assert!(
        !elite.contains("per ONE unit of the index, gross of the statutory charge stack"),
        "{elite}"
    );
    assert!(
        elite.contains("one unit of an index, or one share, GROSS OF EVERY CHARGE"),
        "the filter's caveat names a share's unit and charges: {elite}"
    );
    let sweep = doc_before(
        include_str!("audited_stored_tests.rs"),
        "fn a_stored_sweep_of_a_share_says_its_ranking_is_gross_of_every_charge(",
    );
    assert!(
        !sweep.contains("An index's sweep is unchanged."),
        "the doc claims a byte comparison the test does not make: {sweep}"
    );
    assert!(
        sweep.contains("It is not compared byte for byte"),
        "the doc says what is and is not asserted of an index: {sweep}"
    );
    let label = doc_before(include_str!("lib.rs"), "\nconst EQUITY_RANKING_GROSS: &str");
    assert!(
        !label.contains("one of three copies"),
        "the label's doc counts three copies of four: {label}"
    );
    assert!(
        label.contains("one of four copies")
            && label.contains("`runner::audit::CASH_EQUITY_GROSS`")
            && label.contains("does not say \"none is subtracted\""),
        "the label's doc names the fourth copy and the check on the fact: {label}"
    );
}

/// `text` with every run of whitespace folded to one space, so two copies of a
/// sentence wrapped or indented differently compare by their words.
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether `text` carries a digit or a percent sign outside a decision number
/// (`D-` and exactly four digits) and the name `Selection V6`, which is what
/// quoting a rate needs.
fn names_a_rate(text: &str) -> bool {
    let text = text.replace("Selection V6", "Selection");
    let mut pieces = text.split("D-");
    let first = pieces.next().unwrap_or_default();
    std::iter::once(first)
        .chain(pieces.map(|piece| {
            let number = piece.bytes().take(4).filter(u8::is_ascii_digit).count() == 4;
            if number {
                piece.get(4..).unwrap_or_default()
            } else {
                piece
            }
        }))
        .any(|piece| piece.contains(|c: char| c.is_ascii_digit() || c == '%'))
}

/// Why `said` -- one copy of the charge statement, whitespace folded -- does
/// not state the audit header's fact that none of the charges is subtracted,
/// or `Ok` when it does. `excluded` is the header's cost-excluded sentence,
/// whose "NOT A NET RESULT" is the one place a copy may use the word.
///
/// Every copy states the fact in the same four words, "none is subtracted",
/// and speaks of subtraction nowhere else. So a copy that ALSO says something
/// is subtracted, calls a figure net, or speaks of a deduction contradicts
/// the header while still carrying its charge list and sentence -- which is
/// exactly what passed every test before this check (D-0696).
fn denies_subtraction(said: &str, excluded: &str) -> Result<(), &'static str> {
    if !said.contains("none is subtracted") {
        return Err("does not say that none is subtracted");
    }
    let rest = said.replace(excluded, " ").to_lowercase();
    if rest.matches("subtract").count() != 1 {
        return Err("speaks of subtraction beyond `none is subtracted`");
    }
    if rest.contains("deduct") {
        return Err("speaks of a deduction");
    }
    if rest
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| word == "net")
    {
        return Err("calls a figure net outside `NOT A NET RESULT`");
    }
    Ok(())
}

/// The audit header's equity text with whitespace folded, its charge list,
/// and its cost-excluded sentence with the Selection V6 clause -- each read
/// out of what `runner::audit::render` prints, and the header's own fact that
/// none of the charges is subtracted checked as a premise.
fn the_headers_charge_fact() -> (String, String, String) {
    use runner::audit::CostScope;
    let header = words(&runner::audit::render(
        CostScope::CashEquity,
        None,
        None,
        None,
        None,
        None,
        0,
    ));
    let (before, _) = header
        .split_once(" all apply to a share trade")
        .expect("the header names what a share trade pays");
    let (_, charges) = before.rsplit_once(" so ").expect("the charge list");
    assert!(
        charges.contains("STT") && charges.contains("GST"),
        "premise: the list was read: {charges:?}"
    );
    let from = header
        .find("COST-EXCLUDED RESEARCH, NOT A NET RESULT")
        .expect("the header's cost-excluded sentence");
    let to = header
        .get(from..)
        .and_then(|tail| tail.find("stack exists."))
        .expect("which ends the Selection V6 clause");
    let excluded = header
        .get(from..from + to + "stack exists.".len())
        .expect("a sentence of the header");
    assert!(
        excluded.contains("No equity result carries Selection V6"),
        "premise: {excluded:?}"
    );
    assert!(
        header.contains("so NONE of those charges is subtracted anywhere in this run"),
        "premise: the header states the fact every copy must: {header}"
    );
    let (charges, excluded) = (charges.to_owned(), excluded.to_owned());
    (header, charges, excluded)
}

/// **The subtraction check refuses each listed way of contradicting the
/// header that keeps its words, and each of its four clauses is the one that
/// refuses at least one of them.** D-0696.
///
/// The review's label kept the charge list, the sentence and "GROSS OF EVERY
/// CHARGE" and said the engine subtracts every charge. Each contradiction
/// below is written into the FINDINGS label, keeps the list and the sentence,
/// and must be refused by [`denies_subtraction`], so the check in
/// `every_equity_charge_statement_is_the_audit_headers_own_and_names_no_rate`
/// is not vacuous.
///
/// **A list, not every way.** These are five rewrites, and a contradiction
/// in words the check does not look for would pass it. What the list does
/// establish is that no clause is idle: each case is refused by the reason
/// named beside it, and the first clause -- "none is subtracted" must be said
/// -- is the only one that refuses the last case, which has one "subtract",
/// no "net" and no "deduct". Before that case was listed, deleting the first
/// clause left every test green, and the label "this engine subtracts all of
/// them, so the ranking above is on GROSS returns" then passed the binding
/// test (measured by the review).
#[test]
fn the_subtraction_check_refuses_each_contradiction_of_the_header() {
    let (_, charges, excluded) = the_headers_charge_fact();
    let label = words(super::EQUITY_RANKING_GROSS);
    assert_eq!(
        denies_subtraction(&label, &excluded),
        Ok(()),
        "premise: the label itself states the fact: {label}"
    );
    let unsaid = "does not say that none is subtracted";
    for (contradiction, from, to, refused) in [
        (
            "the review's label",
            "and none is subtracted, so the ranking above is on GROSS returns",
            "and this engine subtracts all of them, so the ranking above is on NET returns",
            unsaid,
        ),
        (
            "a second claim beside the fact",
            "and none is subtracted,",
            "and none is subtracted, but brokerage is subtracted,",
            "speaks of subtraction beyond `none is subtracted`",
        ),
        (
            "a net figure beside the fact",
            "is on GROSS returns",
            "is on net returns",
            "calls a figure net outside `NOT A NET RESULT`",
        ),
        (
            "a deduction beside the fact",
            "and none is subtracted,",
            "and none is subtracted, though every charge is deducted,",
            "speaks of a deduction",
        ),
        (
            "a contradiction with no net figure",
            "and none is subtracted,",
            "and this engine subtracts all of them,",
            unsaid,
        ),
    ] {
        assert!(label.contains(from), "premise: {contradiction}: {label}");
        let broken = label.replace(from, to);
        assert!(
            broken.contains(&charges) && broken.contains(&excluded),
            "premise: {contradiction} keeps the list and the sentence"
        );
        assert_eq!(
            denies_subtraction(&broken, &excluded),
            Err(refused),
            "{contradiction} must be refused, and for its own reason: {broken}"
        );
    }
}

/// **Every equity charge statement `cli` prints is the audit header's own,
/// and none names a rate.** D-0696.
///
/// The FINDINGS label, `cli top`'s share legend and `pool`'s opening are three
/// copies of what `runner::audit`'s equity header says, and the first version
/// of each pinned only its own literal. They had already drifted: three sets
/// of decision citations, and two copies without the header's Selection V6
/// clause. So the charge list and the cost-excluded sentence are read out of
/// the header `runner::audit::render` prints, and every copy must carry both
/// word for word. A change to the header's charges -- the day a
/// charter-sourced stack lands -- fails here until every copy follows it.
///
/// `CLAUDE.md` §3 rule 1: no copy may quote a rate, because
/// `docs/00-charter.md` sources none. Asserted on the copies themselves, so a
/// rate added to one of them fails whether or not a page test reads it.
///
/// **There were four copies, not three.** D-0694 gave `runner::audit`
/// `CASH_EQUITY_GROSS`, which `CostScope::report_note` prints at the head of
/// every stored share page (`top` and `sweep-stored` among them), of
/// `sweep-all` and of the Boolean research heading. It said "This is
/// cost-excluded research, not a net result" with no Selection V6 clause, so
/// those pages stated the charge fact in two wordings and this test read one.
/// It now carries the header's sentence and is checked with the other three
/// (D-0696).
///
/// **The four constants, not every sentence `cli` prints.** "Every equity
/// charge statement" in the name means these four texts. Other pages over a
/// share carry wordings of their own that this test does not read -- a priced
/// expression search's "Cost-excluded, unvalidated research" note and the
/// research page's "Discovery policy requested: no charges" line among them.
///
/// **And the fact itself, not only its words.** Each copy was held to the
/// header's charge list and sentence and to "GROSS OF EVERY CHARGE", and to
/// nothing about subtraction: a label rewritten to say "this engine subtracts
/// all of them, so the ranking above is on NET returns" kept all three and
/// passed every equity test (measured by the review). The header says NONE of
/// those charges is subtracted, and [`denies_subtraction`] now holds every
/// copy to saying so in one wording and to contradicting it nowhere.
///
/// **And every other word of each copy.** [`denies_subtraction`] refuses the
/// phrasings it lists, and a clause in other words -- "Every charge is taken
/// off each trade before it is ranked." appended to the label -- passed it and
/// every other assertion here (measured by the review, D-0696). So each copy,
/// with the header's list and sentence replaced by `<charges>` and
/// `<sentence>`, must equal the words written beside it below. The list and
/// the sentence still follow the header; everything around them is pinned per
/// copy, and a clause added to a copy fails until this test is edited with it.
#[test]
fn every_equity_charge_statement_is_the_audit_headers_own_and_names_no_rate() {
    let (header, charges, excluded) = the_headers_charge_fact();
    assert!(
        !names_a_rate(&header),
        "premise: the header itself quotes no rate: {header}"
    );

    for (copy, text, around) in [
        (
            "the FINDINGS label",
            super::EQUITY_RANKING_GROSS,
            "CASH EQUITY: EVERY FIGURE IN THIS RANKING IS GROSS OF EVERY CHARGE. A share \
             trade pays <charges>, and none is subtracted, so the ranking above is on GROSS \
             returns. <sentence>",
        ),
        (
            "`cli top`'s share legend",
            super::SHARE_MEAN_LEGEND,
            "ONE share, GROSS OF EVERY CHARGE: <charges> all apply to a share trade and \
             none is subtracted. <sentence>",
        ),
        (
            "`pool`'s opening",
            super::pool::EQUITY_TOTALS_GROSS,
            "NO COST OF ANY KIND IS CHARGED. Correct on an index by charter; NOT correct \
             on a cash equity, where <charges> all apply and none is subtracted: every \
             equity total is GROSS OF EVERY CHARGE. <sentence>",
        ),
        (
            "the stored banner's gross paragraph (`report_note`)",
            runner::audit::CASH_EQUITY_GROSS,
            "CASH EQUITY. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE: <charges> apply to \
             a share trade and none is subtracted. <sentence>",
        ),
    ] {
        let said = words(text);
        assert!(
            said.contains(&charges),
            "{copy} must name the header's charges {charges:?}: {said}"
        );
        assert!(
            said.contains(&excluded),
            "{copy} must carry the header's sentence {excluded:?}: {said}"
        );
        assert!(
            said.contains("GROSS OF EVERY CHARGE"),
            "{copy} must say it is gross: {said}"
        );
        assert!(!names_a_rate(text), "{copy} quotes a rate: {text}");
        let verdict = denies_subtraction(&said, &excluded);
        assert!(
            verdict.is_ok(),
            "{copy} {verdict:?}, against the header's fact: {said}"
        );
        assert_eq!(
            said.replace(&charges, "<charges>")
                .replace(&excluded, "<sentence>"),
            around,
            "{copy}: every word around the header's list and sentence is pinned, so a \
             clause added beside them fails here"
        );
    }

    // The checker is not vacuous: a quoted rate trips it, a decision number
    // does not, and a five-digit "decision" is a number like any other.
    assert!(names_a_rate("STT alone is 0.025% of every sell"));
    assert!(names_a_rate("brokerage of 20 rupees"));
    assert!(names_a_rate("a rate in percent %"));
    assert!(names_a_rate("(D-06961)"));
    assert!(names_a_rate("Selection V7"));
    assert!(!names_a_rate("(D-0509, D-0525, D-0681) Selection V6"));
}

/// GAP11-3: `record_unadmitted` published its frontier, receipt and ledger
/// row with neither the process-wide `LEDGER` mutex nor the cross-process
/// result-set lock, and with no directory barrier before the ledger row. Every
/// other result-set commit holds both locks and confirms the directory first.
/// A writer holding the result-set lock must keep this commit from writing a
/// single child until it releases. D-1745.
#[test]
fn an_unadmitted_commit_waits_for_the_result_set_writer_lock() {
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
    let root = root();
    let held = super::ResultSetLock::acquire(&root).expect("the competing writer locks");
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let into = options(&root).recording.expect("fixture recording target");
            done_tx
                .send(super::record_unadmitted(into, &id, &what))
                .expect("the waiting test receives the outcome");
        });
        assert!(
            done_rx
                .recv_timeout(std::time::Duration::from_millis(200))
                .is_err(),
            "the unadmitted commit finished while another writer held the result set"
        );
        assert!(
            !super::frontier::Frontier::path(&root).exists(),
            "no child was prepared under another writer's lock"
        );
        assert!(!super::results::Results::path(&root).exists());
        held.0.release().expect("the competing writer unlocks");
        let report = done_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the commit proceeds once the lock is free")
            .expect("the unadmitted commit records");
        assert!(report.contains("No final screened candidate was admitted"));
    });
    assert!(super::results::Results::path(&root).is_file());
    assert!(super::frontier::Frontier::path(&root).is_file());
}

/// The unadmitted commit names both locks before its first child and the
/// directory barrier between its receipt and its ledger row, in that order.
#[test]
fn the_unadmitted_commit_takes_both_locks_and_the_directory_barrier_in_order() {
    let source = include_str!("lib.rs");
    let body = source
        .split_once("\nfn record_unadmitted(")
        .and_then(|(_, after)| after.split_once("\n}\n"))
        .map(|(body, _)| body)
        .expect("record_unadmitted remains one function");
    let at = |needle: &str| {
        body.find(needle)
            .expect("record_unadmitted names every step")
    };
    let order = [
        at("LEDGER.lock()"),
        at("ResultSetLock::acquire(into.root)"),
        at("record_frontier("),
        at("ensure_detail_receipt("),
        at("confirm_result_directory(into.root)"),
        at("record_swept_run("),
        at(".release()"),
    ];
    assert!(order.is_sorted(), "persistence order {order:?}");
}

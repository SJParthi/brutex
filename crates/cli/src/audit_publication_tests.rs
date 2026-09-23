//! Fault injection into actual audit finalization, using generated finite bars.
#![allow(clippy::expect_used, reason = "a failed fixture must fail its test")]

use crate::sweep_evidence::{self, Attempt, Completion, Evidence, Operation};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

type Fault = Box<dyn FnOnce(&Attempt)>;
thread_local! {
    static BEFORE_TERMINAL: RefCell<Option<Fault>> = const { RefCell::new(None) };
}

pub(super) fn before_terminal(attempt: Option<&Attempt>) {
    if let Some(attempt) = attempt {
        let fault = BEFORE_TERMINAL.with(|slot| slot.borrow_mut().take());
        if let Some(fault) = fault {
            fault(attempt);
        }
    }
}

pub(crate) struct FaultGuard;
impl FaultGuard {
    pub(crate) fn install(fault: impl FnOnce(&Attempt) + 'static) -> Self {
        BEFORE_TERMINAL.with(|slot| {
            assert!(slot.borrow().is_none(), "test fault cannot replace another");
            *slot.borrow_mut() = Some(Box::new(fault));
        });
        Self
    }
}
impl Drop for FaultGuard {
    fn drop(&mut self) {
        BEFORE_TERMINAL.with(|slot| *slot.borrow_mut() = None);
    }
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-audit-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("unique fixture root");
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn identity(bars: &[indicators::Candle], min_hits: u64) -> runner::identity::RunId {
    let key = crate::stored::swept_index("NIFTY").expect("fixture key");
    runner::identity::identity(&runner::identity::Run {
        mask: vocab::ConditionMask::default(),
        direction: runner::identity::Direction::Undirected,
        instrument: &key,
        timeframe: "1min",
        params: runner::identity::Params::of(
            engine::Ladder::with_min_hits(min_hits).with_ceiling(64),
        ),
        data_digest: runner::identity::data_digest(bars),
        commit: "generated-audit-publication-fixture",
        feed: "zerodha",
    })
}

fn options(root: &Path) -> crate::AuditOptions<'_> {
    crate::AuditOptions {
        prepared_column: None,
        replay: None,
        execution: None,
        native_minute_execution: true,
        recording: Some(crate::Recording {
            root,
            feed: "zerodha",
            underlying: "NIFTY",
            timeframe: "1min",
            from: (2025, 5),
            to: (2025, 5),
            attempt: None,
            months_asked: 1,
            months_found: 1,
        }),
        rules: crate::Rules::BASELINE,
        lens: runner::rank::Lens::Detectability,
        ceiling: Some(64),
        validate: false,
    }
}

#[test]
fn strict_source_refusal_is_saved_before_any_audit_computation() {
    let fixture = Fixture::new();
    let bars = runner::synthetic::sessions(7);
    let min_hits = u64::try_from(bars.len()).expect("bounded bars") + 1;
    let id = identity(&bars, min_hits);
    let calls = Cell::new(0);
    let refuse_source = || {
        calls.set(calls.get() + 1);
        Err("retained source was replaced".to_owned())
    };
    let report = crate::audit_bars_guarded(
        &crate::evaluator(),
        bars,
        "GENERATED TEST FIXTURE",
        min_hits,
        Some(&id),
        options(&fixture.0),
        Some(&refuse_source),
    );
    assert_eq!(calls.get(), 1);
    assert!(crate::carries_refusal(&report), "{report}");
    assert!(report.contains("retained source was replaced"), "{report}");
    let saved = sweep_evidence::read(&fixture.0, id.bytes(), 1_048_576)
        .expect("read refused attempt")
        .expect("source refusal must be durable");
    assert_eq!(saved.completion, Completion::Refused);
    assert_eq!(saved.depth_rows, 0);
    assert!(!saved.ranked_available);
    assert!(!crate::results::Results::path(&fixture.0).exists());
}

#[test]
fn strict_source_must_remain_current_until_the_real_audit_terminal() {
    for replaced in [false, true] {
        let fixture = Fixture::new();
        let bars = runner::synthetic::sessions(7);
        let min_hits = u64::try_from(bars.len()).expect("bounded bars") + 1;
        let id = identity(&bars, min_hits);
        let current = Rc::new(Cell::new(true));
        let at_terminal = Rc::clone(&current);
        let _fault = FaultGuard::install(move |_| at_terminal.set(!replaced));
        let calls = Cell::new(0);
        let check_source = || {
            calls.set(calls.get() + 1);
            if current.get() {
                Ok(())
            } else {
                Err("retained source changed after ranking".to_owned())
            }
        };
        let report = crate::audit_bars_guarded(
            &crate::evaluator(),
            bars,
            "GENERATED TEST FIXTURE",
            min_hits,
            Some(&id),
            options(&fixture.0),
            Some(&check_source),
        );
        assert_eq!(calls.get(), 2, "{report}");
        let saved = sweep_evidence::read(&fixture.0, id.bytes(), 1_048_576)
            .expect("read final source-guarded audit")
            .expect("audit exists");
        assert!(saved.depth_rows > 0);
        assert!(saved.ranked_available);
        assert_eq!(
            saved.completion,
            if replaced {
                Completion::Refused
            } else {
                Completion::Completed
            }
        );
        assert_eq!(crate::carries_refusal(&report), replaced, "{report}");
        assert_eq!(
            crate::results::Results::path(&fixture.0).exists(),
            !replaced
        );
        if replaced {
            assert!(
                report.contains("retained source changed after ranking"),
                "{report}"
            );
        }
    }
}

fn corrupt_acknowledged(root: &Path, attempt: &Attempt, kind: &str, corrupt: bool) {
    let saved = sweep_evidence::read(root, attempt.identity(), 1_048_576)
        .expect("valid actual acknowledged evidence")
        .expect("actual audit start");
    assert_eq!(saved.operation, Operation::Audit);
    assert_eq!(saved.completion, Completion::Running);
    assert!(saved.depth_rows > 0, "actual ladder callback ran");
    assert!(saved.ranked_available, "actual ranking publication ran");
    let path = root
        .join("results/sweep-evidence-v1")
        .join(crate::identity_hex(&attempt.identity()))
        .join(format!("{}-{kind}.bin", attempt.token()));
    let mut bytes = fs::read(&path).expect("acknowledged actual child exists");
    assert!(bytes.len() >= 16);
    if corrupt {
        *bytes.last_mut().expect("fixed header or row seal") ^= 1;
        fs::write(path, bytes).expect("corrupt only this temporary attempt");
    } else {
        fs::remove_file(path).expect("lose only this temporary attempt child");
    }
}

#[test]
fn actual_audit_late_depth_or_ranking_loss_cannot_publish_a_parent() {
    for kind in ["levels", "ranked"] {
        for corrupt in [false, true] {
            let fixture = Fixture::new();
            let bars = runner::synthetic::sessions(7);
            let min_hits = u64::try_from(bars.len()).expect("bounded bars") + 1;
            let id = identity(&bars, min_hits);
            let invoked = Rc::new(Cell::new(false));
            let seen = Rc::clone(&invoked);
            let root = fixture.0.clone();
            let _fault = FaultGuard::install(move |attempt| {
                seen.set(true);
                corrupt_acknowledged(&root, attempt, kind, corrupt);
            });
            let report = crate::audit_bars(
                &crate::evaluator(),
                bars,
                "GENERATED TEST FIXTURE",
                min_hits,
                Some(&id),
                options(&fixture.0),
            );
            assert!(
                invoked.get(),
                "actual audit must reach finalization: {report}"
            );
            assert!(report.contains(crate::NOT_RECORDED), "{report}");
            assert!(!report.contains("RESULT RECORDED"), "{report}");
            assert!(!crate::results::Results::path(&fixture.0).exists());
            assert!(!matches!(
                sweep_evidence::read(&fixture.0, id.bytes(), 1_048_576),
                Ok(Some(Evidence {
                    completion: Completion::Completed,
                    ..
                }))
            ));
        }
    }
}

#[test]
fn actual_audit_publishes_only_after_the_real_empty_ranking_is_sealed() {
    let fixture = Fixture::new();
    let bars = runner::synthetic::sessions(7);
    let min_hits = u64::try_from(bars.len()).expect("bounded bars") + 1;
    let id = identity(&bars, min_hits);
    let invoked = Rc::new(Cell::new(false));
    let seen = Rc::clone(&invoked);
    let root = fixture.0.clone();
    let _fault = FaultGuard::install(move |_| {
        seen.set(true);
        assert!(!crate::results::Results::path(&root).exists());
    });
    let report = crate::audit_bars(
        &crate::evaluator(),
        bars,
        "GENERATED TEST FIXTURE",
        min_hits,
        Some(&id),
        options(&fixture.0),
    );
    assert!(invoked.get(), "actual audit reached finalization: {report}");
    assert!(report.contains("RESULT RECORDED"), "{report}");
    let saved = sweep_evidence::read(&fixture.0, id.bytes(), 1_048_576)
        .expect("authenticated terminal")
        .expect("actual attempt");
    assert_eq!(saved.completion, Completion::Completed);
    assert!(saved.depth_rows > 0);
    assert!(saved.ranked_available);
    assert_eq!(saved.ranked_rows, 0);
    assert!(
        crate::results::Results::open_read(&fixture.0)
            .expect("actual parent")
            .holds(&id.bytes())
    );
}

#[test]
fn test_fault_guard_cannot_leak_an_unconsumed_callback() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let seen = Rc::clone(&invoked);
    {
        let _guard = FaultGuard::install(move |_| seen.set(true));
    }
    let attempt =
        sweep_evidence::begin(&fixture.0, [91; 32], Operation::Audit).expect("fixture attempt");
    before_terminal(Some(&attempt));
    assert!(!invoked.get());
    attempt
        .finish(Completion::Refused)
        .expect("fixture cleanup");
}

fn sealed_capture<'a>(
    root: &Path,
    attempt: &Attempt,
    bars: &'a [indicators::Candle],
    column: &'a indicators::column::Column,
) -> crate::candidate_trades::Capture<'a> {
    use crate::candidate_trades::{Capture, Evaluated, Tier};
    use costs::fill::Direction;
    use runner::{grid, outcome::Horizon};

    let capture = Capture::begin(root, attempt, bars, column).expect("actual capture start");
    let tier = capture
        .tier(Tier {
            index: 0,
            eligible: 1,
            evaluated: 1,
            horizon: 3,
            rungs: 1,
            step_ppm: Some(100),
            forced_ppm: None,
            ratios: true,
            rules: crate::Rules::BASELINE,
            stops_ppm: vec![100],
        })
        .expect("actual policy tier");
    let mask = vocab::ConditionMask::default();
    for direction in [Direction::Long, Direction::Short] {
        let grid = grid::evaluate(
            bars,
            column,
            &mask,
            Horizon::bars(3).expect("bounded horizon"),
            crate::side_of_direction(direction),
            grid::Levels {
                rungs: 1,
                step_ppm: Some(100),
                forced: None,
                ratios: true,
                stops_ppm: &[100],
            },
        );
        capture
            .record(
                &tier,
                &Evaluated {
                    rank: 1,
                    mask: &mask,
                    direction,
                    grid: &grid,
                    selected: crate::shown_cell(&grid, tier.rules),
                },
            )
            .expect("exact selected cell and trades for this side");
    }
    let summary = capture.finish().expect("sealed exact capture");
    assert_eq!(summary.candidates, 2);
    capture
}

#[test]
fn audit_finalizer_rechecks_exact_priced_catalog_and_children_before_terminal() {
    let bars = runner::synthetic::sessions(7);
    let column = indicators::column::Column::build(
        &bars,
        &mut crate::evaluator().expect("generated fixture evaluator"),
    );
    for file in [
        None,
        Some("catalog.bin"),
        Some("tier-0.bin"),
        Some("0-1-0-candidate.bin"),
        Some("0-1-0-trades.bin"),
    ] {
        let fixture = Fixture::new();
        let id = identity(&bars, 1);
        let attempt = sweep_evidence::begin(&fixture.0, id.bytes(), Operation::Audit)
            .expect("exact audit identity");
        attempt.ranked(&[]).expect("explicit empty signal ranking");
        let capture = sealed_capture(&fixture.0, &attempt, &bars, &column);
        if let Some(file) = file {
            let path = fixture
                .0
                .join("results/candidate-trades-v1")
                .join(id.hex())
                .join(attempt.token().to_string())
                .join(file);
            assert!(
                fs::metadata(&path)
                    .expect("acknowledged priced evidence")
                    .len()
                    > 0
            );
            fs::remove_file(path).expect("lose only this exact fixture child");
        }
        let mut evidence = crate::AuditEvidence {
            attempt: Some(attempt),
            source_check: None,
        };
        let result = evidence.seal(Completion::Completed, Some(&capture));
        assert_eq!(result.is_ok(), file.is_none(), "{file:?}: {result:?}");
        assert!(
            evidence.attempt.is_none(),
            "taken terminal is never resealed later"
        );
        let saved = sweep_evidence::read(&fixture.0, id.bytes(), 1_048_576)
            .expect("audit terminal")
            .expect("exact attempt");
        assert_eq!(
            saved.completion,
            if file.is_none() {
                Completion::Completed
            } else {
                Completion::Refused
            }
        );
        assert!(!crate::results::Results::path(&fixture.0).exists());
    }
}

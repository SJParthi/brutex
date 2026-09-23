//! Generated evidence fixtures proving the shared stored-month publication order.
#![allow(clippy::expect_used, reason = "a fixture failure must fail its test")]

use crate::sweep_evidence::{self, Attempt, Completion, DepthRow, Evidence, Operation, RankedRow};
use runner::identity::{Direction, Params, Run, RunId};
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const LIMIT: u64 = 1_048_576;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    id: RunId,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-stored-month-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("unique generated fixture");
        let key = crate::stored::swept_index("NIFTY").expect("canonical fixture key");
        let id = runner::identity::identity(&Run {
            mask: vocab::ConditionMask::default(),
            direction: Direction::Undirected,
            instrument: &key,
            timeframe: "1min",
            params: Params::of(engine::Ladder::with_min_hits(1)),
            data_digest: [89; 32],
            commit: "generated-publication-fixture",
            feed: "zerodha",
        });
        Self { root, id }
    }

    fn attempt(&self) -> Attempt {
        let attempt = sweep_evidence::begin(&self.root, self.id.bytes(), Operation::Sweep)
            .expect("durable identity before fixture evidence");
        attempt
            .level(DepthRow {
                k: 1,
                generated: 3,
                duplicates: 0,
                pruned: 0,
                infrequent: 2,
                frequent: 1,
                admitted: 3,
                pairs: 0,
                reconciles: true,
                excluded: 0,
            })
            .expect("acknowledged depth");
        attempt
            .ranked(&[RankedRow {
                rank: 1,
                mask_words: [1, 0, 0, 0, 0, 0],
                hits: 1,
                observations: 1,
                mean_bits: 1.0_f64.to_bits(),
                t_bits: 0,
                refused: 0,
                mismatched: 0,
                wins: 1,
                losses: 0,
                win_sum_bits: 1.0_f64.to_bits(),
                loss_sum_bits: 0,
                adverse_sum_bits: 0,
                favourable_sum_bits: 1.0_f64.to_bits(),
            }])
            .expect("acknowledged ranked row");
        attempt
    }

    fn read(&self) -> Evidence {
        sweep_evidence::read(&self.root, self.id.bytes(), LIMIT)
            .expect("valid evidence")
            .expect("the exact durable attempt")
    }

    fn child(&self, token: u64, kind: &str) -> PathBuf {
        self.root
            .join("results/sweep-evidence-v1")
            .join(self.id.hex())
            .join(format!("{token}-{kind}.bin"))
    }

    fn publish(&self) -> Result<String, String> {
        let sweep = engine::keep::Streamed {
            bars: 1,
            min_hits: 1,
            ..engine::keep::Streamed::default()
        };
        crate::record_swept_run(
            crate::Recording {
                root: &self.root,
                feed: "zerodha",
                underlying: "NIFTY",
                timeframe: "1min",
                from: (2025, 5),
                to: (2025, 5),
                attempt: None,
                months_asked: 1,
                months_found: 1,
            },
            &self.id,
            &sweep,
            1,
            1,
        )
        .map(|(report, _)| report)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn lost_or_corrupt_acknowledged_children_never_reach_the_real_parent_writer() {
    for kind in ["levels", "ranked"] {
        for corrupt in [false, true] {
            let fixture = Fixture::new();
            let attempt = fixture.attempt();
            let path = fixture.child(attempt.token(), kind);
            if corrupt {
                let mut bytes = fs::read(&path).expect("acknowledged child");
                *bytes.last_mut().expect("child seal") ^= 1;
                fs::write(&path, bytes).expect("corrupt only the scratch child");
            } else {
                fs::remove_file(&path).expect("remove only the scratch child");
            }
            let published = Cell::new(false);
            let result = crate::finish_stored_month(
                attempt,
                Completion::Completed,
                || Ok(()),
                || {
                    published.set(true);
                    fixture.publish()
                },
            );
            assert!(result.is_err(), "{kind}, corruption={corrupt}: {result:?}");
            assert!(!published.get(), "parent callback must remain unreachable");
            assert!(!crate::results::Results::path(&fixture.root).exists());
            assert!(
                !matches!(
                    sweep_evidence::read(&fixture.root, fixture.id.bytes(), LIMIT),
                    Ok(Some(Evidence {
                        completion: Completion::Completed,
                        ..
                    }))
                ),
                "damaged acknowledged evidence cannot gain a completed terminal"
            );
        }
    }
}

#[test]
fn strict_guard_refusal_precedes_terminal_and_parent_publication() {
    let fixture = Fixture::new();
    let attempt = fixture.attempt();
    let result = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || {
            assert_eq!(fixture.read().completion, Completion::Running);
            Err("exact retained source changed".to_owned())
        },
        || fixture.publish(),
    );
    assert_eq!(result, Err("exact retained source changed".to_owned()));
    assert_eq!(fixture.read().completion, Completion::Refused);
    assert!(!crate::results::Results::path(&fixture.root).exists());
}

#[test]
fn genuine_terminal_and_all_acknowledged_rows_exist_before_parent_publication() {
    let fixture = Fixture::new();
    let attempt = fixture.attempt();
    let checked = Cell::new(false);
    let report = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || {
            assert_eq!(fixture.read().completion, Completion::Running);
            checked.set(true);
            Ok(())
        },
        || {
            assert!(checked.get(), "source check precedes terminal and callback");
            let sealed = fixture.read();
            assert_eq!(sealed.completion, Completion::Completed);
            assert_eq!((sealed.depth_rows, sealed.ranked_rows), (1, 1));
            assert!(sealed.ranked_available);
            assert!(!crate::results::Results::path(&fixture.root).exists());
            fixture.publish()
        },
    )
    .expect("shared publication boundary");
    assert!(report.contains("RESULT RECORDED"));
    let mut results = crate::results::Results::open_read(&fixture.root).expect("actual parent");
    assert!(results.holds(&fixture.id.bytes()));
    assert_eq!(
        results.read(0).expect("one exact parent").identity,
        fixture.id.bytes()
    );
}

#[test]
fn parent_write_refusal_keeps_completed_computation_and_returns_the_exact_failure() {
    let fixture = Fixture::new();
    let attempt = fixture.attempt();
    let result = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || Ok(()),
        || {
            assert_eq!(fixture.read().completion, Completion::Completed);
            Err::<(), _>("parent append was refused".to_owned())
        },
    );
    assert_eq!(result, Err("parent append was refused".to_owned()));
    let saved = fixture.read();
    assert_eq!(saved.completion, Completion::Completed);
    assert_eq!((saved.depth_rows, saved.ranked_rows), (1, 1));
    assert!(!crate::results::Results::path(&fixture.root).exists());
}

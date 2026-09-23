//! Public-API adversarial checks for durable sweep attempt evidence.
//! Every mutation below targets a unique temporary fixture, never a market store.

#![allow(clippy::expect_used, reason = "test fixtures must refuse loudly")]

use cli::sweep_evidence::{self as evidence, Completion, DepthRow, Evidence, Operation, RankedRow};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

const LIMIT: u64 = 1_000_000;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("positive fixture clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "brutex-sweep-evidence-test-{}-{nanos}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("a unique temporary fixture");
        Self(root)
    }
    fn base(&self) -> PathBuf {
        self.0.join("results/sweep-evidence-v1")
    }
    fn child(&self, saved: &Evidence, kind: &str) -> PathBuf {
        let mut identity = String::with_capacity(64);
        for byte in saved.identity {
            write!(identity, "{byte:02x}").expect("format identity into a string");
        }
        self.base()
            .join(identity)
            .join(format!("{}-{kind}.bin", saved.attempt))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn depth(k: u64) -> DepthRow {
    DepthRow {
        k,
        generated: 5,
        duplicates: 0,
        pruned: 1,
        infrequent: 2,
        frequent: 2,
        admitted: 7,
        pairs: 11,
        reconciles: true,
        excluded: 0,
    }
}

fn rank(index: u64) -> RankedRow {
    RankedRow {
        rank: index,
        mask_words: [1_u64 << 63, index, 0, 0, 0, 1_u64 << 49],
        hits: 37,
        observations: 31,
        mean_bits: 0x3ff0_0000_0000_0001,
        t_bits: (-0.0_f64).to_bits(),
        refused: 2,
        mismatched: 4,
        wins: 19,
        losses: 12,
        win_sum_bits: 123.456_789_123_456_f64.to_bits(),
        loss_sum_bits: (-98.765_432_198_765_f64).to_bits(),
        adverse_sum_bits: (-765.432_198_765_432_f64).to_bits(),
        favourable_sum_bits: 987.654_321_987_654_f64.to_bits(),
    }
}

fn read(fixture: &Fixture, identity: [u8; 32]) -> Evidence {
    evidence::read(&fixture.0, identity, LIMIT)
        .expect("read valid fixture")
        .expect("durable attempt")
}

fn completed(fixture: &Fixture, identity: [u8; 32]) -> Evidence {
    let attempt = evidence::begin(&fixture.0, identity, Operation::Sweep).expect("durable start");
    attempt.level(depth(1)).expect("first depth");
    attempt.level(depth(2)).expect("second depth");
    attempt
        .ranked(&[rank(1), rank(2)])
        .expect("ranked evidence");
    attempt
        .finish(Completion::Completed)
        .expect("durable completion");
    read(fixture, identity)
}

fn flip(path: &Path, offset: usize) {
    let mut bytes = fs::read(path).expect("fixture file exists");
    *bytes.get_mut(offset).expect("corruption offset exists") ^= 0x80;
    fs::write(path, bytes).expect("write only the temporary fixture corruption");
}

#[test]
fn lifecycle_pages_and_full_precision_rows_round_trip_without_identity_loss() {
    let fixture = Fixture::new();
    assert_eq!(
        evidence::read(&fixture.0, [1; 32], LIMIT).expect("missing identity"),
        None
    );
    assert_eq!(
        evidence::latest(&fixture.0, LIMIT).expect("empty global journal"),
        None
    );
    let attempt = evidence::begin(&fixture.0, [1; 32], Operation::Sweep).expect("start");
    let token = attempt.token();
    assert_eq!(attempt.identity(), [1; 32]);
    let running = read(&fixture, [1; 32]);
    assert_eq!(running.completion, Completion::Running);
    assert_eq!(running.attempt, token);
    assert!(!running.ranked_available);
    attempt.level(depth(1)).expect("depth one");
    attempt.level(depth(2)).expect("depth two");
    attempt.ranked(&[rank(1), rank(2)]).expect("ranks");
    attempt.finish(Completion::Completed).expect("finish");
    let saved = read(&fixture, [1; 32]);
    assert_eq!((saved.depth_rows, saved.ranked_rows), (2, 2));
    assert!(saved.ranked_available);
    assert_eq!(saved.completion, Completion::Completed);
    assert!(saved.updated_micros >= saved.started_micros);
    assert_eq!(
        evidence::latest(&fixture.0, LIMIT).expect("latest"),
        Some(saved)
    );
    assert_eq!(
        evidence::depth_page(&fixture.0, &saved, 0, 1, LIMIT).expect("first depth page"),
        [depth(1)]
    );
    assert_eq!(
        evidence::depth_page(&fixture.0, &saved, 1, 5, LIMIT).expect("last depth page"),
        [depth(2)]
    );
    assert_eq!(
        evidence::ranked_page(&fixture.0, &saved, 0, 2, LIMIT).expect("exact ranked row bytes"),
        [rank(1), rank(2)]
    );
    for offset in [2, 999, u64::MAX] {
        assert!(
            evidence::ranked_page(&fixture.0, &saved, offset, 4, LIMIT)
                .expect("past-end page")
                .is_empty()
        );
    }
    assert!(
        evidence::depth_page(&fixture.0, &saved, 0, 0, LIMIT)
            .expect("empty page")
            .is_empty()
    );
    assert!(evidence::depth_page(&fixture.0, &saved, 0, 4097, LIMIT).is_err());
    assert!(evidence::ranked_page(&fixture.0, &saved, 0, usize::MAX, LIMIT).is_err());
    assert!(
        evidence::read(&fixture.0, [1; 32], 16).is_err(),
        "read limit must not silently truncate"
    );
}

#[test]
fn explicit_empty_ranking_is_distinct_from_unpublished_and_cannot_be_republished() {
    for first in [Vec::new(), vec![rank(1)]] {
        let fixture = Fixture::new();
        let attempt = evidence::begin(&fixture.0, [2; 32], Operation::Sweep).expect("start");
        attempt
            .ranked(&first)
            .expect("first publication, including empty");
        assert!(
            attempt.ranked(&[]).is_err(),
            "duplicate publication must refuse even after zero rows"
        );
        assert!(
            attempt.finish(Completion::Completed).is_err(),
            "a poisoned attempt cannot seal success"
        );
        let saved = read(&fixture, [2; 32]);
        assert_eq!(saved.completion, Completion::Refused);
        assert!(saved.ranked_available);
        assert_eq!(
            evidence::ranked_page(&fixture.0, &saved, 0, 10, LIMIT).expect("original publication"),
            first
        );
    }
}

#[test]
fn malformed_ranks_and_running_as_terminal_refuse_and_record_refused() {
    for invalid in [vec![rank(0)], vec![rank(2)], vec![rank(1), rank(3)]] {
        let fixture = Fixture::new();
        let attempt = evidence::begin(&fixture.0, [3; 32], Operation::Sweep).expect("start");
        assert!(attempt.ranked(&invalid).is_err());
        assert!(attempt.finish(Completion::Completed).is_err());
        assert_eq!(read(&fixture, [3; 32]).completion, Completion::Refused);
    }
    let fixture = Fixture::new();
    let attempt = evidence::begin(&fixture.0, [3; 32], Operation::Sweep).expect("start");
    assert!(attempt.finish(Completion::Running).is_err());
    assert_eq!(read(&fixture, [3; 32]).completion, Completion::Refused);
}

#[test]
fn validation_request_and_completion_kind_remain_explicit() {
    let fixture = Fixture::new();
    for (operation, validation, status, byte) in [
        (Operation::Audit, Some(true), Completion::Completed, 10),
        (Operation::Audit, Some(false), Completion::Halted, 11),
        (Operation::AutoSearch, None, Completion::Refused, 12),
        (Operation::AutoProbe, None, Completion::Halted, 13),
        (Operation::Expression, None, Completion::Completed, 14),
        (Operation::ExpressionSearch, None, Completion::Halted, 16),
        (Operation::BooleanOos, None, Completion::Completed, 17),
        (
            Operation::BooleanQualification,
            None,
            Completion::Completed,
            18,
        ),
    ] {
        let attempt =
            evidence::begin_with_validation(&fixture.0, [byte; 32], operation, validation)
                .expect("declared operation");
        attempt.finish(status).expect("declared terminal state");
        let saved = read(&fixture, [byte; 32]);
        assert_eq!(
            (
                saved.operation,
                saved.validation_requested,
                saved.completion
            ),
            (operation, validation, status)
        );
    }
    assert!(
        evidence::begin_with_validation(&fixture.0, [15; 32], Operation::Sweep, Some(true))
            .is_err()
    );
}

#[test]
fn exact_attempt_reader_never_substitutes_a_newer_search_attempt() {
    let fixture = Fixture::new();
    let first =
        evidence::begin(&fixture.0, [31; 32], Operation::ExpressionSearch).expect("first search");
    let first_token = first.token();
    first
        .finish(Completion::Halted)
        .expect("bounded search pauses");
    let second =
        evidence::begin(&fixture.0, [31; 32], Operation::ExpressionSearch).expect("resumed search");
    let second_token = second.token();
    second
        .finish(Completion::Completed)
        .expect("grammar exhaustion may complete");
    let exact = evidence::read_attempt(&fixture.0, [31; 32], first_token, u64::MAX)
        .expect("read first")
        .expect("first exists");
    assert_eq!(exact.attempt, first_token);
    assert_eq!(exact.completion, Completion::Halted);
    let latest = read(&fixture, [31; 32]);
    assert_eq!(latest.attempt, second_token);
    assert_eq!(latest.completion, Completion::Completed);
    assert!(evidence::read_attempt(&fixture.0, [31; 32], 0, u64::MAX).is_err());
    assert!(
        evidence::read_attempt(&fixture.0, [32; 32], first_token, u64::MAX)
            .expect("other identity")
            .is_none()
    );
}

#[test]
fn absent_drop_and_interleaved_reruns_cannot_fabricate_completed_latest_attempts() {
    let fixture = Fixture::new();
    let abandoned = evidence::begin(&fixture.0, [4; 32], Operation::Sweep).expect("start");
    std::mem::forget(abandoned); // Deliberately suppress Drop: model a stopped process.
    assert_eq!(read(&fixture, [4; 32]).completion, Completion::Running);
    let first = evidence::begin(&fixture.0, [4; 32], Operation::Sweep).expect("first rerun");
    let second = evidence::begin(&fixture.0, [4; 32], Operation::Sweep).expect("second rerun");
    assert!(second.token() > first.token());
    let latest_token = second.token();
    second
        .finish(Completion::Completed)
        .expect("newer finishes first");
    first
        .finish(Completion::Completed)
        .expect("older finishes last");
    assert_eq!(read(&fixture, [4; 32]).attempt, latest_token);
    let dropped = evidence::begin(&fixture.0, [4; 32], Operation::Sweep).expect("third rerun");
    let dropped_token = dropped.token();
    drop(dropped);
    let saved = read(&fixture, [4; 32]);
    assert_eq!(
        (saved.attempt, saved.completion),
        (dropped_token, Completion::Refused)
    );
}

#[test]
fn deleting_a_declared_child_is_corruption_including_an_explicit_empty_ranking() {
    for kind in ["levels", "ranked"] {
        let fixture = Fixture::new();
        let saved = completed(&fixture, [5; 32]);
        fs::remove_file(fixture.child(&saved, kind)).expect("delete fixture child");
        assert!(evidence::read(&fixture.0, saved.identity, LIMIT).is_err());
    }
    let fixture = Fixture::new();
    let attempt = evidence::begin(&fixture.0, [5; 32], Operation::Sweep).expect("start");
    attempt.ranked(&[]).expect("legitimate empty ranking");
    attempt.finish(Completion::Completed).expect("finish");
    let saved = read(&fixture, [5; 32]);
    assert!(
        evidence::ranked_page(&fixture.0, &saved, 0, 1, LIMIT)
            .expect("empty ranking")
            .is_empty()
    );
    fs::remove_file(fixture.child(&saved, "ranked")).expect("delete empty declared child");
    assert!(evidence::read(&fixture.0, saved.identity, LIMIT).is_err());
    assert!(evidence::ranked_page(&fixture.0, &saved, 0, 1, LIMIT).is_err());
}

#[test]
fn acknowledged_rows_lost_before_finish_cannot_be_relabelled_as_a_completed_empty_run() {
    for kind in ["levels", "ranked"] {
        for truncate in [false, true] {
            let fixture = Fixture::new();
            let attempt = evidence::begin(&fixture.0, [32; 32], Operation::Sweep).expect("start");
            attempt.level(depth(1)).expect("acknowledged depth");
            attempt.ranked(&[rank(1)]).expect("acknowledged ranking");
            let running = read(&fixture, [32; 32]);
            assert_eq!((running.depth_rows, running.ranked_rows), (1, 1));
            let child = fixture.child(&running, kind);
            if truncate {
                OpenOptions::new()
                    .write(true)
                    .open(&child)
                    .expect("fixture child")
                    .set_len(16)
                    .expect("lose acknowledged records but preserve valid header");
            } else {
                fs::remove_file(child).expect("lose acknowledged child");
            }
            assert!(
                attempt.finish(Completion::Completed).is_err(),
                "{kind} truncate={truncate}: lost acknowledged rows cannot seal a success"
            );
            let observed = evidence::read(&fixture.0, [32; 32], LIMIT);
            assert!(!observed.is_ok_and(|row| row.is_some_and(|row| row.completion == Completion::Completed)),
                "a failed completion cannot leave an empty completed lifecycle");
        }
    }
}

#[test]
fn same_count_foreign_rows_before_finish_cannot_seal_acknowledged_local_evidence() {
    for kind in ["levels", "ranked"] {
        for replace_inode in [false, true] {
            let fixture = Fixture::new();
            let target =
                evidence::begin(&fixture.0, [33; 32], Operation::Sweep).expect("local start");
            target.level(depth(1)).expect("local depth");
            target.ranked(&[rank(1)]).expect("local ranking");
            let running = read(&fixture, [33; 32]);
            let donor =
                evidence::begin(&fixture.0, [34; 32], Operation::Sweep).expect("foreign start");
            donor.level(depth(1)).expect("foreign depth");
            donor.ranked(&[rank(1)]).expect("foreign ranking");
            donor
                .finish(Completion::Completed)
                .expect("foreign complete");
            let foreign = read(&fixture, [34; 32]);
            let destination = fixture.child(&running, kind);
            let source = fixture.child(&foreign, kind);
            assert_eq!(
                fs::metadata(&destination).expect("local shape").len(),
                fs::metadata(&source).expect("foreign shape").len()
            );
            if replace_inode {
                fs::remove_file(&destination).expect("remove only local fixture child");
                fs::hard_link(&source, &destination).expect("replace with foreign sealed inode");
            } else {
                fs::write(
                    &destination,
                    fs::read(&source).expect("valid foreign bytes"),
                )
                .expect("overwrite local inode with equally sized foreign evidence");
            }
            assert!(
                target.finish(Completion::Completed).is_err(),
                "{kind} replace_inode={replace_inode}: equal cardinality does not prove local provenance"
            );
        }
    }
}

#[test]
fn torn_children_and_corrupt_requested_rows_refuse_instead_of_returning_partial_pages() {
    for kind in ["levels", "ranked"] {
        let fixture = Fixture::new();
        let saved = completed(&fixture, [6; 32]);
        let path = fixture.child(&saved, kind);
        let original = fs::read(&path).expect("fixture child");
        fs::write(
            &path,
            original
                .get(..original.len() - 1)
                .expect("nonempty fixture"),
        )
        .expect("torn final record");
        assert!(evidence::read(&fixture.0, saved.identity, LIMIT).is_err());
        fs::write(&path, &original).expect("restore own fixture");
        flip(&path, 16 + 40); // A payload byte, preserving length and the header.
        let refused = if kind == "levels" {
            evidence::depth_page(&fixture.0, &saved, 0, 2, LIMIT).is_err()
        } else {
            evidence::ranked_page(&fixture.0, &saved, 0, 2, LIMIT).is_err()
        };
        assert!(
            refused,
            "{kind}: corruption must never yield a partial successful page"
        );
    }
}

#[test]
fn foreign_identity_or_attempt_rows_cannot_be_imported_as_local_evidence() {
    for same_identity in [false, true] {
        for kind in ["levels", "ranked"] {
            let fixture = Fixture::new();
            let target = completed(&fixture, [7; 32]);
            let donor = completed(&fixture, if same_identity { [7; 32] } else { [8; 32] });
            assert_ne!(target.attempt, donor.attempt);
            fs::copy(fixture.child(&donor, kind), fixture.child(&target, kind))
                .expect("foreign but valid sealed child");
            let refused = if kind == "levels" {
                evidence::depth_page(&fixture.0, &target, 0, 2, LIMIT).is_err()
            } else {
                evidence::ranked_page(&fixture.0, &target, 0, 2, LIMIT).is_err()
            };
            assert!(
                refused,
                "{kind}: a valid checksum cannot authorize a foreign identity/attempt"
            );
        }
    }
}

#[test]
fn lifecycle_corruption_missing_start_and_torn_global_history_are_explicit_refusals() {
    let fixture = Fixture::new();
    let saved = completed(&fixture, [9; 32]);
    flip(&fixture.child(&saved, "lifecycle"), 16 + 96 + 48);
    assert!(evidence::read(&fixture.0, saved.identity, LIMIT).is_err());
    let other = completed(&fixture, [10; 32]);
    fs::remove_file(fixture.base().join(format!("{}-start.bin", other.attempt)))
        .expect("delete own reservation");
    assert!(evidence::read(&fixture.0, other.identity, LIMIT).is_err());
    let mut global = OpenOptions::new()
        .append(true)
        .open(fixture.base().join("attempts.bin"))
        .expect("global history");
    global.write_all(&[0x80]).expect("one torn suffix byte");
    global.sync_all().expect("persist test fault");
    assert!(evidence::latest(&fixture.0, LIMIT).is_err());
    assert!(evidence::begin(&fixture.0, [11; 32], Operation::Sweep).is_err());
}

#[test]
fn concurrent_same_identity_starts_have_unique_tokens_and_preserve_the_newest_start() {
    let fixture = Fixture::new();
    let barrier = Arc::new(Barrier::new(8));
    let outcomes = std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for _ in 0..8 {
            let barrier = Arc::clone(&barrier);
            let root = &fixture.0;
            threads.push(scope.spawn(move || {
                barrier.wait();
                let attempt = evidence::begin(root, [12; 32], Operation::Sweep)?;
                let token = attempt.token();
                attempt.finish(Completion::Completed)?;
                Ok::<_, String>(token)
            }));
        }
        threads
            .into_iter()
            .map(|thread| thread.join().expect("evidence writer cannot panic"))
            .collect::<Vec<_>>()
    });
    let mut tokens = BTreeSet::new();
    for outcome in outcomes {
        match outcome {
            Ok(token) => assert!(tokens.insert(token), "concurrent starts reused a token"),
            Err(why) => assert!(
                why.contains("superseded"),
                "unexpected concurrent refusal: {why}"
            ),
        }
    }
    let latest = *tokens
        .last()
        .expect("at least the newest start must succeed");
    let saved = read(&fixture, [12; 32]);
    assert_eq!(
        (saved.attempt, saved.completion),
        (latest, Completion::Completed)
    );
}

#[test]
fn concurrent_different_identities_keep_separate_rows_and_globally_unique_attempts() {
    let fixture = Fixture::new();
    let barrier = Arc::new(Barrier::new(8));
    let outcomes = std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for byte in 20..28_u8 {
            let barrier = Arc::clone(&barrier);
            let root = &fixture.0;
            threads.push(scope.spawn(move || {
                barrier.wait();
                let attempt = evidence::begin(root, [byte; 32], Operation::Sweep)
                    .expect("distinct identity start");
                let token = attempt.token();
                attempt
                    .level(depth(u64::from(byte)))
                    .expect("identity-specific depth");
                attempt
                    .ranked(&[rank(1)])
                    .expect("identity-specific ranking");
                attempt
                    .finish(Completion::Completed)
                    .expect("distinct identity completion");
                (byte, token)
            }));
        }
        threads
            .into_iter()
            .map(|thread| thread.join().expect("distinct writer cannot panic"))
            .collect::<Vec<_>>()
    });
    let mut tokens = BTreeSet::new();
    for (byte, token) in outcomes {
        assert!(
            tokens.insert(token),
            "attempt sequence is global across identities"
        );
        let saved = read(&fixture, [byte; 32]);
        assert_eq!(
            (saved.attempt, saved.completion),
            (token, Completion::Completed)
        );
        assert_eq!(
            evidence::depth_page(&fixture.0, &saved, 0, 8, LIMIT).expect("own depth"),
            [depth(u64::from(byte))]
        );
        assert_eq!(
            evidence::ranked_page(&fixture.0, &saved, 0, 8, LIMIT).expect("own ranking"),
            [rank(1)]
        );
    }
    assert_eq!(tokens.len(), 8);
}

#[test]
fn rolled_back_global_history_cannot_reuse_an_immutable_attempt_reservation() {
    let fixture = Fixture::new();
    let saved = completed(&fixture, [30; 32]);
    let reservation = fixture.base().join(format!("{}-start.bin", saved.attempt));
    let before = fs::read(&reservation).expect("immutable start");
    OpenOptions::new()
        .write(true)
        .open(fixture.base().join("attempts.bin"))
        .expect("fixture global history")
        .set_len(16)
        .expect("model rollback to the header in this fixture only");
    let refused = evidence::begin(&fixture.0, [31; 32], Operation::Sweep)
        .err()
        .expect("rolled-back token must refuse before computation");
    assert!(refused.contains("reservation"), "{refused}");
    assert_eq!(
        fs::read(reservation).expect("original reservation retained"),
        before
    );
    assert_eq!(read(&fixture, [30; 32]), saved);
    assert_eq!(
        evidence::read(&fixture.0, [31; 32], LIMIT).expect("new identity was not published"),
        None
    );
}

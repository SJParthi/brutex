//! Durability before OOS work, using the shared append-only attempt authority.
//! Plan identity is explicitly a source/request identity; exact per-strategy
//! Execution run identities are recorded once their stored bytes are admitted.
use std::path::Path;

use super::{GlobalReplayV4Bounds, SelectionV6Snapshot, StoredPostTrainingOosRequestV1};
use crate::stored_post_training_oos::{StoredOosComputationV1, StoredOosObserverV1};
use crate::sweep_evidence::{self, Attempt, Completion, Operation};

pub(super) fn plan_identity(
    snapshots: &[SelectionV6Snapshot; 8],
    request: StoredPostTrainingOosRequestV1,
    bounds: GlobalReplayV4Bounds,
) -> Result<[u8; 32], String> {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(b"brutex-global-replay-v4-plan-v1\0");
    hash.update(&super::codec::header(request)?);
    for snapshot in snapshots {
        hash.update(&snapshot.identity);
        hash.update(&snapshot.envelope);
    }
    for bound in [
        request.signal_bound().max_records(),
        request.minute_bound().max_records(),
        request.daily_bound().max_records(),
        bounds.records,
        bounds.bytes,
    ] {
        hash.update(&bound.to_le_bytes());
    }
    Ok(hash.finalize())
}

/// A persistence error returns before `work`; its ordinary errors receive a
/// checked Refused terminal. A crash without a terminal remains incomplete.
pub(super) fn recorded<T>(
    root: &Path,
    identity: [u8; 32],
    work: impl FnOnce(&mut StoredOosObserverV1<'_>) -> Result<T, String>,
) -> Result<T, String> {
    let parent = sweep_evidence::begin(root, identity, Operation::GlobalReplay)?;
    let mut recorder = Recorder {
        root,
        parent_identity: identity,
        parent_token: parent.token(),
        active: None,
    };
    let mut result = work(&mut |event| recorder.observe(event));
    if result.is_ok() && recorder.active.is_some() {
        result = Err("Global Replay V4 computation returned with an unfinished stage".to_owned());
    }
    if let Some((_, attempt)) = recorder.active.take() {
        result = combine(result, attempt.finish(Completion::Refused));
    }
    let status = if result.is_ok() {
        Completion::Completed
    } else {
        Completion::Refused
    };
    // Keep both causes when calculation and durable refusal publication fail.
    combine(result, parent.finish(status))
}

fn combine<T>(value: Result<T, String>, durability: Result<(), String>) -> Result<T, String> {
    match (value, durability) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), Ok(())) => Err(why),
        (Ok(_), Err(why)) => Err(format!(
            "Global Replay V4 lifecycle persistence refused: {why}"
        )),
        (Err(why), Err(write)) => Err(format!(
            "{why}; Global Replay V4 lifecycle persistence also refused: {write}"
        )),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Fold,
    Replay,
}

struct Recorder<'a> {
    root: &'a Path,
    parent_identity: [u8; 32],
    parent_token: u64,
    active: Option<(Stage, Attempt)>,
}

impl Recorder<'_> {
    fn observe(&mut self, event: StoredOosComputationV1) -> Result<(), String> {
        match event {
            StoredOosComputationV1::FoldStarted(identity) => self.start(Stage::Fold, identity),
            StoredOosComputationV1::ReplayStarted(identity) => self.start(Stage::Replay, identity),
            StoredOosComputationV1::FoldCompleted => self.complete(Stage::Fold),
            StoredOosComputationV1::ReplayCompleted => self.complete(Stage::Replay),
        }
    }

    fn start(&mut self, stage: Stage, identity: [u8; 32]) -> Result<(), String> {
        if self.active.is_some() {
            return Err("Global Replay V4 overlapping computation stages refused".to_owned());
        }
        let operation = match stage {
            Stage::Fold => Operation::Preparation,
            Stage::Replay => Operation::GlobalReplayStream,
        };
        let attempt = sweep_evidence::begin(self.root, identity, operation)?;
        let parent = crate::identity_hex(&self.parent_identity);
        let child = crate::identity_hex(&identity);
        crate::note(
            &telemetry::Event::info(
                "cli.global-replay-v4",
                "stored computation identity recorded",
            )
            .with("parent_identity", parent.as_str())
            .with("parent_attempt", self.parent_token)
            .with("identity", child.as_str())
            .with("attempt", attempt.token())
            .with("operation", operation.as_str()),
        );
        self.active = Some((stage, attempt));
        Ok(())
    }

    fn complete(&mut self, stage: Stage) -> Result<(), String> {
        if self
            .active
            .as_ref()
            .is_none_or(|(active, _)| *active != stage)
        {
            return Err("Global Replay V4 completed a missing or different stage".to_owned());
        }
        self.active
            .take()
            .ok_or("Global Replay V4 active stage disappeared")?
            .1
            .finish(Completion::Completed)
    }
}

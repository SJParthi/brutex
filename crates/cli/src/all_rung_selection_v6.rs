//! Eight retained V6 authorities, including all terminal and actual-prefix facts.
use crate::selection_v6::{CommittedStoredSelectionV6, SelectionV6Snapshot};
use crate::stored_post_training_oos::{
    StoredPostTrainingOosRequestV1, StoredPostTrainingOosWitnessV1,
};

const RUNGS: [u32; 8] = [60, 120, 180, 300, 600, 900, 1800, 3600];

/// No detached receipt or winner array can construct this capability.
pub(crate) struct AllRungSelectionV6 {
    selections: [CommittedStoredSelectionV6; 8],
}

impl AllRungSelectionV6 {
    pub(crate) fn new(selections: [CommittedStoredSelectionV6; 8]) -> Result<Self, String> {
        let mut result = Self { selections };
        result.snapshot()?;
        Ok(result)
    }

    pub(crate) fn snapshot(&mut self) -> Result<[SelectionV6Snapshot; 8], String> {
        let mut snapshots = Vec::new();
        snapshots
            .try_reserve_exact(8)
            .map_err(|why| why.to_string())?;
        for selection in &mut self.selections {
            snapshots.push(selection.snapshot()?);
        }
        let snapshots = snapshots
            .try_into()
            .map_err(|_| "Selection V6 all-rung topology lacks an exact authority".to_owned())?;
        validate_topology(&snapshots)?;
        Ok(snapshots)
    }

    /// All eight sources preflight before the first stored replay is minted;
    /// every source is reauthenticated again before returning the complete set.
    pub(crate) fn replay(
        &mut self,
        expected: &[SelectionV6Snapshot; 8],
        request: StoredPostTrainingOosRequestV1,
        max_candidates: u64,
        observer: &mut crate::stored_post_training_oos::StoredOosObserverV1<'_>,
    ) -> Result<Vec<StoredPostTrainingOosWitnessV1>, String> {
        if &self.snapshot()? != expected {
            return Err("Selection V6 all-rung source changed before replay".to_owned());
        }
        let mut witnesses = Vec::new();
        let mut remaining = max_candidates;
        witnesses
            .try_reserve_exact(200)
            .map_err(|why| why.to_string())?;
        for (selection, snapshot) in self.selections.iter_mut().zip(expected) {
            let mut minted =
                selection.stored_oos_witnesses(snapshot, request, remaining, observer)?;
            for witness in &minted {
                remaining = remaining
                    .checked_sub(
                        u64::try_from(witness.candidate_count()).map_err(|why| why.to_string())?,
                    )
                    .ok_or("Selection V6 all-rung OOS candidate ceiling exceeded")?;
            }
            witnesses.append(&mut minted);
        }
        if &self.snapshot()? != expected {
            return Err("Selection V6 all-rung source changed during replay".to_owned());
        }
        Ok(witnesses)
    }
}

fn validate_topology(snapshots: &[SelectionV6Snapshot; 8]) -> Result<(), String> {
    let mut identities = std::collections::HashSet::new();
    identities.try_reserve(8).map_err(|why| why.to_string())?;
    for (snapshot, rung) in snapshots.iter().zip(RUNGS) {
        if snapshot.rung_seconds != rung
            || snapshot.winners.len() > 25
            || snapshot.identity == [0; 32]
            || !identities.insert(snapshot.identity)
            || snapshot
                .winners
                .iter()
                .enumerate()
                .any(|(index, winner)| usize::try_from(winner.rank).ok() != Some(index))
        {
            return Err(
                "Selection V6 all-rung topology is reordered, duplicated or oversized".to_owned(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;
    fn empty_topology() -> [SelectionV6Snapshot; 8] {
        std::array::from_fn(|index| SelectionV6Snapshot {
            rung_seconds: RUNGS[index],
            identity: [u8::try_from(index + 1).expect("eight IDs"); 32],
            envelope: [0; 960],
            winners: Vec::new(),
        })
    }
    #[test]
    fn all_eight_empty_terminal_slots_are_retained_and_every_wrong_rung_refuses() {
        let original = empty_topology();
        validate_topology(&original).expect("eight actual empty slots");
        for index in 0..8 {
            let mut wrong = original.clone();
            wrong[index].rung_seconds = 86400;
            assert!(validate_topology(&wrong).is_err());
            wrong = original.clone();
            wrong[index].identity = [0; 32];
            assert!(validate_topology(&wrong).is_err());
            for other in 0..8 {
                if index != other {
                    wrong = original.clone();
                    wrong.swap(index, other);
                    assert!(validate_topology(&wrong).is_err());
                    wrong = original.clone();
                    wrong[index].identity = wrong[other].identity;
                    assert!(validate_topology(&wrong).is_err());
                }
            }
        }
    }
}

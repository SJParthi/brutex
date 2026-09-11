//! Full bounded checkpoint-history observation. Child details are a separate door.
use super::{
    NAMESPACE, ROW, RungScope, Slot, decode, field, header_size, selection_of,
    transition_for_rungs, wire_size,
};
use crate::search_checkpoint::{Saved, Snapshot};
use std::path::Path;

/// Read-only saved eight-slot progress. No writer lock or namespace is created.
pub struct Reader {
    rungs: RungScope,
    snapshot: Snapshot,
    identity: [u8; 32],
    descriptor: [u8; 32],
    sequence: u64,
    pin: [u8; 32],
    slots: [Slot; 8],
    history: Vec<Saved>,
    admitted_bytes: u64,
    max_bytes: u64,
}
impl Reader {
    /// Authenticate every acknowledged predecessor under one saved-byte bound.
    /// Child receipt/body links remain recorded observations until opened separately.
    /// # Errors
    /// Missing, changed, skipped, foreign, noncanonical or oversized history refuses.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Self, String> {
        let snapshot =
            Snapshot::open(root, NAMESPACE, identity)?.ok_or("qualified campaign not recorded")?;
        let sequence = snapshot
            .latest
            .ok_or("qualified campaign has no acknowledged snapshot")?;
        let saved = snapshot.read(sequence, max_bytes)?;
        let pin = saved.seal;
        let descriptor = field::<32>(&saved.payload, 40)?;
        let rungs = selection_of(&saved.payload)?;
        let mut units = Vec::new();
        units.try_reserve_exact(8).map_err(super::display)?;
        for n in 0..8 {
            units.push(field::<32>(&saved.payload, header_size(rungs) + n * ROW)?);
        }
        let units: [[u8; 32]; 8] = units
            .try_into()
            .map_err(|_| "qualified campaign unit count differs")?;
        let mut sorted = units;
        sorted.sort_unstable();
        if descriptor == [0; 32]
            || sorted.contains(&[0; 32])
            || sorted.windows(2).any(|pair| matches!(pair,[a,b]if a==b))
        {
            return Err("qualified campaign declared scope invalid".into());
        }
        let (slots, _) = decode(&saved, identity, descriptor, units)?;
        let mut current = Some(saved);
        let mut history = Vec::new();
        let mut charged = 0_u64;
        while let Some(saved) = current {
            charged = charged
                .checked_add(
                    u64::try_from(wire_size(rungs) + 96 + size_of::<Saved>())
                        .map_err(super::display)?,
                )
                .ok_or("qualified history byte overflow")?;
            if charged > max_bytes {
                return Err("qualified campaign complete history exceeds admission".into());
            }
            let (_, previous) = decode(&saved, identity, descriptor, units)?;
            if history.len() == history.capacity() {
                let ceiling = usize::try_from(
                    max_bytes
                        / u64::try_from(wire_size(rungs) + 96 + size_of::<Saved>())
                            .map_err(super::display)?,
                )
                .map_err(super::display)?;
                let desired = history.len().saturating_mul(2).max(4).min(ceiling);
                history
                    .try_reserve_exact(desired.saturating_sub(history.len()))
                    .map_err(super::display)?;
            }
            history.push(saved);
            current = match previous {
                None => None,
                Some((sequence, pin)) => {
                    let saved = snapshot.read(sequence, max_bytes)?;
                    if saved.seal != pin {
                        return Err("qualified campaign predecessor pin differs".into());
                    }
                    Some(saved)
                }
            };
        }
        if history.len() as u64 != snapshot.acknowledged {
            return Err("qualified campaign chain omits acknowledged checkpoints".into());
        }
        let mut before = units.map(|unit| Slot {
            unit,
            started: false,
            complete: None,
            reason: String::new(),
        });
        for (index, saved) in history.iter().rev().enumerate() {
            let (after, _) = decode(saved, identity, descriptor, units)?;
            if index == 0 && after != before {
                return Err(
                    "qualified campaign initial scope was not acknowledged before work".into(),
                );
            }
            transition_for_rungs(&before, &after, rungs)?;
            before = after;
        }
        let reader = Self {
            rungs,
            snapshot,
            identity,
            descriptor,
            sequence,
            pin,
            slots,
            history,
            admitted_bytes: charged,
            max_bytes,
        };
        reader.require_current()?;
        Ok(reader)
    }
    /// Exact saved campaign identity.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    /// Selected intraday units; all physical slot indices remain unchanged.
    #[must_use]
    pub const fn rungs(&self) -> RungScope {
        self.rungs
    }
    /// Exact predeclared scope identity.
    #[must_use]
    pub const fn descriptor(&self) -> [u8; 32] {
        self.descriptor
    }
    /// Latest acknowledged sequence at observation time.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Latest acknowledged pin at observation time.
    #[must_use]
    pub const fn pin(&self) -> [u8; 32] {
        self.pin
    }
    /// Every declared unit in original order, including waiting units.
    #[must_use]
    pub const fn slots(&self) -> &[Slot; 8] {
        &self.slots
    }
    /// Whether an owner lock conflicted during this observation, not liveness.
    #[must_use]
    pub const fn owner_observed(&self) -> bool {
        self.snapshot.writer_observed
    }
    /// Number of separately acknowledged history records.
    #[must_use]
    pub fn history_records(&self) -> usize {
        self.history.len()
    }
    /// Serialized checkpoints plus bounded retained-index accounting, not RSS.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.admitted_bytes
    }
    /// Verify the exact pinned history again; newer appended snapshots are permitted.
    /// # Errors
    /// Replaced, missing or altered retained records refuse without fallback.
    pub fn require_current(&self) -> Result<(), String> {
        for old in &self.history {
            let now = self.snapshot.read(old.sequence, self.max_bytes)?;
            if old.seal != now.seal || old.payload != now.payload {
                return Err("qualified campaign history changed during observation".into());
            }
        }
        Ok(())
    }
}

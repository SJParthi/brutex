//! Predeclared eight-slot qualification scope. A retry never reallocates a slot.
use crate::boolean_campaign::RungScope;
use crate::search_checkpoint::{Journal, Saved};
use brutex_core::blake3::Hasher;
use std::path::Path;

const NAMESPACE: &str = "boolean-qualified-campaign-v1";
const HEADER: usize = 112;
const ROW: usize = 1136;
const BYTES: usize = HEADER + 8 * ROW;
const REASON: usize = 1024;
type Previous = Option<(u64, [u8; 32])>;
#[path = "boolean_qualified_observer.rs"]
mod observer;
pub use observer::Reader;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Recorded child link only; this value cannot authorize qualification work.
pub struct Link {
    /// Exact child identity.
    pub identity: [u8; 32],
    /// Exact child completion digest, not authenticated by the overview alone.
    pub pin: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// One predeclared slot's saved state, without child authoring authority.
pub struct Slot {
    /// Exact predeclared unit identity.
    pub unit: [u8; 32],
    /// Whether a separate acknowledged start exists.
    pub started: bool,
    /// Completion link recorded by the producer; details verify its body.
    pub complete: Option<Link>,
    /// Bounded saved refusal, empty when none is recorded.
    pub reason: String,
}

pub(crate) struct Writer {
    rungs: RungScope,
    journal: Journal,
    identity: [u8; 32],
    descriptor: [u8; 32],
    previous: Previous,
    slots: [Slot; 8],
    bytes: u64,
    records: u64,
    poisoned: bool,
}

#[cfg(test)]
pub(crate) fn identity(descriptor: [u8; 32], units: [[u8; 32]; 8]) -> [u8; 32] {
    identity_for_rungs(descriptor, units, RungScope::ALL)
}

pub(crate) fn identity_for_rungs(
    descriptor: [u8; 32],
    units: [[u8; 32]; 8],
    rungs: RungScope,
) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.qualified-eight-slot-campaign.v1\0");
    if rungs != RungScope::ALL {
        h.update(b"selected-intraday-rungs-v2\0");
        h.update(&[rungs.mask()]);
    }
    h.update(&descriptor);
    for unit in units {
        h.update(&unit);
    }
    h.finalize()
}

impl Writer {
    #[cfg(test)]
    pub(crate) fn open(
        root: &Path,
        descriptor: [u8; 32],
        units: [[u8; 32]; 8],
        bytes: u64,
        records: u64,
        check: impl FnMut(usize, [u8; 32], Link) -> Result<(), String>,
    ) -> Result<Self, String> {
        Self::open_for_rungs(
            root,
            descriptor,
            units,
            RungScope::ALL,
            bytes,
            records,
            check,
        )
    }

    pub(crate) fn open_for_rungs(
        root: &Path,
        descriptor: [u8; 32],
        units: [[u8; 32]; 8],
        rungs: RungScope,
        bytes: u64,
        records: u64,
        mut check: impl FnMut(usize, [u8; 32], Link) -> Result<(), String>,
    ) -> Result<Self, String> {
        let mut canonical_units = units;
        canonical_units.sort_unstable();
        if descriptor == [0; 32]
            || units.contains(&[0; 32])
            || canonical_units
                .windows(2)
                .any(|pair| matches!(pair, [a, b] if a == b))
            || bytes < (wire_size(rungs) + 96) as u64
            || records < 3
        {
            return Err(
                "qualified campaign scope or complete checkpoint admission is invalid".into(),
            );
        }
        let identity = identity_for_rungs(descriptor, units, rungs);
        let journal = Journal::open(root, NAMESPACE, identity)?;
        let latest = journal.latest(bytes)?;
        let mut chain = Vec::new();
        let mut prior = latest.as_ref().map(|s| (s.sequence, s.seal));
        while let Some((sequence, pin)) = prior {
            if chain.len() as u64 >= records
                || (chain.len() as u64)
                    .checked_add(1)
                    .and_then(|n| n.checked_mul(40))
                    .is_none_or(|n| n > bytes)
            {
                return Err("qualified campaign history exceeds its admission".into());
            }
            let saved = journal.read(sequence, bytes)?;
            if saved.seal != pin {
                return Err("qualified campaign predecessor pin differs".into());
            }
            let (_, old) = decode(&saved, identity, descriptor, units)?;
            if chain.len() == chain.capacity() {
                let ceiling = records.min(bytes / 40);
                let desired = (chain.len() as u64).saturating_mul(2).max(4).min(ceiling);
                chain
                    .try_reserve_exact(
                        usize::try_from(desired)
                            .map_err(display)?
                            .saturating_sub(chain.len()),
                    )
                    .map_err(display)?;
            }
            chain.push((sequence, pin));
            prior = old;
        }
        if chain.len() as u64 != journal.acknowledged() {
            return Err("qualified campaign chain omits acknowledged checkpoints".into());
        }
        chain.reverse();
        let mut slots = units.map(|unit| Slot {
            unit,
            started: false,
            complete: None,
            reason: String::new(),
        });
        for (index, (sequence, pin)) in chain.into_iter().enumerate() {
            let saved = journal.read(sequence, bytes)?;
            if saved.seal != pin {
                return Err("qualified campaign history changed during restore".into());
            }
            let (next, _) = decode(&saved, identity, descriptor, units)?;
            if index == 0 && next != slots {
                return Err(
                    "qualified campaign initial scope was not acknowledged before work".into(),
                );
            }
            transition_for_rungs(&slots, &next, rungs)?;
            slots = next;
        }
        for (index, slot) in slots.iter().enumerate() {
            if let Some(link) = slot.complete {
                check(index, slot.unit, link)?;
            }
        }
        let mut result = Self {
            rungs,
            journal,
            identity,
            descriptor,
            previous: latest.as_ref().map(|s| (s.sequence, s.seal)),
            slots,
            bytes,
            records,
            poisoned: false,
        };
        if latest.is_none() {
            result.publish_next(&result.slots.clone())?;
        }
        Ok(result)
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) fn slots(&self) -> &[Slot; 8] {
        &self.slots
    }
    pub(crate) fn completed(&self) -> bool {
        !self.poisoned
            && self.rungs.indices().all(|index| {
                self.slots
                    .get(index)
                    .is_some_and(|slot| slot.complete.is_some())
            })
    }
    pub(crate) const fn rungs(&self) -> RungScope {
        self.rungs
    }
    pub(crate) fn pin(&self) -> Result<[u8; 32], String> {
        self.require_healthy()?;
        self.previous
            .map(|p| p.1)
            .ok_or_else(|| "qualified campaign has no acknowledged snapshot".into())
    }
    pub(crate) fn begin(&mut self, index: usize) -> Result<(), String> {
        self.require_healthy()?;
        if !self.rungs.contains(index) {
            return Err("qualification timeframe is outside the declared selection".into());
        }
        if self
            .slots
            .iter()
            .take(index)
            .enumerate()
            .any(|(prior, slot)| self.rungs.contains(prior) && slot.complete.is_none())
        {
            return Err("qualified campaign cannot skip an unfinished unit".into());
        }
        let mut next = self.slots.clone();
        let slot = next
            .get_mut(index)
            .ok_or("qualified campaign unit is outside eight rungs")?;
        if slot.complete.is_some() {
            return Err("completed qualification slot cannot be reassigned".into());
        }
        if slot.started && slot.reason.is_empty() {
            return self.admit(1);
        }
        self.admit(2)?;
        slot.started = true;
        slot.reason.clear();
        self.publish_next(&next)
    }
    pub(crate) fn finish(&mut self, index: usize, link: Link) -> Result<(), String> {
        self.require_healthy()?;
        if link.identity == [0; 32] || link.pin == [0; 32] {
            return Err("qualified campaign completion is empty".into());
        }
        let mut next = self.slots.clone();
        let slot = next.get_mut(index).ok_or("qualification unit missing")?;
        if !slot.started || slot.complete.is_some() {
            return Err("qualification completion has no pending reservation".into());
        }
        slot.complete = Some(link);
        slot.reason.clear();
        self.publish_next(&next)
    }
    pub(crate) fn refuse(&mut self, index: usize, why: &str) -> Result<(), String> {
        self.require_healthy()?;
        if why.trim().is_empty() {
            return Err("qualification refusal requires a nonempty reason".into());
        }
        let mut next = self.slots.clone();
        let slot = next.get_mut(index).ok_or("qualification unit missing")?;
        if !slot.started || slot.complete.is_some() {
            return Err("qualification refusal has no pending reservation".into());
        }
        slot.reason = bounded_reason(why);
        self.publish_next(&next)
    }
    fn admit(&self, needed: u64) -> Result<(), String> {
        self.require_healthy()?;
        let ceiling = self
            .records
            .min(self.bytes / 40)
            .min(crate::search_checkpoint::DIRECTORY_LIMIT as u64 - 1);
        if needed
            .checked_sub(1)
            .and_then(|extra| self.journal.next_sequence().checked_add(extra))
            .is_none_or(|n| n > ceiling)
        {
            return Err(
                "qualified campaign checkpoint capacity exhausted before computation".into(),
            );
        }
        Ok(())
    }
    fn require_healthy(&self) -> Result<(), String> {
        if self.poisoned {
            Err("qualified campaign writer must reopen after uncertain publication".into())
        } else {
            Ok(())
        }
    }
    fn publish_next(&mut self, next: &[Slot; 8]) -> Result<(), String> {
        self.publish_observed(next, |journal, sequence, bytes| {
            journal.read(sequence, bytes)
        })
    }
    fn publish_observed(
        &mut self,
        next: &[Slot; 8],
        observe: impl FnOnce(&Journal, u64, u64) -> Result<Saved, String>,
    ) -> Result<(), String> {
        self.admit(1)?;
        transition_for_rungs(&self.slots, next, self.rungs)?;
        let body = encode_for_rungs(
            self.identity,
            self.descriptor,
            self.previous,
            next,
            self.rungs,
        )?;
        // Once publication is attempted, even an error may have left durable
        // bytes. Keep the old acknowledged view and require a validating reopen.
        self.poisoned = true;
        let (sequence, pin) = self.journal.publish(&body, self.bytes)?;
        let saved = observe(&self.journal, sequence, self.bytes)?;
        if saved.seal != pin || saved.payload != body {
            return Err("qualified campaign snapshot changed before acknowledgment".into());
        }
        self.slots.clone_from(next);
        self.previous = Some((sequence, pin));
        self.poisoned = false;
        Ok(())
    }
}

fn bounded_reason(why: &str) -> String {
    if why.len() <= REASON {
        return why.to_owned();
    }
    let suffix = " [truncated]";
    let mut prefix = String::new();
    for character in why.chars() {
        if prefix.len() + character.len_utf8() > REASON - suffix.len() {
            break;
        }
        prefix.push(character);
    }
    prefix.push_str(suffix);
    prefix
}

fn transition_for_rungs(old: &[Slot; 8], new: &[Slot; 8], rungs: RungScope) -> Result<(), String> {
    if old.iter().zip(new).filter(|(old, new)| old != new).count() > 1 {
        return Err("qualified campaign snapshot skipped a unit transition".into());
    }
    let mut unfinished = false;
    for (index, (old, new)) in old.iter().zip(new).enumerate() {
        if !rungs.contains(index) {
            if old != new || new.started || new.complete.is_some() || !new.reason.is_empty() {
                return Err("unselected qualification unit carries a work transition".into());
            }
            continue;
        }
        if old.unit != new.unit
            || (old.started && !new.started)
            || (old.complete.is_some() && old != new)
            || (unfinished && new.started)
            || (new.complete.is_some() && (!new.started || !new.reason.is_empty()))
            || (new.complete != old.complete && !old.started)
            || (!old.started && !new.reason.is_empty())
            || (!new.started && !new.reason.is_empty())
        {
            return Err("qualified campaign history retracts or skips an allocated unit".into());
        }
        unfinished |= new.complete.is_none();
    }
    Ok(())
}

#[cfg(test)]
fn encode(
    id: [u8; 32],
    descriptor: [u8; 32],
    previous: Previous,
    slots: &[Slot; 8],
) -> Result<Vec<u8>, String> {
    encode_for_rungs(id, descriptor, previous, slots, RungScope::ALL)
}
fn encode_for_rungs(
    id: [u8; 32],
    descriptor: [u8; 32],
    previous: Previous,
    slots: &[Slot; 8],
    rungs: RungScope,
) -> Result<Vec<u8>, String> {
    let mut raw = Vec::new();
    raw.try_reserve_exact(wire_size(rungs)).map_err(display)?;
    raw.extend_from_slice(if rungs == RungScope::ALL {
        b"BRQCAM01"
    } else {
        b"BRQCAM02"
    });
    raw.extend_from_slice(&id);
    raw.extend_from_slice(&descriptor);
    let (sequence, pin) = previous.unwrap_or((0, [0; 32]));
    raw.extend_from_slice(&sequence.to_le_bytes());
    raw.extend_from_slice(&pin);
    if rungs != RungScope::ALL {
        raw.extend_from_slice(&u64::from(rungs.mask()).to_le_bytes());
    }
    for slot in slots {
        if slot.reason.len() > REASON {
            return Err("qualified campaign reason exceeds fixed width".into());
        }
        raw.extend_from_slice(&slot.unit);
        raw.extend_from_slice(&u64::from(slot.started).to_le_bytes());
        let link = slot.complete.unwrap_or(Link {
            identity: [0; 32],
            pin: [0; 32],
        });
        raw.extend_from_slice(&link.identity);
        raw.extend_from_slice(&link.pin);
        raw.extend_from_slice(&(slot.reason.len() as u64).to_le_bytes());
        raw.extend_from_slice(slot.reason.as_bytes());
        raw.resize(raw.len() + REASON - slot.reason.len(), 0);
    }
    if raw.len() != wire_size(rungs) {
        return Err("qualified campaign format width differs".into());
    }
    Ok(raw)
}

fn decode(
    saved: &Saved,
    id: [u8; 32],
    descriptor: [u8; 32],
    units: [[u8; 32]; 8],
) -> Result<([Slot; 8], Previous), String> {
    let raw = &saved.payload;
    let rungs = selection_of(raw)?;
    if raw.len() != wire_size(rungs)
        || field::<32>(raw, 8)? != id
        || field::<32>(raw, 40)? != descriptor
    {
        return Err("qualified campaign manifest differs".into());
    }
    let sequence = u64::from_le_bytes(field(raw, 72)?);
    let pin = field::<32>(raw, 80)?;
    let previous = match (sequence, pin) {
        (0, p) if p == [0; 32] => None,
        (n, p) if n > 0 && n < saved.sequence && p != [0; 32] => Some((n, p)),
        _ => return Err("qualified campaign predecessor is invalid".into()),
    };
    let mut slots = Vec::new();
    slots.try_reserve_exact(8).map_err(display)?;
    for (row, expected_unit) in raw
        .get(header_size(rungs)..)
        .ok_or("qualified campaign rows missing")?
        .chunks_exact(ROW)
        .zip(units)
    {
        let unit = field::<32>(row, 0)?;
        let started = match u64::from_le_bytes(field(row, 32)?) {
            0 => false,
            1 => true,
            _ => return Err("qualified campaign state tag is invalid".into()),
        };
        let identity = field::<32>(row, 40)?;
        let pin = field::<32>(row, 72)?;
        let complete = match (identity, pin) {
            (id, pin) if id == [0; 32] && pin == [0; 32] => None,
            (id, pin) if id != [0; 32] && pin != [0; 32] => Some(Link { identity: id, pin }),
            _ => return Err("qualified campaign half completion".into()),
        };
        let length = usize::try_from(u64::from_le_bytes(field(row, 104)?)).map_err(display)?;
        let end = 112_usize
            .checked_add(length)
            .ok_or("reason length overflow")?;
        let reason = row.get(112..end).ok_or("reason width")?;
        if unit != expected_unit
            || length > REASON
            || row
                .get(end..)
                .ok_or("reason padding missing")?
                .iter()
                .any(|b| *b != 0)
        {
            return Err("qualified campaign unit or padding differs".into());
        }
        slots.push(Slot {
            unit,
            started,
            complete,
            reason: std::str::from_utf8(reason).map_err(display)?.to_owned(),
        });
    }
    let slots: [Slot; 8] = slots
        .try_into()
        .map_err(|_| "qualified campaign requires all eight units")?;
    if encode_for_rungs(id, descriptor, previous, &slots, rungs)? != *raw
        || identity_for_rungs(descriptor, units, rungs) != id
    {
        return Err("qualified campaign noncanonical snapshot".into());
    }
    for (index, slot) in slots.iter().enumerate() {
        if !rungs.contains(index)
            && (slot.started || slot.complete.is_some() || !slot.reason.is_empty())
        {
            return Err("unselected qualification unit carries saved evidence".into());
        }
    }
    Ok((slots, previous))
}
const fn header_size(rungs: RungScope) -> usize {
    HEADER + if rungs.mask() == u8::MAX { 0 } else { 8 }
}
const fn wire_size(rungs: RungScope) -> usize {
    BYTES + if rungs.mask() == u8::MAX { 0 } else { 8 }
}
fn selection_of(raw: &[u8]) -> Result<RungScope, String> {
    match &field::<8>(raw, 0)? {
        b"BRQCAM01" => Ok(RungScope::ALL),
        b"BRQCAM02" => {
            let rungs = RungScope::from_mask(
                u8::try_from(u64::from_le_bytes(field(raw, HEADER)?)).map_err(display)?,
            )?;
            if rungs == RungScope::ALL {
                return Err(
                    "all-eight qualification checkpoint requires its original encoding".into(),
                );
            }
            Ok(rungs)
        }
        _ => Err("qualified campaign format differs".into()),
    }
}
fn field<const N: usize>(raw: &[u8], at: usize) -> Result<[u8; N], String> {
    raw.get(at..at.saturating_add(N))
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| "qualified campaign field missing".into())
}
fn display(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "boolean_qualified_journal_tests.rs"]
mod tests;

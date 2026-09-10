//! V4-only fixed 1024-byte records. No prior replay format is decoded.
use super::PathProjection;
use super::{
    Attempt, GlobalReplayV4Audit, SelectionV6Snapshot, SelectionV6Winner,
    StoredPostTrainingOosRequestV1, VixStamp,
};
use runner::portfolio::{Disposition, IntentRefusal};

pub(super) const STRIDE: usize = 1024;
const SEAL_AT: usize = STRIDE - 32;
pub(super) type Record = [u8; STRIDE];
const MAGIC: &[u8; 8] = b"BTXGRV4\0";

pub(super) struct Writer {
    bytes: Record,
    at: usize,
}
impl Writer {
    pub(super) fn new(kind: u64, sequence: u64) -> Self {
        let mut bytes = [0; STRIDE];
        if let Some(slot) = bytes.get_mut(..8) {
            slot.copy_from_slice(MAGIC);
        }
        if let Some(slot) = bytes.get_mut(8..16) {
            slot.copy_from_slice(&kind.to_le_bytes());
        }
        if let Some(slot) = bytes.get_mut(16..24) {
            slot.copy_from_slice(&sequence.to_le_bytes());
        }
        Self { bytes, at: 24 }
    }
    pub(super) fn bytes(&mut self, data: &[u8]) -> Result<(), String> {
        let end = self
            .at
            .checked_add(data.len())
            .ok_or("Global Replay V4 encoder overflow")?;
        if end > SEAL_AT {
            return Err("Global Replay V4 fixed record capacity exceeded".to_owned());
        }
        self.bytes
            .get_mut(self.at..end)
            .ok_or("Global Replay V4 encoding range")?
            .copy_from_slice(data);
        self.at = end;
        Ok(())
    }
    pub(super) fn word(&mut self, value: u64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }
    pub(super) fn signed(&mut self, value: i64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }
    pub(super) fn finish(mut self) -> Result<Record, String> {
        let seal = record_seal(&self.bytes)?;
        self.bytes
            .get_mut(SEAL_AT..)
            .ok_or("Global Replay V4 seal slot")?
            .copy_from_slice(&seal);
        Ok(self.bytes)
    }
}
pub(super) fn count(value: usize) -> Result<u64, String> {
    u64::try_from(value).map_err(|why| why.to_string())
}
pub(super) fn kind(row: &Record) -> u64 {
    row.get(8..16)
        .and_then(|bytes| bytes.try_into().ok())
        .map_or(u64::MAX, u64::from_le_bytes)
}
fn record_seal(row: &Record) -> Result<[u8; 32], String> {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(b"brutex-global-replay-v4-record\0");
    hash.update(row.get(..SEAL_AT).ok_or("Global Replay V4 payload range")?);
    Ok(hash.finalize())
}
pub(super) fn verify_record(row: &Record) -> Result<(), String> {
    if row.get(..8) != Some(MAGIC.as_slice())
        || kind(row) > 6
        || row.get(SEAL_AT..) != Some(record_seal(row)?.as_slice())
    {
        return Err("Global Replay V4 wrong version, record kind or seal".to_owned());
    }
    Ok(())
}
pub(super) fn digest_records<'a>(
    domain: &[u8],
    rows: impl Iterator<Item = &'a Record>,
) -> [u8; 32] {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(domain);
    for row in rows {
        hash.update(row);
    }
    hash.finalize()
}
pub(super) fn header(request: StoredPostTrainingOosRequestV1) -> Result<Record, String> {
    let mut out = Writer::new(0, 0);
    for value in [
        4,
        STRIDE as u64,
        8,
        25,
        u64::from(request.from().0),
        u64::from(request.from().1),
        u64::from(request.to().0),
        u64::from(request.to().1),
    ] {
        out.word(value)?;
    }
    out.finish()
}
pub(super) fn witness(
    stream: u64,
    snapshot: &SelectionV6Snapshot,
    winner: &SelectionV6Winner,
    cohort: [u8; 32],
    stored: [u8; 32],
    replay: &runner::exit_grid_policy::GlobalReplayWitnessUniverseV1,
) -> Result<Record, String> {
    let mut out = Writer::new(2, stream);
    for id in [
        snapshot.identity,
        winner.ranked.candidate.strategy_digest.bytes(),
        winner.disposition_id,
        winner.selected_exit_digest,
        cohort,
        stored,
        replay.universe_digest(),
        replay.run_id().bytes(),
    ] {
        out.bytes(&id)?;
    }
    for value in [
        u64::from(snapshot.rung_seconds),
        u64::from(winner.rank) + 1,
        u64::from(winner.family == "BANKNIFTY") + 1,
        u64::from(replay.direction() == costs::fill::Direction::Short) + 1,
        count(replay.candidates().len())?,
        winner.ranked.score,
    ] {
        out.word(value)?;
    }
    let feed = replay.feed().as_str().as_bytes();
    let mut fixed_feed = [0; 32];
    fixed_feed
        .get_mut(..feed.len())
        .ok_or("Global Replay V4 canonical feed width")?
        .copy_from_slice(feed);
    out.word(count(feed.len())?)?;
    out.bytes(&fixed_feed)?;
    for word in winner.ranked.candidate.mask_words {
        out.word(word)?;
    }
    let m = winner.ranked.candidate.metrics;
    for word in [
        m.drawdown,
        m.worst_loss,
        m.losing_rate_ppm,
        m.losing_trades,
        m.winning_trades,
        m.win_rate_ppm,
        m.average_win,
        m.average_loss,
        m.assurance_ppm,
    ] {
        out.word(word)?;
    }
    out.signed(m.pessimistic_profit)?;
    for ratio in [m.loss_ratio_ppm, m.reward_to_risk_ppm] {
        out.word(u64::from(ratio.is_some()))?;
        out.word(ratio.unwrap_or(0))?;
    }
    out.finish()
}
pub(super) fn candidate(attempt: &Attempt) -> Result<Record, String> {
    let mut out = Writer::new(3, attempt.ordinal);
    out.word(attempt.stream)?;
    let c = attempt.candidate;
    for value in [
        count(c.signal_bar())?,
        count(c.entry_bar())?,
        count(c.occupied_through_bar())?,
    ] {
        out.word(value)?;
    }
    for value in [
        c.signal_micros(),
        c.entry_micros(),
        c.occupied_through_micros(),
    ] {
        out.signed(value)?;
    }
    out.word(match c.path() {
        PathProjection::Priceable(_) => 1,
        PathProjection::BlockOnly => 2,
        PathProjection::CrossingRefused => 3,
        PathProjection::BlockOnlyAndCrossingRefused => 4,
    })?;
    if let PathProjection::Priceable(price) = c.path() {
        let row = price.row();
        for value in [
            count(row.signal_bar)?,
            count(row.entry_bar)?,
            count(row.exit_bar)?,
        ] {
            out.word(value)?;
        }
        for value in [
            row.best,
            row.worst,
            row.entry_micros,
            row.exit_micros,
            row.adverse,
            row.adverse_paisa,
            row.favourable,
            row.favourable_paisa,
        ] {
            out.signed(value)?;
        }
        out.word(price.ambiguous_bars())?;
        out.word(price.gap_fills())?;
    }
    out.finish()
}
pub(super) fn decision(
    sequence: u64,
    attempt: &Attempt,
    disposition: Disposition,
) -> Result<Record, String> {
    let mut out = Writer::new(4, sequence);
    out.word(attempt.stream)?;
    out.word(attempt.ordinal)?;
    out.bytes(&attempt.constituent.strategy_digest.bytes())?;
    out.signed(attempt.candidate.entry_micros())?;
    match disposition {
        Disposition::Admitted {
            occupied_through_micros,
        } => {
            out.word(1)?;
            out.signed(occupied_through_micros)?;
        }
        Disposition::BlockedOccupied {
            occupied_through_micros,
        } => {
            out.word(2)?;
            out.signed(occupied_through_micros)?;
        }
        Disposition::BlockedSimultaneous { admitted } => {
            out.word(3)?;
            out.bytes(&admitted.bytes())?;
        }
        Disposition::Unreachable => out.word(4)?,
        Disposition::Refused(why) => {
            out.word(5)?;
            out.word(match why {
                IntentRefusal::Upstream => 1,
                IntentRefusal::OccupancyEndsBeforeEntry => 2,
            })?;
        }
    }
    out.finish()
}
pub(super) fn vix(sequence: u64, entry: VixStamp, exit: VixStamp) -> Result<Record, String> {
    let mut out = Writer::new(5, sequence);
    for stamp in [entry, exit] {
        match stamp {
            VixStamp::Absent => {
                out.word(0)?;
                out.bytes(&[0; 56])?;
            }
            VixStamp::Exact(bar) => {
                out.word(1)?;
                for value in [
                    bar.ts_micros,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume,
                    bar.open_interest,
                ] {
                    out.signed(value)?;
                }
            }
        }
    }
    out.finish()
}
pub(super) fn completion(audit: &GlobalReplayV4Audit, preceding: u64) -> Result<Record, String> {
    let mut out = Writer::new(6, preceding);
    out.bytes(&audit.replay_id)?;
    out.bytes(&audit.publication_id)?;
    let c = audit.counters;
    for value in [
        preceding,
        audit.witnesses,
        audit.candidates,
        audit.money_rows,
        audit.pricing_refused,
        audit.admitted_pricing_refused,
        c.offered,
        c.admitted,
        c.blocked_occupied,
        c.blocked_simultaneous,
        c.unreachable,
        c.refused,
    ] {
        out.word(value)?;
    }
    out.bytes(&audit.pessimistic_paisa.to_le_bytes())?;
    out.bytes(&audit.optimistic_paisa.to_le_bytes())?;
    out.finish()
}

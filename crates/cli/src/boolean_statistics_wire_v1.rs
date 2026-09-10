//! Fixed 512-byte records; source observations remain in the linked candidate body.
use super::{Bounds, Group, MeasurementsV1, PopulationStatisticsProcedureV2, display};

const STRIDE: u64 = 512;

pub(super) fn required_bytes(group: &Group) -> Result<u64, String> {
    (group.sources.len() as u64)
        .checked_add(group.candidates as u64)
        .and_then(|n| n.checked_add(group.layout.split_count()))
        .and_then(|n| n.checked_add(2))
        .and_then(|n| n.checked_mul(STRIDE))
        .ok_or_else(|| "Boolean statistics fixed record cardinality overflow".to_owned())
}

pub(super) fn encode(
    group: &Group,
    identity: [u8; 32],
    procedure: PopulationStatisticsProcedureV2,
    bounds: Bounds,
    measured: &MeasurementsV1,
) -> Result<Vec<u8>, String> {
    if measured.candidates.len() != group.candidates
        || measured.splits.len() as u64 != group.layout.split_count()
    {
        return Err("Boolean statistics output omitted a coordinate or split".to_owned());
    }
    let bytes = required_bytes(group)?;
    if bytes > bounds.bytes {
        return Err("Boolean statistics fixed body exceeds byte admission".to_owned());
    }
    let mut body = Vec::new();
    body.try_reserve_exact(usize::try_from(bytes).map_err(display)?)
        .map_err(display)?;
    body.extend_from_slice(&manifest(group, identity, procedure, bounds)?.bytes);
    for source in &group.sources {
        let mut row = Record::new(2)?;
        row.put(&source.family().encode())?;
        row.put(&source.identity())?;
        row.put(&source.completion_digest())?;
        row.u64(source.rows().len() as u64)?;
        body.extend_from_slice(&row.bytes);
    }
    for (index, candidate) in measured.candidates.iter().enumerate() {
        let source = group
            .sources
            .get(candidate.family)
            .and_then(|source| source.rows().get(candidate.coordinate))
            .ok_or("Boolean statistics coordinate projection disappeared")?;
        if source.identity() != candidate.identity {
            return Err("Boolean statistics coordinate identity changed".to_owned());
        }
        let mut row = Record::new(3)?;
        row.u64(candidate.family as u64)?;
        row.u64(candidate.coordinate as u64)?;
        row.put(&candidate.identity)?;
        row.put(&candidate.period_digest)?;
        row.put(&source.run_id())?;
        row.u64(source.program_index() as u64)?;
        row.u64(match source.side() {
            runner::excursion::Side::Long => 1,
            runner::excursion::Side::Short => 2,
        })?;
        row.u64(source.ordinal())?;
        row.u64(source.execution_refusal_bits().bits())?;
        row.u64(candidate.trades)?;
        row.u64(candidate.wins)?;
        row.put(&candidate.return_paisa.to_le_bytes())?;
        row.u64(candidate.wilson_lower_bits)?;
        row.u64(measured.romano_availability.code())?;
        if let Some(romano) = &measured.romano {
            let value = romano
                .candidate(index)
                .ok_or("Boolean statistics Romano-Wolf coordinate missing")?;
            for word in [
                value.strategy() as u64,
                value.stepdown_rank() as u64,
                value.observed_statistic().to_bits(),
                value.strict_exceedances() as u64,
                value.initial_p_value().numerator() as u64,
                value.initial_p_value().denominator() as u64,
                value.adjusted_p_value().numerator() as u64,
                value.adjusted_p_value().denominator() as u64,
            ] {
                row.u64(word)?;
            }
        }
        body.extend_from_slice(&row.bytes);
    }
    for split in &measured.splits {
        let mut row = Record::new(4)?;
        for word in [
            split.train_mask,
            split.test_mask,
            u64::from(split.bottom_half),
            u64::from(split.rankable),
        ] {
            row.u64(word)?;
        }
        row.put(&split.scores_digest)?;
        body.extend_from_slice(&row.bytes);
    }
    body.extend_from_slice(&family_tests(measured)?.bytes);
    if body.len() as u64 != bytes {
        return Err("Boolean statistics fixed body cardinality disagrees".to_owned());
    }
    Ok(body)
}

fn manifest(
    group: &Group,
    identity: [u8; 32],
    procedure: PopulationStatisticsProcedureV2,
    bounds: Bounds,
) -> Result<Record, String> {
    let mut row = Record::new(1)?;
    row.put(b"BTXBST01")?;
    row.put(&identity)?;
    row.put(&group.scope.digest())?;
    row.put(&group.cohort)?;
    row.put(&group.layout.digest())?;
    for word in [
        group.sources.len() as u64,
        group.candidates as u64,
        group.layout.period_count(),
        u64::from(group.layout.segment_count()),
        group.layout.periods_per_segment(),
        group.layout.split_count(),
        procedure.draws(),
        procedure.seed(),
        procedure.block_length(),
    ]
    .into_iter()
    .chain(bounds.words())
    {
        row.u64(word)?;
    }
    Ok(row)
}

fn family_tests(measured: &MeasurementsV1) -> Result<Record, String> {
    let mut row = Record::new(5)?;
    row.put(&measured.white.family_digest())?;
    row.put(&measured.spa.family_digest())?;
    row.put(&measured.romano.as_ref().map_or(
        [0; 32],
        runner::bootstrap::RomanoWolfAdjustedReceiptV1::family_digest,
    ))?;
    for word in [
        measured.white.statistic_bits(),
        measured.white.p_value_bits(),
        measured.white.exact_p_value().numerator() as u64,
        measured.white.exact_p_value().denominator() as u64,
        measured.white.matched_or_exceeded() as u64,
        measured.white.draws() as u64,
        measured.white.strategies() as u64,
        measured.white.periods() as u64,
        measured.spa.statistic_bits(),
        measured.spa.p_value_bits(),
        measured.spa.exact_p_value().numerator() as u64,
        measured.spa.exact_p_value().denominator() as u64,
        measured.spa.matched_or_exceeded() as u64,
        measured.spa.draws() as u64,
        measured.spa.strategies() as u64,
        measured.spa.periods() as u64,
        measured.romano_availability.code(),
        measured.contributing_splits,
        measured.bottom_half_splits,
    ] {
        row.u64(word)?;
    }
    Ok(row)
}

struct Record {
    bytes: [u8; 512],
    used: usize,
}
impl Record {
    fn new(kind: u64) -> Result<Self, String> {
        let mut row = Self {
            bytes: [0; 512],
            used: 0,
        };
        row.u64(kind)?;
        Ok(row)
    }
    fn put(&mut self, bytes: &[u8]) -> Result<(), String> {
        let end = self
            .used
            .checked_add(bytes.len())
            .ok_or("Boolean statistics record offset overflow")?;
        self.bytes
            .get_mut(self.used..end)
            .ok_or("Boolean statistics record exceeds fixed stride")?
            .copy_from_slice(bytes);
        self.used = end;
        Ok(())
    }
    fn u64(&mut self, value: u64) -> Result<(), String> {
        self.put(&value.to_le_bytes())
    }
}

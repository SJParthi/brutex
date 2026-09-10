//! New fixed-stride qualification sections; old admission bytes are immutable.
use super::{Bounds, Manifest, Row, display, identity};
use crate::boolean_qualification_plan::ObservedPlan;
use runner::admission::AdmissionPolicyV1;
const HEADER: usize = 1024;
const FAMILY: usize = 128;
const ROW: usize = 1024;
type LaterLink = ([u8; 32], [u8; 32], usize);

#[cfg(test)]
#[path = "boolean_qualification_codec_tests.rs"]
mod tests;

fn proof_bytes(folds: usize) -> Result<usize, String> {
    folds
        .checked_mul(64)
        .and_then(|n| n.checked_add(320))
        .ok_or_else(|| "qualification proof width overflow".into())
}
pub(super) fn required_bytes(m: &Manifest) -> Result<u64, String> {
    let rows = ROW
        .checked_add(proof_bytes(m.folds)?)
        .and_then(|n| n.checked_mul(m.count));
    let bytes = m
        .later
        .len()
        .checked_mul(FAMILY)
        .and_then(|n| n.checked_add(HEADER))
        .and_then(|n| n.checked_add(m.plan.len()))
        .and_then(|n| n.checked_add(rows?))
        .ok_or("qualification fixed-section byte overflow")?;
    u64::try_from(bytes).map_err(display)
}
pub(super) fn encode(m: &Manifest, rows: &[Row]) -> Result<Vec<u8>, String> {
    let declared = validate_plan(m)?;
    validate_family(m, rows)?;
    let bytes = required_bytes(m)?;
    if rows.len() != m.count || bytes > m.bounds.bytes {
        return Err("qualification complete body exceeds declared count/budget".into());
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(usize::try_from(bytes).map_err(display)?)
        .map_err(display)?;
    output.extend_from_slice(b"BRBQLF01");
    for digest in [m.identity, m.original, m.original_pin, m.scope, m.unit] {
        output.extend_from_slice(&digest);
    }
    output.extend_from_slice(&m.policy.canonical_bytes());
    for word in m
        .procedure
        .into_iter()
        .chain([
            m.allocation.batch(),
            m.allocation.batches(),
            m.allocation.rung(),
            m.allocation.rungs(),
            m.allocation.alpha_ppm(),
        ])
        .chain(m.bounds.words())
        .chain([
            m.folds as u64,
            m.count as u64,
            m.periods as u64,
            m.later.len() as u64,
            m.plan.len() as u64,
        ])
    {
        output.extend_from_slice(&word.to_le_bytes());
    }
    output.extend_from_slice(&m.family_digest);
    output.extend_from_slice(&m.shared_digest.unwrap_or([0; 32]));
    for word in m.family_tests.into_iter().flatten() {
        output.extend_from_slice(&word.to_le_bytes());
    }
    pad(&mut output, HEADER)?;
    output.extend_from_slice(&m.plan);
    for (id, pin, count) in &m.later {
        let end = output
            .len()
            .checked_add(FAMILY)
            .ok_or("qualification family offset overflow")?;
        output.extend_from_slice(id);
        output.extend_from_slice(pin);
        output.extend_from_slice(&(*count as u64).to_le_bytes());
        pad(&mut output, end)?;
    }
    let proof_bytes = proof_bytes(m.folds)?;
    for row in rows {
        let end = output
            .len()
            .checked_add(ROW)
            .ok_or("qualification row offset overflow")?;
        output.extend_from_slice(&row.original);
        output.extend_from_slice(&row.later);
        for word in [
            row.family as u64,
            row.coordinate as u64,
            u64::from(row.fold.is_some()),
        ]
        .into_iter()
        .chain(row.romano)
        {
            output.extend_from_slice(&word.to_le_bytes());
        }
        output.extend_from_slice(&row.projection.canonical_bytes());
        pad(&mut output, end)?;
        match &row.fold {
            Some(proof) if proof.len() == proof_bytes => {
                let saved = runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1::decode(proof,m.bounds.bytes,m.folds)?;
                validate_windows(&declared, &saved)?;
                output.extend_from_slice(proof);
            }
            None => output.resize(output.len() + proof_bytes, 0),
            _ => return Err("qualification fold proof width differs".into()),
        }
    }
    if output.len() as u64 != bytes {
        return Err("qualification encoded body cardinality differs".into());
    }
    Ok(output)
}
fn pad(raw: &mut Vec<u8>, end: usize) -> Result<(), String> {
    if raw.len() > end {
        return Err("qualification field exceeds fixed stride".into());
    }
    raw.resize(end, 0);
    Ok(())
}

pub(super) fn decode(raw: &[u8], id: [u8; 32]) -> Result<(Manifest, Vec<Row>), String> {
    let mut header = Input::new(raw.get(..HEADER).ok_or("qualification header absent")?);
    if header.take::<8>()? != *b"BRBQLF01" || header.take::<32>()? != id {
        return Err("qualification domain/identity differs".into());
    }
    let original = header.take()?;
    let original_pin = header.take()?;
    let scope = header.take()?;
    let unit = header.take()?;
    let policy = AdmissionPolicyV1::from_canonical_bytes(&header.take::<310>()?)
        .map_err(|why| format!("qualification policy: {why:?}"))?;
    let procedure = [header.word()?, header.word()?, header.word()?];
    let allocation = runner::family_allocation_v1::allocate(
        header.word()?,
        header.word()?,
        header.word()?,
        header.word()?,
        header.word()?,
    )
    .map_err(|why| format!("qualification allocation: {why:?}"))?;
    let bounds = Bounds {
        candidates: header.word()?,
        observations: header.word()?,
        work: header.word()?,
        memory: header.word()?,
        bytes: header.word()?,
    };
    let folds = header.size()?;
    let count = header.size()?;
    let periods = header.size()?;
    let families = header.size()?;
    let plan_len = header.size()?;
    let family_digest = header.take()?;
    let shared = header.take::<32>()?;
    let mut family_tests = [[0; 5]; 2];
    for row in &mut family_tests {
        for word in row {
            *word = header.word()?;
        }
    }
    header.padding()?;
    let mut input = Input::new(raw.get(HEADER..).ok_or("qualification sections absent")?);
    let plan = input.slice(plan_len)?.to_vec();
    if families == 0
        || families > runner::research_family::RESEARCH_FAMILY_CAPACITY_V1
        || count == 0
        || folds == 0
        || periods < 2
        || count as u64 > bounds.candidates
        || raw.len() as u64 > bounds.bytes
        || allocation.batches() != 1
        || allocation.rungs() != 8
        || scope == [0; 32]
        || unit == [0; 32]
        || original == [0; 32]
        || original_pin == [0; 32]
        || family_digest == [0; 32]
        || procedure[0] == 0
        || procedure[2] == 0
        || (count as u64)
            .checked_mul(periods as u64)
            .is_none_or(|n| n > bounds.observations)
    {
        return Err("qualification manifest admission/domain differs".into());
    }
    let later = decode_links(&mut input, families, count)?;
    let m = Manifest {
        identity: id,
        original,
        original_pin,
        scope,
        unit,
        policy,
        procedure,
        allocation,
        bounds,
        folds,
        count,
        periods,
        family_digest,
        shared_digest: (shared != [0; 32]).then_some(shared),
        family_tests,
        later,
        plan,
    };
    if required_bytes(&m)? != raw.len() as u64 || identity(&m) != id {
        return Err("qualification manifest does not reproduce complete identity".into());
    }
    let declared = validate_plan(&m)?;
    let rows = decode_rows(&m, &mut input, &declared)?;
    input.padding()?;
    validate_family(&m, &rows)?;
    if encode(&m, &rows)? != raw {
        return Err("qualification noncanonical body".into());
    }
    Ok((m, rows))
}

fn decode_links(
    input: &mut Input<'_>,
    families: usize,
    expected: usize,
) -> Result<Vec<LaterLink>, String> {
    let mut later = Vec::new();
    later.try_reserve_exact(families).map_err(display)?;
    let mut total = 0_usize;
    for _ in 0..families {
        let mut row = Input::new(input.slice(FAMILY)?);
        let id = row.take()?;
        let pin = row.take()?;
        let count = row.size()?;
        row.padding()?;
        if id == [0; 32] || pin == [0; 32] || count == 0 {
            return Err("qualification empty later link".into());
        }
        total = total
            .checked_add(count)
            .ok_or("qualification family count overflow")?;
        later.push((id, pin, count));
    }
    if total != expected {
        return Err("qualification family count differs".into());
    }
    Ok(later)
}

fn decode_rows(
    m: &Manifest,
    input: &mut Input<'_>,
    declared: &ObservedPlan,
) -> Result<Vec<Row>, String> {
    let width = proof_bytes(m.folds)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(m.count).map_err(display)?;
    for (family, (_, _, coordinates)) in m.later.iter().enumerate() {
        for coordinate in 0..*coordinates {
            let mut row = Input::new(input.slice(ROW)?);
            let original = row.take()?;
            let later = row.take()?;
            if row.size()? != family || row.size()? != coordinate {
                return Err("qualification coordinate order differs".into());
            }
            let tag = row.word()?;
            let mut romano = [0; 12];
            for word in &mut romano {
                *word = row.word()?;
            }
            let projection = runner::admission::research_projection_codec::decode(
                &m.policy,
                &row.take::<448>()?,
            )?;
            row.padding()?;
            let proof = input.slice(width)?;
            let fold = match tag {
                0 if proof.iter().all(|v| *v == 0) => None,
                1 => {
                    let saved = runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1::decode(proof,m.bounds.bytes,m.folds)?;
                    validate_windows(declared, &saved)?;
                    Some(proof.to_vec())
                }
                _ => return Err("qualification proof tag/padding differs".into()),
            };
            numeric(m, romano)?;
            if original == [0; 32] || later == [0; 32] {
                return Err("qualification empty coordinate identity".into());
            }
            rows.push(Row {
                original,
                later,
                family,
                coordinate,
                projection,
                romano,
                fold,
            });
        }
    }
    Ok(rows)
}

fn numeric(m: &Manifest, r: [u64; 12]) -> Result<(), String> {
    let [class, n, d, present, ..] = r;
    super::probability(n, d)?;
    if !(1..=3).contains(&class)
        || present > 1
        || (class == 1 && (present != 1 || n != r[10] || d != r[11]))
        || d != m.procedure[0]
            .checked_add(1)
            .ok_or("qualification draw overflow")?
        || (class != 1 && n != d)
        || (class == 2
            && (present != 0
                || r.get(4..)
                    .ok_or("RW fields absent")?
                    .iter()
                    .any(|v| *v != 0)))
        || (class == 3 && present != 1)
    {
        return Err("qualification Romano-Wolf mapping differs".into());
    }
    if present == 1 {
        let statistic = f64::from_bits(r[6]);
        if !statistic.is_finite()
            || (class == 1 && statistic <= 0.0)
            || (class == 3 && statistic > 0.0)
            || r[4] >= m.count as u64
            || r[5] >= m.count as u64
            || r[7] > m.procedure[0]
            || r[8]
                != r[7]
                    .checked_add(1)
                    .ok_or("qualification exceedance overflow")?
            || r[9]
                != m.procedure[0]
                    .checked_add(1)
                    .ok_or("qualification draw overflow")?
            || r[11] != r[9]
            || r[10] < r[8]
            || r[10] > r[11]
        {
            return Err("qualification shared Romano-Wolf facts invalid".into());
        }
    }
    Ok(())
}

pub(super) fn validate_plan(m: &Manifest) -> Result<ObservedPlan, String> {
    let plan = ObservedPlan::decode(&m.plan, m.bounds.memory, m.folds as u64)?;
    let policy = m.policy.values();
    let rung = usize::try_from(m.allocation.rung()).map_err(display)?;
    let [bytes, records] = plan.source_limits();
    let numerical = plan.numerical_bounds()?;
    let procedure = plan.procedure();
    if plan.descriptor() != m.scope
        || plan.units().get(rung) != Some(&m.unit)
        || plan.allocation(rung)? != m.allocation
        || plan.policy_digest() != m.policy.digest()
        || plan.minimum_folds() != [policy.min_decided_folds, policy.min_profitable_oos_folds]
        || plan.alpha_ceilings()
            != [
                policy.max_fwer_p_value_ppm,
                policy.max_romano_wolf_p_value_ppm,
            ]
        || [
            procedure.draws(),
            procedure.seed(),
            procedure.block_length(),
        ] != m.procedure
        || plan.windows().len() != m.folds
        || m.bounds.candidates != records
        || m.bounds.observations != records
        || m.bounds.bytes != bytes
        || m.bounds.work != numerical.max_work
        || m.bounds.memory != numerical.max_bytes
    {
        return Err("qualification manifest differs from its full predeclared plan".into());
    }
    Ok(plan)
}
pub(super) fn validate_windows(
    plan: &ObservedPlan,
    saved: &runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1,
) -> Result<(), String> {
    if saved.requested() != plan.requested()?
        || saved.folds().len() != plan.windows().len()
        || saved
            .folds()
            .iter()
            .zip(plan.windows())
            .any(|(fold, window)| fold.window != *window)
    {
        return Err("qualification proof differs from predeclared civil windows".into());
    }
    Ok(())
}
fn validate_family(m: &Manifest, rows: &[Row]) -> Result<(), String> {
    let denominator = m.procedure[0]
        .checked_add(1)
        .ok_or("qualification draw overflow")?;
    for test in m.family_tests {
        if !f64::from_bits(test[0]).is_finite()
            || test[4] > m.procedure[0]
            || test[2]
                != test[4]
                    .checked_add(1)
                    .ok_or("qualification family exceedance overflow")?
            || test[3] != denominator
            || test[1] != probability_bits(test[2], test[3])?
        {
            return Err("qualification family probability fields differ".into());
        }
    }
    let active = rows.iter().filter(|row| row.romano[3] == 1).count();
    if m.shared_digest.is_some() != (active != 0) {
        return Err("qualification shared family presence differs".into());
    }
    let mut ranks = Vec::new();
    ranks.try_reserve_exact(active).map_err(display)?;
    ranks.resize(active, None);
    let mut strategy = 0;
    for row in rows {
        numeric(m, row.romano)?;
        if row.romano[3] == 0 {
            continue;
        }
        let r = row.romano;
        let rank = usize::try_from(r[5]).map_err(display)?;
        let slot = ranks
            .get_mut(rank)
            .ok_or("qualification shared rank outside active family")?;
        if r[4] != strategy || slot.is_some() {
            return Err("qualification active strategy/rank mapping differs".into());
        }
        *slot = Some((r[6], r[4], r[8], r[10]));
        strategy += 1;
    }
    let mut previous = None;
    let mut adjusted = 0;
    for record in ranks {
        let (stat, strategy, initial, saved) = record.ok_or("qualification shared rank omitted")?;
        if previous.is_some_and(|(last, index)| {
            f64::from_bits(last)
                .total_cmp(&f64::from_bits(stat))
                .is_lt()
                || (last == stat && index >= strategy)
        }) {
            return Err("qualification observed statistic rank order differs".into());
        }
        adjusted = adjusted.max(initial);
        if saved != adjusted {
            return Err("qualification exact stepdown recurrence differs".into());
        }
        previous = Some((stat, strategy));
    }
    Ok(())
}
#[expect(
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    reason = "reproduce unrounded statistical probability display bits, never prices"
)]
fn probability_bits(n: u64, d: u64) -> Result<u64, String> {
    super::probability(n, d)?;
    Ok((n as f64 / d as f64).to_bits())
}

struct Input<'a> {
    raw: &'a [u8],
    at: usize,
}
impl<'a> Input<'a> {
    fn new(raw: &'a [u8]) -> Self {
        Self { raw, at: 0 }
    }
    fn slice(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(n)
            .ok_or("qualification field overflow")?;
        let value = self
            .raw
            .get(self.at..end)
            .ok_or("qualification truncated field")?;
        self.at = end;
        Ok(value)
    }
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.slice(N)?.try_into().map_err(display)
    }
    fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take()?))
    }
    fn size(&mut self) -> Result<usize, String> {
        usize::try_from(self.word()?).map_err(display)
    }
    fn padding(&self) -> Result<(), String> {
        if self
            .raw
            .get(self.at..)
            .ok_or("qualification padding absent")?
            .iter()
            .any(|v| *v != 0)
        {
            Err("qualification reserved padding differs".into())
        } else {
            Ok(())
        }
    }
}

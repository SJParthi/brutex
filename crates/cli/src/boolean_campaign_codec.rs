//! Fixed campaign payload; row/family counts are validated before allocation.
use super::{Catalog, Link, REASON_BYTES, Rung, RungScope, State, Status};
use brutex_core::blake3::Hasher;
use runner::research_family::{RESEARCH_FAMILY_CAPACITY_V1, ResearchFamilyV1, ResearchScopeV1};
const HEADER: usize = 256;
const ROW: u64 = 1168;
const FAMILY: u64 = 192;
const MAGIC: &[u8; 8] = b"BRBCAM01";

#[cfg(test)]
pub(super) fn size(families: usize) -> Result<u64, String> {
    size_for(families, RungScope::ALL)
}
pub(super) fn size_for(families: usize, rungs: RungScope) -> Result<u64, String> {
    if families == 0 || families > RESEARCH_FAMILY_CAPACITY_V1 {
        return Err("campaign family count invalid".into());
    }
    (families as u64)
        .checked_mul(FAMILY)
        .and_then(|n| n.checked_mul(rungs.count() as u64))
        .and_then(|n| n.checked_add(8 * ROW))
        .and_then(|n| n.checked_add(HEADER as u64))
        .ok_or_else(|| "campaign body size overflow".into())
}
pub(super) fn identity(state: &State) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex-boolean-campaign-identity-v1\0");
    if state.rungs != RungScope::ALL {
        h.update(b"selected-intraday-rungs-v2\0");
        h.update(&[state.rungs.mask()]);
    }
    h.update(&state.descriptor);
    h.update(&state.programs);
    h.update(&state.program_count.to_le_bytes());
    for n in [month(state.from), month(state.to), u64::from(state.horizon)] {
        h.update(&n.to_le_bytes());
    }
    for row in &state.rows {
        h.update(&(row.catalogs.len() as u64).to_le_bytes());
        h.update(&(row.rung.len() as u64).to_le_bytes());
        h.update(row.rung.as_bytes());
        for catalog in &row.catalogs {
            h.update(&catalog.family.encode());
            h.update(&catalog.expected);
        }
    }
    h.finalize()
}
pub(super) fn same_request(left: &State, right: &State) -> Result<(), String> {
    if identity(left) != identity(right) || left.identity != right.identity {
        return Err(
            "saved campaign differs from exact resolved source/policy/program request".into(),
        );
    }
    Ok(())
}
pub(super) fn initial(state: &State) -> Result<(), String> {
    if !matches!(state.status, Status::Waiting | Status::Running)
        || state.rows.iter().enumerate().any(|(index, row)| {
            if !state.rungs.contains(index) {
                return row.status != Status::Excluded;
            }
            row.catalogs.iter().any(|c| c.completion.is_some())
                || row.statistics.is_some()
                || row.admission.is_some()
                || !row.reason.is_empty()
                || !matches!(row.status, Status::Waiting | Status::Running)
                || (Some(index) != state.rungs.indices().next() && row.status != Status::Waiting)
        })
    {
        return Err(
            "campaign initial snapshot claims work without acknowledged predecessors".into(),
        );
    }
    Ok(())
}
pub(super) fn transition(previous: &State, next: &State) -> Result<(), String> {
    same_request(previous, next)?;
    if previous.status == Status::Completed {
        return Err("completed campaign has a later work transition".into());
    }
    for (old, new) in previous.rows.iter().zip(&next.rows) {
        if matches!(old.status, Status::Completed | Status::Excluded) && old != new {
            return Err("campaign completed rung changed".into());
        }
        for (before, after) in old.catalogs.iter().zip(&new.catalogs) {
            if before.completion.is_some() && before.completion != after.completion {
                return Err("campaign acknowledged catalog completion changed".into());
            }
        }
        if old.statistics.is_some() && old.statistics != new.statistics
            || old.admission.is_some() && old.admission != new.admission
        {
            return Err("campaign acknowledged stage completion changed".into());
        }
        if old.status == Status::Waiting && new.status == Status::Completed {
            return Err("campaign timeframe completed without a recorded start".into());
        }
    }
    Ok(())
}
pub(super) fn encode(state: &State) -> Result<Vec<u8>, String> {
    validate(state)?;
    let count = state
        .rows
        .iter()
        .find(|row| row.status != Status::Excluded)
        .ok_or("campaign rows absent")?
        .catalogs
        .len();
    let bytes = usize::try_from(size_for(count, state.rungs)?).map_err(|why| why.to_string())?;
    let mut out = Vec::new();
    out.try_reserve_exact(bytes)
        .map_err(|why| why.to_string())?;
    out.extend_from_slice(if state.rungs == RungScope::ALL {
        MAGIC
    } else {
        b"BRBCAM02"
    });
    out.extend_from_slice(&state.identity);
    out.extend_from_slice(&state.descriptor);
    out.extend_from_slice(&state.programs);
    for n in [
        state.program_count,
        count as u64,
        month(state.from),
        month(state.to),
        u64::from(state.horizon),
        state.status as u64,
        state.previous.map_or(u64::MAX, |v| v.0),
    ] {
        put(&mut out, n);
    }
    out.extend_from_slice(&state.previous.map_or([0; 32], |v| v.1));
    if state.rungs != RungScope::ALL {
        put(&mut out, u64::from(state.rungs.mask()));
    }
    out.resize(HEADER, 0);
    for row in &state.rows {
        put(&mut out, row.status as u64);
        put_link(&mut out, row.statistics);
        put_link(&mut out, row.admission);
        put(&mut out, row.reason.len() as u64);
        out.extend_from_slice(row.reason.as_bytes());
        out.resize(out.len() + REASON_BYTES - row.reason.len(), 0);
        for catalog in &row.catalogs {
            out.extend_from_slice(&catalog.family.encode());
            out.extend_from_slice(&catalog.expected);
            out.extend_from_slice(&catalog.completion.unwrap_or([0; 32]));
        }
    }
    if out.len() != bytes {
        return Err("campaign encoder width differs".into());
    }
    Ok(out)
}
pub(super) fn decode(raw: &[u8]) -> Result<State, String> {
    let mut r = Read { raw, position: 0 };
    let scoped = match r.take(8)? {
        b"BRBCAM01" => false,
        b"BRBCAM02" => true,
        _ => return Err("campaign magic differs".into()),
    };
    let id = r.array()?;
    let descriptor = r.array()?;
    let programs = r.array()?;
    let program_count = r.word()?;
    let count = usize::try_from(r.word()?).map_err(|why| why.to_string())?;
    let from = decode_month(r.word()?)?;
    let to = decode_month(r.word()?)?;
    let horizon = u32::try_from(r.word()?).map_err(|why| why.to_string())?;
    let aggregate_status = status(r.word()?)?;
    let previous = r.word()?;
    let previous_pin = r.array()?;
    let previous = if previous == u64::MAX {
        if previous_pin != [0; 32] {
            return Err("campaign initial predecessor padding differs".into());
        }
        None
    } else {
        Some((previous, previous_pin))
    };
    let rungs = if scoped {
        let scope = RungScope::from_mask(u8::try_from(r.word()?).map_err(|why| why.to_string())?)?;
        if scope == RungScope::ALL {
            return Err("all-eight campaign requires its original encoding".into());
        }
        scope
    } else {
        RungScope::ALL
    };
    if size_for(count, rungs)? != raw.len() as u64 {
        return Err("campaign fixed body width/count differs".into());
    }
    r.zero(HEADER - r.position)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(8).map_err(|why| why.to_string())?;
    for (index, rung) in crate::ledger_all::LEDGER_RUNGS.into_iter().enumerate() {
        let status = status(r.word()?)?;
        let statistics = r.link()?;
        let admission = r.link()?;
        let length = usize::try_from(r.word()?).map_err(|why| why.to_string())?;
        if length > REASON_BYTES {
            return Err("campaign reason width exceeds bound".into());
        }
        let reason = std::str::from_utf8(r.take(length)?)
            .map_err(|why| why.to_string())?
            .to_owned();
        r.zero(REASON_BYTES - length)?;
        let mut catalogs = Vec::new();
        catalogs
            .try_reserve_exact(count)
            .map_err(|why| why.to_string())?;
        for _ in 0..if rungs.contains(index) { count } else { 0 } {
            let family = ResearchFamilyV1::decode(r.take(128)?).map_err(|why| why.to_string())?;
            let expected = r.array()?;
            let pin = r.array()?;
            catalogs.push(Catalog {
                family,
                expected,
                completion: (pin != [0; 32]).then_some(pin),
            });
        }
        rows.push(Rung {
            rung,
            status,
            reason,
            catalogs,
            statistics,
            admission,
        });
    }
    let state = State {
        rungs,
        identity: id,
        descriptor,
        programs,
        program_count,
        from,
        to,
        horizon,
        status: aggregate_status,
        previous,
        rows,
    };
    validate(&state)?;
    Ok(state)
}
fn validate(state: &State) -> Result<(), String> {
    if state.rows.len() != 8
        || state.status == Status::Excluded
        || state.program_count == 0
        || state.horizon == 0
        || state.from > state.to
        || state.identity != identity(state)
    {
        return Err("campaign request/identity shape differs".into());
    }
    for (year, month) in [state.from, state.to] {
        pull::session::Day::new(year, month, 1).map_err(|why| why.to_string())?;
    }
    let first = state
        .rows
        .iter()
        .find(|row| row.status != Status::Excluded)
        .ok_or("campaign selected rows missing")?;
    let keys: Vec<_> = first
        .catalogs
        .iter()
        .map(|c| c.family.instrument())
        .collect();
    let scope =
        ResearchScopeV1::new(&keys, RESEARCH_FAMILY_CAPACITY_V1).map_err(|why| why.to_string())?;
    let mut unfinished = false;
    let mut active = 0;
    let mut waiting = false;
    for (index, (rung, row)) in crate::ledger_all::LEDGER_RUNGS
        .iter()
        .zip(&state.rows)
        .enumerate()
    {
        if !state.rungs.contains(index) {
            validate_excluded(row, rung)?;
            continue;
        }
        if row.rung != *rung
            || row.status == Status::Excluded
            || row.catalogs.len() != scope.families().len()
            || row.reason.len() > REASON_BYTES
        {
            return Err("campaign canonical timeframe/family shape differs".into());
        }
        for (catalog, family) in row.catalogs.iter().zip(scope.families()) {
            if catalog.family != *family
                || catalog.expected == [0; 32]
                || catalog.completion == Some([0; 32])
            {
                return Err("campaign canonical family/identity differs".into());
            }
        }
        let all = row.catalogs.iter().all(|c| c.completion.is_some());
        if (row.statistics.is_some() && !all)
            || (row.admission.is_some() && row.statistics.is_none())
            || (row.status == Status::Completed
                && (row.admission.is_none() || !row.reason.is_empty()))
            || (row.status == Status::Waiting
                && (row.statistics.is_some()
                    || row.catalogs.iter().any(|c| c.completion.is_some())
                    || !row.reason.is_empty()))
            || (matches!(row.status, Status::Paused | Status::Refused) && row.reason.is_empty())
            || (row.status == Status::Running && !row.reason.is_empty())
        {
            return Err("campaign stage completion is inconsistent".into());
        }
        if row.status == Status::Completed {
            if unfinished {
                return Err("campaign completed timeframe is not a prefix".into());
            }
        } else {
            unfinished = true;
            if row.status == Status::Waiting {
                waiting = true;
            } else {
                if waiting {
                    return Err("campaign skipped an earlier waiting timeframe".into());
                }
                active += 1;
            }
        }
    }
    if active > 1
        || (state.status == Status::Completed) == unfinished
        || (state.status == Status::Waiting
            && state
                .rows
                .iter()
                .any(|r| !matches!(r.status, Status::Waiting | Status::Excluded)))
        || matches!(state.status, Status::Paused | Status::Refused)
            && !state.rows.iter().any(|r| r.status == state.status)
        || (state.status == Status::Running
            && state
                .rows
                .iter()
                .any(|r| matches!(r.status, Status::Paused | Status::Refused)))
    {
        return Err("campaign aggregate state differs from its complete timeframe rows".into());
    }
    Ok(())
}
fn validate_excluded(row: &Rung, rung: &str) -> Result<(), String> {
    if row.rung != rung
        || row.status != Status::Excluded
        || !row.catalogs.is_empty()
        || row.statistics.is_some()
        || row.admission.is_some()
        || !row.reason.is_empty()
    {
        return Err("excluded campaign timeframe carries work or evidence".into());
    }
    Ok(())
}
fn month(value: (u16, u8)) -> u64 {
    u64::from(value.0) * 100 + u64::from(value.1)
}
fn decode_month(value: u64) -> Result<(u16, u8), String> {
    Ok((
        u16::try_from(value / 100).map_err(|why| why.to_string())?,
        u8::try_from(value % 100).map_err(|why| why.to_string())?,
    ))
}
fn put(out: &mut Vec<u8>, n: u64) {
    out.extend_from_slice(&n.to_le_bytes());
}
fn put_link(out: &mut Vec<u8>, link: Option<Link>) {
    let link = link.unwrap_or(Link {
        identity: [0; 32],
        completion: [0; 32],
    });
    out.extend_from_slice(&link.identity);
    out.extend_from_slice(&link.completion);
}
fn status(value: u64) -> Result<Status, String> {
    match value {
        0 => Ok(Status::Waiting),
        1 => Ok(Status::Running),
        2 => Ok(Status::Paused),
        3 => Ok(Status::Refused),
        4 => Ok(Status::Completed),
        5 => Ok(Status::Excluded),
        _ => Err("campaign status byte invalid".into()),
    }
}
struct Read<'a> {
    raw: &'a [u8],
    position: usize,
}
impl<'a> Read<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .position
            .checked_add(n)
            .ok_or("campaign offset overflow")?;
        let value = self
            .raw
            .get(self.position..end)
            .ok_or("campaign body truncated")?;
        self.position = end;
        Ok(value)
    }
    fn array(&mut self) -> Result<[u8; 32], String> {
        self.take(32)?
            .try_into()
            .map_err(|_| "campaign digest truncated".into())
    }
    fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| "campaign word truncated")?,
        ))
    }
    fn zero(&mut self, n: usize) -> Result<(), String> {
        if self.take(n)?.iter().any(|b| *b != 0) {
            return Err("campaign padding differs".into());
        }
        Ok(())
    }
    fn link(&mut self) -> Result<Option<Link>, String> {
        let identity = self.array()?;
        let completion = self.array()?;
        match (identity == [0; 32], completion == [0; 32]) {
            (true, true) => Ok(None),
            (false, false) => Ok(Some(Link {
                identity,
                completion,
            })),
            _ => Err("campaign partial stage link".into()),
        }
    }
}

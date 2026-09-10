//! Observation-only later-period codec. It never reconstructs live replay authority.
use super::super::{
    BooleanCoordinateV1, Chosen, CommittedBooleanFamilyV1, Expression, Path, Request,
    ResearchFamilyV1, Source, TradeRow, display, persistence,
};
use crate::candidate_universe::boolean_candidate_v1::reader::{
    Coordinate, Decode, Decoded, Session,
};
pub(super) const HEADER: usize = 232;
/// Exact saved relation between original and later observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Original complete candidate catalog, never a selected-only catalog.
    pub parent: [u8; 32],
    /// Exact original completion pin.
    pub parent_completion: [u8; 32],
    /// Strict later source/receipt identity.
    pub source: [u8; 32],
    /// Actual later one-minute OHLCV bytes digest.
    pub execution: [u8; 32],
    /// First actual later execution timestamp.
    pub first_micros: i64,
    /// Last actual later execution timestamp.
    pub last_micros: i64,
    /// Actual later execution records.
    pub bars: u64,
    /// Explicit first later month.
    pub from: (u16, u8),
    /// Explicit last later month.
    pub to: (u16, u8),
}
/// Authenticated comparison plus its complete original candidate artifact.
/// Cold work is linear in both bodies; warm pages copy at most256 rows.
pub struct Reader {
    observation: persistence::Observation,
    parent: super::super::reader::Reader,
    summary: Summary,
    decoded: Decoded,
}
impl Reader {
    pub(crate) fn with_rows<T>(
        &self,
        work: impl FnOnce(&[super::super::BooleanCoordinateV1]) -> Result<T, String>,
    ) -> Result<T, String> {
        self.observation.with_current(|| {
            self.parent.require_current()?;
            let result = work(&self.decoded.4)?;
            self.parent.require_current()?;
            Ok(result)
        })
    }
    /// Authenticate a comparison and its original catalog within one aggregate bound.
    /// # Errors
    /// Refuses missing, corrupt, changed, cross-linked or over-budget artifacts.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Self, String> {
        let (observation, (summary, decoded)) =
            persistence::Observation::open(root, "boolean-oos-v1", identity, max_bytes, |body| {
                decode(body, identity)
            })?;
        let remaining = max_bytes
            .checked_sub(observation.body_bytes())
            .and_then(|value| value.checked_sub(112))
            .ok_or("Boolean later aggregate byte ceiling")?;
        let parent = super::super::reader::Reader::open(root, summary.parent, remaining)?;
        let reader = Self {
            observation,
            parent,
            summary,
            decoded,
        };
        reader.check_parent()?;
        reader.require_current()?;
        Ok(reader)
    }
    /// Exact completed later comparison identity.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.observation.identity()
    }
    /// Required immutable page pin.
    #[must_use]
    pub const fn completion_digest(&self) -> [u8; 32] {
        self.observation.completion_digest()
    }
    /// This comparison body; the linked original body is additionally admitted.
    #[must_use]
    pub const fn body_bytes(&self) -> u64 {
        self.observation.body_bytes()
    }
    /// Exact original and later source relation.
    #[must_use]
    pub const fn summary(&self) -> Summary {
        self.summary
    }
    /// Authenticated original catalog for full training comparison/navigation.
    #[must_use]
    pub const fn parent(&self) -> &super::super::reader::Reader {
        &self.parent
    }
    /// Full approved family identity.
    #[must_use]
    pub fn family(&self) -> ResearchFamilyV1 {
        self.decoded.1
    }
    /// Original complete executable programs.
    #[must_use]
    pub fn programs(&self) -> &[Expression] {
        &self.decoded.2
    }
    /// Every accepted later execution session, including zero-trade sessions.
    #[must_use]
    pub fn sessions(&self) -> &[i64] {
        &self.decoded.3
    }
    /// Complete original-coordinate population observed later.
    #[must_use]
    pub fn coordinate_count(&self) -> usize {
        self.decoded.4.len()
    }
    /// Recheck both immutable artifacts. This does not reread raw market files.
    /// # Errors
    /// Refuses missing or changed files and active publishers.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation
            .with_current(|| self.parent.require_current())
    }
    /// Later facts at the same coordinate index as the original catalog.
    /// # Errors
    /// Refuses foreign pins, missing coordinates, invalid pages or changed artifacts.
    pub fn coordinates(
        &self,
        pin: [u8; 32],
        start: usize,
        limit: usize,
    ) -> Result<Vec<Coordinate>, String> {
        self.project(pin, || {
            Ok(page(&self.decoded.4, start, limit)?
                .iter()
                .map(|r| Coordinate {
                    identity: r.identity,
                    run: r.run,
                    program_index: r.program_index,
                    side: r.side,
                    ordinal: r.ordinal,
                    cell: r.cell,
                    execution_refusal_bits: r.refusal.bits(),
                    truth: r.summary,
                    support_sessions: r.support_sessions,
                })
                .collect())
        })
    }
    /// Exact later trade facts for one original coordinate.
    /// # Errors
    /// Refuses foreign pins, invalid pages or changed artifacts.
    pub fn trades(
        &self,
        pin: [u8; 32],
        coordinate: usize,
        start: usize,
        limit: usize,
    ) -> Result<Vec<TradeRow>, String> {
        self.project(pin, || {
            Ok(page(
                &self
                    .decoded
                    .4
                    .get(coordinate)
                    .ok_or("Boolean later coordinate absent")?
                    .trades,
                start,
                limit,
            )?
            .to_vec())
        })
    }
    /// Exact later sessions, retaining explicit zeros.
    /// # Errors
    /// Refuses foreign pins, invalid pages or changed artifacts.
    pub fn periods(
        &self,
        pin: [u8; 32],
        coordinate: usize,
        start: usize,
        limit: usize,
    ) -> Result<Vec<Session>, String> {
        self.project(pin, || {
            Ok(page(
                &self
                    .decoded
                    .4
                    .get(coordinate)
                    .ok_or("Boolean later coordinate absent")?
                    .periods,
                start,
                limit,
            )?
            .iter()
            .map(|p| Session {
                day: p.day,
                return_paisa: p.return_paisa,
                trades: p.trades,
                wins: p.wins,
            })
            .collect())
        })
    }
    fn project<T>(
        &self,
        pin: [u8; 32],
        f: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        if pin != self.completion_digest() {
            return Err("Boolean later completion pin differs".into());
        }
        self.observation.with_current(|| {
            self.parent.require_current()?;
            let result = f()?;
            self.parent.require_current()?;
            Ok(result)
        })
    }
    fn check_parent(&self) -> Result<(), String> {
        if self.parent.completion_digest() != self.summary.parent_completion
            || self.parent.cohort_digest() != self.decoded.0
            || self.parent.family() != self.decoded.1
            || self.parent.programs() != self.decoded.2
            || self.parent.grids() != &self.decoded.5
        {
            return Err(
                "Boolean later original catalog/program/family/grid linkage differs".into(),
            );
        }
        self.parent.with_coordinates(|original| {
            if original.len() != self.decoded.4.len() {
                return Err("Boolean later complete population differs".into());
            }
            for (old, later) in original.iter().zip(&self.decoded.4) {
                if (
                    old.program_index,
                    old.side,
                    old.ordinal,
                    Chosen::from_cell(&old.cell),
                ) != (
                    later.program_index,
                    later.side,
                    later.ordinal,
                    Chosen::from_cell(&later.cell),
                ) || old.run == later.run
                {
                    return Err("Boolean later original coordinate/run linkage differs".into());
                }
            }
            Ok(())
        })
    }
}
fn page<T>(rows: &[T], start: usize, limit: usize) -> Result<&[T], String> {
    if limit == 0 || limit > 256 || start > rows.len() {
        return Err("Boolean later page bounds refused".into());
    }
    rows.get(
        start
            ..start
                .checked_add(limit)
                .ok_or("Boolean later page overflow")?
                .min(rows.len()),
    )
    .ok_or_else(|| "Boolean later page absent".into())
}
pub(super) fn encode(
    training: &CommittedBooleanFamilyV1,
    source: &Source,
    request: &Request<'_>,
    identity: [u8; 32],
    sessions: &[i64],
    rows: &[BooleanCoordinateV1],
) -> Result<Vec<u8>, String> {
    let nested = persistence::encode(
        (identity, training.cohort),
        training.family(),
        &training.programs,
        sessions,
        rows,
        (&training.resolutions, training.training_context.horizon),
        request
            .bounds
            .bytes
            .checked_sub(HEADER as u64)
            .ok_or("Boolean later header exceeds byte ceiling")?,
    )?;
    let execution = super::execution(source);
    let first = execution
        .first()
        .ok_or("Boolean later execution absent")?
        .ts_micros;
    let last = execution
        .last()
        .ok_or("Boolean later execution absent")?
        .ts_micros;
    let mut out = Vec::new();
    out.try_reserve_exact(
        HEADER
            .checked_add(nested.len())
            .ok_or("Boolean later body size overflow")?,
    )
    .map_err(display)?;
    out.extend_from_slice(b"BRBOOS01");
    for value in [
        identity,
        training.identity,
        training.completion,
        source.identity,
        runner::identity::data_digest(execution),
    ] {
        out.extend_from_slice(&value);
    }
    out.extend_from_slice(&first.to_le_bytes());
    out.extend_from_slice(&last.to_le_bytes());
    for value in [
        execution.len() as u64,
        u64::from(request.from.0),
        u64::from(request.from.1),
        u64::from(request.to.0),
        u64::from(request.to.1),
        nested.len() as u64,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    if out.len() != HEADER {
        return Err("Boolean later header stride differs".into());
    }
    out.extend_from_slice(&nested);
    Ok(out)
}
pub(super) fn decode(body: &[u8], identity: [u8; 32]) -> Result<(Summary, Decoded), String> {
    let mut input = Decode::new(body.get(..HEADER).ok_or("Boolean later header truncated")?);
    if input.bytes::<8>()? != *b"BRBOOS01" || input.bytes::<32>()? != identity {
        return Err("Boolean later domain/identity differs".into());
    }
    let parent = input.bytes()?;
    let parent_completion = input.bytes()?;
    let source = input.bytes()?;
    let execution = input.bytes()?;
    let first_micros = input.signed()?;
    let last_micros = input.signed()?;
    let bars = input.word()?;
    let from = (
        u16::try_from(input.word()?).map_err(display)?,
        u8::try_from(input.word()?).map_err(display)?,
    );
    let to = (
        u16::try_from(input.word()?).map_err(display)?,
        u8::try_from(input.word()?).map_err(display)?,
    );
    let length = input.size()?;
    for (year, month) in [from, to] {
        pull::session::Day::new(year, month, 1).map_err(display)?;
    }
    if from > to
        || bars == 0
        || first_micros > last_micros
        || first_micros.rem_euclid(60_000_000) != 0
        || last_micros.rem_euclid(60_000_000) != 0
        || body.len().checked_sub(HEADER) != Some(length)
    {
        return Err("Boolean later span/count/stride invalid".into());
    }
    let decoded = super::super::reader::decode(
        body.get(HEADER..).ok_or("Boolean later body absent")?,
        identity,
    )?;
    for stamp in [first_micros, last_micros] {
        let date = pull::session::Day::from_days(u32::try_from(day(stamp)).map_err(display)?)
            .map_err(display)?;
        if (date.year(), date.month()) < from || (date.year(), date.month()) > to {
            return Err("Boolean later actual execution lies outside declared months".into());
        }
    }
    for grid in &decoded.5 {
        let last =
            pull::session::Day::from_days(u32::try_from(day(grid.last_micros)).map_err(display)?)
                .map_err(display)?;
        if from <= (last.year(), last.month()) {
            return Err("Boolean later declared months overlap training".into());
        }
    }
    validate_actual(&decoded, first_micros, last_micros, bars)?;
    Ok((
        Summary {
            parent,
            parent_completion,
            source,
            execution,
            first_micros,
            last_micros,
            bars,
            from,
            to,
        },
        decoded,
    ))
}
fn day(ts: i64) -> i128 {
    (i128::from(ts) + 19_800_000_000).div_euclid(86_400_000_000)
}
pub(super) fn validate_actual(
    decoded: &Decoded,
    first: i64,
    last: i64,
    bars: u64,
) -> Result<(), String> {
    for grid in &decoded.5 {
        if day(first) <= day(grid.last_micros) {
            return Err("Boolean later body overlaps training session".into());
        }
    }
    for session in &decoded.3 {
        if i128::from(*session) < day(first) || i128::from(*session) > day(last) {
            return Err("Boolean later session outside execution span".into());
        }
    }
    for row in &decoded.4 {
        let mut previous = None;
        for trade in &row.trades {
            if trade.entry_micros < first
                || trade.exit_micros > last
                || trade.exit_bar as u64 >= bars
                || trade.signal_bar > trade.entry_bar
                || previous.is_some_and(|exit| trade.entry_bar <= exit)
                || trade.entry_micros.rem_euclid(60_000_000) != 0
                || trade.exit_micros.rem_euclid(60_000_000) != 0
                || day(trade.entry_micros) != day(trade.exit_micros)
                || (i128::from(trade.exit_micros) + 19_800_000_000).rem_euclid(86_400_000_000)
                    > 54_540_000_000
            {
                return Err("Boolean later trade outside exact same-day deadline/source".into());
            }
            previous = Some(trade.exit_bar);
        }
        reconcile_periods(row)?;
    }
    Ok(())
}
fn reconcile_periods(row: &BooleanCoordinateV1) -> Result<(), String> {
    let mut trades = row.trades.iter().peekable();
    for period in &row.periods {
        let mut totals = (0_i64, 0_u64, 0_u64);
        while trades
            .peek()
            .is_some_and(|trade| day(trade.exit_micros) == i128::from(period.day))
        {
            let trade = trades
                .next()
                .ok_or("Boolean later trade cursor disappeared")?;
            totals.0 = totals
                .0
                .checked_add(trade.worst)
                .ok_or("Boolean later period return overflow")?;
            totals.1 = totals
                .1
                .checked_add(1)
                .ok_or("Boolean later period count overflow")?;
            totals.2 = totals
                .2
                .checked_add(u64::from(trade.worst > 0))
                .ok_or("Boolean later period wins overflow")?;
        }
        if totals != (period.return_paisa, period.trades, period.wins) {
            return Err("Boolean later per-session trades and returns differ".into());
        }
    }
    if trades.next().is_some() {
        return Err("Boolean later trade omitted from session observations".into());
    }
    Ok(())
}

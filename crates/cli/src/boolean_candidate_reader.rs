//! Bounded observation-only projections; decoded bytes never mint authoring authority.
use super::{
    BooleanCoordinateV1, BooleanSessionV1, Expression, ResearchFamilyV1, Side, display, persistence,
};
use brutex_core::blake3::hash;
use runner::exit_grid_policy::ExecutionRefusalBitsV1;
use runner::grid::{Cell, TradeRow, Ttp};
use std::path::Path;

/// One saved candidate's exact measured facts, before statistical admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coordinate {
    /// Full program/family/run/coordinate identity.
    pub identity: [u8; 32],
    /// Exact directional program run.
    pub run: [u8; 32],
    /// Index into this reader's complete program catalog.
    pub program_index: usize,
    /// Priced direction.
    pub side: Side,
    /// Complete-grid ordinal in this program/direction.
    pub ordinal: u64,
    /// Actual measured cell, including zero-trade coordinates.
    pub cell: Cell,
    /// Execution-policy refusal reasons; zero does not imply institutional admission.
    pub execution_refusal_bits: u64,
    /// True, false and unknown outcomes on the exact execution column.
    pub truth: runner::expression::Summary,
    /// Distinct actual IST sessions containing a definite signal, independent of trades.
    pub support_sessions: u64,
}

/// Explicit zero-inclusive accepted-session observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    /// Exact IST civil day since epoch.
    pub day: i64,
    /// Pessimistic per-unit paisa, preserved without rounding.
    pub return_paisa: i64,
    /// Actual completed trades exiting in this session.
    pub trades: u64,
    /// Strictly positive pessimistic trades.
    pub wins: u64,
}

/// Cold opening is O(saved bytes); each warm page checks fixed generations and
/// copies at most256 items. This read-only projection is not admission authority.
pub struct Reader {
    observation: persistence::Observation,
    cohort: [u8; 32],
    family: ResearchFamilyV1,
    programs: Vec<Expression>,
    sessions: Vec<i64>,
    rows: Vec<BooleanCoordinateV1>,
    grids: [super::grid_context::GridContext; 2],
}

impl Reader {
    /// Authenticate a complete immutable catalog within one explicit byte limit.
    /// # Errors
    /// Refuses missing/busy/nonregular/changed files, corrupt receipt/body,
    /// incomplete order/counts, invalid programs or an exceeded byte ceiling.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Self, String> {
        let (observation, (cohort, family, programs, sessions, rows, grids)) =
            persistence::Observation::open(
                root,
                "boolean-candidates-v1",
                identity,
                max_bytes,
                |body| decode(body, identity),
            )?;
        Ok(Self {
            observation,
            cohort,
            family,
            programs,
            sessions,
            rows,
            grids,
        })
    }
    /// Exact saved catalog identity.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.observation.identity()
    }
    /// Full immutable completion bytes digest, suitable for pagination pins.
    #[must_use]
    pub const fn completion_digest(&self) -> [u8; 32] {
        self.observation.completion_digest()
    }
    /// Exact authenticated body bytes, excluding its 112-byte completion.
    #[must_use]
    pub const fn body_bytes(&self) -> u64 {
        self.observation.body_bytes()
    }
    /// Exact recorded common program/evaluator/rung/span/grid policy identity.
    #[must_use]
    pub const fn cohort_digest(&self) -> [u8; 32] {
        self.cohort
    }
    pub(super) fn with_coordinates<T>(
        &self,
        project: impl FnOnce(&[BooleanCoordinateV1]) -> Result<T, String>,
    ) -> Result<T, String> {
        self.observation.with_current(|| project(&self.rows))
    }
    /// Actual full instrument and snapshot-membership scope.
    #[must_use]
    pub const fn family(&self) -> ResearchFamilyV1 {
        self.family
    }
    /// Complete requested finite program catalog.
    #[must_use]
    pub fn programs(&self) -> &[Expression] {
        &self.programs
    }
    /// Every accepted IST session, including zero-trade sessions.
    #[must_use]
    pub fn sessions(&self) -> &[i64] {
        &self.sessions
    }
    /// Both actual frozen side grids, including exact policy inputs and price levels.
    #[must_use]
    pub const fn grids(&self) -> &[super::grid_context::GridContext; 2] {
        &self.grids
    }
    /// Full coordinate count before statistical admission.
    #[must_use]
    pub fn coordinate_count(&self) -> usize {
        self.rows.len()
    }
    /// Refuse any evidence generation change since the authenticated cold read.
    /// # Errors
    /// A replaced, removed, truncated or modified body/receipt refuses.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation.require_current()
    }
    /// Up to256 canonical candidate facts at a pinned completion.
    /// # Errors
    /// Refuses wrong completion pins, invalid pages or changed files; no prefix escapes.
    pub fn coordinates(
        &self,
        pin: [u8; 32],
        start: usize,
        limit: usize,
    ) -> Result<Vec<Coordinate>, String> {
        self.pin(pin)?;
        self.with_coordinates(|all| {
            let rows = page(all, start, limit)?
                .iter()
                .map(|row| Coordinate {
                    identity: row.identity,
                    run: row.run,
                    program_index: row.program_index,
                    side: row.side,
                    ordinal: row.ordinal,
                    cell: row.cell,
                    execution_refusal_bits: row.refusal.bits(),
                    truth: row.summary,
                    support_sessions: row.support_sessions,
                })
                .collect();
            Ok(rows)
        })
    }
    /// Exact trade rows for the selected canonical candidate, never a winner substitute.
    /// # Errors
    /// Refuses wrong pins, absent candidates, invalid pages or changed evidence.
    pub fn trades(
        &self,
        pin: [u8; 32],
        candidate: usize,
        start: usize,
        limit: usize,
    ) -> Result<Vec<TradeRow>, String> {
        self.pin(pin)?;
        self.with_coordinates(|all| {
            let row = all.get(candidate).ok_or("Boolean candidate absent")?;
            let rows = page(&row.trades, start, limit)?.to_vec();
            Ok(rows)
        })
    }
    /// Zero-inclusive exact session observations for one candidate.
    /// # Errors
    /// The same strict pin, page and generation rules as trade pages apply.
    pub fn periods(
        &self,
        pin: [u8; 32],
        candidate: usize,
        start: usize,
        limit: usize,
    ) -> Result<Vec<Session>, String> {
        self.pin(pin)?;
        self.with_coordinates(|all| {
            let row = all.get(candidate).ok_or("Boolean candidate absent")?;
            let rows = page(&row.periods, start, limit)?
                .iter()
                .map(|period| Session {
                    day: period.day,
                    return_paisa: period.return_paisa,
                    trades: period.trades,
                    wins: period.wins,
                })
                .collect();
            Ok(rows)
        })
    }
    fn pin(&self, pin: [u8; 32]) -> Result<(), String> {
        if pin != self.completion_digest() {
            return Err("Boolean completion pin mismatch".to_owned());
        }
        Ok(())
    }
}

fn page<T>(values: &[T], start: usize, limit: usize) -> Result<&[T], String> {
    if limit == 0 || limit > 256 || start > values.len() {
        return Err("Boolean page bounds refused".to_owned());
    }
    let end = start
        .checked_add(limit)
        .ok_or("Boolean page overflow")?
        .min(values.len());
    values
        .get(start..end)
        .ok_or_else(|| "Boolean page outside retained evidence".to_owned())
}

pub(super) type Decoded = (
    [u8; 32],
    ResearchFamilyV1,
    Vec<Expression>,
    Vec<i64>,
    Vec<BooleanCoordinateV1>,
    [super::grid_context::GridContext; 2],
);
pub(super) fn decode(raw: &[u8], identity: [u8; 32]) -> Result<Decoded, String> {
    let mut input = Decode::new(raw);
    if input.bytes::<8>()? != *b"BRBOOL01" || input.bytes::<32>()? != identity {
        return Err("Boolean body identity mismatch".to_owned());
    }
    let cohort = input.bytes::<32>()?;
    let family = ResearchFamilyV1::decode(&input.bytes::<128>()?).map_err(display)?;
    let programs = input.size()?;
    let sessions = input.size()?;
    let rows = input.size()?;
    if programs == 0 || sessions == 0 || rows == 0 {
        return Err("Boolean completed catalog has an empty required population".to_owned());
    }
    input.admit(programs, vocab::expression::ENCODED_LEN)?;
    let mut catalog = Vec::new();
    catalog.try_reserve_exact(programs).map_err(display)?;
    let mut unique = std::collections::HashSet::new();
    unique.try_reserve(programs).map_err(display)?;
    for _ in 0..programs {
        let encoded = input.bytes::<{ vocab::expression::ENCODED_LEN }>()?;
        if !unique.insert(hash(&encoded)) {
            return Err("Boolean saved catalog duplicates a program".to_owned());
        }
        catalog.push(
            Expression::decode(&encoded)
                .map_err(|why| format!("Boolean program decode: {why:?}"))?,
        );
    }
    input.admit(sessions, 8)?;
    let mut days = Vec::new();
    days.try_reserve_exact(sessions).map_err(display)?;
    for _ in 0..sessions {
        let day = input.signed()?;
        if days.last().is_some_and(|previous| *previous >= day) {
            return Err("Boolean sessions are not chronological".to_owned());
        }
        days.push(day);
    }
    let grids = super::grid_context::decode(&mut input)?;
    let cells = |group: usize| {
        grids
            .get(group % 2)
            .map(|grid| grid.cells)
            .ok_or("Boolean group has no frozen side grid")
    };
    input.admit(rows, 392)?;
    let mut candidates = Vec::new();
    candidates.try_reserve_exact(rows).map_err(display)?;
    let mut previous: Option<(usize, u64)> = None;
    for _ in 0..rows {
        let row = decode_row(&mut input, &days)?;
        let group = row
            .program_index
            .checked_mul(2)
            .and_then(|value| value.checked_add(usize::from(row.side == Side::Short)))
            .ok_or("Boolean group overflow")?;
        match previous {
            None if group == 0 && row.ordinal == 0 => {}
            Some((last, ordinal))
                if (group == last && ordinal.checked_add(1) == Some(row.ordinal))
                    || (last.checked_add(1) == Some(group)
                        && row.ordinal == 0
                        && ordinal.checked_add(1) == Some(cells(last)?)) => {}
            _ => {
                return Err(
                    "Boolean saved coordinate sequence is incomplete or reordered".to_owned(),
                );
            }
        }
        if row.program_index >= programs || row.ordinal >= cells(group)? {
            return Err("Boolean coordinate program outside catalog".to_owned());
        }
        previous = Some((group, row.ordinal));
        candidates.push(row);
    }
    if previous.map(|(group, _)| group.checked_add(1)) != Some(programs.checked_mul(2)) {
        return Err("Boolean saved catalog omitted a program or side".to_owned());
    }
    let (group, ordinal) = previous.ok_or("Boolean completed catalog has no coordinates")?;
    if ordinal.checked_add(1) != Some(cells(group)?) {
        return Err(
            "Boolean coordinate population differs from frozen grid cardinality".to_owned(),
        );
    }
    input.done()?;
    Ok((cohort, family, catalog, days, candidates, grids))
}

fn decode_row(input: &mut Decode<'_>, days: &[i64]) -> Result<BooleanCoordinateV1, String> {
    let identity = input.bytes()?;
    let program_index = input.size()?;
    let run = input.bytes()?;
    let side = match input.word()? {
        0 => Side::Long,
        1 => Side::Short,
        _ => return Err("Boolean side invalid".to_owned()),
    };
    let ordinal = input.word()?;
    let cell = decode_cell(input)?;
    let refusal = ExecutionRefusalBitsV1::from_bits(input.word()?)
        .ok_or("Boolean execution refusal bits invalid")?;
    let periods = input.size()?;
    let trades = input.size()?;
    let support_sessions = input.word()?;
    let summary = runner::expression::Summary {
        evaluated: input.word()?,
        hits: input.word()?,
        misses: input.word()?,
        unknown: input.word()?,
    };
    if summary
        .hits
        .checked_add(summary.misses)
        .and_then(|count| count.checked_add(summary.unknown))
        != Some(summary.evaluated)
        || support_sessions > days.len() as u64
        || periods != days.len()
        || trades as u64 != cell.trades
    {
        return Err("Boolean candidate counts do not reconcile".to_owned());
    }
    input.admit(periods, 32)?;
    let mut observations = Vec::new();
    observations.try_reserve_exact(periods).map_err(display)?;
    let mut totals = (0_i64, 0_u64, 0_u64);
    for day in days {
        let actual = input.signed()?;
        let return_paisa = input.signed()?;
        let trades = input.word()?;
        let wins = input.word()?;
        if actual != *day || wins > trades {
            return Err("Boolean period identity/count invalid".to_owned());
        }
        totals.0 = totals
            .0
            .checked_add(return_paisa)
            .ok_or("Boolean period return overflow")?;
        totals.1 = totals
            .1
            .checked_add(trades)
            .ok_or("Boolean period trades overflow")?;
        totals.2 = totals
            .2
            .checked_add(wins)
            .ok_or("Boolean period wins overflow")?;
        observations.push(BooleanSessionV1 {
            day: actual,
            return_paisa,
            trades,
            wins,
        });
    }
    if totals != (cell.pessimistic, cell.trades, cell.wins) {
        return Err("Boolean periods differ from measured cell".to_owned());
    }
    let actual_trades = decode_trades(input, trades, &cell)?;
    Ok(BooleanCoordinateV1 {
        identity,
        program_index,
        run,
        side,
        ordinal,
        cell,
        refusal,
        periods: observations,
        trades: actual_trades,
        selected: None,
        summary,
        support_sessions,
    })
}

fn decode_trades(
    input: &mut Decode<'_>,
    trades: usize,
    cell: &Cell,
) -> Result<Vec<TradeRow>, String> {
    input.admit(trades, 88)?;
    let mut actual_trades = Vec::new();
    actual_trades.try_reserve_exact(trades).map_err(display)?;
    let mut returns = (0_i64, 0_i64, 0_u64);
    for _ in 0..trades {
        let row = TradeRow {
            signal_bar: input.size()?,
            entry_bar: input.size()?,
            exit_bar: input.size()?,
            best: input.signed()?,
            worst: input.signed()?,
            entry_micros: input.signed()?,
            exit_micros: input.signed()?,
            adverse: input.signed()?,
            adverse_paisa: input.signed()?,
            favourable: input.signed()?,
            favourable_paisa: input.signed()?,
        };
        if row.entry_bar > row.exit_bar
            || row.entry_micros > row.exit_micros
            || indicators::ist_day(row.entry_micros) != indicators::ist_day(row.exit_micros)
        {
            return Err("Boolean trade chronology invalid".to_owned());
        }
        returns.0 = returns
            .0
            .checked_add(row.worst)
            .ok_or("Boolean trade return overflow")?;
        returns.1 = returns
            .1
            .checked_add(row.best)
            .ok_or("Boolean trade optimistic overflow")?;
        returns.2 = returns
            .2
            .checked_add(u64::from(row.worst > 0))
            .ok_or("Boolean trade wins overflow")?;
        actual_trades.push(row);
    }
    if returns != (cell.pessimistic, cell.optimistic, cell.wins) {
        return Err("Boolean actual trades differ from measured cell".to_owned());
    }
    Ok(actual_trades)
}

pub(super) struct Decode<'a> {
    raw: &'a [u8],
    at: usize,
}
impl<'a> Decode<'a> {
    pub(super) fn new(raw: &'a [u8]) -> Self {
        Self { raw, at: 0 }
    }
    pub(super) fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self.at.checked_add(N).ok_or("Boolean decode overflow")?;
        let mut out = [0; N];
        out.copy_from_slice(
            self.raw
                .get(self.at..end)
                .ok_or("Boolean payload truncated")?,
        );
        self.at = end;
        Ok(out)
    }
    pub(super) fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
    pub(super) fn signed(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.bytes()?))
    }
    pub(super) fn size(&mut self) -> Result<usize, String> {
        usize::try_from(self.word()?).map_err(display)
    }
    fn optional(&mut self) -> Result<Option<usize>, String> {
        match self.word()? {
            u64::MAX => Ok(None),
            value => usize::try_from(value).map(Some).map_err(display),
        }
    }
    pub(super) fn admit(&self, count: usize, stride: usize) -> Result<(), String> {
        if count > self.raw.len().saturating_sub(self.at) / stride {
            Err("Boolean declared count exceeds admitted bytes".to_owned())
        } else {
            Ok(())
        }
    }
    fn done(&self) -> Result<(), String> {
        if self.at == self.raw.len() {
            Ok(())
        } else {
            Err("Boolean payload has extra bytes".to_owned())
        }
    }
}
fn decode_cell(input: &mut Decode<'_>) -> Result<Cell, String> {
    let stop = input.optional()?;
    let target = input.optional()?;
    let tsl = input.optional()?;
    let arm = input.optional()?;
    let trail = input.optional()?;
    let ttp = match (arm, trail) {
        (None, None) => None,
        (Some(arm), Some(trail)) => Some(Ttp { arm, trail }),
        _ => return Err("Boolean TTP coordinate incomplete".to_owned()),
    };
    Ok(Cell {
        stop,
        target,
        tsl,
        ttp,
        trades: input.word()?,
        wins: input.word()?,
        pessimistic: input.signed()?,
        optimistic: input.signed()?,
        fill_cost: input.signed()?,
        stopped: input.word()?,
        trailed_stop: input.word()?,
        trailed_profit: input.word()?,
        targeted: input.word()?,
        timed_out: input.word()?,
        ambiguous_bars: input.word()?,
        gapped: input.word()?,
        winner_mae: input.signed()?,
        winner_mfe: input.signed()?,
        all_mae: input.signed()?,
        worst_mae: input.signed()?,
        gross_win: input.signed()?,
        gross_loss: input.signed()?,
        best_trade: input.signed()?,
        min_win: input.signed()?,
        bars_held: input.word()?,
        max_losing_streak: u32::try_from(input.word()?).map_err(display)?,
        max_winning_streak: u32::try_from(input.word()?).map_err(display)?,
        worst_trade: input.signed()?,
        max_drawdown: input.signed()?,
    })
}

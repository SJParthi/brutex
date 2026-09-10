//! Fixed version-one codecs; every variable section is counted before its rows.

use super::{Candidate, Key, Tier};
use crate::Rules;
use costs::fill::Direction;
use runner::grid;

#[derive(Default)]
pub(super) struct Encoder(pub Vec<u8>);
impl Encoder {
    pub(super) fn word(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    pub(super) fn signed(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    pub(super) fn bytes(&mut self, value: &[u8]) {
        self.0.extend_from_slice(value);
    }
    pub(super) fn flag(&mut self, value: bool) {
        self.word(u64::from(value));
    }
    pub(super) fn optional(&mut self, value: Option<usize>) {
        self.word(value.map_or(u64::MAX, |v| v as u64));
    }
    pub(super) fn list(&mut self, values: &[i64]) {
        self.word(values.len() as u64);
        for value in values {
            self.signed(*value);
        }
    }
}
pub(super) struct Decoder<'a> {
    raw: &'a [u8],
    at: usize,
}
impl<'a> Decoder<'a> {
    pub(super) fn new(raw: &'a [u8]) -> Self {
        Self { raw, at: 0 }
    }
    pub(super) fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self.at.checked_add(N).ok_or("candidate offset overflow")?;
        let mut out = [0; N];
        out.copy_from_slice(
            self.raw
                .get(self.at..end)
                .ok_or("candidate payload is truncated")?,
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
        usize::try_from(self.word()?)
            .map_err(|_| "candidate count exceeds address space".to_owned())
    }
    pub(super) fn flag(&mut self) -> Result<bool, String> {
        match self.word()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("unknown candidate flag".to_owned()),
        }
    }
    pub(super) fn optional(&mut self) -> Result<Option<usize>, String> {
        match self.word()? {
            u64::MAX => Ok(None),
            value => usize::try_from(value)
                .map(Some)
                .map_err(|_| "candidate index exceeds address space".to_owned()),
        }
    }
    pub(super) fn list(&mut self) -> Result<Vec<i64>, String> {
        let count = self.size()?;
        if count > self.raw.len().saturating_sub(self.at) / 8 {
            return Err("candidate list exceeds payload".to_owned());
        }
        (0..count).map(|_| self.signed()).collect()
    }
    pub(super) fn done(&self) -> Result<(), String> {
        if self.at == self.raw.len() {
            Ok(())
        } else {
            Err("candidate payload has unrecognized trailing fields".to_owned())
        }
    }
}
pub(super) fn direction(value: Direction) -> u64 {
    match value {
        Direction::Long => 0,
        Direction::Short => 1,
    }
}
pub(super) fn decode_direction(value: u64) -> Result<Direction, String> {
    match value {
        0 => Ok(Direction::Long),
        1 => Ok(Direction::Short),
        _ => Err("unknown candidate direction".to_owned()),
    }
}
fn rules(out: &mut Encoder, value: Rules) {
    out.signed(value.max_mae_ppm);
    out.signed(value.min_rr_bp);
    out.signed(value.min_win_rate_bp);
    out.word(value.min_trades);
    out.signed(value.min_assurance_bp);
    out.signed(value.min_weakest_bp);
    out.signed(value.min_ret_over_dd_bp);
    out.flag(value.require_protective_exits);
    out.signed(value.min_fill_headroom_bp);
    out.signed(value.min_avg_rr_bp);
    out.word(value.top as u64);
}
fn decode_rules(input: &mut Decoder<'_>) -> Result<Rules, String> {
    Ok(Rules {
        max_mae_ppm: input.signed()?,
        min_rr_bp: input.signed()?,
        min_win_rate_bp: input.signed()?,
        min_trades: input.word()?,
        min_assurance_bp: input.signed()?,
        min_weakest_bp: input.signed()?,
        min_ret_over_dd_bp: input.signed()?,
        require_protective_exits: input.flag()?,
        min_fill_headroom_bp: input.signed()?,
        min_avg_rr_bp: input.signed()?,
        top: input.size()?,
    })
}
pub(super) fn tier(value: &Tier) -> Vec<u8> {
    let mut out = Encoder::default();
    out.word(value.index);
    out.word(value.eligible);
    out.word(value.evaluated);
    out.word(value.horizon);
    out.word(value.rungs);
    out.flag(value.step_ppm.is_some());
    out.signed(value.step_ppm.unwrap_or(0));
    out.flag(value.forced_ppm.is_some());
    out.signed(value.forced_ppm.unwrap_or(0));
    out.flag(value.ratios);
    rules(&mut out, value.rules);
    out.list(&value.stops_ppm);
    out.0
}
pub(super) fn decode_tier(raw: &[u8]) -> Result<Tier, String> {
    let mut input = Decoder::new(raw);
    let index = input.word()?;
    let eligible = input.word()?;
    let evaluated = input.word()?;
    let horizon = input.word()?;
    let rungs = input.word()?;
    let has_step = input.flag()?;
    let step = input.signed()?;
    let has_forced = input.flag()?;
    let forced = input.signed()?;
    let value = Tier {
        index,
        eligible,
        evaluated,
        horizon,
        rungs,
        step_ppm: has_step.then_some(step),
        forced_ppm: has_forced.then_some(forced),
        ratios: input.flag()?,
        rules: decode_rules(&mut input)?,
        stops_ppm: input.list()?,
    };
    input.done()?;
    if evaluated > eligible
        || horizon == 0
        || (!has_step && step != 0)
        || (!has_forced && forced != 0)
    {
        return Err("candidate tier has invalid bounds".to_owned());
    }
    Ok(value)
}
fn cell(out: &mut Encoder, value: &grid::Cell) {
    out.optional(value.stop);
    out.optional(value.target);
    out.optional(value.tsl);
    out.optional(value.ttp.map(|v| v.arm));
    out.optional(value.ttp.map(|v| v.trail));
    out.word(value.trades);
    out.word(value.wins);
    out.signed(value.pessimistic);
    out.signed(value.optimistic);
    out.signed(value.fill_cost);
    out.word(value.stopped);
    out.word(value.trailed_stop);
    out.word(value.trailed_profit);
    out.word(value.targeted);
    out.word(value.timed_out);
    out.word(value.ambiguous_bars);
    out.word(value.gapped);
    out.signed(value.winner_mae);
    out.signed(value.winner_mfe);
    out.signed(value.all_mae);
    out.signed(value.worst_mae);
    out.signed(value.gross_win);
    out.signed(value.gross_loss);
    out.signed(value.best_trade);
    out.signed(value.min_win);
    out.word(value.bars_held);
    out.word(u64::from(value.max_losing_streak));
    out.word(u64::from(value.max_winning_streak));
    out.signed(value.worst_trade);
    out.signed(value.max_drawdown);
}
fn decode_cell(input: &mut Decoder<'_>) -> Result<grid::Cell, String> {
    let stop = input.optional()?;
    let target = input.optional()?;
    let tsl = input.optional()?;
    let arm = input.optional()?;
    let trail = input.optional()?;
    let ttp = match (arm, trail) {
        (None, None) => None,
        (Some(arm), Some(trail)) => Some(grid::Ttp { arm, trail }),
        _ => return Err("incomplete candidate trailing take profit".to_owned()),
    };
    Ok(grid::Cell {
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
        max_losing_streak: u32::try_from(input.word()?)
            .map_err(|_| "candidate losing streak overflow")?,
        max_winning_streak: u32::try_from(input.word()?)
            .map_err(|_| "candidate winning streak overflow")?,
        worst_trade: input.signed()?,
        max_drawdown: input.signed()?,
    })
}
pub(super) fn candidate(value: &Candidate) -> Vec<u8> {
    let mut out = Encoder::default();
    out.bytes(&value.identity);
    out.word(value.attempt);
    out.word(value.key.tier);
    out.word(value.key.rank);
    out.word(direction(value.key.direction));
    for word in value.mask_words {
        out.word(word);
    }
    out.bytes(&value.execution_digest);
    out.bytes(&value.tier_digest);
    out.flag(value.admitted);
    out.flag(value.cell.is_some());
    cell(&mut out, &value.cell.unwrap_or_default());
    out.word(value.signals);
    out.word(value.refused_paths);
    out.list(&value.stops);
    out.list(&value.targets);
    out.list(&value.trails);
    out.bytes(&value.trade_identity);
    out.bytes(&value.trades_digest);
    out.0
}
pub(super) fn decode_candidate(raw: &[u8]) -> Result<Candidate, String> {
    let mut input = Decoder::new(raw);
    let identity = input.bytes()?;
    let attempt = input.word()?;
    let key = Key {
        tier: input.word()?,
        rank: input.word()?,
        direction: decode_direction(input.word()?)?,
    };
    let mut mask_words = [0; 6];
    for value in &mut mask_words {
        *value = input.word()?;
    }
    let execution_digest = input.bytes()?;
    let tier_digest = input.bytes()?;
    let admitted = input.flag()?;
    let has_cell = input.flag()?;
    let cell = decode_cell(&mut input)?;
    if !has_cell && (cell != grid::Cell::default() || admitted) {
        return Err("candidate without cell carries a cell verdict".to_owned());
    }
    let value = Candidate {
        identity,
        attempt,
        key,
        mask_words,
        execution_digest,
        tier_digest,
        admitted,
        cell: has_cell.then_some(cell),
        signals: input.word()?,
        refused_paths: input.word()?,
        stops: input.list()?,
        targets: input.list()?,
        trails: input.list()?,
        trade_identity: input.bytes()?,
        trades_digest: input.bytes()?,
    };
    input.done()?;
    Ok(value)
}

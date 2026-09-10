//! Exact trade/session arithmetic from native rows, without a grid capability.
use super::{Snapshot, display};
use crate::candidate_universe::population_base_evidence_v2::{
    measured_rate, ratio_observed, return_drawdown,
};
use runner::admission::{
    AdmissionEvidenceValuesV1, CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1,
};

#[derive(Default)]
struct Trades {
    gross_win: u64,
    gross_loss: u64,
    min_win: u64,
    max_win: u64,
    max_loss: u64,
    losing_streak: u64,
    winning_streak: u64,
    ambiguous: u64,
}
fn trade_facts<S: Snapshot>(row: &S) -> Result<Trades, String> {
    let mut out = Trades::default();
    let mut streaks = (0_u64, 0_u64);
    for trade in row.trades() {
        if trade.pessimistic_paisa > 0 {
            let win = u64::try_from(trade.pessimistic_paisa).map_err(display)?;
            out.gross_win = out
                .gross_win
                .checked_add(win)
                .ok_or("single-stop gross wins overflow")?;
            out.max_win = out.max_win.max(win);
            out.min_win = if out.min_win == 0 {
                win
            } else {
                out.min_win.min(win)
            };
            streaks.0 = streaks
                .0
                .checked_add(1)
                .ok_or("single-stop winning streak overflow")?;
            streaks.1 = 0;
            out.winning_streak = out.winning_streak.max(streaks.0);
        } else {
            let loss = trade.pessimistic_paisa.unsigned_abs();
            out.gross_loss = out
                .gross_loss
                .checked_add(loss)
                .ok_or("single-stop gross losses overflow")?;
            out.max_loss = out.max_loss.max(loss);
            streaks.0 = 0;
            streaks.1 = streaks
                .1
                .checked_add(1)
                .ok_or("single-stop losing streak overflow")?;
            out.losing_streak = out.losing_streak.max(streaks.1);
        }
        out.ambiguous = out
            .ambiguous
            .checked_add(u64::from(trade.optimistic_paisa != trade.pessimistic_paisa))
            .ok_or("single-stop fill-bound count overflow")?;
    }
    Ok(out)
}
pub(super) fn execution_complete<S: Snapshot>(row: &S) -> bool {
    let m = row.metrics();
    m.unreachable == 0
        && m.entry_refused == 0
        && m.path_refused == 0
        && m.closing_refused == 0
        && row
            .periods()
            .iter()
            .all(|p| p.unavailable_minutes == 0 && p.close_verified && p.refused == 0)
}
fn independent_sessions<S: Snapshot>(row: &S) -> Result<u64, String> {
    let mut previous = None;
    let mut count = 0_u64;
    for event in row.events() {
        let day = event
            .signal_micros
            .checked_add(indicators::IST_OFFSET_MICROS)
            .ok_or("single-stop support day overflow")?
            .div_euclid(86_400_000_000);
        if previous != Some(day) {
            count = count
                .checked_add(1)
                .ok_or("single-stop support sessions overflow")?;
            previous = Some(day);
        }
    }
    Ok(count)
}
fn weakest<S: Snapshot>(row: &S) -> Result<ObservedI64V1, String> {
    if row.trades().is_empty() {
        return Ok(ObservedI64V1::Unmeasured);
    }
    let mut weakest = i64::MAX;
    for grain in crate::stability::GRAINS {
        let mut previous = None;
        let mut total = 0_i64;
        for trade in row.trades() {
            let key = grain.bucket(trade.entry_micros);
            if previous.is_some_and(|p| p != key) {
                weakest = weakest.min(total);
                total = 0;
            }
            total = total
                .checked_add(trade.pessimistic_paisa)
                .ok_or("single-stop weakest period overflow")?;
            previous = Some(key);
        }
        weakest = weakest.min(total);
    }
    Ok(ObservedI64V1::Measured(weakest))
}

pub(super) fn base<S: Snapshot>(row: &S) -> Result<AdmissionEvidenceValuesV1, String> {
    let m = row.metrics();
    let t = trade_facts(row)?;
    let rate = |part, total, name| measured_rate(part, total, name).map_err(display);
    let ratio = |part, total, name| ratio_observed(part, total, name).map_err(display);
    let measured = |n| {
        if m.trades > 0 {
            ObservedU64V1::Measured(n)
        } else {
            ObservedU64V1::Unmeasured
        }
    };
    let losses = m
        .trades
        .checked_sub(m.wins)
        .ok_or("single-stop wins exceed trades")?;
    let calendar = super::assessment(row, false)?;
    Ok(AdmissionEvidenceValuesV1 {
        support_hits: ObservedU64V1::Measured(row.truth().hits),
        independent_sessions: ObservedU64V1::Measured(independent_sessions(row)?),
        trades: ObservedU64V1::Measured(m.trades),
        max_mae_paisa: measured(u64::try_from(m.adverse_paisa).map_err(display)?),
        worst_reward_risk_ppm: ratio(t.min_win, t.max_loss, "native worst reward/risk")?,
        win_rate_ppm: rate(m.wins, m.trades, "native win rate")?,
        wilson_win_rate_ppm: ObservedU64V1::Unmeasured,
        return_drawdown_ppm: if m.trades > 0 {
            return_drawdown(m.pessimistic_paisa, m.drawdown_paisa).map_err(display)?
        } else {
            ObservedU64V1::Unmeasured
        },
        weakest_period_return_paisa: weakest(row)?,
        pbo_ppm: ObservedU64V1::Unmeasured,
        fwer_p_value_ppm: ObservedU64V1::Unmeasured,
        spa_p_value_ppm: ObservedU64V1::Unmeasured,
        decided_folds: ObservedU64V1::Unmeasured,
        ambiguous_fill_rate_ppm: rate(t.ambiguous, m.trades, "native fill bound rate")?,
        gap_affected_rate_ppm: rate(m.stop_gaps, m.trades, "native stop gap rate")?,
        session_concentration_ppm: rate(
            row.periods().iter().map(|p| p.trades).max().unwrap_or(0),
            m.trades,
            "native session concentration",
        )?,
        largest_trade_profit_share_ppm: rate(
            t.max_win,
            t.gross_win,
            "native largest profit share",
        )?,
        execution_complete: if execution_complete(row) {
            CompletenessV1::Complete
        } else {
            CompletenessV1::Refused
        },
        data_complete: CompletenessV1::Complete,
        calendar_complete: if calendar.summary.missing_days > 0 {
            CompletenessV1::Refused
        } else if calendar.summary.unmeasured_days > 0 {
            CompletenessV1::Unmeasured
        } else {
            CompletenessV1::Complete
        },
        population_complete: CompletenessV1::Complete,
        drawdown_paisa: ObservedU64V1::Measured(u64::try_from(m.drawdown_paisa).map_err(display)?),
        worst_trade_loss_paisa: ObservedU64V1::Measured(t.max_loss),
        losing_trade_rate_ppm: rate(losses, m.trades, "native losing trade rate")?,
        losing_trades: ObservedU64V1::Measured(losses),
        pessimistic_profit_paisa: ObservedI64V1::Measured(m.pessimistic_paisa),
        winning_trades: ObservedU64V1::Measured(m.wins),
        average_win_paisa: t
            .gross_win
            .checked_div(m.wins)
            .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
        average_loss_paisa: t
            .gross_loss
            .checked_div(losses)
            .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
        profit_factor_ppm: ratio(t.gross_win, t.gross_loss, "native profit factor")?,
        consecutive_losing_streak: measured(t.losing_streak),
        consecutive_winning_streak: measured(t.winning_streak),
        bootstrap_draws: ObservedU64V1::Unmeasured,
        bootstrap_strategies: ObservedU64V1::Unmeasured,
        bootstrap_periods: ObservedU64V1::Unmeasured,
        pbo_contributing_folds: ObservedU64V1::Unmeasured,
        pbo_unrankable_folds: ObservedU64V1::Unmeasured,
        profitable_oos_folds: ObservedU64V1::Unmeasured,
        oos_pessimistic_return_paisa: ObservedI64V1::Unmeasured,
        white_reality_p_value_ppm: ObservedU64V1::Unmeasured,
        romano_wolf_p_value_ppm: ObservedU64V1::Unmeasured,
        white_reality_decision: HypothesisDecisionV1::Unmeasured,
        romano_wolf_decision: HypothesisDecisionV1::Unmeasured,
        full_precision_statistics_complete: CompletenessV1::Unmeasured,
    })
}

//! Exact saved policy/evidence values; classification remains in the producer.
use cli::boolean_evidence::{
    AdmissionEvidenceValuesV1, AdmissionPolicyV1, AdmissionReasonV1, AdmissionRow,
    AdmissionStatusV1, CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1,
};
use serde_json::{Value, json};
#[cfg(test)]
#[path = "booleanadmission_projection_tests.rs"]
mod tests;

pub(crate) fn row(index: usize, row: &AdmissionRow) -> Result<Value, String> {
    let verdict = row.verdict;
    if !verdict.reconciles() {
        return Err("saved admission verdict partitions do not reconcile".to_owned());
    }
    let checks: Vec<_> = AdmissionReasonV1::ALL
        .into_iter()
        .enumerate()
        .map(|(index, reason)| {
            let state = if verdict.refused().contains(reason) {
                "refused"
            } else if verdict.unmeasured().contains(reason) {
                "unmeasured"
            } else if verdict.failed().contains(reason) {
                "failed"
            } else {
                "passed"
            };
            json!({"index":index.to_string(),"name":reason.name(),"state":state})
        })
        .collect();
    Ok(
        json!({"index":index.to_string(),"identity":crate::server::hex32(row.identity),"source_index":row.source_index.to_string(),"status":match verdict.status(){AdmissionStatusV1::Admitted=>"admitted",AdmissionStatusV1::Rejected=>"rejected",AdmissionStatusV1::Unmeasured=>"unmeasured",AdmissionStatusV1::Refused=>"refused"},"failed":verdict.failed().bits().to_string(),"unmeasured":verdict.unmeasured().bits().to_string(),"refused":verdict.refused().bits().to_string(),"checks":checks,"values":evidence(&row.values)?}),
    )
}
fn unsigned(value: ObservedU64V1) -> Value {
    match value {
        ObservedU64V1::Measured(value) => json!({"state":"measured","value":value.to_string()}),
        ObservedU64V1::Unmeasured => json!({"state":"unmeasured","value":null}),
        ObservedU64V1::Refused => json!({"state":"refused","value":null}),
    }
}
fn signed(value: ObservedI64V1) -> Value {
    match value {
        ObservedI64V1::Measured(value) => json!({"state":"measured","value":value.to_string()}),
        ObservedI64V1::Unmeasured => json!({"state":"unmeasured","value":null}),
        ObservedI64V1::Refused => json!({"state":"refused","value":null}),
    }
}
fn completeness(value: CompletenessV1) -> Value {
    json!({"state":match value {CompletenessV1::Complete=>"complete",CompletenessV1::Incomplete=>"incomplete",CompletenessV1::Unmeasured=>"unmeasured",CompletenessV1::Refused=>"refused"},"value":null})
}
fn decision(value: HypothesisDecisionV1) -> Value {
    json!({"state":match value {HypothesisDecisionV1::RejectedNull=>"rejected-null",HypothesisDecisionV1::DidNotReject=>"did-not-reject",HypothesisDecisionV1::Unmeasured=>"unmeasured",HypothesisDecisionV1::Refused=>"refused"},"value":null})
}
fn evidence(values: &AdmissionEvidenceValuesV1) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(44);
    macro_rules! fields {
        ($render:ident; $($field:ident),+ $(,)?)=>{$({let mut value=$render(values.$field);super::put(&mut value,"name",json!(stringify!($field)))?;rows.push(value);})+};
    }
    fields!(unsigned;
        support_hits,independent_sessions,trades,max_mae_paisa,worst_reward_risk_ppm,
        win_rate_ppm,wilson_win_rate_ppm,return_drawdown_ppm,pbo_ppm,fwer_p_value_ppm,
        spa_p_value_ppm,decided_folds,ambiguous_fill_rate_ppm,gap_affected_rate_ppm,
        session_concentration_ppm,largest_trade_profit_share_ppm,drawdown_paisa,
        worst_trade_loss_paisa,losing_trade_rate_ppm,losing_trades,winning_trades,
        average_win_paisa,average_loss_paisa,profit_factor_ppm,consecutive_losing_streak,
        consecutive_winning_streak,bootstrap_draws,bootstrap_strategies,bootstrap_periods,
        pbo_contributing_folds,pbo_unrankable_folds,profitable_oos_folds,
        white_reality_p_value_ppm,romano_wolf_p_value_ppm);
    fields!(signed;weakest_period_return_paisa,pessimistic_profit_paisa,oos_pessimistic_return_paisa);
    fields!(completeness;execution_complete,data_complete,calendar_complete,population_complete,full_precision_statistics_complete);
    fields!(decision;white_reality_decision,romano_wolf_decision);
    Ok(rows)
}
pub(crate) fn policy(policy: &AdmissionPolicyV1) -> Value {
    let values = policy.values();
    let mut rows = Vec::with_capacity(39);
    macro_rules! fields {
        ($($field:ident),+ $(,)?)=>{$(rows.push(json!({"name":stringify!($field),"value":values.$field.to_string()}));)+};
    }
    fields!(
        min_support_hits,
        min_independent_sessions,
        min_trades,
        max_mae_paisa,
        min_worst_reward_risk_ppm,
        min_win_rate_ppm,
        min_wilson_win_rate_ppm,
        min_return_drawdown_ppm,
        min_weakest_period_return_paisa,
        max_pbo_ppm,
        max_fwer_p_value_ppm,
        max_spa_p_value_ppm,
        min_decided_folds,
        max_ambiguous_fill_rate_ppm,
        max_gap_affected_rate_ppm,
        max_session_concentration_ppm,
        max_largest_trade_profit_share_ppm,
        max_drawdown_paisa,
        max_worst_trade_loss_paisa,
        max_losing_trade_rate_ppm,
        max_losing_trades,
        min_pessimistic_profit_paisa,
        min_winning_trades,
        min_average_win_paisa,
        max_average_loss_paisa,
        min_profit_factor_ppm,
        max_consecutive_losing_streak,
        min_consecutive_winning_streak,
        min_bootstrap_draws,
        min_bootstrap_strategies,
        min_bootstrap_periods,
        min_pbo_contributing_folds,
        max_pbo_unrankable_folds,
        min_profitable_oos_folds,
        min_oos_pessimistic_return_paisa,
        max_white_reality_p_value_ppm,
        max_romano_wolf_p_value_ppm
    );
    for (name, value) in [
        (
            "require_white_reality_rejection",
            values.require_white_reality_rejection,
        ),
        (
            "require_romano_wolf_rejection",
            values.require_romano_wolf_rejection,
        ),
    ] {
        rows.push(json!({"name":name,"value":value}));
    }
    json!({"digest":crate::server::hex32(policy.digest()),"values":rows})
}

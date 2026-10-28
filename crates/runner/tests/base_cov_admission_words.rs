//! BASE-COV-LIBS: every admission field has its own wire name, in the
//! table's order, and no two fields share one.

use std::collections::BTreeSet;

use runner::admission::AdmissionFieldV1;

const NAMES: [&str; 39] = [
    "min_support_hits",
    "min_independent_sessions",
    "min_trades",
    "max_mae_paisa",
    "min_worst_reward_risk_ppm",
    "min_win_rate_ppm",
    "min_wilson_win_rate_ppm",
    "min_return_drawdown_ppm",
    "min_weakest_period_return_paisa",
    "max_pbo_ppm",
    "max_fwer_p_value_ppm",
    "max_spa_p_value_ppm",
    "min_decided_folds",
    "max_ambiguous_fill_rate_ppm",
    "max_gap_affected_rate_ppm",
    "max_session_concentration_ppm",
    "max_largest_trade_profit_share_ppm",
    "max_drawdown_paisa",
    "max_worst_trade_loss_paisa",
    "max_losing_trade_rate_ppm",
    "max_losing_trades",
    "min_pessimistic_profit_paisa",
    "min_winning_trades",
    "min_average_win_paisa",
    "max_average_loss_paisa",
    "min_profit_factor_ppm",
    "max_consecutive_losing_streak",
    "min_consecutive_winning_streak",
    "min_bootstrap_draws",
    "min_bootstrap_strategies",
    "min_bootstrap_periods",
    "min_pbo_contributing_folds",
    "max_pbo_unrankable_folds",
    "min_profitable_oos_folds",
    "min_oos_pessimistic_return_paisa",
    "max_white_reality_p_value_ppm",
    "require_white_reality_rejection",
    "max_romano_wolf_p_value_ppm",
    "require_romano_wolf_rejection",
];

#[test]
fn every_admission_field_has_its_own_wire_name_in_table_order() {
    let said: Vec<&str> = AdmissionFieldV1::ALL
        .iter()
        .map(|field| field.name())
        .collect();
    assert_eq!(said, NAMES);
    let distinct: BTreeSet<&str> = said.iter().copied().collect();
    assert_eq!(distinct.len(), AdmissionFieldV1::ALL.len());
    for name in said {
        assert!(
            name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "{name}"
        );
        assert!(
            name.starts_with("min_") || name.starts_with("max_") || name.starts_with("require_")
        );
    }
}

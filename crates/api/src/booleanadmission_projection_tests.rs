//! Exact field/state projection only; no policy or market authority is minted.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded projection assertions"
)]
use super::*;

fn values() -> AdmissionEvidenceValuesV1 {
    use ObservedI64V1::Unmeasured as I;
    use ObservedU64V1::Unmeasured as U;
    AdmissionEvidenceValuesV1 {
        support_hits: ObservedU64V1::Measured(u64::MAX),
        independent_sessions: U,
        trades: U,
        max_mae_paisa: U,
        worst_reward_risk_ppm: U,
        win_rate_ppm: U,
        wilson_win_rate_ppm: U,
        return_drawdown_ppm: U,
        weakest_period_return_paisa: ObservedI64V1::Measured(i64::MIN),
        pbo_ppm: U,
        fwer_p_value_ppm: U,
        spa_p_value_ppm: U,
        decided_folds: U,
        ambiguous_fill_rate_ppm: U,
        gap_affected_rate_ppm: U,
        session_concentration_ppm: U,
        largest_trade_profit_share_ppm: U,
        execution_complete: CompletenessV1::Complete,
        data_complete: CompletenessV1::Incomplete,
        calendar_complete: CompletenessV1::Unmeasured,
        population_complete: CompletenessV1::Refused,
        drawdown_paisa: U,
        worst_trade_loss_paisa: U,
        losing_trade_rate_ppm: U,
        losing_trades: U,
        pessimistic_profit_paisa: I,
        winning_trades: U,
        average_win_paisa: U,
        average_loss_paisa: U,
        profit_factor_ppm: U,
        consecutive_losing_streak: U,
        consecutive_winning_streak: U,
        bootstrap_draws: U,
        bootstrap_strategies: U,
        bootstrap_periods: U,
        pbo_contributing_folds: U,
        pbo_unrankable_folds: U,
        profitable_oos_folds: U,
        oos_pessimistic_return_paisa: ObservedI64V1::Refused,
        white_reality_p_value_ppm: U,
        romano_wolf_p_value_ppm: ObservedU64V1::Refused,
        white_reality_decision: HypothesisDecisionV1::RejectedNull,
        romano_wolf_decision: HypothesisDecisionV1::DidNotReject,
        full_precision_statistics_complete: CompletenessV1::Complete,
    }
}

#[test]
fn all_44_evidence_fields_preserve_exact_extremes_and_each_unavailable_tag() {
    let rows = evidence(&values()).unwrap();
    assert_eq!(rows.len(), 44);
    let names: std::collections::BTreeSet<_> = rows
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 44);
    let get = |name: &str| rows.iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(get("support_hits")["value"], u64::MAX.to_string());
    assert_eq!(
        get("weakest_period_return_paisa")["value"],
        i64::MIN.to_string()
    );
    for (name, state) in [
        ("execution_complete", "complete"),
        ("data_complete", "incomplete"),
        ("calendar_complete", "unmeasured"),
        ("population_complete", "refused"),
        ("white_reality_decision", "rejected-null"),
        ("romano_wolf_decision", "did-not-reject"),
        ("oos_pessimistic_return_paisa", "refused"),
        ("romano_wolf_p_value_ppm", "refused"),
        ("trades", "unmeasured"),
    ] {
        assert_eq!(get(name)["state"], state);
        assert!(get(name)["value"].is_null());
    }
    assert_eq!(unsigned(ObservedU64V1::Measured(0))["value"], "0");
    assert_eq!(
        signed(ObservedI64V1::Measured(9_007_199_254_740_993))["value"],
        "9007199254740993"
    );
    assert_eq!(
        decision(HypothesisDecisionV1::Unmeasured)["state"],
        "unmeasured"
    );
    assert_eq!(decision(HypothesisDecisionV1::Refused)["state"], "refused");
}

#[test]
fn all_44_reason_positions_render_their_exact_saved_partition_and_status() {
    for (failed, unmeasured, refused, status) in [
        (0_u64, 0_u64, 0_u64, 0_u8),
        (1, 0, 0, 1),
        (1, 2, 0, 2),
        (1, 2, 1_u64 << 43, 3),
    ] {
        let mut bytes = b"BADM\x03\0\x01\0\x21\0\0\0".to_vec();
        for mask in [failed | unmeasured | refused, failed, unmeasured, refused] {
            bytes.extend_from_slice(&mask.to_le_bytes());
        }
        bytes.push(status);
        let verdict =
            cli::boolean_evidence::AdmissionVerdictV1::from_canonical_bytes(&bytes).unwrap();
        let actual = row(
            9,
            &AdmissionRow {
                identity: [0x42; 32],
                source_index: 9,
                values: values(),
                verdict,
            },
        )
        .unwrap();
        assert_eq!(actual["source_index"], "9");
        assert_eq!(actual["checks"].as_array().unwrap().len(), 44);
        for (index, reason) in AdmissionReasonV1::ALL.into_iter().enumerate() {
            let check = &actual["checks"][index];
            let bit = 1_u64 << index;
            assert_eq!(check["name"], reason.name());
            assert_eq!(check["index"], index.to_string());
            assert_eq!(
                check["state"],
                if refused & bit != 0 {
                    "refused"
                } else if unmeasured & bit != 0 {
                    "unmeasured"
                } else if failed & bit != 0 {
                    "failed"
                } else {
                    "passed"
                }
            );
        }
        assert_eq!(
            actual["status"],
            ["admitted", "rejected", "unmeasured", "refused"][usize::from(status)]
        );
    }
}

#[test]
fn all_39_policy_values_preserve_complete_numeric_values_and_false_requirements() {
    // Independent canonical policy fixture, not an approved research profile.
    let mut bytes = b"BADM\x01\0\x01\0\x2a\x01\0\0".to_vec();
    for value in 1_u64..=36 {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.push(0);
    bytes.extend_from_slice(&37_u64.to_le_bytes());
    bytes.push(0);
    let decoded = AdmissionPolicyV1::from_canonical_bytes(&bytes).unwrap();
    let actual = policy(&decoded);
    let rows = actual["values"].as_array().unwrap();
    assert_eq!(rows.len(), 39);
    let names: std::collections::BTreeSet<_> = rows
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 39);
    for (index, row) in rows.iter().take(37).enumerate() {
        assert_eq!(row["value"], (index + 1).to_string());
    }
    assert_eq!(
        rows[37],
        json!({"name":"require_white_reality_rejection","value":false})
    );
    assert_eq!(
        rows[38],
        json!({"name":"require_romano_wolf_rejection","value":false})
    );
    assert_eq!(actual["digest"], crate::server::hex32(decoded.digest()));
}

#[test]
fn search_policy_exposes_effective_limits_without_replacing_the_original() {
    // Canonical generated numbers exercise the JSON boundary only; no saved
    // market qualification or approved research profile is created here.
    let mut bytes = b"BADM\x01\0\x01\0\x2a\x01\0\0".to_vec();
    for value in 1_u64..=36 {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.push(0);
    bytes.extend_from_slice(&37_u64.to_le_bytes());
    bytes.push(0);
    let original = AdmissionPolicyV1::from_canonical_bytes(&bytes).unwrap();
    let effective = original.with_search_probability_ceiling(10);
    let saved = super::super::search_policy(&original);
    let displayed = super::super::search_policy(&effective);
    assert_eq!(saved, policy(&original));
    assert_eq!(saved["values"][10]["value"], "11");
    assert_eq!(saved["values"][11]["value"], "12");
    assert_eq!(saved["values"][35]["value"], "36");
    assert_eq!(saved["values"][36]["value"], "37");
    for index in 0..39 {
        if [10, 11, 35, 36].contains(&index) {
            assert_eq!(displayed["values"][index]["value"], "10");
        } else {
            assert_eq!(displayed["values"][index], saved["values"][index]);
        }
    }
    assert_ne!(saved["digest"], displayed["digest"]);
    assert_eq!(policy(&original), saved);
}

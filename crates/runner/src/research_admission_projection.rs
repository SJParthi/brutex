//! Detached comparison arithmetic for versioned research callers.
//! Caller-supplied numbers never create stored, statistical or selected authority.
//! Only the CLI successor can join these values with its retained typed sources.

use super::{
    AdmissionEvidenceRefusalV1, AdmissionEvidenceV1, AdmissionEvidenceValuesV1,
    AdmissionExactProbabilityV2, AdmissionPolicyV1, AdmissionStatusV1, AdmissionVerdictV1,
    CanonicalWriter, HypothesisDecisionV1, canonical_wilson_projection_v2, hypothesis_decision_v2,
    put_evidence_values, validate_evidence_values_v2,
};

/// Fixed arithmetic projection width, distinct from all legacy evidence versions.
pub const RESEARCH_ADMISSION_PROJECTION_BYTES_V1: usize = 448;

/// Comparison only. There is no constructor or decoder for a durable seal here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResearchAdmissionProjectionV1 {
    policy_digest: [u8; 32],
    values: AdmissionEvidenceValuesV1,
    verdict: AdmissionVerdictV1,
}

impl AdmissionPolicyV1 {
    /// Derive the same policy with all family probability ceilings capped by
    /// one shared search allowance. Every other threshold and requirement is
    /// retained. This only tightens existing validated values; a larger cap
    /// cannot relax the policy or create stored evidence authority.
    #[must_use]
    pub fn with_search_probability_ceiling(mut self, maximum_ppm: u64) -> Self {
        self.values.max_fwer_p_value_ppm = self.values.max_fwer_p_value_ppm.min(maximum_ppm);
        self.values.max_spa_p_value_ppm = self.values.max_spa_p_value_ppm.min(maximum_ppm);
        self.values.max_white_reality_p_value_ppm =
            self.values.max_white_reality_p_value_ppm.min(maximum_ppm);
        self.values.max_romano_wolf_p_value_ppm =
            self.values.max_romano_wolf_p_value_ppm.min(maximum_ppm);
        self
    }

    /// Validate all comparison domains and run the existing complete reason table.
    ///
    /// This does not attest the supplied observations. A storage caller must bind
    /// every measured field to its opaque source and retain missing inputs as
    /// unmeasured/refused. No legacy PBO record is constructed or reinterpreted.
    ///
    /// # Errors
    /// Refuses invalid probability/count domains and contradictory evidence.
    pub fn evaluate_research_projection(
        self,
        values: AdmissionEvidenceValuesV1,
    ) -> Result<ResearchAdmissionProjectionV1, AdmissionEvidenceRefusalV1> {
        validate_evidence_values_v2(&values)?;
        Ok(ResearchAdmissionProjectionV1 {
            policy_digest: self.digest(),
            values,
            verdict: self.evaluate(&AdmissionEvidenceV1 { values }),
        })
    }
}

impl ResearchAdmissionProjectionV1 {
    /// Exact input comparison values, including explicit missing-state tags.
    #[must_use]
    pub const fn values(self) -> AdmissionEvidenceValuesV1 {
        self.values
    }
    /// Full shared verdict and every failed, unmeasured and refused reason bit.
    #[must_use]
    pub const fn verdict(self) -> AdmissionVerdictV1 {
        self.verdict
    }
    /// The complete explicit policy used by the common arithmetic.
    #[must_use]
    pub const fn policy_digest(self) -> [u8; 32] {
        self.policy_digest
    }
    /// New arithmetic-only wire domain; never accepted by any legacy decoder.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; RESEARCH_ADMISSION_PROJECTION_BYTES_V1] {
        let mut bytes = [0; RESEARCH_ADMISSION_PROJECTION_BYTES_V1];
        let mut writer = CanonicalWriter {
            bytes: &mut bytes,
            offset: 0,
        };
        writer.put_slice(b"BRAPRO01");
        writer.put_slice(&self.policy_digest);
        put_evidence_values(&mut writer, &self.values);
        for bits in [
            self.verdict.reasons(),
            self.verdict.failed(),
            self.verdict.unmeasured(),
            self.verdict.refused(),
        ] {
            writer.put_u64(bits.bits());
        }
        writer.put_slice(&[match self.verdict.status() {
            AdmissionStatusV1::Admitted => 0,
            AdmissionStatusV1::Rejected => 1,
            AdmissionStatusV1::Unmeasured => 2,
            AdmissionStatusV1::Refused => 3,
        }]);
        writer.put_slice(&[0; 35]);
        writer.finish();
        bytes
    }
}

/// Recheck full-precision Wilson source bits before comparison-only projection.
#[must_use]
pub fn wilson_ppm(wins: u64, trades: u64, bits: u64) -> Option<u64> {
    if wins > trades {
        return None;
    }
    let (expected, ppm) = canonical_wilson_projection_v2(wins, trades);
    (bits == expected).then_some(ppm)
}

/// Reuse the existing exact finite-resample family decision, never rounded ppm.
#[must_use]
pub fn hypothesis_decision(probability: AdmissionExactProbabilityV2) -> HypothesisDecisionV1 {
    hypothesis_decision_v2(probability)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a fixture policy that does not validate must fail the test loudly"
)]
mod tests {
    use super::super::{AdmissionPolicyDraftV1, AdmissionPolicyV1};

    /// A complete policy whose four family probability ceilings all differ.
    fn policy() -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(100),
            min_independent_sessions: Some(20),
            min_trades: Some(30),
            max_mae_paisa: Some(500),
            min_worst_reward_risk_ppm: Some(2_000_000),
            min_win_rate_ppm: Some(600_000),
            min_wilson_win_rate_ppm: Some(550_000),
            min_return_drawdown_ppm: Some(3_000_000),
            min_weakest_period_return_paisa: Some(0),
            max_pbo_ppm: Some(100_000),
            max_fwer_p_value_ppm: Some(50_000),
            max_spa_p_value_ppm: Some(40_000),
            min_decided_folds: Some(10),
            max_ambiguous_fill_rate_ppm: Some(100_000),
            max_gap_affected_rate_ppm: Some(100_000),
            max_session_concentration_ppm: Some(300_000),
            max_largest_trade_profit_share_ppm: Some(200_000),
            max_drawdown_paisa: Some(10_000),
            max_worst_trade_loss_paisa: Some(2_000),
            max_losing_trade_rate_ppm: Some(400_000),
            max_losing_trades: Some(20),
            min_pessimistic_profit_paisa: Some(1_000),
            min_winning_trades: Some(30),
            min_average_win_paisa: Some(300),
            max_average_loss_paisa: Some(150),
            min_profit_factor_ppm: Some(2_000_000),
            max_consecutive_losing_streak: Some(3),
            min_consecutive_winning_streak: Some(3),
            min_bootstrap_draws: Some(1_000),
            min_bootstrap_strategies: Some(100),
            min_bootstrap_periods: Some(300),
            min_pbo_contributing_folds: Some(10),
            max_pbo_unrankable_folds: Some(2),
            min_profitable_oos_folds: Some(8),
            min_oos_pessimistic_return_paisa: Some(1_000),
            max_white_reality_p_value_ppm: Some(30_000),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(20_000),
            require_romano_wolf_rejection: Some(true),
        })
        .expect("a complete policy validates")
    }

    /// BASE-COV-LIBS-LS2: A SEARCH CEILING ONLY TIGHTENS, each family's
    /// ceiling separately, and touches no other threshold: below every
    /// ceiling, between them, at one exactly, above all, zero and the largest
    /// allowance the type holds.
    #[test]
    fn a_search_ceiling_caps_each_family_probability_and_nothing_else() {
        let original = policy().values();
        for (cap, want) in [
            (0, [0, 0, 0, 0]),
            (10_000, [10_000, 10_000, 10_000, 10_000]),
            (35_000, [35_000, 35_000, 30_000, 20_000]),
            (40_000, [40_000, 40_000, 30_000, 20_000]),
            (50_000, [50_000, 40_000, 30_000, 20_000]),
            (u64::MAX, [50_000, 40_000, 30_000, 20_000]),
        ] {
            let capped = policy().with_search_probability_ceiling(cap).values();
            assert_eq!(
                [
                    capped.max_fwer_p_value_ppm,
                    capped.max_spa_p_value_ppm,
                    capped.max_white_reality_p_value_ppm,
                    capped.max_romano_wolf_p_value_ppm,
                ],
                want,
                "{cap}"
            );
            let mut rest = capped;
            rest.max_fwer_p_value_ppm = original.max_fwer_p_value_ppm;
            rest.max_spa_p_value_ppm = original.max_spa_p_value_ppm;
            rest.max_white_reality_p_value_ppm = original.max_white_reality_p_value_ppm;
            rest.max_romano_wolf_p_value_ppm = original.max_romano_wolf_p_value_ppm;
            assert_eq!(rest, original, "{cap}: no other threshold moves");
        }
        // Capping twice is capping once by the smaller allowance.
        assert_eq!(
            policy()
                .with_search_probability_ceiling(35_000)
                .with_search_probability_ceiling(25_000),
            policy().with_search_probability_ceiling(25_000)
        );
    }
}

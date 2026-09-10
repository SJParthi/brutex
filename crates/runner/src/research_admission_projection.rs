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

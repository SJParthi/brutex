//! Cold observation parser for detached arithmetic projections, never authority.
use super::{
    AdmissionCanonicalRecordV1, AdmissionPolicyV1, CanonicalReader, read_evidence_values,
    research_projection::ResearchAdmissionProjectionV1,
};

/// Decode the fixed research projection and recheck every value and verdict
/// against the supplied complete policy. This produces comparison arithmetic
/// only; callers must authenticate the surrounding saved source receipts.
///
/// # Errors
/// Refuses any wrong width, header, policy, value, status, reason or padding byte.
pub fn decode(
    policy: &AdmissionPolicyV1,
    bytes: &[u8],
) -> Result<ResearchAdmissionProjectionV1, String> {
    let bytes: &[u8; 448] = bytes
        .try_into()
        .map_err(|_| "research projection width or header refused".to_owned())?;
    if bytes.get(..8) != Some(b"BRAPRO01".as_slice()) {
        return Err("research projection width or header refused".to_owned());
    }
    if bytes.get(8..40) != Some(policy.digest().as_slice()) {
        return Err("research projection policy differs".to_owned());
    }
    let mut reader = CanonicalReader {
        bytes,
        offset: 40,
        record: AdmissionCanonicalRecordV1::Evidence,
    };
    let values = read_evidence_values(&mut reader).map_err(|why| format!("{why:?}"))?;
    let projection = policy
        .evaluate_research_projection(values)
        .map_err(|why| format!("research projection values refused: {why:?}"))?;
    if projection.canonical_bytes().as_slice() != bytes.as_slice() {
        return Err("research projection verdict or canonical bytes disagree".to_owned());
    }
    Ok(projection)
}

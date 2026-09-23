//! Canonical representation of a persisted run's source commit.

/// Whether `candidate` is exactly one full lowercase SHA-1 object name.
///
/// Git abbreviations are context-dependent; uppercase and trimmed spellings
/// would give one commit multiple byte identities. Persistence accepts neither.
#[must_use]
pub(crate) fn canonical(candidate: &str) -> bool {
    candidate.len() == 40
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

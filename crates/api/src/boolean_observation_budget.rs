//! One server-owned serialized-evidence budget shared by Boolean detail readers.
use std::ffi::OsStr;

const ENV: &str = "BRUTEX_BOOLEAN_OBSERVATION_BYTES";

/// Admission for one complete saved body tree, not an RSS or latency estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BooleanObservationBudget(u64);

impl BooleanObservationBudget {
    pub(crate) fn load() -> Result<Self, String> {
        Self::from_value(std::env::var_os(ENV).as_deref())
    }

    pub(crate) fn from_value(value: Option<&OsStr>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self(super::MAX_SCAN_BYTES));
        };
        let invalid = || {
            format!(
                "{ENV} must be a canonical positive decimal byte count that fits the addressable detail budget; no default substituted"
            )
        };
        let value = value.to_str().ok_or_else(invalid)?;
        if value.is_empty()
            || value.starts_with('0')
            || value.len() > 20
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(invalid());
        }
        let bytes = value.parse::<u64>().map_err(|_| invalid())?;
        let aggregate = u128::from(bytes) * super::MAX_CONCURRENT as u128;
        if aggregate > isize::MAX as u128 {
            return Err(invalid());
        }
        Ok(Self(bytes))
    }

    pub(crate) const fn bytes(self) -> u64 {
        self.0
    }

    pub(crate) fn context(self, why: &str) -> String {
        format!(
            "{why}; complete serialized evidence admission is {} bytes from {ENV} (default {}); receipt authentication remains required, no body prefix returned",
            self.0,
            super::MAX_SCAN_BYTES
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_defaults_and_exact_positive_runtime_budgets_are_retained() -> Result<(), String> {
        assert_eq!(
            BooleanObservationBudget::from_value(None)?.bytes(),
            67_108_864
        );
        for bytes in [1_u64, 67_108_864, 384 * 1024 * 1024 * 4] {
            let text = bytes.to_string();
            assert_eq!(
                BooleanObservationBudget::from_value(Some(OsStr::new(&text)))?.bytes(),
                bytes
            );
        }
        Ok(())
    }

    /// Extremes (BASE-COV-API-AS2): (1) an empty value; (2) zero, once and
    /// twice; (3) a leading zero; (4) a sign either way; (5) a space either
    /// side; (6) a decimal or an exponent; (7) `u64::MAX` and one past it,
    /// both past what eight concurrent reads may address; (8) past twenty
    /// digits, refused by length before any parse; and accepted: the exact
    /// most the pool may address, one more refused. Combined: (9) a sign AND
    /// a leading zero AND a space at once; (10) twenty-four nines, past the
    /// width and the value. Out of the box: (11) a value that is not UTF-8
    /// (the test below).
    #[test]
    fn malformed_or_unaddressable_values_refuse_without_default() -> Result<(), String> {
        for value in [
            "",
            "0",
            "00",
            "01",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "1e6",
            "18446744073709551615",
            "18446744073709551616",
            // PAST TWENTY DIGITS: refused by length before any parse
            // (BASE-COV-API-AS2).
            "100000000000000000000",
            "999999999999999999999999",
            " +01",
        ] {
            let refused = BooleanObservationBudget::from_value(Some(OsStr::new(value)))
                .map(BooleanObservationBudget::bytes);
            assert!(
                refused.is_err(),
                "invalid budget accepted: {value}: {refused:?}"
            );
            let why = refused.err().unwrap_or_default();
            assert!(why.contains(ENV) && why.contains("no default substituted"));
        }
        let maximum = (isize::MAX as u128 / super::super::MAX_CONCURRENT as u128).to_string();
        assert!(BooleanObservationBudget::from_value(Some(OsStr::new(&maximum))).is_ok());
        let exceeded = (isize::MAX as u128 / super::super::MAX_CONCURRENT as u128 + 1).to_string();
        assert!(BooleanObservationBudget::from_value(Some(OsStr::new(&exceeded))).is_err());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_configuration_and_authentication_failure_are_explicit() -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;
        assert!(BooleanObservationBudget::from_value(Some(OsStr::from_bytes(&[0xff]))).is_err());
        let budget = BooleanObservationBudget::from_value(Some(OsStr::new("123")))?;
        let why = budget.context("receipt seal differs");
        assert!(
            why.contains("receipt seal differs")
                && why.contains("123 bytes")
                && why.contains("no body prefix")
        );
        Ok(())
    }
}

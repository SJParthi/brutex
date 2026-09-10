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
        ] {
            let Err(why) = BooleanObservationBudget::from_value(Some(OsStr::new(value))) else {
                return Err(format!("invalid budget accepted: {value}"));
            };
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

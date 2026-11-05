//! Server-owned admission for grammar replay, independent of untrusted receipts.
use std::ffi::OsStr;
const ENV: &str = "BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES";

#[derive(Clone, Copy)]
pub(crate) struct ReplayBudget(u64);
impl ReplayBudget {
    pub(crate) fn load() -> Result<Self, String> {
        Self::parse(std::env::var_os(ENV).as_deref())
    }
    pub(super) fn parse(value: Option<&OsStr>) -> Result<Self, String> {
        let error = || {
            format!(
                "{ENV} is required as a canonical positive u64 node allowance; no receipt-derived default or request override"
            )
        };
        let value = value.and_then(OsStr::to_str).ok_or_else(error)?;
        if value.is_empty()
            || value.starts_with('0')
            || value.len() > 20
            || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(error());
        }
        Ok(Self(value.parse().map_err(|_| error())?))
    }
    pub(crate) const fn nodes(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Extremes (BASE-COV-API-AS2): (1) no value; (2) an empty one; (3) zero;
    /// (4) a leading zero; (5) a sign either way; (6) a space either side;
    /// (7) a decimal or an exponent; (8) one past `u64::MAX`; (9) past twenty
    /// digits, refused by length before any parse; and accepted: 1 and
    /// `u64::MAX` exactly. Combined: (10) a sign AND a leading zero AND a
    /// space at once; (11) twenty-four nines, past both the width and the
    /// value. Out of the box: (12) a value that is not UTF-8 (the test below).
    #[test]
    fn required_positive_exact_replay_allowance_has_no_fallback() -> Result<(), String> {
        assert!(ReplayBudget::parse(None).is_err());
        for invalid in [
            "",
            "0",
            "01",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "1e6",
            "18446744073709551616",
            // PAST TWENTY DIGITS: refused by length before any parse
            // (BASE-COV-API-AS2).
            "100000000000000000000",
            "999999999999999999999999",
            " +01",
        ] {
            assert!(ReplayBudget::parse(Some(OsStr::new(invalid))).is_err());
        }
        for value in [1, 9_007_199_254_740_993, u64::MAX] {
            assert_eq!(
                ReplayBudget::parse(Some(OsStr::new(&value.to_string())))?.nodes(),
                value
            );
        }
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn non_utf8_replay_configuration_refuses() {
        use std::os::unix::ffi::OsStrExt;
        assert!(ReplayBudget::parse(Some(OsStr::from_bytes(&[0xff]))).is_err());
    }
}

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

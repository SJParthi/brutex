//! Explicit server-side physical limits for checksum-admitted range execution.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

const ROOT: &str = "BRUTEX_CHECKSUM_RECEIPTS";
const BYTES: &str = "BRUTEX_CHECKSUM_MAX_BYTES";
const RECORDS: &str = "BRUTEX_CHECKSUM_MAX_RECORDS";

/// A complete immutable snapshot of explicit server configuration.
/// These are source I/O and allocation ceilings, not financial acceptance rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrictConfig {
    receipt_root: PathBuf,
    max_bytes: u64,
    max_records: u64,
}

/// Names of every absent or unusable physical setting. Values are never included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigRefusal {
    /// Missing or blank setting names, in canonical order.
    pub missing: Vec<&'static str>,
    /// Present but invalid setting names, in canonical order.
    pub invalid: Vec<&'static str>,
}

impl std::fmt::Display for ConfigRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "strict input configuration is unavailable; missing [{}]; invalid [{}]. No ordinary-audit fallback is permitted",
            self.missing.join(", "),
            self.invalid.join(", ")
        )
    }
}

impl std::error::Error for ConfigRefusal {}

impl StrictConfig {
    pub(super) fn explicit(receipt_root: &Path, max_bytes: u64, max_records: u64) -> Self {
        Self {
            receipt_root: receipt_root.to_owned(),
            max_bytes,
            max_records,
        }
    }
    /// Read the three server environment fields exactly once for this command.
    /// No request body, optional engine knob or financial default is consulted.
    ///
    /// # Errors
    /// Names every missing/blank setting or invalid positive integer. The receipt
    /// root must be an absolute existing directory whose final component is not
    /// a symlink. Later retained receipt/source checks still enforce race refusal.
    pub fn from_env() -> Result<Self, ConfigRefusal> {
        Self::from_values(
            std::env::var_os(ROOT),
            std::env::var_os(BYTES),
            std::env::var_os(RECORDS),
        )
    }

    /// Parse one explicit server configuration snapshot without changing files.
    ///
    /// # Errors
    /// The same complete field-name refusal as [`Self::from_env`].
    pub fn from_values(
        root: Option<OsString>,
        bytes: Option<OsString>,
        records: Option<OsString>,
    ) -> Result<Self, ConfigRefusal> {
        let mut refusal = ConfigRefusal {
            missing: Vec::new(),
            invalid: Vec::new(),
        };
        let root_valid = value(root.as_deref(), ROOT, &mut refusal).is_some();
        let receipt_root = root.filter(|_| root_valid).map(PathBuf::from);
        if let Some(path) = receipt_root.as_ref()
            && (!path.is_absolute()
                || !std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir()))
        {
            refusal.invalid.push(ROOT);
        }
        let max_bytes = positive(bytes, BYTES, &mut refusal);
        let max_records = positive(records, RECORDS, &mut refusal);
        match (receipt_root, max_bytes, max_records) {
            (Some(receipt_root), Some(max_bytes), Some(max_records))
                if refusal.invalid.is_empty() =>
            {
                Ok(Self {
                    receipt_root,
                    max_bytes,
                    max_records,
                })
            }
            _ => Err(refusal),
        }
    }

    /// Server-owned receipt directory snapshot.
    #[must_use]
    pub fn receipt_root(&self) -> &Path {
        &self.receipt_root
    }

    /// Maximum complete source bytes audited per physical source file.
    #[must_use]
    pub const fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    /// Maximum summed unique raw source records admitted across this range.
    #[must_use]
    pub const fn max_records(&self) -> u64 {
        self.max_records
    }
}

fn value<'a>(
    raw: Option<&'a OsStr>,
    name: &'static str,
    refusal: &mut ConfigRefusal,
) -> Option<&'a OsStr> {
    match raw {
        None => {
            refusal.missing.push(name);
            None
        }
        Some(raw) if raw.is_empty() || raw.to_str().is_some_and(|text| text.trim().is_empty()) => {
            refusal.missing.push(name);
            None
        }
        Some(raw) => Some(raw),
    }
}

fn positive(raw: Option<OsString>, name: &'static str, refusal: &mut ConfigRefusal) -> Option<u64> {
    value(raw.as_deref(), name, refusal)?;
    let parsed = raw
        .and_then(|raw| raw.into_string().ok())
        .and_then(|text| text.trim().parse::<u64>().ok())
        .filter(|value| *value > 0);
    if parsed.is_none() {
        refusal.invalid.push(name);
    }
    parsed
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test assertions require the exact success or refusal variant"
)]
mod tests {
    use super::*;

    #[test]
    fn missing_physical_configuration_names_every_field_without_defaults() {
        let failure = StrictConfig::from_values(None, Some(" ".into()), None).expect_err("absent");
        assert_eq!(failure.missing, [ROOT, BYTES, RECORDS]);
        assert!(failure.invalid.is_empty());
        assert!(failure.to_string().contains("No ordinary-audit fallback"));
    }

    #[test]
    fn explicit_physical_configuration_preserves_limits_and_refuses_unusable_fields() {
        let root =
            std::env::temp_dir().join(format!("brutex-strict-config-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("private configuration directory");
        let exact = StrictConfig::from_values(
            Some(root.clone().into()),
            Some(" 1 ".into()),
            Some(u64::MAX.to_string().into()),
        )
        .expect("explicit positive limits");
        assert_eq!(exact.receipt_root(), root);
        assert_eq!((exact.max_bytes(), exact.max_records()), (1, u64::MAX));
        for bad in ["0", "-1", "1.5", "18446744073709551616", "unknown"] {
            let failure = StrictConfig::from_values(
                Some(root.clone().into()),
                Some(bad.into()),
                Some(bad.into()),
            )
            .expect_err("unusable");
            assert!(failure.missing.is_empty());
            assert_eq!(failure.invalid, [BYTES, RECORDS]);
        }
        let failure = StrictConfig::from_values(
            Some("relative/path".into()),
            Some("1".into()),
            Some("2".into()),
        )
        .expect_err("relative root");
        assert_eq!(failure.invalid, [ROOT]);
    }

    #[cfg(unix)]
    #[test]
    fn strict_receipt_root_refuses_missing_regular_file_and_final_symlink() -> std::io::Result<()> {
        use std::os::unix::fs::symlink;
        let root =
            std::env::temp_dir().join(format!("brutex-strict-config-root-{}", std::process::id()));
        std::fs::create_dir(&root)?;
        let file = root.join("file");
        let link = root.join("link");
        std::fs::write(&file, b"owned fixture")?;
        symlink(&root, &link)?;
        for path in [root.join("missing"), file, link] {
            let failure =
                StrictConfig::from_values(Some(path.into()), Some("1".into()), Some("1".into()))
                    .expect_err("not an exact existing directory");
            assert_eq!(failure.invalid, [ROOT]);
            assert!(failure.missing.is_empty());
        }
        std::fs::remove_dir_all(root)
    }
}

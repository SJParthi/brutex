//! Prepared browser access to the existing qualified-search command.
//! Preparation resolves scope and configuration without reading historical bars
//! or creating a search journal. Execution keeps the command's original identity.
use super::{Prepared, campaign};
use std::path::{Path, PathBuf};

#[path = "boolean_search_sizing.rs"]
mod sizing;
pub use sizing::{WorkModel, work_model};

/// One acknowledged search observation, independent of invocation completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Exact identity whose declaration is already durably readable.
    pub identity: [u8; 32],
    /// Journal-verified grammar exhaustion; absent during preparation/pricing.
    pub exhausted: Option<bool>,
    /// Acknowledged complete batches, absent before the final read.
    pub completed_batches: Option<u64>,
}

/// Complete server-owned physical limits and resolved research policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    /// Explicit checksum source limits and receipt directory.
    pub strict: crate::audited_range_command::StrictConfig,
    /// Digest of all 39 effective research policy values.
    pub policy_digest: [u8; 32],
    /// Canonical policy for the common saved-evidence projection.
    pub policy: runner::admission::AdmissionPolicyV1,
}

/// A declaration checked before the API acknowledges a worker.
pub struct Admission {
    arguments: Vec<String>,
    fingerprint: [u8; 32],
    root: PathBuf,
    rungs: Vec<&'static str>,
    /// Policy and explicit source limits used for admission.
    pub configuration: Configuration,
}

/// Maximum family list, derived from the canonical research membership table.
#[must_use]
pub const fn max_symbols() -> usize {
    runner::research_family::RESEARCH_FAMILY_CAPACITY_V1
}

/// Resolve the selected research file and every effective gate, without bars.
///
/// # Errors
/// Missing/invalid explicit file, unsupported runtime settings, invalid money
/// bounds or missing physical configuration. No partial-policy default exists.
pub fn configuration(max_points: u64) -> Result<Configuration, String> {
    if max_points == 0 || max_points.checked_mul(100).is_none() {
        return Err("max_points must be positive and fit integer paisa".into());
    }
    if std::env::var_os(crate::research_policy::SETTING).is_none() {
        return Err("BRUTEX_ADMISSION_POLICY_FILE is required for browser Boolean research".into());
    }
    crate::audited_range_command::validate_boolean_runtime().map_err(|why| why.to_string())?;
    let strict =
        crate::audited_range_command::StrictConfig::from_env().map_err(|why| why.to_string())?;
    let (policy, _) = crate::ledger_all::admission_policy(
        &crate::ledger_all::LedgerAllRequest {
            vendor: "",
            from: (1, 1),
            to: (1, 1),
            support_ppm: 0,
            max_points,
            root: Path::new(""),
        },
        "boolean-launch-preflight",
    )?;
    Ok(Configuration {
        strict,
        policy_digest: policy.digest(),
        policy,
    })
}

/// Validate the existing 17-argument grammar and exact research scope without I/O.
///
/// # Errors
/// Any malformed span, alphabet, allowance, feed or unsupported/duplicate family.
pub fn validate(arguments: &[String]) -> Result<(), String> {
    validate_for_rungs(arguments, &crate::ledger_all::LEDGER_RUNGS)
}

/// Validate an explicit intraday selection alongside the original grammar.
///
/// # Errors
/// Refuses every ordinary argument error and empty, duplicate or nonintraday rungs.
pub fn validate_for_rungs(arguments: &[String], rungs: &[&str]) -> Result<(), String> {
    campaign::RungScope::new(rungs)?;
    if arguments.len() != 17
        || arguments
            .iter()
            .try_fold(0_usize, |sum, argument| sum.checked_add(argument.len()))
            .is_none_or(|bytes| bytes > 32_768)
    {
        return Err("Boolean launch arguments exceed their fixed protocol bound".into());
    }
    let borrowed: Vec<_> = arguments.iter().map(String::as_str).collect();
    let request = super::parse(&borrowed)?;
    crate::parse_vendor(request.input.vendor)?;
    let keys = request
        .input
        .symbols
        .split(',')
        .map(crate::stored::swept_index)
        .collect::<Result<Vec<_>, _>>()?;
    runner::research_family::ResearchScopeV1::new(&keys, max_symbols())
        .map_err(|why| why.to_string())?;
    Ok(())
}

/// Admit the request under the server's store and immutable configuration.
///
/// # Errors
/// Scope/parser failures, changed store roots, missing policy or unusable budgets.
/// This creates neither historical results nor a search declaration.
pub fn prepare(arguments: Vec<String>, root: &Path) -> Result<Admission, String> {
    prepare_for_rungs(arguments, root, &crate::ledger_all::LEDGER_RUNGS)
}

/// Admit the exact selected intraday scope without reading bars or saving results.
/// All-eight selection retains the existing declaration and identity bytes.
///
/// # Errors
/// Refuses invalid selection, changed roots, missing policy and unusable budgets.
pub fn prepare_for_rungs(
    arguments: Vec<String>,
    root: &Path,
    rungs: &[&str],
) -> Result<Admission, String> {
    validate_for_rungs(&arguments, rungs)?;
    let rungs = campaign::RungScope::new(rungs)?;
    let borrowed: Vec<_> = arguments.iter().map(String::as_str).collect();
    let request = super::parse(&borrowed)?;
    let configuration = configuration(request.input.max_points)?;
    let canonical = root.canonicalize().map_err(|why| why.to_string())?;
    if crate::store_root()?
        .canonicalize()
        .map_err(|why| why.to_string())?
        != canonical
        || request
            .input
            .output
            .canonicalize()
            .map_err(|why| why.to_string())?
            != canonical
    {
        return Err("Boolean launch input/output root differs from the serving store".into());
    }
    let input = Prepared::new(request.input, &mut String::new())?;
    if input.policy().digest() != configuration.policy_digest
        || input.strict != configuration.strict
        || request.nodes > input.strict.max_records()
    {
        return Err(
            "Boolean launch policy/limits changed or node allowance exceeds admission".into(),
        );
    }
    let fingerprint = fingerprint(&input);
    Ok(Admission {
        arguments,
        fingerprint,
        root: canonical,
        rungs: rungs.labels(),
        configuration,
    })
}

impl Admission {
    /// Canonical selected timeframes; ordering does not create a different request.
    #[must_use]
    pub fn rungs(&self) -> &[&'static str] {
        &self.rungs
    }

    /// Execute this admitted declaration through the original journaled kernel.
    ///
    /// # Errors
    /// Changed configuration/source, unverified build, refused research evidence,
    /// storage failure or an observer that can no longer publish exact status.
    pub fn execute(
        self,
        notify: &mut dyn FnMut(Progress) -> Result<(), String>,
    ) -> Result<String, String> {
        crate::commit_stamp().ok_or("Boolean launch requires a clean build identity")?;
        if crate::store_root()?
            .canonicalize()
            .map_err(|why| why.to_string())?
            != self.root
        {
            return Err("Boolean launch store changed after admission".into());
        }
        let borrowed: Vec<_> = self.arguments.iter().map(String::as_str).collect();
        let mut request = super::parse(&borrowed)?;
        request.rungs = campaign::RungScope::new(&self.rungs)?;
        let mut out = String::new();
        let result = super::execute_observed(
            &request,
            &mut out,
            Prepared::new,
            campaign::prepare,
            Some(self.fingerprint),
            notify,
        );
        match result {
            Ok(()) => Ok(out),
            Err(why) => Err(format!("refused: {why}\n{out}")),
        }
    }
}

pub(super) fn fingerprint(input: &Prepared) -> [u8; 32] {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(b"brutex-boolean-browser-preflight-v1\0");
    hash.update(&input.policy_digest());
    for path in [
        &input.source,
        &input.output,
        &input.strict.receipt_root().to_path_buf(),
    ] {
        let bytes = path.as_os_str().as_encoded_bytes();
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    for policy in input.exit_policy_digests() {
        hash.update(&policy);
    }
    hash.finalize()
}

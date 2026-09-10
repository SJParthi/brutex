//! Typed source-free admission for the Backtest page's audited HTTP worker.
use super::{
    Configuration, Cursor, Limits, Observation, Path, PopulationStatisticsProcedureV2, Progress,
    Request, RungScope, debug, display, execute_observed, month_days, qualification, validate,
};
use std::path::PathBuf;

/// Internal worker operation; launched by Run sweep on the Backtest page.
pub const COMMAND: &str = "index-stop-qualified-search-stored";

/// Fully explicit operator declaration, without filesystem or grid controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchInput {
    /// Canonical stored feed.
    pub feed: String,
    /// Canonical NSE-NIFTY or NSE-BANKNIFTY research index.
    pub index: String,
    /// Exact intraday selection in physical order.
    pub timeframes: Vec<String>,
    /// Original training months.
    pub training: ((u16, u8), (u16, u8)),
    /// Strictly later months, fixed before evaluation.
    pub later: ((u16, u8), (u16, u8)),
    /// `all` or exact ascending live condition IDs.
    pub bits: String,
    /// Full program count per declared batch.
    pub batch_programs: u64,
    /// Syntax-node work allowance per batch.
    pub node_allowance: u64,
    /// Batches this invocation may finish; not an exhaustion claim.
    pub batch_allowance: u64,
    /// Existing institutional worst-trade research limit in index points.
    /// This NEVER determines the execution stop price or a money investment.
    pub max_loss_points: u64,
    /// Exact research policy presented before launch.
    pub expected_policy_digest: [u8; 32],
}
impl LaunchInput {
    /// Validate syntax/scope alone, without configuration, source or filesystem I/O.
    /// # Errors
    /// Invalid index/feed/timeframes/periods/alphabet, digest or positive bounds.
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.index.as_str(), "NSE-NIFTY" | "NSE-BANKNIFTY")
            || self.training.0 > self.training.1
            || self.later.0 > self.later.1
            || self.later.0 <= self.training.1
            || self.batch_programs == 0
            || self.node_allowance == 0
            || self.batch_allowance == 0
            || self.max_loss_points == 0
            || self.max_loss_points.checked_mul(100).is_none()
            || self.expected_policy_digest == [0; 32]
        {
            return Err(
                "single-stop declared scope, spans, bounds or policy fingerprint refused".into(),
            );
        }
        crate::parse_vendor(&self.feed)?;
        month_days(self.training)?;
        month_days(self.later)?;
        alphabet(&self.bits)?;
        rungs(&self.timeframes)?;
        Ok(())
    }
}

/// A bounded launch validated before the HTTP worker acknowledges the request.
pub struct Launch {
    input: LaunchInput,
    root: PathBuf,
    fingerprint: [u8; 32],
}

/// Resolve existing research thresholds and physical admissions without bars.
/// Defaults for resampling retain the existing documented procedure; optional
/// server overrides are read once and bound into the search before execution.
///
/// # Errors
/// Missing/invalid explicit policy or source limits, unusable positive values,
/// arithmetic overflow or program cardinality beyond admission.
pub fn configuration(max_loss_points: u64, batch_programs: u64) -> Result<Configuration, String> {
    let common = crate::boolean_search_launch::configuration(max_loss_points)?;
    let records = common.strict.max_records();
    if batch_programs == 0
        || batch_programs
            .checked_mul(2)
            .is_none_or(|count| count > records)
    {
        return Err("single-stop batch must admit both directions for every program".into());
    }
    let draws = setting(
        "BRUTEX_INDEX_STOP_BOOTSTRAP_DRAWS",
        crate::ledger_all::BOOTSTRAP_DRAWS,
        false,
    )?;
    let seed = setting(
        "BRUTEX_INDEX_STOP_BOOTSTRAP_SEED",
        crate::ledger_all::BOOTSTRAP_SEED,
        true,
    )?;
    let block = setting(
        "BRUTEX_INDEX_STOP_BOOTSTRAP_BLOCK",
        crate::ledger_all::BOOTSTRAP_BLOCK,
        false,
    )?;
    let procedure = PopulationStatisticsProcedureV2::new(draws, seed, block)?;
    let bootstrap_work = records
        .checked_mul(draws)
        .ok_or("single-stop numerical work admission overflow")?;
    let available = std::thread::available_parallelism().map_err(display)?.get();
    let workers = usize::try_from(setting(
        "BRUTEX_INDEX_STOP_WORKERS",
        u64::try_from(available.min(crate::ledger_all::LEDGER_RUNGS.len())).map_err(display)?,
        false,
    )?)
    .map_err(display)?;
    let bytes = common.strict.max_bytes();
    let qualification_bytes = bytes
        .checked_mul(4)
        .ok_or("single-stop source plus qualification observation admission overflow")?;
    let history_bytes = setting(
        "BRUTEX_BOOLEAN_OBSERVATION_BYTES",
        qualification_bytes,
        false,
    )?;
    let replay_nodes = setting(
        "BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES",
        records
            .checked_mul(2)
            .ok_or("single-stop history replay admission overflow")?,
        false,
    )?;
    let qualification = qualification::Bounds {
        candidates: records,
        bootstrap_work: bootstrap_work.min(replay_nodes),
        split_work: bootstrap_work.min(replay_nodes),
        memory_bytes: bytes.min(history_bytes),
        bytes: qualification_bytes,
    };
    Ok(Configuration {
        strict: common.strict,
        policy: common.policy,
        procedure,
        capture: Limits {
            programs: batch_programs,
            records,
            bytes,
        },
        qualification,
        workers: workers.min(available),
        history_bytes,
        source_observation_bytes: crate::index_stop::source_context::observation_budget()?,
        replay_nodes,
    })
}

/// Validate exact scope/configuration before accepting an execution worker.
/// This does not inspect or certify historical data.
///
/// # Errors
/// Changed or absent store/policy, invalid scope/alphabet/window/admission.
pub fn prepare(input: LaunchInput, root: &Path) -> Result<Launch, String> {
    input.validate()?;
    let configuration = configuration(input.max_loss_points, input.batch_programs)?;
    if input.expected_policy_digest != configuration.policy.digest() {
        return Err("The displayed institutional research policy changed; no single-stop worker was accepted".into());
    }
    let root = root.canonicalize().map_err(display)?;
    if crate::store_root()?.canonicalize().map_err(display)? != root {
        return Err("single-stop serving store differs from the configured source store".into());
    }
    let alphabet = alphabet(&input.bits)?;
    let rungs = rungs(&input.timeframes)?;
    validate(&request(&input, &root, &configuration, &alphabet, rungs))?;
    Ok(Launch {
        input,
        root,
        fingerprint: fingerprint(&configuration),
    })
}

impl Launch {
    /// Canonical admitted operator declaration, available for exact worker matching.
    #[must_use]
    pub const fn input(&self) -> &LaunchInput {
        &self.input
    }

    /// Execute the same admitted request through the native journaled kernel.
    /// # Errors
    /// Policy/resource/root changes, clean-build refusal, source failures,
    /// observer/persistence refusal or a bound preventing a complete batch.
    pub fn execute(
        self,
        observe: &mut dyn FnMut(Progress) -> Result<(), String>,
    ) -> Result<String, String> {
        self.execute_observed(&mut |next| match next {
            Observation::Search(progress) => observe(progress),
            Observation::Preparing { .. } => Ok(()),
        })
    }

    /// Execute with bounded source and per-timeframe progress observations.
    /// # Errors
    /// Every execution refusal or lost observer; no later boundary starts after refusal.
    pub fn execute_observed(
        self,
        observe: &mut dyn FnMut(Observation) -> Result<(), String>,
    ) -> Result<String, String> {
        let configuration = configuration(self.input.max_loss_points, self.input.batch_programs)?;
        if fingerprint(&configuration) != self.fingerprint
            || crate::store_root()?.canonicalize().map_err(display)? != self.root
        {
            return Err("single-stop launch policy, physical limits or source store changed after admission".into());
        }
        let alphabet = alphabet(&self.input.bits)?;
        let rungs = rungs(&self.input.timeframes)?;
        execute_observed(
            &request(&self.input, &self.root, &configuration, &alphabet, rungs),
            observe,
        )
    }
}

fn request<'a>(
    input: &'a LaunchInput,
    root: &'a Path,
    configuration: &'a Configuration,
    alphabet: &'a [u32],
    rungs: RungScope,
) -> Request<'a> {
    Request {
        root,
        feed: &input.feed,
        index: &input.index,
        training: input.training,
        later: input.later,
        rungs,
        alphabet,
        batch_programs: input.batch_programs,
        node_allowance: input.node_allowance,
        batch_allowance: input.batch_allowance,
        configuration,
    }
}
fn alphabet(raw: &str) -> Result<Vec<u32>, String> {
    if raw == "all" {
        return Ok(runner::live_positions());
    }
    if raw.is_empty() || raw.len() > 4096 {
        return Err("single-stop alphabet input exceeds protocol bound".into());
    }
    let mut bits = Vec::new();
    for part in raw.split(',') {
        let bit: u32 = part.parse().map_err(display)?;
        if bit.to_string() != part || bits.last().is_some_and(|last| *last >= bit) {
            return Err("single-stop condition IDs must be canonical, unique and ascending".into());
        }
        bits.push(bit);
    }
    Cursor::new(&bits).map_err(debug)?;
    Ok(bits)
}
fn rungs(labels: &[String]) -> Result<RungScope, String> {
    let labels = labels.iter().map(String::as_str).collect::<Vec<_>>();
    let rungs = RungScope::new(&labels)?;
    if rungs.labels() != labels {
        return Err("single-stop timeframes must have canonical physical order".into());
    }
    Ok(rungs)
}
fn fingerprint(config: &Configuration) -> [u8; 32] {
    let mut state = brutex_core::blake3::Hasher::new();
    state.update(b"brutex-index-stop-launch-configuration-v1\0");
    state.update(&config.policy.digest());
    state.update(config.strict.receipt_root().as_os_str().as_encoded_bytes());
    for value in [
        config.strict.max_bytes(),
        config.strict.max_records(),
        config.capture.programs,
        config.capture.records,
        config.capture.bytes,
        config.procedure.draws(),
        config.procedure.seed(),
        config.procedure.block_length(),
        config.qualification.candidates,
        config.qualification.bootstrap_work,
        config.qualification.split_work,
        config.qualification.memory_bytes,
        config.qualification.bytes,
    ] {
        state.update(&value.to_le_bytes());
    }
    state.finalize()
}
fn setting(name: &str, default: u64, allow_zero: bool) -> Result<u64, String> {
    let Some(raw) = std::env::var_os(name) else {
        return Ok(default);
    };
    let raw = raw
        .into_string()
        .map_err(|_| format!("{name} must be UTF-8"))?;
    let value: u64 = raw
        .parse()
        .map_err(|_| format!("{name} must be an exact whole number"))?;
    if value.to_string() != raw || value == 0 && !allow_zero {
        return Err(format!(
            "{name} must be a canonical {}whole number",
            if allow_zero {
                "nonnegative "
            } else {
                "positive "
            }
        ));
    }
    Ok(value)
}

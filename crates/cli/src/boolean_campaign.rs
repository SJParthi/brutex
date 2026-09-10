//! Durable finite-catalog orchestration across the eight existing intraday rungs.
//! Completion records computation stages, never whole-grammar or trading approval.
use crate::boolean_catalog_command::prepared::{self, Stage};
use crate::candidate_universe::boolean_candidate_v1 as candidate;
use crate::search_checkpoint::{Journal, Snapshot};
use brutex_core::blake3::Hasher;
use runner::expression::Expression;
use runner::outcome::Horizon;
use runner::research_family::ResearchFamilyV1;
use std::fmt::Write as _;
use std::path::Path;

#[path = "boolean_campaign_codec.rs"]
mod codec;
#[path = "boolean_campaign_reader.rs"]
mod reader;
pub use reader::Reader;
#[path = "boolean_rung_scope.rs"]
mod rung_scope;
pub use rung_scope::RungScope;

const NAMESPACE: &str = "boolean-campaign-v1";
const MAX_SNAPSHOT: u64 = 2 * 1024 * 1024;
const MAX_CHECKPOINTS: u64 = 1024;
const REASON_BYTES: usize = 1024;

/// Recorded stage state. Running alone is not evidence of current process life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    /// No work start was acknowledged.
    Waiting = 0,
    /// Work start was acknowledged; liveness is observed separately.
    Running = 1,
    /// The explicit per-invocation rung allowance ended.
    Paused = 2,
    /// A named failure prevented completion.
    Refused = 3,
    /// All required saved stage receipts were acknowledged.
    Completed = 4,
    /// Outside the immutable requested timeframe scope; no work was performed.
    Excluded = 5,
}
impl Status {
    /// Stable UI label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Refused => "refused",
            Self::Completed => "completed",
            Self::Excluded => "excluded",
        }
    }
}
/// Exact immutable child identity and completion receipt pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    /// Full stage identity.
    pub identity: [u8; 32],
    /// Full completion-receipt hash.
    pub completion: [u8; 32],
}
/// One canonical family expected in a timeframe's complete catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    /// Exact eligible instrument and membership snapshot.
    pub family: ResearchFamilyV1,
    /// Source/policy/program identity derived before computation.
    pub expected: [u8; 32],
    /// Recorded immutable completion, if acknowledged.
    pub completion: Option<[u8; 32]>,
}
/// One of the fixed eight timeframe rows; no absent timeframe is synthesized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rung {
    /// Canonical intraday label.
    pub rung: &'static str,
    /// Recorded status, separate from process ownership.
    pub status: Status,
    /// Bounded recorded explanation, with explicit truncation when necessary.
    pub reason: String,
    /// Every canonical expected family, including uncompleted children.
    pub catalogs: Vec<Catalog>,
    /// Exact saved complete statistics stage, if acknowledged.
    pub statistics: Option<Link>,
    /// Exact saved complete training admission stage, if acknowledged.
    pub admission: Option<Link>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    rungs: RungScope,
    identity: [u8; 32],
    descriptor: [u8; 32],
    programs: [u8; 32],
    program_count: u64,
    from: (u16, u8),
    to: (u16, u8),
    horizon: u32,
    status: Status,
    previous: Option<(u64, [u8; 32])>,
    rows: Vec<Rung>,
}

/// Typed finite catalog, never a text roundtrip of a fixed-wire grammar program.
pub(crate) struct Request<'a> {
    pub rungs: RungScope,
    pub vendor: &'a str,
    pub symbols: &'a str,
    pub from: (u16, u8),
    pub to: (u16, u8),
    pub programs: &'a [Expression],
    pub horizon: Horizon,
    pub max_points: u64,
    pub output: &'a Path,
    pub max_rung_jobs: usize,
}
/// A resolved immutable policy and all-source preflight for one catalog batch.
pub(crate) struct Prepared {
    input: prepared::Prepared,
    programs: Vec<Expression>,
    fingerprints: Vec<Vec<candidate::Fingerprint>>,
    state: State,
    max_rung_jobs: usize,
    snapshot_bytes: u64,
    ancestry_bytes: u64,
}
impl Prepared {
    pub(crate) const fn rungs(&self) -> RungScope {
        self.state.rungs
    }
    pub(crate) fn expectedcatalog_digest(&self, rung: usize) -> Result<[u8; 32], String> {
        if !self.rungs().contains(rung) {
            return Err("original timeframe is outside the declared selection".into());
        }
        let rows = self
            .fingerprints
            .get(rung)
            .ok_or("planned original rung absent")?;
        for row in rows {
            row.require_current()?;
        }
        Ok(crate::boolean_qualification_plan::ordered_identities(
            rows.iter().map(|row| row.identity),
        ))
    }
    pub(crate) fn expectedsource_digest(&self, rung: usize) -> Result<[u8; 32], String> {
        if !self.rungs().contains(rung) {
            return Err("later timeframe is outside the declared selection".into());
        }
        let rows = self
            .fingerprints
            .get(rung)
            .ok_or("planned later rung absent")?;
        for row in rows {
            row.require_current()?;
        }
        Ok(crate::boolean_qualification_plan::ordered_identities(
            rows.iter().map(|row| row.source),
        ))
    }
    pub(crate) fn require_catalog(
        &self,
        rung: usize,
        family: usize,
        identity: [u8; 32],
    ) -> Result<(), String> {
        let expected = self
            .fingerprints
            .get(rung)
            .and_then(|rows| rows.get(family))
            .ok_or("planned catalog absent")?;
        expected.require_current()?;
        if expected.identity != identity {
            return Err("computed original catalog differs from predeclared scope".into());
        }
        Ok(())
    }
    pub(crate) fn require_later_source(
        &self,
        rung: usize,
        family: usize,
        source: [u8; 32],
    ) -> Result<(), String> {
        let expected = self
            .fingerprints
            .get(rung)
            .and_then(|rows| rows.get(family))
            .ok_or("planned later source absent")?;
        expected.require_current()?;
        if expected.source != source {
            return Err("computed later source differs from predeclared scope".into());
        }
        Ok(())
    }
    pub(crate) const fn span(&self) -> ((u16, u8), (u16, u8)) {
        (self.state.from, self.state.to)
    }
    pub(crate) const fn program_digest(&self) -> [u8; 32] {
        self.state.programs
    }
    pub(crate) const fn program_count(&self) -> u64 {
        self.state.program_count
    }
    pub(crate) const fn horizon(&self) -> u32 {
        self.state.horizon
    }
    pub(crate) fn scope_digest(&self) -> [u8; 32] {
        self.input.scope.digest()
    }
    pub(crate) fn policy(&self) -> &runner::admission::AdmissionPolicyV1 {
        self.input.policy_ref()
    }
    pub(crate) const fn procedure(
        &self,
    ) -> crate::population_statistics_v2::PopulationStatisticsProcedureV2 {
        self.input.procedure()
    }
    pub(crate) fn source_limits(&self) -> (u64, u64) {
        (
            self.input.strict.max_bytes(),
            self.input.strict.max_records(),
        )
    }
    pub(crate) fn preparation_policy_digest(&self) -> [u8; 32] {
        self.input.policy_digest()
    }
    pub(crate) const fn descriptor_digest(&self) -> [u8; 32] {
        self.state.descriptor
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.state.identity
    }
    pub(crate) const fn verification_bytes(&self) -> u64 {
        self.ancestry_bytes
    }
    pub(crate) fn require_current(&self) -> Result<(), String> {
        for fingerprint in self.fingerprints.iter().flatten() {
            fingerprint.require_current()?;
        }
        Ok(())
    }
}
/// Authenticated recorded campaign result; not raw-source or admission authority.
pub(crate) struct CampaignReceipt {
    state: State,
    pin: [u8; 32],
}
impl CampaignReceipt {
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.state.identity
    }
    pub(crate) const fn pin(&self) -> [u8; 32] {
        self.pin
    }
    pub(crate) const fn descriptor_digest(&self) -> [u8; 32] {
        self.state.descriptor
    }
    pub(crate) const fn program_digest(&self) -> [u8; 32] {
        self.state.programs
    }
    pub(crate) fn require_complete(&self) -> Result<(), String> {
        if self.state.status != Status::Completed {
            return Err(format!(
                "Boolean campaign is {}, not complete",
                self.state.status.as_str()
            ));
        }
        Ok(())
    }
}

pub(crate) fn program_digest(programs: &[Expression]) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-boolean-campaign-programs-v1\0");
    hash.update(&(programs.len() as u64).to_le_bytes());
    for program in programs {
        hash.update(&program.encode());
    }
    hash.finalize()
}

pub(crate) fn prepare(request: &Request<'_>, out: &mut String) -> Result<Prepared, String> {
    let commit = crate::commit_stamp().ok_or("Boolean campaign requires a clean build identity")?;
    if !(1..=8).contains(&request.max_rung_jobs) || request.programs.is_empty() {
        return Err("campaign requires a nonempty finite catalog and 1..=8 rung jobs".into());
    }
    let input = prepared::Prepared::new(
        prepared::Input {
            vendor: request.vendor,
            symbols: request.symbols,
            from: request.from,
            to: request.to,
            horizon: request.horizon,
            max_points: request.max_points,
            output: request.output,
        },
        out,
    )?;
    prepare_resolved(request, input, commit)
}

fn prepare_resolved(
    request: &Request<'_>,
    input: prepared::Prepared,
    commit: &str,
) -> Result<Prepared, String> {
    if !(1..=8).contains(&request.max_rung_jobs) || request.programs.is_empty() {
        return Err("campaign requires a nonempty finite catalog and 1..=8 rung jobs".into());
    }
    let programs_bytes = (request.programs.len() as u64)
        .checked_mul(runner::expression::ENCODED_LEN as u64)
        .and_then(|n| n.checked_mul(2))
        .ok_or("campaign program preparation byte budget overflow")?;
    if programs_bytes > input.strict.max_bytes() {
        return Err("campaign complete program buffers exceed byte admission".into());
    }
    let snapshot_bytes = codec::size_for(input.scope.families().len(), request.rungs)?
        .checked_add(96)
        .ok_or("campaign snapshot size overflow")?;
    if snapshot_bytes > input.strict.max_bytes().min(MAX_SNAPSHOT) {
        return Err("complete eight-rung campaign snapshot exceeds byte admission".into());
    }
    let ancestry_bytes = input
        .strict
        .max_bytes()
        .checked_add(112 * (input.scope.families().len() as u64 + 2))
        .and_then(|n| n.checked_mul(3))
        .ok_or("campaign ancestry byte budget overflow")?;
    let mut fingerprints = Vec::new();
    let mut rows = Vec::new();
    let mut descriptor = Hasher::new();
    descriptor.update(b"brutex-boolean-campaign-descriptor-v1\0");
    descriptor.update(&input.policy_digest());
    if request.rungs != RungScope::ALL {
        descriptor.update(b"selected-intraday-rungs-v2\0");
        descriptor.update(&[request.rungs.mask()]);
    }
    for (index, rung) in crate::ledger_all::LEDGER_RUNGS.into_iter().enumerate() {
        let mut found = Vec::new();
        let mut catalogs = Vec::new();
        for family in input
            .scope
            .families()
            .iter()
            .filter(|_| request.rungs.contains(index))
        {
            let key = family.instrument();
            let fingerprint = candidate::fingerprint(
                &input.request(rung, request.programs, key.underlying.as_str()),
                commit,
            )?;
            descriptor.update(&fingerprint.descriptor);
            catalogs.push(Catalog {
                family: *family,
                expected: fingerprint.identity,
                completion: None,
            });
            found.push(fingerprint);
        }
        fingerprints.push(found);
        rows.push(Rung {
            rung,
            status: if request.rungs.contains(index) {
                Status::Waiting
            } else {
                Status::Excluded
            },
            reason: String::new(),
            catalogs,
            statistics: None,
            admission: None,
        });
    }
    let mut state = State {
        rungs: request.rungs,
        identity: [0; 32],
        descriptor: descriptor.finalize(),
        programs: program_digest(request.programs),
        program_count: request.programs.len() as u64,
        from: request.from,
        to: request.to,
        horizon: request.horizon.as_bars(),
        status: Status::Waiting,
        previous: None,
        rows,
    };
    state.identity = codec::identity(&state);
    let prepared = Prepared {
        input,
        programs: request.programs.to_vec(),
        fingerprints,
        state,
        max_rung_jobs: request.max_rung_jobs,
        snapshot_bytes,
        ancestry_bytes,
    };
    prepared.require_current()?;
    Ok(prepared)
}

pub(crate) fn run_prepared(
    prepared: Prepared,
    out: &mut String,
) -> Result<CampaignReceipt, String> {
    let result = run_borrowed(&prepared, out);
    drop(prepared);
    result
}

fn run_borrowed(prepared: &Prepared, out: &mut String) -> Result<CampaignReceipt, String> {
    let _ = writeln!(
        out,
        "\nCampaign dashboard: /backtest?boolean_campaign={}",
        crate::identity_hex(&prepared.identity())
    );
    let mut journal = Journal::open(&prepared.input.output, NAMESPACE, prepared.identity())?;
    let (mut state, mut position) = restore(prepared, &journal)?;
    prepared.require_current()?;
    if state.status == Status::Completed {
        let receipt = CampaignReceipt {
            state,
            pin: position.ok_or("complete campaign has no checkpoint")?.1,
        };
        render_receipt(&receipt, out);
        return Ok(receipt);
    }
    let _ = writeln!(
        out,
        "\nBOOLEAN CAMPAIGN {}: finite catalog across selected intraday timeframes {:?}; per-rung statistics, not whole-grammar multiple-testing evidence.",
        crate::identity_hex(&state.identity),
        state.rungs.labels()
    );
    let mut jobs = 0;
    for index in prepared.rungs().indices() {
        if state
            .rows
            .get(index)
            .is_some_and(|row| row.status == Status::Completed)
        {
            continue;
        }
        if jobs == prepared.max_rung_jobs {
            state.status = Status::Paused;
            row_mut(&mut state, index)?.status = Status::Paused;
            row_mut(&mut state, index)?.reason =
                "Explicit per-invocation timeframe allowance reached; resume the same request"
                    .into();
            publish(
                &mut journal,
                &mut state,
                &mut position,
                prepared.snapshot_bytes,
            )?;
            break;
        }
        jobs += 1;
        state.status = Status::Running;
        let row = row_mut(&mut state, index)?;
        row.status = Status::Running;
        row.reason.clear();
        publish(
            &mut journal,
            &mut state,
            &mut position,
            prepared.snapshot_bytes,
        )?;
        let result = execute_rung(
            prepared,
            index,
            &mut journal,
            &mut state,
            &mut position,
            out,
        );
        if let Err(why) = result {
            state.status = Status::Refused;
            let row = row_mut(&mut state, index)?;
            row.status = Status::Refused;
            row.reason = bounded_reason(&why);
            return match publish(
                &mut journal,
                &mut state,
                &mut position,
                prepared.snapshot_bytes,
            ) {
                Ok(()) => Err(format!(
                    "campaign {} refused: {why}",
                    crate::identity_hex(&state.identity)
                )),
                Err(terminal) => Err(format!(
                    "{why}; campaign refusal checkpoint also failed: {terminal}"
                )),
            };
        }
    }
    let pin = position.ok_or("campaign produced no checkpoint")?.1;
    let receipt = CampaignReceipt { state, pin };
    render_receipt(&receipt, out);
    Ok(receipt)
}

fn render_receipt(receipt: &CampaignReceipt, out: &mut String) {
    let _ = writeln!(
        out,
        "CAMPAIGN {}: {}; snapshot {}. Completed {}timeframes {}/{}.",
        crate::identity_hex(&receipt.identity()),
        receipt.state.status.as_str(),
        crate::identity_hex(&receipt.pin()),
        if receipt.state.rungs == RungScope::ALL {
            ""
        } else {
            "selected "
        },
        receipt
            .state
            .rows
            .iter()
            .filter(|row| row.status == Status::Completed)
            .count(),
        receipt.state.rungs.count()
    );
}

type Position = Option<(u64, [u8; 32])>;

fn restore(prepared: &Prepared, journal: &Journal) -> Result<(State, Position), String> {
    let Some(saved) = journal.latest(prepared.snapshot_bytes)? else {
        return Ok((prepared.state.clone(), None));
    };
    let state = codec::decode(&saved.payload)?;
    codec::same_request(&prepared.state, &state)?;
    verify_history(&saved, |sequence| {
        journal.read(sequence, prepared.snapshot_bytes)
    })?;
    check_saved_rows(&prepared.input.output, &state, prepared.ancestry_bytes)?;
    Ok((state, Some((saved.sequence, saved.seal))))
}

fn execute_rung(
    prepared: &Prepared,
    index: usize,
    journal: &mut Journal,
    state: &mut State,
    position: &mut Option<(u64, [u8; 32])>,
    out: &mut String,
) -> Result<(), String> {
    prepared.require_current()?;
    let rung = row_mut(state, index)?.rung;
    let admission = prepared.input.run(rung, &prepared.programs, out, |stage| {
        prepared.require_current()?;
        record_stage(row_mut(state, index)?, stage)?;
        publish(journal, state, position, prepared.snapshot_bytes)
    })?;
    admission.require_current()?;
    prepared.require_current()?;
    let row = row_mut(state, index)?;
    if row.admission
        != Some(Link {
            identity: admission.identity(),
            completion: admission.completion_digest(),
        })
    {
        return Err("campaign admission receipt was not acknowledged".into());
    }
    row.status = Status::Completed;
    state.status = if state
        .rows
        .iter()
        .all(|row| matches!(row.status, Status::Completed | Status::Excluded))
    {
        Status::Completed
    } else {
        Status::Running
    };
    publish(journal, state, position, prepared.snapshot_bytes)
}

fn record_stage(row: &mut Rung, stage: Stage<'_>) -> Result<(), String> {
    match stage {
        Stage::Families(results) => {
            if results.len() != row.catalogs.len() {
                return Err("campaign family result count differs".into());
            }
            for (expected, result) in row.catalogs.iter_mut().zip(results) {
                if let Ok(family) = result {
                    if expected.family != family.family() || expected.expected != family.identity()
                    {
                        return Err("campaign preflight family/source identity differs from computed catalog".into());
                    }
                    let pin = family.completion_digest();
                    if expected.completion.is_some_and(|old| old != pin) {
                        return Err("campaign retry candidate completion differs".into());
                    }
                    expected.completion = Some(pin);
                }
            }
        }
        Stage::Statistics(statistics) => {
            if statistics.sources().len() != row.catalogs.len() {
                return Err("campaign statistics scope differs".into());
            }
            for (expected, source) in row.catalogs.iter().zip(statistics.sources()) {
                if expected.expected != source.identity()
                    || expected.completion != Some(source.completion_digest())
                {
                    return Err("campaign statistics parent differs from recorded catalogs".into());
                }
            }
            set_link(
                &mut row.statistics,
                Link {
                    identity: statistics.identity(),
                    completion: statistics.completion_digest(),
                },
            )?;
        }
        Stage::Admission(admission) => {
            if row.statistics
                != Some(Link {
                    identity: admission.statistics().identity(),
                    completion: admission.statistics().completion_digest(),
                })
            {
                return Err("campaign admission parent differs from recorded statistics".into());
            }
            set_link(
                &mut row.admission,
                Link {
                    identity: admission.identity(),
                    completion: admission.completion_digest(),
                },
            )?;
        }
    }
    Ok(())
}
fn set_link(slot: &mut Option<Link>, link: Link) -> Result<(), String> {
    if slot.is_some_and(|old| old != link) {
        return Err("campaign retry stage completion differs".into());
    }
    *slot = Some(link);
    Ok(())
}
fn row_mut(state: &mut State, index: usize) -> Result<&mut Rung, String> {
    state
        .rows
        .get_mut(index)
        .ok_or_else(|| "campaign timeframe index differs".into())
}
fn publish(
    journal: &mut Journal,
    state: &mut State,
    position: &mut Option<(u64, [u8; 32])>,
    max_bytes: u64,
) -> Result<(), String> {
    if journal.next_sequence() >= MAX_CHECKPOINTS || journal.interrupted() >= MAX_CHECKPOINTS {
        return Err("campaign checkpoint history ceiling reached; no history was removed".into());
    }
    state.previous = *position;
    let payload = codec::encode(state)?;
    *position = Some(journal.publish(&payload, max_bytes)?);
    Ok(())
}
fn bounded_reason(why: &str) -> String {
    if why.len() <= REASON_BYTES {
        return why.into();
    }
    let mut end = REASON_BYTES - 32;
    while !why.is_char_boundary(end) {
        end -= 1;
    }
    format!("{} [reason truncated to fit]", &why[..end])
}

pub(crate) fn verify_complete(
    root: &Path,
    identity: [u8; 32],
    pin: [u8; 32],
    max_bytes: u64,
) -> Result<CampaignReceipt, String> {
    let reader = Reader::open(root, identity, max_bytes)?;
    if reader.pin() != pin {
        return Err("campaign snapshot pin differs".into());
    }
    let receipt = CampaignReceipt {
        state: reader.state.clone(),
        pin,
    };
    receipt.require_complete()?;
    let snapshot = Snapshot::open(root, NAMESPACE, identity)?.ok_or("campaign disappeared")?;
    let latest = snapshot.read(reader.sequence(), max_bytes.min(MAX_SNAPSHOT))?;
    verify_history(&latest, |sequence| {
        snapshot.read(sequence, max_bytes.min(MAX_SNAPSHOT))
    })?;
    check_saved_rows(root, &receipt.state, max_bytes)?;
    reader.require_current()?;
    Ok(receipt)
}

fn verify_history(
    latest: &crate::search_checkpoint::Saved,
    mut read: impl FnMut(u64) -> Result<crate::search_checkpoint::Saved, String>,
) -> Result<(), String> {
    let mut state = codec::decode(&latest.payload)?;
    let mut sequence = latest.sequence;
    for _ in 0..MAX_CHECKPOINTS {
        let Some((prior_sequence, pin)) = state.previous else {
            return codec::initial(&state);
        };
        if prior_sequence >= sequence {
            return Err("campaign predecessor order differs".into());
        }
        let prior = read(prior_sequence)?;
        if prior.seal != pin {
            return Err("campaign predecessor seal differs".into());
        }
        let previous = codec::decode(&prior.payload)?;
        codec::transition(&previous, &state)?;
        state = previous;
        sequence = prior_sequence;
    }
    Err("campaign predecessor history exceeds bounded work".into())
}
fn check_saved_rows(root: &Path, state: &State, max_bytes: u64) -> Result<(), String> {
    for row in &state.rows {
        if row.status != Status::Completed {
            continue;
        }
        let link = row
            .admission
            .ok_or("completed campaign rung lacks admission")?;
        let saved = crate::boolean_evidence::Admission::open(root, link.identity, max_bytes)?;
        if saved.completion_digest() != link.completion
            || row.statistics
                != Some(Link {
                    identity: saved.statistics().identity(),
                    completion: saved.statistics().completion_digest(),
                })
        {
            return Err("campaign saved admission/statistics link differs".into());
        }
        let sources = saved.statistics().sources();
        if sources.len() != row.catalogs.len() {
            return Err("campaign saved scope differs".into());
        }
        for (index, (expected, source)) in row.catalogs.iter().zip(sources).enumerate() {
            if expected.family != source.family
                || expected.expected != source.identity
                || expected.completion != Some(source.completion)
            {
                return Err("campaign saved candidate source differs".into());
            }
            saved
                .statistics()
                .with_catalog(index, |catalog| verify_catalog_request(catalog, state))?;
        }
        saved.require_current()?;
    }
    Ok(())
}

fn verify_catalog_request(
    catalog: &candidate::reader::Reader,
    state: &State,
) -> Result<(), String> {
    if catalog.programs().len() as u64 != state.program_count
        || program_digest(catalog.programs()) != state.programs
        || catalog
            .grids()
            .iter()
            .any(|grid| grid.horizon_bars != state.horizon)
    {
        return Err("campaign saved catalog programs or horizon differ".into());
    }
    let first =
        pull::session::Day::new(state.from.0, state.from.1, 1).map_err(|why| why.to_string())?;
    let last = pull::session::Day::new(state.to.0, state.to.1, 1)
        .map_err(|why| why.to_string())?
        .end_of_month();
    if catalog.sessions().iter().any(|day| {
        *day < i64::from(first.days_from_epoch()) || *day > i64::from(last.days_from_epoch())
    }) {
        return Err("campaign saved catalog session lies outside declared span".into());
    }
    Ok(())
}

pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    match command_inner(args, out) {
        Ok(receipt) => match receipt.require_complete() {
            Ok(()) => crate::OK,
            Err(why) => crate::refuse(out, &why),
        },
        Err(why) => crate::refuse(out, &why),
    }
}
fn command_inner(args: &[&str], out: &mut String) -> Result<CampaignReceipt, String> {
    let commit = crate::commit_stamp().ok_or("Boolean campaign requires a clean build identity")?;
    let [
        vendor,
        symbols,
        fy,
        fm,
        ty,
        tm,
        catalog,
        horizon,
        points,
        jobs,
        output,
    ] = args
    else {
        return Err("boolean-campaign-stored requires its 11 explicit arguments".into());
    };
    let parse_month = |y: &str, m: &str| -> Result<(u16, u8), String> {
        Ok((
            y.parse().map_err(|_| "invalid year")?,
            m.parse().map_err(|_| "invalid month")?,
        ))
    };
    let horizon =
        crate::knobs::horizon_count(horizon).ok_or("positive one-minute HORIZON required")?;
    // Resolve policy and all physical limits before opening the text catalog.
    let common = prepared::Prepared::new(
        prepared::Input {
            vendor,
            symbols,
            from: parse_month(fy, fm)?,
            to: parse_month(ty, tm)?,
            horizon,
            max_points: points.parse().map_err(|_| "invalid MAX_POINTS")?,
            output: Path::new(output),
        },
        out,
    )?;
    let programs =
        crate::boolean_catalog_command::catalog(Path::new(catalog), common.strict.max_bytes())?;
    let request = Request {
        rungs: RungScope::ALL,
        vendor,
        symbols,
        from: common.from,
        to: common.to,
        programs: &programs,
        horizon,
        max_points: points.parse().map_err(|_| "invalid MAX_POINTS")?,
        output: Path::new(output),
        max_rung_jobs: jobs.parse().map_err(|_| "invalid MAX_RUNG_JOBS")?,
    };
    run_prepared(prepare_resolved(&request, common, commit)?, out)
}

#[cfg(test)]
#[path = "boolean_campaign_tests.rs"]
pub(crate) mod tests;

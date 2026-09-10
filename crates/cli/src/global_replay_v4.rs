//! Terminal-aware global replay from eight retained Selection V6 authorities.
//! Actual zero-through-25 prefixes replace V3's mandatory 200-winner topology.
//! V1/V2/V3 files are never opened. No loose caller-authored replay row is an
//! authority. The shared Runner scheduler owns inclusive global occupancy.
//! Whole replay, sorting, authentication and persistence are input-dependent.
use std::collections::HashMap;
use std::path::Path;

use brutex_core::vendor::Vendor;
use runner::grid::{ReplayCandidateV1, ReplayPathV1};
use runner::portfolio::{
    Constituent, Counters, Disposition, Evidence, GlobalSinglePositionV1, Intent,
};

use crate::all_rung_selection_v6::AllRungSelectionV6;
use crate::selection_v6::{SelectionV6Snapshot, SelectionV6Winner};
use crate::stored_post_training_oos::StoredPostTrainingOosRequestV1;
use crate::vix_reference::{VixReferenceMonth, VixStamp};

#[path = "global_replay_v4_codec.rs"]
mod codec;
#[path = "global_replay_v4_lifecycle.rs"]
mod lifecycle;
#[path = "global_replay_v4_store.rs"]
mod storage;

use codec::{Record, Writer};

pub(crate) const GLOBAL_REPLAY_V4_RECORD_BYTES: u64 = codec::STRIDE as u64;

/// Explicit physical bounds. These never become admission thresholds.
#[derive(Clone, Copy)]
pub(crate) struct GlobalReplayV4Bounds {
    records: u64,
    bytes: u64,
}
impl GlobalReplayV4Bounds {
    pub(crate) fn new(records: u64, bytes: u64) -> Result<Self, String> {
        if records < 10
            || records
                .checked_mul(codec::STRIDE as u64)
                .is_none_or(|need| need > bytes)
        {
            return Err(
                "Global Replay V4 requires explicit capacity for its complete fixed records"
                    .to_owned(),
            );
        }
        Ok(Self { records, bytes })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GlobalReplayV4Audit {
    pub(crate) replay_id: [u8; 32],
    pub(crate) publication_id: [u8; 32],
    pub(crate) witnesses: u64,
    pub(crate) candidates: u64,
    pub(crate) money_rows: u64,
    pub(crate) pricing_refused: u64,
    pub(crate) admitted_pricing_refused: u64,
    pub(crate) counters: Counters,
    pub(crate) pessimistic_paisa: i128,
    pub(crate) optimistic_paisa: i128,
}

/// A freshly compared complete publication; no arbitrary path creates one.
pub(crate) struct CommittedStoredGlobalReplayV4 {
    path: std::path::PathBuf,
    digest: [u8; 32],
    records: u64,
    bounds: GlobalReplayV4Bounds,
    audit: GlobalReplayV4Audit,
    written: bool,
    strict_inputs: crate::step3_orchestrator::strict::Guards,
}
impl CommittedStoredGlobalReplayV4 {
    pub(crate) const fn was_written(&self) -> bool {
        self.written
    }
    pub(crate) fn audit(&self) -> Result<GlobalReplayV4Audit, String> {
        self.strict_inputs.require_current()?;
        storage::verify(&self.path, self.bounds, self.records, self.digest)?;
        self.strict_inputs.require_current()?;
        Ok(self.audit)
    }
}

struct Prepared {
    records: Vec<Record>,
    audit: GlobalReplayV4Audit,
    strict_inputs: crate::step3_orchestrator::strict::Guards,
}

#[derive(Clone, Copy)]
struct Attempt {
    stream: u64,
    ordinal: u64,
    constituent: Constituent,
    feed: Vendor,
    candidate: CandidateProjection,
}

/// Private lossless scheduling projection, minted only from an integrity-checked
/// opaque Runner witness on the production path.
#[derive(Clone, Copy)]
struct CandidateProjection {
    signal_bar: usize,
    entry_bar: usize,
    occupied_through_bar: usize,
    signal_micros: i64,
    entry_micros: i64,
    occupied_through_micros: i64,
    path: PathProjection,
}
#[derive(Clone, Copy)]
enum PathProjection {
    Priceable(PriceProjection),
    BlockOnly,
    CrossingRefused,
    BlockOnlyAndCrossingRefused,
}
#[derive(Clone, Copy)]
struct PriceProjection {
    row: runner::grid::TradeRow,
    ambiguous_bars: u64,
    gap_fills: u64,
}
impl PriceProjection {
    const fn row(self) -> runner::grid::TradeRow {
        self.row
    }
    const fn ambiguous_bars(self) -> u64 {
        self.ambiguous_bars
    }
    const fn gap_fills(self) -> u64 {
        self.gap_fills
    }
}
impl CandidateProjection {
    fn from_runner(candidate: ReplayCandidateV1) -> Self {
        Self {
            signal_bar: candidate.signal_bar(),
            entry_bar: candidate.entry_bar(),
            occupied_through_bar: candidate.occupied_through_bar(),
            signal_micros: candidate.signal_micros(),
            entry_micros: candidate.entry_micros(),
            occupied_through_micros: candidate.occupied_through_micros(),
            path: match candidate.path() {
                ReplayPathV1::Priceable(price) => PathProjection::Priceable(PriceProjection {
                    row: price.row(),
                    ambiguous_bars: price.ambiguous_bars(),
                    gap_fills: price.gap_fills(),
                }),
                ReplayPathV1::BlockOnly => PathProjection::BlockOnly,
                ReplayPathV1::CrossingRefused => PathProjection::CrossingRefused,
                ReplayPathV1::BlockOnlyAndCrossingRefused => {
                    PathProjection::BlockOnlyAndCrossingRefused
                }
            },
        }
    }
    const fn signal_bar(self) -> usize {
        self.signal_bar
    }
    const fn entry_bar(self) -> usize {
        self.entry_bar
    }
    const fn occupied_through_bar(self) -> usize {
        self.occupied_through_bar
    }
    const fn signal_micros(self) -> i64 {
        self.signal_micros
    }
    const fn entry_micros(self) -> i64 {
        self.entry_micros
    }
    const fn occupied_through_micros(self) -> i64 {
        self.occupied_through_micros
    }
    const fn path(self) -> PathProjection {
        self.path
    }
    const fn pricing_refused(self) -> bool {
        !matches!(self.path, PathProjection::Priceable(_))
    }
}

/// Requires a real later stored period; the caller cannot invent a replay feed,
/// instrument, direction, selected exit, mask or trade. VIX loading happens only
/// after the global scheduler admits a priceable trade, and never feeds back
/// into the economic replay identity or decision.
pub(crate) fn commit_stored_global_replay_v4(
    root: &Path,
    bounds: GlobalReplayV4Bounds,
    mut selection: AllRungSelectionV6,
    request: StoredPostTrainingOosRequestV1,
    store_root: &Path,
) -> Result<CommittedStoredGlobalReplayV4, String> {
    let snapshots = selection.snapshot()?;
    let plan = lifecycle::plan_identity(&snapshots, request, bounds)?;
    lifecycle::recorded(root, plan, |observer| {
        commit_recorded(
            root,
            bounds,
            &mut selection,
            &snapshots,
            request,
            store_root,
            observer,
        )
    })
}

fn commit_recorded(
    root: &Path,
    bounds: GlobalReplayV4Bounds,
    selection: &mut AllRungSelectionV6,
    snapshots: &[SelectionV6Snapshot; 8],
    request: StoredPostTrainingOosRequestV1,
    store_root: &Path,
    observer: &mut crate::stored_post_training_oos::StoredOosObserverV1<'_>,
) -> Result<CommittedStoredGlobalReplayV4, String> {
    let selected_count = snapshots.iter().try_fold(0_u64, |sum, snapshot| {
        sum.checked_add(codec::count(snapshot.winners.len())?)
            .ok_or_else(|| "Global Replay V4 selected count overflow".to_owned())
    })?;
    let candidate_budget =
        bounds.records.checked_sub(10 + selected_count).ok_or(
            "Global Replay V4 record cap cannot hold its exact roster and selected streams",
        )? / 2;
    let witnesses = selection.replay(snapshots, request, candidate_budget, observer)?;
    let prepared = prepare(snapshots, witnesses, request, bounds, store_root)?;
    prepared.strict_inputs.require_current()?;
    if &selection.snapshot()? != snapshots {
        return Err("Global Replay V4 selection changed before publication".to_owned());
    }
    let (path, written, digest) = storage::persist(
        root,
        bounds,
        &prepared.records,
        prepared.audit.publication_id,
    )?;
    prepared.strict_inputs.require_current()?;
    if &selection.snapshot()? != snapshots {
        return Err(
            "Global Replay V4 selection changed during publication; no authority returned"
                .to_owned(),
        );
    }
    let committed = CommittedStoredGlobalReplayV4 {
        path,
        digest,
        records: codec::count(prepared.records.len())?,
        bounds,
        audit: prepared.audit,
        written,
        strict_inputs: prepared.strict_inputs,
    };
    committed.audit()?;
    Ok(committed)
}

fn prepare(
    snapshots: &[SelectionV6Snapshot; 8],
    witnesses: Vec<crate::stored_post_training_oos::StoredPostTrainingOosWitnessV1>,
    request: StoredPostTrainingOosRequestV1,
    bounds: GlobalReplayV4Bounds,
    store_root: &Path,
) -> Result<Prepared, String> {
    let strict_inputs = crate::step3_orchestrator::strict::Guards::from_sources(
        witnesses
            .iter()
            .map(crate::stored_post_training_oos::StoredPostTrainingOosWitnessV1::strict_inputs),
    );
    strict_inputs.require_current()?;
    let mut records = Vec::new();
    push(&mut records, &codec::header(request)?, bounds)?;
    for (rung, snapshot) in snapshots.iter().enumerate() {
        let mut row = Writer::new(1, codec::count(rung)?);
        row.bytes(&snapshot.envelope)?;
        push(&mut records, &row.finish()?, bounds)?;
    }
    let expected: usize = snapshots.iter().map(|row| row.winners.len()).sum();
    if witnesses.len() != expected || expected > 200 {
        return Err(
            "Global Replay V4 witness count differs from actual eight-rung prefixes".to_owned(),
        );
    }
    let mut attempts = Vec::new();
    let mut witnesses = witnesses.into_iter();
    let mut stream = 0_u64;
    for snapshot in snapshots {
        for winner in &snapshot.winners {
            let held = witnesses
                .next()
                .ok_or("Global Replay V4 missing stored witness")?;
            let (cohort, stored_witness, replay) = held.into_parts();
            replay.require_integrity().map_err(|why| why.to_string())?;
            let constituent = exact_constituent(snapshot, winner, &replay)?;
            push(
                &mut records,
                &codec::witness(stream, snapshot, winner, cohort, stored_witness, &replay)?,
                bounds,
            )?;
            let mut previous = None;
            for (ordinal, candidate) in replay.candidates().iter().copied().enumerate() {
                check_candidate(&candidate, replay.first_oos(), previous)?;
                previous = Some((candidate.entry_bar(), candidate.entry_micros()));
                let attempt = Attempt {
                    stream,
                    ordinal: codec::count(ordinal)?,
                    constituent,
                    feed: replay.feed(),
                    candidate: CandidateProjection::from_runner(candidate),
                };
                push(&mut records, &codec::candidate(&attempt)?, bounds)?;
                attempts.try_reserve(1).map_err(|why| why.to_string())?;
                attempts.push(attempt);
            }
            stream += 1;
        }
    }
    let mut vix = VixCatalog {
        root: store_root,
        months: HashMap::new(),
    };
    let mut audit = schedule(&attempts, &mut records, bounds, &mut vix)?;
    audit.witnesses = stream;
    audit.candidates = codec::count(attempts.len())?;
    // Reference-only VIX rows do not enter economic identity.
    audit.replay_id = codec::digest_records(
        b"brutex-global-replay-v4-economic\0",
        records.iter().filter(|row| codec::kind(row) != 5),
    );
    audit.publication_id =
        codec::digest_records(b"brutex-global-replay-v4-publication\0", records.iter());
    let completion = codec::completion(&audit, codec::count(records.len())?)?;
    push(&mut records, &completion, bounds)?;
    strict_inputs.require_current()?;
    Ok(Prepared {
        records,
        audit,
        strict_inputs,
    })
}

fn exact_constituent(
    snapshot: &SelectionV6Snapshot,
    winner: &SelectionV6Winner,
    replay: &runner::exit_grid_policy::GlobalReplayWitnessUniverseV1,
) -> Result<Constituent, String> {
    let instrument = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        winner.family,
    )
    .map_err(|why| why.to_string())?;
    if replay.selected_exit_digest() != winner.selected_exit_digest
        || replay.instrument() != instrument
        || replay.direction() != winner.ranked.candidate.direction
        || !winner.ranked.candidate.admitted
    {
        return Err(
            "Global Replay V4 stored witness differs from exact selected authority".to_owned(),
        );
    }
    Ok(Constituent {
        priority: u16::try_from(winner.rank + 1).map_err(|why| why.to_string())?,
        strategy_digest: winner.ranked.candidate.strategy_digest,
        instrument,
        direction: replay.direction(),
        rung_minutes: u16::try_from(snapshot.rung_seconds / 60).map_err(|why| why.to_string())?,
    })
}

fn check_candidate(
    candidate: &ReplayCandidateV1,
    first_oos: usize,
    previous: Option<(usize, i64)>,
) -> Result<(), String> {
    if candidate.signal_bar() < first_oos
        // The authenticated aligned column stores the already-delayed fill
        // coordinate as its signal coordinate. Equality is therefore valid;
        // Runner's alignment supplies the causal delay before this boundary.
        || candidate.entry_bar() < candidate.signal_bar()
        || candidate.occupied_through_bar() < candidate.entry_bar()
        || candidate.entry_micros() < candidate.signal_micros()
        || candidate.occupied_through_micros() < candidate.entry_micros()
        || previous
            .is_some_and(|(bar, ts)| candidate.entry_bar() <= bar || candidate.entry_micros() <= ts)
    {
        return Err(format!(
            "Global Replay V4 candidate order, causal boundary or occupancy is invalid: signal_bar={}, entry_bar={}, through_bar={}, signal_micros={}, entry_micros={}, through_micros={}, first_oos={first_oos}, previous={previous:?}",
            candidate.signal_bar(),
            candidate.entry_bar(),
            candidate.occupied_through_bar(),
            candidate.signal_micros(),
            candidate.entry_micros(),
            candidate.occupied_through_micros()
        ));
    }
    for ts in [
        candidate.signal_micros(),
        candidate.entry_micros(),
        candidate.occupied_through_micros(),
    ] {
        if ts.rem_euclid(60_000_000) != 0 {
            return Err("Global Replay V4 candidate timestamp is not an exact minute".to_owned());
        }
    }
    if let ReplayPathV1::Priceable(price) = candidate.path() {
        let row = price.row();
        if row.signal_bar != candidate.signal_bar()
            || row.entry_bar != candidate.entry_bar()
            || row.exit_bar != candidate.occupied_through_bar()
            || row.exit_bar < row.entry_bar
            || row.entry_micros != candidate.entry_micros()
            || row.exit_micros != candidate.occupied_through_micros()
            || row.exit_micros < row.entry_micros
            || row.worst > row.best
            || row.adverse < 0
            || row.adverse_paisa < 0
            || row.favourable < 0
            || row.favourable_paisa < 0
            || price.ambiguous_bars() > 1
            || price.gap_fills() > 1
        {
            return Err(
                "Global Replay V4 priceable row escapes its exact occupied interval".to_owned(),
            );
        }
    }
    Ok(())
}

fn push(
    records: &mut Vec<Record>,
    row: &Record,
    bounds: GlobalReplayV4Bounds,
) -> Result<(), String> {
    if codec::count(records.len())? >= bounds.records {
        return Err("Global Replay V4 record ceiling reached; no complete publication".to_owned());
    }
    records.try_reserve(1).map_err(|why| why.to_string())?;
    records.push(*row);
    Ok(())
}

trait VixLookup {
    fn stamp(&mut self, feed: Vendor, ts: i64) -> Result<VixStamp, String>;
}
struct VixCatalog<'a> {
    root: &'a Path,
    months: HashMap<(Vendor, store::path::YearMonth), VixReferenceMonth>,
}
impl VixLookup for VixCatalog<'_> {
    fn stamp(&mut self, feed: Vendor, ts: i64) -> Result<VixStamp, String> {
        let moment = pull::session::IstMoment::from_epoch_secs(ts.div_euclid(1_000_000))
            .map_err(|why| why.to_string())?;
        let month = moment.day().year_month().map_err(|why| why.to_string())?;
        if !self.months.contains_key(&(feed, month)) {
            self.months.try_reserve(1).map_err(|why| why.to_string())?;
            self.months.insert(
                (feed, month),
                VixReferenceMonth::open(self.root, feed, month)?,
            );
        }
        self.months
            .get(&(feed, month))
            .ok_or("Global Replay V4 VIX month disappeared")?
            .stamp(ts)
    }
}

fn schedule(
    attempts: &[Attempt],
    records: &mut Vec<Record>,
    bounds: GlobalReplayV4Bounds,
    vix: &mut impl VixLookup,
) -> Result<GlobalReplayV4Audit, String> {
    let mut order: Vec<_> = attempts.iter().collect();
    order.sort_unstable_by_key(|attempt| {
        (
            attempt.candidate.entry_micros(),
            attempt.constituent.priority,
            attempt.constituent.strategy_digest,
        )
    });
    let mut scheduler = GlobalSinglePositionV1::new();
    let mut cursor = 0;
    let mut audit = GlobalReplayV4Audit {
        replay_id: [0; 32],
        publication_id: [0; 32],
        witnesses: 0,
        candidates: 0,
        money_rows: 0,
        pricing_refused: 0,
        admitted_pricing_refused: 0,
        counters: Counters::default(),
        pessimistic_paisa: 0,
        optimistic_paisa: 0,
    };
    while cursor < order.len() {
        let ts = order
            .get(cursor)
            .ok_or("Global Replay V4 cursor")?
            .candidate
            .entry_micros();
        let mut end = cursor + 1;
        while order
            .get(end)
            .is_some_and(|attempt| attempt.candidate.entry_micros() == ts)
        {
            end += 1;
        }
        let group = order
            .get(cursor..end)
            .ok_or("Global Replay V4 minute group")?;
        let minute = schedule_group(group, ts, &mut scheduler)?;
        for decision in minute.decisions() {
            let attempt = group
                .iter()
                .find(|attempt| attempt.constituent == decision.constituent)
                .ok_or("Global Replay V4 scheduler returned an unoffered candidate")?;
            account_decision(
                attempt,
                decision.disposition,
                &mut audit,
                records,
                bounds,
                vix,
            )?;
        }
        cursor = end;
    }
    audit.counters = scheduler.counters();
    if !audit.counters.reconciles()
        || audit.counters.offered != codec::count(attempts.len())?
        || audit.money_rows.checked_add(audit.admitted_pricing_refused)
            != Some(audit.counters.admitted)
    {
        return Err(
            "Global Replay V4 complete candidate/decision/money counts do not reconcile".to_owned(),
        );
    }
    Ok(audit)
}

fn schedule_group(
    group: &[&Attempt],
    ts: i64,
    scheduler: &mut GlobalSinglePositionV1,
) -> Result<runner::portfolio::MinuteSchedule, String> {
    if group.len() > runner::portfolio::MAX_INTENTS_PER_MINUTE {
        return Err("Global Replay V4 minute exceeds actual 200-stream bound".to_owned());
    }
    let intents: Vec<_> = group
        .iter()
        .map(|attempt| Intent {
            constituent: attempt.constituent,
            evidence: Evidence::Reachable {
                occupied_through_micros: attempt.candidate.occupied_through_micros(),
            },
        })
        .collect();
    scheduler
        .schedule_minute(ts, &intents)
        .map_err(|why| format!("Global Replay V4 scheduler refused: {why:?}"))
}

fn account_decision(
    attempt: &Attempt,
    disposition: Disposition,
    audit: &mut GlobalReplayV4Audit,
    records: &mut Vec<Record>,
    bounds: GlobalReplayV4Bounds,
    vix: &mut impl VixLookup,
) -> Result<(), String> {
    let sequence = codec::count(records.len())?;
    push(
        records,
        &codec::decision(sequence, attempt, disposition)?,
        bounds,
    )?;
    audit.pricing_refused += u64::from(attempt.candidate.pricing_refused());
    if matches!(disposition, Disposition::Admitted { .. }) {
        if let PathProjection::Priceable(price) = attempt.candidate.path() {
            let row = price.row();
            let entry = vix.stamp(attempt.feed, row.entry_micros)?;
            let exit = vix.stamp(attempt.feed, row.exit_micros)?;
            push(records, &codec::vix(sequence, entry, exit)?, bounds)?;
            audit.money_rows += 1;
            audit.pessimistic_paisa = audit
                .pessimistic_paisa
                .checked_add(i128::from(row.worst))
                .ok_or("Global Replay V4 money overflow")?;
            audit.optimistic_paisa = audit
                .optimistic_paisa
                .checked_add(i128::from(row.best))
                .ok_or("Global Replay V4 money overflow")?;
        } else {
            audit.admitted_pricing_refused += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "global_replay_v4_tests.rs"]
mod tests;

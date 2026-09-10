//! Cold saved-statistics projections. Source links authenticate historical
//! candidate evidence; these bytes cannot re-attest current raw market files.
#[cfg(test)]
#[path = "boolean_evidence_reader_tests.rs"]
mod tests;
use super::super::{persistence::Observation, reader::Reader as Catalog};
use super::{Bounds, CandidateStatisticsV1, PopulationStatisticsProcedureV2, admit_shape, display};
use brutex_core::blake3::Hasher;
use runner::research_family::{ResearchFamilyV1, ResearchScopeV1};
use std::path::Path;

/// Exact source catalog and completion, in canonical family order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatisticsSource {
    /// Full instrument and membership-snapshot capability label.
    pub family: ResearchFamilyV1,
    /// Referenced candidate catalog identity.
    pub identity: [u8; 32],
    /// Required immutable candidate completion.
    pub completion: [u8; 32],
    /// Complete coordinate count, including zero/refused rows.
    pub coordinates: usize,
}

/// Exact saved coordinate statistics; numeric bit patterns are unrounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatisticsRow {
    /// Index into the complete canonical source list.
    pub source: usize,
    /// Coordinate index inside that source catalog.
    pub coordinate: usize,
    /// Complete candidate identity.
    pub identity: [u8; 32],
    /// Hash of all ordered session observations.
    pub period_digest: [u8; 32],
    /// Directional expression execution identity.
    pub run: [u8; 32],
    /// Index in the source's saved program catalog.
    pub program_index: usize,
    /// Canonical label, long or short.
    pub side: &'static str,
    /// Complete resolved-grid ordinal.
    pub ordinal: u64,
    /// Exact execution refusal mask, zero does not mean admission.
    pub execution_refusal_bits: u64,
    /// Actual trade count.
    pub trades: u64,
    /// Strictly profitable pessimistic trades.
    pub wins: u64,
    /// Cost-excluded pessimistic per-unit paisa.
    pub return_paisa: i64,
    /// Full precision Wilson statistic bits.
    pub wilson_lower_bits: u64,
    /// Measured=1, constant-return unavailable=2, numerical refusal=3.
    pub romano_availability: u64,
    /// Strategy, stepdown rank, statistic bits, strict exceedances,
    /// initial numerator/denominator, adjusted numerator/denominator.
    pub romano: Option<[u64; 8]>,
}

/// One canonical complementary cross-validation split.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatisticsSplit {
    /// Equal-session segment mask used for training.
    pub train_mask: u64,
    /// Exact complementary testing mask.
    pub test_mask: u64,
    /// Whether the selected training winner ranked in the test bottom half.
    pub bottom_half: bool,
    /// Whether this split produced a defined rank.
    pub rankable: bool,
    /// Full canonical ordered train/test score hash.
    pub scores_digest: [u8; 32],
}

/// Complete manifest and family test facts from a saved statistics receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatisticsSummary {
    /// Canonical complete family-scope identity.
    pub scope: [u8; 32],
    /// Shared input/catalog/policy cohort identity.
    pub cohort: [u8; 32],
    /// Exact cross-validation layout identity.
    pub layout: [u8; 32],
    /// Family count.
    pub families: usize,
    /// All coordinate count.
    pub candidates: usize,
    /// Accepted execution sessions, with no odd-session deletion or padding.
    pub periods: u64,
    /// Number of equal contiguous segments.
    pub segments: u32,
    /// Exact sessions per segment.
    pub periods_per_segment: u64,
    /// Number of canonical complementary splits.
    pub splits: usize,
    /// Bootstrap draws, seed, block length, in that order.
    pub procedure: [u64; 3],
    /// Families, candidates, observations, bootstrap work, split work,
    /// additional working bytes, saved-body bytes. Not total-process RAM.
    pub bounds: [u64; 7],
    /// White, SPA and optional Romano–Wolf complete return-family digests.
    pub test_families: [[u8; 32]; 3],
    /// Statistic bits, probability bits, numerator, denominator,
    /// exceedances, draws, strategies, periods for White.
    pub white: [u64; 8],
    /// The same exact fields for SPA.
    pub spa: [u64; 8],
    /// Measured=1, constant-return unavailable=2, numerical refusal=3.
    pub romano_availability: u64,
    /// Defined cross-validation ranks.
    pub contributing_splits: u64,
    /// Training winners in the testing bottom half.
    pub bottom_half_splits: u64,
}

/// Cached observation of immutable saved statistics and every linked catalog.
pub struct Statistics {
    observation: Observation,
    summary: StatisticsSummary,
    sources: Vec<StatisticsSource>,
    catalogs: Vec<Catalog>,
    rows: Vec<StatisticsRow>,
    splits: Vec<StatisticsSplit>,
    admitted_bytes: u64,
}

impl Statistics {
    /// Open and authenticate this receipt and every linked candidate body under
    /// one aggregate serialized-byte budget. No live-market authority is minted.
    /// # Errors
    /// Refuses damaged/foreign/changed evidence, invalid layouts/counts or budgets.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Self, String> {
        let (observation, decoded) = Observation::open(
            root,
            "boolean-statistics-v1",
            identity,
            max_bytes,
            |bytes| decode(bytes, identity),
        )?;
        let (summary, sources, rows, splits) = decoded;
        let mut remaining = charge(max_bytes, observation.body_bytes())?;
        let mut catalogs = Vec::new();
        catalogs.try_reserve_exact(sources.len()).map_err(display)?;
        for source in &sources {
            let catalog = Catalog::open(root, source.identity, remaining)?;
            remaining = charge(remaining, catalog.body_bytes())?;
            if catalog.completion_digest() != source.completion
                || catalog.family() != source.family
                || catalog.coordinate_count() != source.coordinates
            {
                return Err("Boolean statistics referenced catalog differs".to_owned());
            }
            catalogs.push(catalog);
        }
        let result = Self {
            observation,
            summary,
            sources,
            catalogs,
            rows,
            splits,
            admitted_bytes: max_bytes - remaining,
        };
        result.check_sources()?;
        result.require_current()?;
        Ok(result)
    }
    /// Exact statistics identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        self.observation.identity()
    }
    /// Completion pin required on every page.
    #[must_use]
    pub fn completion_digest(&self) -> [u8; 32] {
        self.observation.completion_digest()
    }
    /// Complete authenticated manifest and family measurements.
    #[must_use]
    pub const fn summary(&self) -> &StatisticsSummary {
        &self.summary
    }
    /// Exact source navigation identities and completion pins.
    #[must_use]
    pub fn sources(&self) -> &[StatisticsSource] {
        &self.sources
    }
    pub(crate) fn with_catalog<T>(
        &self,
        index: usize,
        work: impl FnOnce(&Catalog) -> Result<T, String>,
    ) -> Result<T, String> {
        let catalog = self
            .catalogs
            .get(index)
            .ok_or("Boolean source catalog absent")?;
        self.observation
            .with_current(|| catalog.with_coordinates(|_| work(catalog)))
    }
    /// Serialized bytes charged for the full referenced evidence tree.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.admitted_bytes
    }
    /// Recheck retained evidence generations. This does not re-attest raw market files.
    /// # Errors
    /// Any replaced/deleted/changed saved ancestor refuses.
    pub fn require_current(&self) -> Result<(), String> {
        self.with_current(|| Ok(()))
    }
    pub(super) fn with_current<T>(
        &self,
        work: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.observation.with_current(|| {
            for source in &self.catalogs {
                source.require_current()?;
            }
            let result = work()?;
            for source in &self.catalogs {
                source.require_current()?;
            }
            Ok(result)
        })
    }
    /// At most256 exact coordinate rows at the accepted completion pin.
    /// # Errors
    /// Refuses wrong pins, invalid pages or changed saved evidence.
    pub fn rows(
        &self,
        pin: [u8; 32],
        start: usize,
        limit: usize,
    ) -> Result<Vec<StatisticsRow>, String> {
        self.pin(pin)?;
        self.with_current(|| page(&self.rows, start, limit))
    }
    /// At most256 exact complementary splits at the accepted completion pin.
    /// # Errors
    /// Refuses wrong pins, invalid pages or changed saved evidence.
    pub fn splits(
        &self,
        pin: [u8; 32],
        start: usize,
        limit: usize,
    ) -> Result<Vec<StatisticsSplit>, String> {
        self.pin(pin)?;
        self.with_current(|| page(&self.splits, start, limit))
    }
    fn pin(&self, pin: [u8; 32]) -> Result<(), String> {
        if pin != self.completion_digest() {
            return Err("Boolean statistics completion pin differs".to_owned());
        }
        Ok(())
    }
}

pub(super) fn charge(remaining: u64, body: u64) -> Result<u64, String> {
    body.checked_add(112)
        .and_then(|bytes| remaining.checked_sub(bytes))
        .ok_or_else(|| "Boolean linked evidence exceeds aggregate read-byte admission".to_owned())
}
pub(super) fn page<T: Clone>(all: &[T], start: usize, limit: usize) -> Result<Vec<T>, String> {
    if limit == 0 || limit > 256 || start > all.len() {
        return Err("Boolean evidence page refused".to_owned());
    }
    let end = start
        .checked_add(limit)
        .ok_or("Boolean evidence page overflow")?
        .min(all.len());
    let slice = all.get(start..end).ok_or("Boolean evidence page absent")?;
    let mut result = Vec::new();
    result.try_reserve_exact(slice.len()).map_err(display)?;
    result.extend_from_slice(slice);
    Ok(result)
}

pub(super) struct Record<'a> {
    raw: &'a [u8],
    at: usize,
}
impl<'a> Record<'a> {
    pub(super) const fn new(raw: &'a [u8]) -> Self {
        Self { raw, at: 0 }
    }
    pub(super) fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self
            .at
            .checked_add(N)
            .ok_or("Boolean record offset overflow")?;
        let value = self
            .raw
            .get(self.at..end)
            .ok_or("Boolean record truncated")?
            .try_into()
            .map_err(display)?;
        self.at = end;
        Ok(value)
    }
    pub(super) fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
    pub(super) fn size(&mut self) -> Result<usize, String> {
        usize::try_from(self.word()?).map_err(display)
    }
    pub(super) fn signed(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.bytes()?))
    }
    pub(super) fn padding(&self) -> Result<(), String> {
        if self
            .raw
            .get(self.at..)
            .ok_or("Boolean record tail absent")?
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err("Boolean record padding is nonzero".to_owned());
        }
        Ok(())
    }
    fn kind(&mut self, expected: u64) -> Result<(), String> {
        if self.word()? != expected {
            return Err("Boolean statistics record kind differs".to_owned());
        }
        Ok(())
    }
}

type Decoded = (
    StatisticsSummary,
    Vec<StatisticsSource>,
    Vec<StatisticsRow>,
    Vec<StatisticsSplit>,
);

fn decode(raw: &[u8], identity: [u8; 32]) -> Result<Decoded, String> {
    if !raw.len().is_multiple_of(512) {
        return Err("Boolean statistics stride differs".to_owned());
    }
    let mut chunks = raw.chunks_exact(512);
    let mut summary = manifest(
        chunks.next().ok_or("Boolean statistics manifest missing")?,
        identity,
    )?;
    let expected = summary
        .families
        .checked_add(summary.candidates)
        .and_then(|n| n.checked_add(summary.splits))
        .and_then(|n| n.checked_add(2))
        .ok_or("Boolean statistics count overflow")?;
    if expected != raw.len() / 512 {
        return Err("Boolean statistics fixed counts disagree".to_owned());
    }
    let mut sources = Vec::new();
    sources
        .try_reserve_exact(summary.families)
        .map_err(display)?;
    for _ in 0..summary.families {
        sources.push(source(chunks.next().ok_or("source missing")?)?);
    }
    let declared = sources.iter().try_fold(0_usize, |sum, source| {
        sum.checked_add(source.coordinates)
            .ok_or("Boolean source coordinate count overflow")
    })?;
    if declared != summary.candidates {
        return Err("Boolean source coordinate counts differ".to_owned());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(summary.candidates)
        .map_err(display)?;
    for (family, source) in sources.iter().enumerate() {
        for coordinate in 0..source.coordinates {
            let row = coordinate_row(chunks.next().ok_or("coordinate missing")?)?;
            if row.source != family || row.coordinate != coordinate {
                return Err("Boolean statistics coordinate sequence differs".to_owned());
            }
            rows.push(row);
        }
    }
    if rows.len() != summary.candidates {
        return Err("Boolean source coordinate counts differ".to_owned());
    }
    let layout = super::derive_layout(usize::try_from(summary.periods).map_err(display)?)?;
    let masks = crate::population_observations_v1::canonical_masks(layout)?;
    let mut splits = Vec::new();
    splits.try_reserve_exact(summary.splits).map_err(display)?;
    for (train, test) in masks {
        let row = split(chunks.next().ok_or("split missing")?)?;
        if (row.train_mask, row.test_mask) != (train, test) {
            return Err("Boolean split masks differ from canonical layout".to_owned());
        }
        splits.push(row);
    }
    family_tests(chunks.next().ok_or("family tests missing")?, &mut summary)?;
    validate_measurements(&summary, &rows, &splits)?;
    verify_identity(identity, &summary, &sources)?;
    let procedure = PopulationStatisticsProcedureV2::new(
        summary.procedure[0],
        summary.procedure[1],
        summary.procedure[2],
    )?;
    let bounds = bounds(summary.bounds);
    admit_shape(
        summary.candidates as u64,
        summary.periods,
        summary.splits as u64,
        procedure,
        bounds,
        raw.len() as u64,
    )?;
    Ok((summary, sources, rows, splits))
}

fn manifest(raw: &[u8], identity: [u8; 32]) -> Result<StatisticsSummary, String> {
    let mut input = Record::new(raw);
    input.kind(1)?;
    if input.bytes::<8>()? != *b"BTXBST01" || input.bytes::<32>()? != identity {
        return Err("Boolean statistics manifest identity differs".to_owned());
    }
    let scope = input.bytes()?;
    let cohort = input.bytes()?;
    let layout = input.bytes()?;
    let families = input.size()?;
    let candidates = input.size()?;
    let periods = input.word()?;
    let segments = u32::try_from(input.word()?).map_err(display)?;
    let periods_per_segment = input.word()?;
    let splits = input.size()?;
    let procedure = [input.word()?, input.word()?, input.word()?];
    let mut limits = [0; 7];
    for value in &mut limits {
        *value = input.word()?;
    }
    input.padding()?;
    let actual = super::derive_layout(usize::try_from(periods).map_err(display)?)?;
    if families == 0
        || families > runner::research_family::RESEARCH_FAMILY_CAPACITY_V1
        || families as u64 > limits[0]
        || layout != actual.digest()
        || segments != actual.segment_count()
        || periods_per_segment != actual.periods_per_segment()
        || splits as u64 != actual.split_count()
    {
        return Err("Boolean statistics manifest layout or scope counts differ".to_owned());
    }
    Ok(StatisticsSummary {
        scope,
        cohort,
        layout,
        families,
        candidates,
        periods,
        segments,
        periods_per_segment,
        splits,
        procedure,
        bounds: limits,
        test_families: [[0; 32]; 3],
        white: [0; 8],
        spa: [0; 8],
        romano_availability: 0,
        contributing_splits: 0,
        bottom_half_splits: 0,
    })
}

fn source(raw: &[u8]) -> Result<StatisticsSource, String> {
    let mut input = Record::new(raw);
    input.kind(2)?;
    let family = ResearchFamilyV1::decode(&input.bytes::<128>()?).map_err(display)?;
    let result = StatisticsSource {
        family,
        identity: input.bytes()?,
        completion: input.bytes()?,
        coordinates: input.size()?,
    };
    input.padding()?;
    if result.coordinates == 0 {
        return Err("Boolean statistics source is empty".to_owned());
    }
    Ok(result)
}

fn coordinate_row(raw: &[u8]) -> Result<StatisticsRow, String> {
    let mut input = Record::new(raw);
    input.kind(3)?;
    let source = input.size()?;
    let coordinate = input.size()?;
    let identity = input.bytes()?;
    let period_digest = input.bytes()?;
    let run = input.bytes()?;
    let program_index = input.size()?;
    let side = match input.word()? {
        1 => "long",
        2 => "short",
        _ => return Err("Boolean statistics side invalid".to_owned()),
    };
    let ordinal = input.word()?;
    let execution_refusal_bits = input.word()?;
    runner::exit_grid_policy::ExecutionRefusalBitsV1::from_bits(execution_refusal_bits)
        .ok_or("Boolean statistics unknown execution reasons")?;
    let trades = input.word()?;
    let wins = input.word()?;
    let return_paisa = input.signed()?;
    let wilson_lower_bits = input.word()?;
    let romano_availability = availability(input.word()?)?;
    let romano = if romano_availability == 1 {
        let mut words = [0; 8];
        for value in &mut words {
            *value = input.word()?;
        }
        Some(words)
    } else {
        None
    };
    input.padding()?;
    runner::admission::research_projection::wilson_ppm(wins, trades, wilson_lower_bits)
        .ok_or("Boolean statistics Wilson source differs")?;
    Ok(StatisticsRow {
        source,
        coordinate,
        identity,
        period_digest,
        run,
        program_index,
        side,
        ordinal,
        execution_refusal_bits,
        trades,
        wins,
        return_paisa,
        wilson_lower_bits,
        romano_availability,
        romano,
    })
}

fn split(raw: &[u8]) -> Result<StatisticsSplit, String> {
    let mut input = Record::new(raw);
    input.kind(4)?;
    let row = StatisticsSplit {
        train_mask: input.word()?,
        test_mask: input.word()?,
        bottom_half: boolean(input.word()?)?,
        rankable: boolean(input.word()?)?,
        scores_digest: input.bytes()?,
    };
    input.padding()?;
    if row.bottom_half && !row.rankable {
        return Err("Boolean unrankable split has bottom-half result".to_owned());
    }
    Ok(row)
}

fn family_tests(raw: &[u8], summary: &mut StatisticsSummary) -> Result<(), String> {
    let mut input = Record::new(raw);
    input.kind(5)?;
    for digest in &mut summary.test_families {
        *digest = input.bytes()?;
    }
    for word in summary.white.iter_mut().chain(summary.spa.iter_mut()) {
        *word = input.word()?;
    }
    summary.romano_availability = availability(input.word()?)?;
    summary.contributing_splits = input.word()?;
    summary.bottom_half_splits = input.word()?;
    input.padding()
}

fn boolean(value: u64) -> Result<bool, String> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err("Boolean statistics flag invalid".to_owned()),
    }
}
fn availability(value: u64) -> Result<u64, String> {
    if (1..=3).contains(&value) {
        Ok(value)
    } else {
        Err("Boolean statistic availability invalid".to_owned())
    }
}
fn bounds(words: [u64; 7]) -> Bounds {
    Bounds {
        families: words[0],
        candidates: words[1],
        observations: words[2],
        bootstrap_work: words[3],
        split_work: words[4],
        memory_bytes: words[5],
        bytes: words[6],
    }
}

fn validate_measurements(
    summary: &StatisticsSummary,
    rows: &[StatisticsRow],
    splits: &[StatisticsSplit],
) -> Result<(), String> {
    let draws = summary.procedure[0];
    let denominator = draws
        .checked_add(1)
        .ok_or("Boolean bootstrap denominator overflow")?;
    for test in [summary.white, summary.spa] {
        if !f64::from_bits(test[0]).is_finite()
            || test[2]
                != test[4]
                    .checked_add(1)
                    .ok_or("Boolean exceedance overflow")?
            || test[3] != denominator
            || test[4] > draws
            || test[5] != draws
            || test[6] != summary.candidates as u64
            || test[7] != summary.periods
            || test[1] != probability_bits(test[2], test[3])?
        {
            return Err("Boolean family test fields do not reconcile".to_owned());
        }
    }
    let mut ranks = Vec::new();
    ranks.try_reserve_exact(rows.len()).map_err(display)?;
    ranks.resize(rows.len(), None);
    for (index, row) in rows.iter().enumerate() {
        if row.romano_availability != summary.romano_availability {
            return Err("Boolean RW availability differs across family".to_owned());
        }
        if let Some(values) = row.romano {
            let rank = usize::try_from(values[1]).map_err(display)?;
            let seen = ranks
                .get_mut(rank)
                .ok_or("Boolean RW rank exceeds family")?;
            if seen.is_some()
                || values[0] != index as u64
                || !f64::from_bits(values[2]).is_finite()
                || values[3] > draws
                || values[4] != values[3].checked_add(1).ok_or("RW exceedance overflow")?
                || values[5] != denominator
                || values[7] != denominator
                || values[6] < values[4]
                || values[6] > denominator
            {
                return Err("Boolean RW coordinate fields disagree".to_owned());
            }
            *seen = Some((values[4], values[6]));
        }
    }
    let mut adjusted = 0;
    for (initial, saved) in ranks.into_iter().flatten() {
        adjusted = adjusted.max(initial);
        if adjusted != saved {
            return Err(
                "Boolean RW adjusted values violate the exact stepdown recurrence".to_owned(),
            );
        }
    }
    if summary.romano_availability != 1 && summary.test_families[2] != [0; 32] {
        return Err("Unavailable Boolean RW has a forged family digest".to_owned());
    }
    if summary.contributing_splits != splits.iter().filter(|row| row.rankable).count() as u64
        || summary.bottom_half_splits != splits.iter().filter(|row| row.bottom_half).count() as u64
    {
        return Err("Boolean PBO counts differ from split rows".to_owned());
    }
    Ok(())
}

#[expect(
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    reason = "reproduce the existing statistic's unrounded exact-fraction display bits; never price arithmetic"
)]
fn probability_bits(numerator: u64, denominator: u64) -> Result<u64, String> {
    runner::admission::AdmissionExactProbabilityV2::new(numerator, denominator)
        .map_err(|why| format!("Boolean probability: {why:?}"))?;
    Ok((numerator as f64 / denominator as f64).to_bits())
}

fn verify_identity(
    identity: [u8; 32],
    summary: &StatisticsSummary,
    sources: &[StatisticsSource],
) -> Result<(), String> {
    let keys: Vec<_> = sources
        .iter()
        .map(|source| source.family.instrument())
        .collect();
    let scope = ResearchScopeV1::new(&keys, sources.len()).map_err(display)?;
    if scope.digest() != summary.scope
        || !scope
            .families()
            .iter()
            .copied()
            .eq(sources.iter().map(|source| source.family))
    {
        return Err("Boolean statistics family order or scope differs".to_owned());
    }
    let mut hash = Hasher::new();
    hash.update(b"brutex-boolean-statistics-v1\0");
    hash.update(&summary.scope);
    hash.update(&summary.cohort);
    hash.update(&summary.layout);
    for word in summary.procedure.into_iter().chain(summary.bounds) {
        hash.update(&word.to_le_bytes());
    }
    for source in sources {
        hash.update(&source.family.encode());
        hash.update(&source.identity);
        hash.update(&source.completion);
    }
    if hash.finalize() != identity {
        return Err("Boolean statistics manifest inputs do not reproduce identity".to_owned());
    }
    Ok(())
}

impl Statistics {
    fn check_sources(&self) -> Result<(), String> {
        let first = self
            .catalogs
            .first()
            .ok_or("Boolean statistics has no source")?;
        let mut at = 0;
        for (family, catalog) in self.catalogs.iter().enumerate() {
            if catalog.cohort_digest() != self.summary.cohort
                || catalog.programs() != first.programs()
                || catalog.sessions() != first.sessions()
                || catalog.sessions().len() as u64 != self.summary.periods
            {
                return Err(
                    "Boolean statistics sources have foreign catalogs or sessions".to_owned(),
                );
            }
            catalog.with_coordinates(|rows| {
                for (coordinate, row) in rows.iter().enumerate() {
                    let saved = self
                        .rows
                        .get(at)
                        .ok_or("Boolean statistics coordinate omitted")?;
                    let (_, actual) =
                        super::numeric::project_row(family, coordinate, catalog.sessions(), row)?;
                    let expected = CandidateStatisticsV1 {
                        family: saved.source,
                        coordinate: saved.coordinate,
                        identity: saved.identity,
                        trades: saved.trades,
                        wins: saved.wins,
                        return_paisa: saved.return_paisa,
                        wilson_lower_bits: saved.wilson_lower_bits,
                        period_digest: saved.period_digest,
                    };
                    let side = match row.side() {
                        runner::excursion::Side::Long => "long",
                        runner::excursion::Side::Short => "short",
                    };
                    if actual != expected
                        || saved.run != row.run_id()
                        || saved.program_index != row.program_index()
                        || saved.side != side
                        || saved.ordinal != row.ordinal()
                        || saved.execution_refusal_bits != row.execution_refusal_bits().bits()
                    {
                        return Err(
                            "Boolean statistics coordinate differs from linked saved source"
                                .to_owned(),
                        );
                    }
                    at += 1;
                }
                Ok(())
            })?;
        }
        if at != self.rows.len() {
            return Err("Boolean statistics has extra source coordinates".to_owned());
        }
        Ok(())
    }
    pub(crate) fn with_source<T>(
        &self,
        row: &StatisticsRow,
        work: impl FnOnce(&super::BooleanCoordinateV1) -> Result<T, String>,
    ) -> Result<T, String> {
        self.catalogs
            .get(row.source)
            .ok_or("Boolean source absent")?
            .with_coordinates(|rows| {
                work(
                    rows.get(row.coordinate)
                        .ok_or("Boolean source coordinate absent")?,
                )
            })
    }
    pub(crate) fn all_rows(&self) -> &[StatisticsRow] {
        &self.rows
    }
    /// Exact statistics body bytes, excluding its linked ancestors.
    #[must_use]
    pub fn body_bytes(&self) -> u64 {
        self.observation.body_bytes()
    }
}

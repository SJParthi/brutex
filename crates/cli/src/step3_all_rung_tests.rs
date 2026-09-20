#![cfg(test)]
//! The real eight-rung transaction over explicitly generated finite input.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::tests::{
    FIXTURE_COMMIT, FIXTURE_TO, StoredSuccessFixture, exit_policy, fixture_bounds, fixture_request,
    maximum_fixture_singleton_support, population_admission_policy, seed_stored_family_month,
};
use super::*;
use crate::all_rung_population_v5::{
    AllRungStoredExecutionV3Request, AllRungStoredPopulationV5Request,
    commit_all_rung_stored_execution_v3, commit_all_rung_with_verified_build_v5,
};
use crate::all_rung_selection_v5::{
    AllRungSelectionV5Request, commit_all_rung_stored_selection_v5,
};
use crate::{
    execution_v3, population_admission_v3, population_finalization_v3, population_v5, selection_v5,
};
use runner::excursion::Side;
use std::collections::BTreeMap;
use store::file::BarFile;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

const ROWS: u64 = 16_384;
const FIRST_MONTH: (u16, u8) = (2025, 5);
const MONTHS: [(u16, u8); 6] = [
    (2025, 4),
    (2025, 5),
    (2025, 6),
    (2025, 7),
    (2025, 8),
    (2025, 9),
];
const RUNGS: [&str; 8] = [
    "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min",
];

fn seed_derived(root: &Path) -> Result<(), String> {
    for symbol in ["NIFTY", "BANKNIFTY"] {
        let key = InstrumentKey::index(Exchange::Nse, symbol).map_err(|why| why.to_string())?;
        let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
        let id = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
        for (year, month) in MONTHS {
            let month = YearMonth::new(year, month).map_err(|why| why.to_string())?;
            let minutes = crate::fold_audit::read_month(
                root,
                Vendor::Zerodha,
                &key,
                Timeframe::MINUTE_1,
                month,
            )?;
            for rung in crate::fold_audit::DERIVED_RUNGS {
                let bucket = pull::fold::Bucket::of_secs(rung.secs()).expect("derived width");
                let bars = pull::fold::fold(&minutes, bucket).map_err(|why| why.to_string())?;
                let path = StorePath::for_key(Vendor::Zerodha, &key, rung, month, FileKind::Bars)
                    .map_err(|why| why.to_string())?;
                BarFile::open_or_create(root, path, id)
                    .map_err(|why| why.to_string())?
                    .append(&bars)
                    .map_err(|why| why.to_string())?;
            }
        }
    }
    Ok(())
}

struct Policies {
    sweepers: [Sweeper; 8],
    long: ExitGridPolicyV1,
    short: ExitGridPolicyV1,
    admission: AdmissionPolicyV1,
}

impl Policies {
    fn new(root: &Path) -> Result<Self, String> {
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let diagnostic = Sweeper::new(engine::Ladder::with_min_hits(1).with_support_lanes(1));
        let mut sweepers = Vec::new();
        for rung in RUNGS {
            let mut supports = Vec::new();
            for symbol in ["NIFTY", "BANKNIFTY"] {
                let mut request = fixture_request(root, symbol, &diagnostic, &long, &short)?;
                request.rung_name = rung;
                request.from = FIRST_MONTH;
                request.bounds = load_bounds()?;
                supports.push(
                    maximum_fixture_singleton_support(&request)
                        .map_err(|why| format!("generated {rung} {symbol} support: {why}"))?,
                );
            }
            let support = supports.into_iter().min().expect("two measured families");
            sweepers.push(Sweeper::new(
                engine::Ladder::with_min_hits(support)
                    .with_ceiling(4_096)
                    .with_pair_budget(65_536)
                    .with_support_lanes(1),
            ));
        }
        Ok(Self {
            sweepers: sweepers
                .try_into()
                .map_err(|_| "exactly eight sweepers".to_owned())?,
            long,
            short,
            admission: population_admission_policy()?,
        })
    }

    fn request<'a>(
        &'a self,
        root: &'a Path,
        authority: &'a Path,
    ) -> Result<AllRungStoredPopulationV5Request<'a>, String> {
        let mut candidate_bounds = load_bounds()?;
        candidate_bounds.candidate = CandidateUniverseBoundsV1::new(1_000_000, 32)?;
        candidate_bounds.pre_admission = PreAdmissionDataBoundsV1::new(32, 16 * 1_024 * 1_024)?;
        Ok(AllRungStoredPopulationV5Request {
            source_root: root,
            authority_root: authority,
            vendor: Vendor::Zerodha,
            from: FIRST_MONTH,
            to: FIXTURE_TO,
            one_minute_sweeper: &self.sweepers[0],
            two_minute_sweeper: &self.sweepers[1],
            three_minute_sweeper: &self.sweepers[2],
            five_minute_sweeper: &self.sweepers[3],
            ten_minute_sweeper: &self.sweepers[4],
            fifteen_minute_sweeper: &self.sweepers[5],
            thirty_minute_sweeper: &self.sweepers[6],
            sixty_minute_sweeper: &self.sweepers[7],
            horizon: Horizon::DEFAULT,
            widths: Widths::pinned().map_err(|why| format!("{why:?}"))?,
            availability: Availability::Absent,
            thresholds: Thresholds::CLASSICAL,
            long_exit_policy: &self.long,
            short_exit_policy: &self.short,
            candidate_bounds,
            observation_bounds: ObservationAuthorityBoundsV1::new(2, 64 * 1_024 * 1_024)?,
            statistics_bounds: PopulationStatisticsV2Bounds::new(
                2,
                1_000_000,
                512,
                1_000_000,
                512 * 1_024 * 1_024,
            )?,
            statistics_procedure: PopulationStatisticsProcedureV2::new(16, 7, 2)?,
            search_bounds: AnchoredSearchLineageV4Bounds::new(
                1,
                2 * crate::anchored_search_lineage_v4::ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES
                    as u64,
                crate::anchored_search_lineage_v4::ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES
                    as u64,
            )?,
            admission_bounds: AdmissionV3Bounds::new(
                ROWS,
                ROWS * population_admission_v3::POPULATION_ADMISSION_V3_DECISION_BYTES as u64,
                1,
                population_admission_v3::POPULATION_ADMISSION_V3_COMPLETION_BYTES as u64,
                ROWS,
            )?,
            admission_policy: &self.admission,
            finalization_bounds: PopulationFinalizationV3Bounds::new(
                ROWS,
                ROWS * population_finalization_v3::POPULATION_FINALIZATION_V3_ROW_BYTES as u64,
                1,
                population_finalization_v3::POPULATION_FINALIZATION_V3_COMPLETION_BYTES as u64,
                ROWS,
            )?,
            population_bounds: population_v5::PopulationV5Bounds::new(
                ROWS,
                ROWS * population_v5::POPULATION_V5_ROW_BYTES as u64,
                1,
                population_v5::POPULATION_V5_COMPLETION_BYTES as u64,
                ROWS,
            )?,
        })
    }
}

fn load_bounds() -> Result<StoredCandidatePreAdmissionBoundsV1, String> {
    let mut bounds = fixture_bounds()?;
    bounds.signal_records = StoredSpanLoadBoundV1::new(100_000)?;
    bounds.minute_records = StoredSpanLoadBoundV1::new(120_000)?;
    bounds.daily_records = StoredSpanLoadBoundV1::new(512)?;
    Ok(bounds)
}

fn roots(parent: &Path) -> Result<[PathBuf; 8], String> {
    let paths = std::array::from_fn(|index| parent.join(RUNGS[index]));
    for path in &paths {
        fs::create_dir_all(path).map_err(|why| why.to_string())?;
    }
    Ok(paths)
}

fn execution_request(paths: &[PathBuf; 8]) -> Result<AllRungStoredExecutionV3Request<'_>, String> {
    let bound = |records: u64, stride: usize| {
        execution_v3::ExecutionV3FileBound::new(
            records,
            records * stride as u64,
            stride,
            "generated all-rung fixture",
        )
    };
    let bounds = execution_v3::ExecutionV3Bounds::new(
        bound(4, execution_v3::EXECUTION_V3_PARAMETER_BYTES)?,
        bound(256, execution_v3::EXECUTION_V3_PERCENTILE_BYTES)?,
        bound(ROWS, execution_v3::EXECUTION_V3_DISPOSITION_BYTES)?,
        bound(1, execution_v3::EXECUTION_V3_COMPLETION_BYTES)?,
        ROWS,
        256,
    )?;
    Ok(AllRungStoredExecutionV3Request {
        one_minute_root: &paths[0],
        one_minute_bounds: bounds,
        two_minute_root: &paths[1],
        two_minute_bounds: bounds,
        three_minute_root: &paths[2],
        three_minute_bounds: bounds,
        five_minute_root: &paths[3],
        five_minute_bounds: bounds,
        ten_minute_root: &paths[4],
        ten_minute_bounds: bounds,
        fifteen_minute_root: &paths[5],
        fifteen_minute_bounds: bounds,
        thirty_minute_root: &paths[6],
        thirty_minute_bounds: bounds,
        sixty_minute_root: &paths[7],
        sixty_minute_bounds: bounds,
    })
}

fn selection_request(paths: &[PathBuf; 8]) -> Result<AllRungSelectionV5Request<'_>, String> {
    let bounds = selection_v5::SelectionV5Bounds::new(
        25,
        25 * selection_v5::SELECTION_V5_ROW_BYTES as u64,
        1,
        selection_v5::SELECTION_V5_COMPLETION_BYTES as u64,
    )?;
    Ok(AllRungSelectionV5Request {
        one_minute_root: &paths[0],
        one_minute_bounds: bounds,
        two_minute_root: &paths[1],
        two_minute_bounds: bounds,
        three_minute_root: &paths[2],
        three_minute_bounds: bounds,
        five_minute_root: &paths[3],
        five_minute_bounds: bounds,
        ten_minute_root: &paths[4],
        ten_minute_bounds: bounds,
        fifteen_minute_root: &paths[5],
        fifteen_minute_bounds: bounds,
        thirty_minute_root: &paths[6],
        thirty_minute_bounds: bounds,
        sixty_minute_root: &paths[7],
        sixty_minute_bounds: bounds,
        ranking_policy: runner::topn::RankingPolicyV1::new(runner::topn::Weights::equal())
            .map_err(|why| format!("{why:?}"))?,
    })
}

fn published(paths: &[PathBuf]) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    for root in paths {
        for entry in fs::read_dir(root).map_err(|why| why.to_string())? {
            let path = entry.map_err(|why| why.to_string())?.path();
            if path.is_file() {
                let bytes = fs::read(&path).map_err(|why| why.to_string())?;
                files.insert(path, bytes);
            }
        }
    }
    assert!(
        !files.is_empty(),
        "a publication proof must observe actual files"
    );
    Ok(files)
}

#[test]
fn all_rung_retained_handles_stay_below_one_sixty_four_kib_stack_budget() {
    use crate::all_rung_population_v5::{
        CommittedStoredAllRungExecutionV3, CommittedStoredAllRungPopulationV5,
    };
    use crate::all_rung_selection_v5::{
        AllRungSelectionV5SuccessorSetV1, CommittedStoredAllRungSelectionV5,
    };
    for (name, bytes) in [
        (
            "Population",
            std::mem::size_of::<CommittedStoredAllRungPopulationV5>(),
        ),
        (
            "Execution",
            std::mem::size_of::<CommittedStoredAllRungExecutionV3>(),
        ),
        (
            "Selection",
            std::mem::size_of::<CommittedStoredAllRungSelectionV5>(),
        ),
        (
            "Successor",
            std::mem::size_of::<AllRungSelectionV5SuccessorSetV1>(),
        ),
    ] {
        assert!(
            bytes <= 64 * 1_024,
            "{name} handle consumes {bytes} stack bytes"
        );
    }
}

#[test]
fn all_eight_stored_rungs_publish_exact_selection_chains_and_reuse_every_byte() -> Result<(), String>
{
    // Identical generated prices keep a shared support threshold exact for
    // both families. Distinct instrument identities must still be preserved.
    let fixture = StoredSuccessFixture::with_family_prices([2_000_000; 2])?;
    // Five requested months also warm EMA200 in the first hourly training
    // prefix (one third of the full span). April supplies prior context.
    for symbol in ["NIFTY", "BANKNIFTY"] {
        for month in &MONTHS[..4] {
            seed_stored_family_month(&fixture.source, Vendor::Zerodha, symbol, *month, 2_000_000)?;
        }
    }
    seed_derived(&fixture.source)?;
    let policies = Policies::new(&fixture.source)?;
    let authority = fixture.base.join("all-rung-population");
    let population_paths = roots(&authority)?;
    let execution_paths = roots(&fixture.base.join("all-rung-execution"))?;
    let selection_paths = roots(&fixture.base.join("all-rung-selection"))?;
    let request = policies.request(&fixture.source, &authority)?;
    let execution_request = execution_request(&execution_paths)?;
    let selection_request = selection_request(&selection_paths)?;
    let all_paths: Vec<_> = population_paths
        .into_iter()
        .chain(execution_paths.iter().cloned())
        .chain(selection_paths.iter().cloned())
        .collect();
    let mut original = None;
    for written in [8, 0] {
        let population = commit_all_rung_with_verified_build_v5(
            &request,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
        )?;
        assert_eq!(population.written_rung_count(), written);
        let execution = commit_all_rung_stored_execution_v3(population, &execution_request)?;
        assert_eq!(execution.written_rung_count(), written);
        let mut selection = commit_all_rung_stored_selection_v5(execution, &selection_request)?;
        assert_eq!(selection.written_rung_count(), written);
        // Each producer reauthenticates before returning; the successor doors
        // also reauthenticate their retained inputs. Do not duplicate those
        // unchanged-source scans here.
        let bytes = published(&all_paths)?;
        if let Some(expected) = &original {
            assert_eq!(&bytes, expected);
        } else {
            original = Some(bytes);
        }
        if written == 8 {
            let completion = selection_paths[7].join("global-selection-completions-v5.bin");
            let saved = fs::read(&completion).map_err(|why| why.to_string())?;
            let mut corrupted = saved.clone();
            *corrupted.last_mut().expect("actual completion") ^= 1;
            fs::write(&completion, corrupted).map_err(|why| why.to_string())?;
            assert!(selection.require_live_topology().is_err());
            fs::write(completion, saved).map_err(|why| why.to_string())?;
            // Restoring bytes does not restore the held file's mtime/ctime
            // epoch. Drop this stale capability before the fresh retry.
        } else {
            let successors = selection.into_successor_set()?;
            let mut visited = 0;
            let refusal = successors
                .visit_canonical(|_| {
                    visited += 1;
                    Ok(())
                })
                .expect_err("generated candidates do not meet the strict admission policy");
            assert!(refusal.contains("requires exactly 25 winners, observed 0"));
            assert_eq!(
                visited, 0,
                "an incomplete cohort must not visit any successor"
            );
        }
    }
    assert_eq!(published(&all_paths)?, original.expect("first publication"));
    Ok(())
}

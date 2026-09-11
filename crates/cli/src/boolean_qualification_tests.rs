//! Complete generated-calendar fixtures; no real-market or admission-success claim.
use super::*;
use crate::boolean_qualification_plan::{GeneratedBindings, Plan, ordered_identities};
use crate::candidate_universe::boolean_candidate_v1::{self as catalog, oos, statistics};
use runner::expression::Expression;

const LIMITS: [u64; 2] = [64 * 1024 * 1024, 100_000];

fn later_request<'a>(
    fixture: &'a catalog::tests::Fixture,
    inputs: &'a crate::audited_range_command::StrictConfig,
) -> oos::LaterRequest<'a> {
    oos::LaterRequest {
        output: &fixture.output,
        inputs,
        from: (2025, 9),
        to: (2025, 9),
        bounds: catalog::Bounds {
            programs: 8,
            coordinates: 512,
            trades: 200_000,
            bytes: 32 * 1024 * 1024,
        },
    }
}

fn fixture_bindings<'a>(
    training: &'a CommittedBooleanAdmissionV1,
    later: oos::LaterRequest<'_>,
) -> Result<GeneratedBindings<'a>, String> {
    // This fixture executes only 1min. The other seven slots are explicit
    // generated descriptor labels, never claimed as actual all-rung evidence.
    let mut catalogs = std::array::from_fn(|rung| {
        hash(format!("unused generated original rung {rung}").as_bytes())
    });
    let mut sources =
        std::array::from_fn(|rung| hash(format!("unused generated later rung {rung}").as_bytes()));
    let actual = training.statistics().sources();
    *catalogs
        .first_mut()
        .ok_or("generated catalog slot absent")? = ordered_identities(
        actual
            .iter()
            .map(catalog::CommittedBooleanFamilyV1::identity),
    );
    let identities = actual
        .iter()
        .map(|source| oos::source_identity_fixture(source, later))
        .collect::<Result<Vec<_>, _>>()?;
    *sources.first_mut().ok_or("generated source slot absent")? =
        ordered_identities(identities.into_iter());
    Ok(GeneratedBindings {
        families: training.statistics().families(),
        catalogs,
        sources,
    })
}

struct Setup {
    fixture: catalog::tests::Fixture,
    training: CommittedBooleanAdmissionV1,
    plan: Plan,
    inputs: crate::audited_range_command::StrictConfig,
    policy: AdmissionPolicyV1,
    procedure: PopulationStatisticsProcedureV2,
    programs: Vec<Expression>,
}
impl Setup {
    fn new() -> Result<Self, String> {
        let fixture = catalog::tests::Fixture::new()?;
        let programs = ["30 | !30", "30 & !30"]
            .into_iter()
            .map(|raw| Expression::parse(raw).map_err(|why| format!("{why:?}")))
            .collect::<Result<Vec<_>, _>>()?;
        let mut sources = Vec::new();
        for name in ["NIFTY", "RELIANCE"] {
            let source = fixture.produce_span(name, &programs, 7, 8)?;
            assert_eq!(
                source.sessions().len(),
                42,
                "complete warmed training census"
            );
            sources.push(source);
        }
        let procedure = PopulationStatisticsProcedureV2::new(7, 49, 2)?;
        let measured = statistics::produce(
            &fixture.output,
            sources,
            procedure,
            statistics::tests::stored_limits(),
        )?;
        let policy = super::super::tests::policy_with_folds(u64::MAX, 2)?;
        let training = super::super::produce(&fixture.output, measured, &policy, LIMITS[0])?;
        let inputs = crate::audited_range_command::StrictConfig::from_values(
            Some(fixture.root.as_os_str().to_owned()),
            Some("4194304".into()),
            Some("40000".into()),
        )
        .map_err(display)?;
        let bindings = fixture_bindings(&training, later_request(&fixture, &inputs))?;
        let plan = Plan::generated_fixture(
            &policy,
            procedure,
            ((2025, 7), (2025, 8)),
            ((2025, 9), (2025, 9)),
            &programs,
            LIMITS,
            &bindings,
        )?;
        Ok(Self {
            fixture,
            training,
            plan,
            inputs,
            policy,
            procedure,
            programs,
        })
    }
    fn later(&self, validated: bool) -> Result<Vec<CommittedBooleanOosV1<'_>>, String> {
        let requested = self.plan.requested()?;
        self.training
            .statistics()
            .sources()
            .iter()
            .map(|training| {
                oos::produce_fixture(
                    training,
                    later_request(&self.fixture, &self.inputs),
                    validated.then(|| oos::ValidationRequest {
                        requested,
                        windows: self.plan.windows(),
                        max_folds: 2,
                        mapping_bytes: LIMITS[0],
                        projection_bytes: LIMITS[0],
                    }),
                )
            })
            .collect()
    }
    fn request<'a>(
        &'a self,
        later: &'a [CommittedBooleanOosV1<'a>],
    ) -> Result<Request<'a>, String> {
        Ok(Request {
            root: &self.fixture.output,
            training: &self.training,
            later,
            policy: self.policy,
            procedure: self.procedure,
            plan: &self.plan,
            rung: 0,
            bounds: Bounds {
                candidates: LIMITS[1],
                observations: LIMITS[1],
                work: self.plan.numerical_bounds()?.max_work,
                memory: LIMITS[0],
                bytes: LIMITS[0],
            },
        })
    }
}

fn verify_complete_rows(committed: &Committed<'_>, saved: &reader::Reader) -> Result<(), String> {
    assert_eq!(saved.identity(), committed.identity());
    assert_eq!(saved.completion_digest(), committed.pin());
    assert_eq!(saved.row_count(), committed.count());
    assert_eq!(
        committed.counts().iter().sum::<u64>(),
        committed.count() as u64
    );
    let mut measured = 0;
    let mut unavailable = 0;
    let mut zero = 0;
    for start in (0..saved.row_count()).step_by(256) {
        let page = saved.rows(saved.completion_digest(), start, 256)?;
        assert!(!page.is_empty());
        for (offset, row) in page.iter().enumerate() {
            let expected = committed.rows.get(start + offset).ok_or("saved row")?;
            assert_eq!(row.original, expected.original);
            assert_eq!(row.later, expected.later);
            assert_eq!(row.values, expected.projection.values());
            assert_eq!(row.verdict, expected.projection.verdict());
            assert_eq!(row.romano, expected.romano);
            assert!(row.verdict.reconciles());
            assert!(matches!(
                row.values.fwer_p_value_ppm,
                ObservedU64V1::Measured(_)
            ));
            assert!(matches!(
                row.values.romano_wolf_p_value_ppm,
                ObservedU64V1::Measured(_)
            ));
            assert_eq!(
                row.values.full_precision_statistics_complete,
                CompletenessV1::Complete
            );
            if let Some(folds) = &row.folds {
                measured += 1;
                assert_eq!(row.values.decided_folds, ObservedU64V1::Measured(2));
                assert_eq!(
                    row.values.profitable_oos_folds,
                    ObservedU64V1::Measured(folds.profitable_oos_folds())
                );
                assert_eq!(
                    row.values.oos_pessimistic_return_paisa,
                    ObservedI64V1::Measured(folds.aggregate_oos_paisa()?)
                );
            } else {
                unavailable += 1;
                assert_eq!(row.values.decided_folds, ObservedU64V1::Unmeasured);
                assert_eq!(
                    row.values.oos_pessimistic_return_paisa,
                    ObservedI64V1::Unmeasured
                );
                assert!(!row.verdict.is_admitted());
            }
            if row.values.trades == ObservedU64V1::Measured(0) {
                zero += 1;
                assert_eq!(row.romano.first(), Some(&2));
                assert_eq!(
                    row.values.romano_wolf_p_value_ppm,
                    ObservedU64V1::Measured(1_000_000)
                );
                assert!(!row.verdict.is_admitted());
            }
        }
    }
    assert!(measured > 0 && unavailable > 0 && zero > 0);
    Ok(())
}

fn verify_resealed_foreign_bindings(
    setup: &Setup,
    body: &[u8],
    id: [u8; 32],
) -> Result<(), String> {
    let (manifest, rows) = wire::decode(body, id)?;
    for originals in [true, false] {
        let mut bindings = fixture_bindings(
            &setup.training,
            later_request(&setup.fixture, &setup.inputs),
        )?;
        let identities = if originals {
            &mut bindings.catalogs
        } else {
            &mut bindings.sources
        };
        identities.swap(0, 1);
        let foreign = Plan::generated_fixture(
            &setup.policy,
            setup.procedure,
            ((2025, 7), (2025, 8)),
            ((2025, 9), (2025, 9)),
            &setup.programs,
            LIMITS,
            &bindings,
        )?;
        let mut changed = manifest.clone();
        changed.scope = foreign.descriptor();
        changed.unit = *foreign.units().first().ok_or("foreign unit absent")?;
        changed.plan = foreign.canonical_bytes()?;
        changed.identity = identity(&changed);
        let encoded = wire::encode(&changed, &rows)?;
        // Defensive disk tampering: even canonical self-consistent bytes and
        // recomputed completion seals cannot substitute another rung's inputs.
        wire::decode(&encoded, changed.identity)?;
        let pending = persistence::prepare_in_namespace(
            &setup.fixture.output,
            NAMESPACE,
            changed.identity,
            &encoded,
        )?;
        pending.finish(changed.identity, hash(&encoded), encoded.len() as u64)?;
        drop(pending);
        let Err(why) =
            reader::Reader::open(&setup.fixture.output, changed.identity, 512 * 1024 * 1024)
        else {
            return Err("resealed foreign rung evidence was accepted".into());
        };
        let expected = if originals {
            "original full catalog identities differ from planned rung"
        } else {
            "later full source identities differ from planned rung"
        };
        assert!(why.contains(expected), "{why}");
    }
    Ok(())
}

#[test]
fn complete_original_later_qualification_reopens_all_coordinates_and_keeps_zero_families()
-> Result<(), String> {
    let setup = Setup::new()?;
    let later = setup.later(true)?;
    let committed = produce(&setup.request(&later)?)?;
    committed.require_current()?;
    let saved = reader::Reader::open(
        &setup.fixture.output,
        committed.identity(),
        512 * 1024 * 1024,
    )?;
    verify_complete_rows(&committed, &saved)?;
    assert!(
        saved
            .index_consistency(saved.completion_digest(), 0)?
            .is_none(),
        "legacy qualification is not reassessed by a newer reader"
    );
    assert!(saved.rows([0; 32], 0, 1).is_err());
    assert!(saved.rows(saved.completion_digest(), 0, 257).is_err());
    assert!(
        saved
            .rows(saved.completion_digest(), usize::MAX, 1)
            .is_err()
    );
    let body = std::fs::read(committed.directory.join("body.bin")).map_err(display)?;
    let retry = produce(&setup.request(&later)?)?;
    assert_eq!(retry.identity(), committed.identity());
    assert_eq!(retry.pin(), committed.pin());
    assert_eq!(
        std::fs::read(retry.directory.join("body.bin")).map_err(display)?,
        body
    );
    let observed = wire::decode(&body, committed.identity())?;
    assert_eq!(wire::encode(&observed.0, &observed.1)?, body);
    verify_resealed_foreign_bindings(&setup, &body, committed.identity())?;
    assert!(
        wire::decode(
            body.get(..body.len() - 1).ok_or("truncated")?,
            committed.identity()
        )
        .is_err()
    );
    let mut wrong = body;
    *wrong.first_mut().ok_or("magic")? ^= 1;
    assert!(wire::decode(&wrong, committed.identity()).is_err());
    std::fs::remove_file(committed.directory.join("complete.bin")).map_err(display)?;
    assert!(saved.require_current().is_err());
    assert!(committed.require_current().is_err());
    assert!(
        reader::Reader::open(
            &setup.fixture.output,
            committed.identity(),
            512 * 1024 * 1024
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn required_consistency_is_persisted_for_every_coordinate_and_missing_receipt_refuses()
-> Result<(), String> {
    let mut setup = Setup::new()?;
    setup.plan = setup.plan.with_index_consistency()?;
    let later = setup.later(true)?;
    let committed = produce(&setup.request(&later)?)?;
    let saved = reader::Reader::open(
        &setup.fixture.output,
        committed.identity(),
        512 * 1024 * 1024,
    )?;
    let receipt = saved
        .index_consistency_receipt()
        .ok_or("required receipt absent")?;
    let mut indices = 0;
    let mut cash = 0;
    for index in 0..saved.row_count() {
        let record = saved
            .index_consistency(saved.completion_digest(), index)?
            .ok_or("required setting missing")?;
        assert_eq!(
            record.sessions.len(),
            record.training_session_count
                + later
                    .get(usize::try_from(record.binding.family).map_err(display)?)
                    .ok_or("saved family missing")?
                    .sessions()
                    .len()
        );
        assert_eq!(
            record.evaluation.first_day,
            setup.plan.training_requested()?.first_day()
        );
        assert_eq!(
            record.evaluation.last_day,
            setup.plan.requested()?.last_day()
        );
        if record.evaluation.outcome == crate::index_consistency::Outcome::NotApplicable {
            cash += 1;
        } else {
            indices += 1;
        }
    }
    assert!(
        indices > 0 && cash > 0,
        "index assessed and cash left outside the index rule"
    );
    let again = produce(&setup.request(&later)?)?;
    assert_eq!(again.pin(), committed.pin());
    assert_eq!(
        again
            .consistency
            .as_ref()
            .ok_or("retry receipt missing")?
            .receipt(),
        receipt
    );
    std::fs::remove_file(
        setup
            .fixture
            .output
            .join("index-consistency-v1")
            .join(crate::identity_hex(&receipt.identity))
            .join("complete.bin"),
    )
    .map_err(display)?;
    assert!(saved.require_current().is_err());
    assert!(
        reader::Reader::open(
            &setup.fixture.output,
            committed.identity(),
            512 * 1024 * 1024
        )
        .is_err(),
        "new missing receipt never becomes historical not-assessed"
    );
    Ok(())
}

#[test]
fn qualification_refuses_missing_original_fold_authority_wrong_plan_and_changed_source()
-> Result<(), String> {
    let setup = Setup::new()?;
    let ordinary = setup.later(false)?;
    assert!(
        ordinary
            .iter()
            .flat_map(CommittedBooleanOosV1::fold_projections)
            .all(Option::is_none)
    );
    assert!(
        produce(&setup.request(&ordinary)?).is_err(),
        "an original selected coordinate requires its declared validation proof"
    );
    let later = setup.later(true)?;
    let bindings = fixture_bindings(
        &setup.training,
        later_request(&setup.fixture, &setup.inputs),
    )?;
    let mut foreign_rung = setup.request(&later)?;
    foreign_rung.rung = 1;
    assert!(
        produce(&foreign_rung).is_err(),
        "exact 1min original/later evidence cannot take an unused rung allocation"
    );
    let wrong = Plan::generated_fixture(
        &setup.policy,
        setup.procedure,
        ((2025, 7), (2025, 8)),
        ((2025, 10), (2025, 10)),
        &setup.programs,
        LIMITS,
        &bindings,
    )?;
    let mut request = setup.request(&later)?;
    request.plan = &wrong;
    assert!(
        produce(&request).is_err(),
        "a different declared later month cannot name September observations"
    );
    let changed_policy = super::super::tests::policy_with_folds(u64::MAX - 1, 2)?;
    let changed_plan = Plan::generated_fixture(
        &changed_policy,
        setup.procedure,
        ((2025, 7), (2025, 8)),
        ((2025, 9), (2025, 9)),
        &setup.programs,
        LIMITS,
        &bindings,
    )?;
    let mut request = setup.request(&later)?;
    request.policy = changed_policy;
    request.plan = &changed_plan;
    assert!(
        produce(&request).is_err(),
        "the original admission policy remains bound"
    );
    let mut request = setup.request(&later)?;
    request.bounds.memory -= 1;
    assert!(
        produce(&request).is_err(),
        "physical plan bounds must match exactly"
    );
    let source = store::path::StorePath::for_key(
        brutex_core::vendor::Vendor::Zerodha,
        &crate::stored::swept_index("RELIANCE")?,
        store::path::Timeframe::MINUTE_1,
        store::path::YearMonth::new(2025, 9).map_err(display)?,
        store::path::FileKind::Bars,
    )
    .map_err(display)?
    .to_path_buf(&setup.fixture.root);
    std::fs::remove_file(source).map_err(display)?;
    assert!(later.iter().any(|later| later.require_current().is_err()));
    assert!(produce(&setup.request(&later)?).is_err());
    Ok(())
}

#[test]
fn qualification_probability_projection_preserves_exact_finite_allocation_boundary()
-> Result<(), String> {
    let fraction = probability(8, 1999)?;
    assert_eq!(fraction.ppm(), 4002, "legacy projection remains unchanged");
    assert_eq!(
        upper_ppm(fraction)?,
        4003,
        "new comparison cannot round a failure down"
    );
    let allocation = runner::family_allocation_v1::allocate(0, 1, 0, 8, 4002)
        .map_err(|why| format!("{why:?}"))?;
    assert!(
        !allocation
            .compare(1, 1999)
            .map_err(|why| format!("{why:?}"))?
    );
    for (numerator, denominator, expected) in
        [(0, 1, 0), (1, 8, 125_000), (u64::MAX, u64::MAX, 1_000_000)]
    {
        assert_eq!(upper_ppm(probability(numerator, denominator)?)?, expected);
    }
    assert!(probability(1, 0).is_err());
    Ok(())
}

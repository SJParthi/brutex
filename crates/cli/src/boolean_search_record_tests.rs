//! Generated codec observations only: invented hashes never authorize market work.
#![expect(
    clippy::unwrap_used,
    reason = "finite format and named refusal assertions"
)]
use super::*;
use crate::boolean_qualification_plan::{GeneratedBindings, Plan};
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use runner::admission::{AdmissionPolicyDraftV1, AdmissionPolicyV1};

const BYTES: u64 = 1024 * 1024;
const RECORDS: u64 = 10_000;

pub(crate) fn generated_policy() -> AdmissionPolicyV1 {
    generated_policy_with_ceilings([50_000; 4])
}

pub(crate) fn generated_policy_with_ceilings(ceilings: [u64; 4]) -> AdmissionPolicyV1 {
    let [fwer, spa, white, romano] = ceilings;
    // These deliberately permissive values exercise numeric comparisons only;
    // they are not the configured research policy or proposed trading settings.
    AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
        min_support_hits: Some(1),
        min_independent_sessions: Some(1),
        min_trades: Some(1),
        max_mae_paisa: Some(u64::MAX),
        min_worst_reward_risk_ppm: Some(0),
        min_win_rate_ppm: Some(0),
        min_wilson_win_rate_ppm: Some(0),
        min_return_drawdown_ppm: Some(0),
        min_weakest_period_return_paisa: Some(i64::MIN),
        max_pbo_ppm: Some(1_000_000),
        max_fwer_p_value_ppm: Some(fwer),
        max_spa_p_value_ppm: Some(spa),
        min_decided_folds: Some(2),
        max_ambiguous_fill_rate_ppm: Some(1_000_000),
        max_gap_affected_rate_ppm: Some(1_000_000),
        max_session_concentration_ppm: Some(1_000_000),
        max_largest_trade_profit_share_ppm: Some(1_000_000),
        max_drawdown_paisa: Some(u64::MAX),
        max_worst_trade_loss_paisa: Some(u64::MAX),
        max_losing_trade_rate_ppm: Some(1_000_000),
        max_losing_trades: Some(u64::MAX),
        min_pessimistic_profit_paisa: Some(i64::MIN),
        min_winning_trades: Some(0),
        min_average_win_paisa: Some(0),
        max_average_loss_paisa: Some(u64::MAX),
        min_profit_factor_ppm: Some(0),
        max_consecutive_losing_streak: Some(u64::MAX),
        min_consecutive_winning_streak: Some(0),
        min_bootstrap_draws: Some(1),
        min_bootstrap_strategies: Some(1),
        min_bootstrap_periods: Some(1),
        min_pbo_contributing_folds: Some(1),
        max_pbo_unrankable_folds: Some(u64::MAX),
        min_profitable_oos_folds: Some(1),
        min_oos_pessimistic_return_paisa: Some(i64::MIN),
        max_white_reality_p_value_ppm: Some(white),
        require_white_reality_rejection: Some(true),
        max_romano_wolf_p_value_ppm: Some(romano),
        require_romano_wolf_rejection: Some(true),
    })
    .unwrap()
}

pub(crate) fn generated_record() -> Record {
    let cursor = Cursor::new(&[30, 31]).unwrap();
    let batch = Batch::prepare(
        cursor.clone(),
        0,
        0,
        Budget {
            programs: 2,
            nodes: 100,
            bytes: BYTES / 4,
        },
    )
    .unwrap();
    let plan = generated_plan(batch.programs());
    let observed = ObservedPlan::decode(&plan, BYTES, RECORDS).unwrap();
    let spec = Spec {
        rungs: RungScope::ALL,
        projection_rule: ProjectionRule::SharedCeilingsV2,
        sources: observed.source_descriptors(),
        policy: observed.policy_digest(),
        initial: cursor.initial_descriptor(),
        programs: 2,
        nodes: 100,
        bytes: BYTES,
        records: RECORDS,
        alpha: observed.alpha_ceilings().into_iter().min().unwrap(),
    };
    Record {
        previous: None,
        spec,
        ordinal: 0,
        phase: 0,
        batch,
        plan,
        campaign: Summary::EMPTY.child,
        summaries: [Summary::EMPTY; 8],
        reason: String::new(),
    }
}

pub(crate) fn generated_plan(programs: &[runner::expression::Expression]) -> Vec<u8> {
    generated_plan_with_limits(programs, [BYTES, RECORDS])
}

pub(crate) fn generated_plan_with_limits(
    programs: &[runner::expression::Expression],
    limits: [u64; 2],
) -> Vec<u8> {
    let (exchange, name) = *brutex_core::instrument::InstrumentKey::SWEPT
        .first()
        .unwrap();
    let key = brutex_core::instrument::InstrumentKey::index(exchange, name).unwrap();
    let family = runner::research_family::ResearchFamilyV1::new(key).unwrap();
    let bindings = GeneratedBindings {
        families: &[family],
        catalogs: std::array::from_fn(|rung| {
            hash(format!("generated original catalog {rung}").as_bytes())
        }),
        sources: std::array::from_fn(|rung| {
            hash(format!("generated later source {rung}").as_bytes())
        }),
    };
    Plan::generated_fixture(
        &generated_policy(),
        PopulationStatisticsProcedureV2::new(1000, 17, 2).unwrap(),
        ((2025, 4), (2025, 5)),
        ((2025, 6), (2025, 8)),
        programs,
        limits,
        &bindings,
    )
    .unwrap()
    .canonical_bytes()
    .unwrap()
}

pub(crate) fn completed_record() -> Record {
    complete_generated_record(generated_record())
}

pub(crate) fn complete_generated_record(mut record: Record) -> Record {
    let plan = record.observed_plan().unwrap();
    record.campaign = Link {
        identity: crate::boolean_qualified_journal::identity_for_rungs(
            plan.descriptor(),
            *plan.units(),
            plan.rungs(),
        ),
        pin: hash(b"generated campaign completion"),
    };
    record.phase = 1;
    record.summaries = std::array::from_fn(|rung| {
        if record.spec.rungs.contains(rung) {
            Summary {
                child: Link {
                    identity: hash(format!("generated qualification {rung}").as_bytes()),
                    pin: hash(format!("generated completion {rung}").as_bytes()),
                },
                count: 10,
                counts: [1, 2, 3, 4],
                projection: hash(format!("generated projection {rung}").as_bytes()),
                allocation: runner::search_allocation_v1::allocate(
                    record.ordinal,
                    rung as u64,
                    record.spec.alpha,
                )
                .unwrap()
                .digest(),
            }
        } else {
            Summary::EMPTY
        }
    });
    record
}

fn duplicate(record: &Record) -> Record {
    Record::decode(&record.encode().unwrap(), BYTES).unwrap()
}

#[test]
fn daily_search_declaration_has_distinct_identity_and_rejects_legacy_plan_reuse() {
    let old = generated_record();
    let old_identity = old.spec.identity();
    let mut ids = std::collections::HashSet::new();
    for mask in 1..=u8::MAX {
        let mut spec = old.spec.clone();
        spec.rungs = RungScope::from_mask(mask).unwrap();
        spec.projection_rule = ProjectionRule::IndexConsistencyV4;
        spec.validate().unwrap();
        let raw = spec.encode();
        assert_eq!(raw.get(..8).unwrap(), b"BRBQSS04");
        assert_eq!(Spec::decode(&raw).unwrap(), spec);
        assert_ne!(spec.identity(), old_identity);
        assert!(ids.insert(spec.identity()));
        let mut altered = raw;
        *altered.last_mut().unwrap() ^= 1;
        assert!(Spec::decode(&altered).is_err());
    }
    let mut newer = generated_record();
    newer.spec.projection_rule = ProjectionRule::IndexConsistencyV4;
    assert!(
        newer.encode().is_err(),
        "old plan lacks required daily policy, so completed search cannot be reused"
    );
}

#[test]
fn scoped_search_records_bind_selection_keep_legacy_bytes_and_refuse_omitted_results() {
    let legacy = generated_record();
    let legacy_bytes = legacy.encode().unwrap();
    let mut identities = std::collections::HashSet::new();
    for mask in 1..=u8::MAX {
        let mut record = generated_record();
        let rungs = RungScope::from_mask(mask).unwrap();
        record.spec.rungs = rungs;
        if rungs != RungScope::ALL {
            record.spec.projection_rule = ProjectionRule::ScopedSharedCeilingsV3;
            // Generated fixed-format observations, never live source capabilities.
            record
                .plan
                .get_mut(..8)
                .unwrap()
                .copy_from_slice(b"BRBQPL02");
            record
                .plan
                .get_mut(8..16)
                .unwrap()
                .copy_from_slice(&2_u64.to_le_bytes());
            record
                .plan
                .get_mut(944..952)
                .unwrap()
                .copy_from_slice(&u64::from(mask).to_le_bytes());
            for index in 0..8 {
                if !rungs.contains(index) {
                    for base in [288, 544] {
                        record
                            .plan
                            .get_mut(base + index * 32..base + (index + 1) * 32)
                            .unwrap()
                            .fill(0);
                    }
                }
            }
        }
        assert!(identities.insert(record.spec.identity()));
        assert_eq!(record.observed_plan().unwrap().rungs(), rungs);
        let raw = record.encode().unwrap();
        assert_eq!(Record::decode(&raw, BYTES).unwrap().encode().unwrap(), raw);
        if rungs == RungScope::ALL {
            assert_eq!(raw, legacy_bytes);
        } else {
            assert_eq!(raw.len(), legacy_bytes.len() + 8);
            assert_eq!(raw.get(..8).unwrap(), b"BRBQSR03");
            let mut declared_all = raw.clone();
            declared_all
                .get_mut(HEADER + SPEC_BYTES..HEADER + SPEC_BYTES + 8)
                .unwrap()
                .copy_from_slice(&255_u64.to_le_bytes());
            assert!(Record::decode(&declared_all, BYTES).is_err());
            let mut complete = complete_generated_record(record);
            assert!(complete.validate().is_ok());
            let omitted = (0..8).find(|&index| !rungs.contains(index)).unwrap();
            *complete.summaries.get_mut(omitted).unwrap() = *complete
                .summaries
                .get(rungs.indices().next().unwrap())
                .unwrap();
            assert!(complete.validate().is_err());
        }
    }
    assert_eq!(identities.len(), 255);
}

#[test]
fn generated_reservation_completion_and_refusal_round_trip_without_changing_binding() {
    let reserved = generated_record();
    let complete = completed_record();
    let mut refused = generated_record();
    refused.phase = 2;
    refused.reason = "generated child refusal: no prices were read".into();
    assert_eq!(reserved.binding().unwrap(), complete.binding().unwrap());
    assert_eq!(reserved.binding().unwrap(), refused.binding().unwrap());
    for record in [&reserved, &complete, &refused] {
        let raw = record.encode().unwrap();
        let decoded = Record::decode(&raw, BYTES).unwrap();
        assert_eq!(decoded.encode().unwrap(), raw);
        assert_eq!(decoded.phase, record.phase);
        assert_eq!(decoded.reason, record.reason);
        assert_eq!(decoded.summaries, record.summaries);
        assert_eq!(decoded.spec, record.spec);
        assert_eq!(decoded.binding().unwrap(), record.binding().unwrap());
        assert_eq!(
            decoded.spec.cursor().unwrap().encode(),
            decoded.spec.initial
        );
    }
}

#[test]
fn declaration_rejects_progressed_cursors_zero_authority_and_invalid_physical_caps() {
    let record = generated_record();
    let original = record.spec;
    assert_eq!(Spec::decode(&original.encode()).unwrap(), original);
    assert_eq!(original.budget().bytes, BYTES / 4);
    for change in 0..9 {
        let mut spec = original.clone();
        match change {
            0 => *spec.sources.first_mut().unwrap() = [0; 32],
            1 => *spec.sources.last_mut().unwrap() = [0; 32],
            2 => spec.policy = [0; 32],
            3 => spec.programs = 0,
            4 => spec.nodes = 0,
            5 => {
                spec.nodes = 1;
                spec.records = 1;
            }
            6 => spec.bytes = (HEADER + SPEC_BYTES + 95) as u64,
            7 => spec.bytes = isize::MAX as u64 + 1,
            _ => spec.alpha = 1_000_001,
        }
        assert!(spec.validate().is_err(), "declaration change {change}");
        assert_ne!(spec.identity(), original.identity());
    }
    let mut excessive_nodes = original.clone();
    excessive_nodes.nodes = excessive_nodes.records + 1;
    assert_eq!(
        excessive_nodes.validate().unwrap_err(),
        "search declaration has invalid source or physical admission"
    );
    let mut progressed = original.clone();
    progressed.initial = record.batch.next_cursor().encode();
    assert_eq!(
        progressed.cursor().unwrap_err(),
        "search declaration is not an initial grammar cursor"
    );
    let mut wrong_header = original.encode();
    *wrong_header.first_mut().unwrap() ^= 1;
    assert_eq!(
        Spec::decode(&wrong_header).unwrap_err(),
        "search declaration format differs"
    );
    for end in [0, 7, SPEC_BYTES - 1] {
        assert_eq!(
            Spec::decode(wrong_header.get(..end).unwrap()).unwrap_err(),
            "search declaration format differs"
        );
    }
}

#[test]
fn every_record_truncation_and_exact_header_length_refuses() {
    let raw = completed_record().encode().unwrap();
    for end in 0..raw.len() {
        assert!(
            Record::decode(raw.get(..end).unwrap(), BYTES).is_err(),
            "truncation {end}"
        );
    }
    assert_eq!(
        Record::decode(&raw, raw.len() as u64 - 1).err().unwrap(),
        "search record header or byte admission refused"
    );
    assert_eq!(
        Record::decode(&raw, decode_bytes(&completed_record(), &raw))
            .unwrap()
            .encode()
            .unwrap(),
        raw
    );
    let mut longer = raw.clone();
    longer.push(0);
    assert_eq!(
        Record::decode(&longer, BYTES).err().unwrap(),
        "search record counts or exact length differ"
    );
    for at in [64, 72, 80] {
        let mut changed = raw.clone();
        changed
            .get_mut(at..at + 8)
            .unwrap()
            .copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            Record::decode(&changed, BYTES).err().unwrap(),
            "search record counts or exact length differ"
        );
    }
    assert_eq!(
        field::<8>(&raw, usize::MAX).unwrap_err(),
        "search field is truncated"
    );
    assert_eq!(field::<0>(&raw, raw.len()).unwrap(), [0_u8; 0]);
}

pub(crate) fn decode_bytes(record: &Record, raw: &[u8]) -> u64 {
    raw.len() as u64
        + record.spec.programs
            * (size_of::<runner::expression::Expression>() + runner::expression::ENCODED_LEN) as u64
        + (2 * CURSOR_BYTES) as u64
}

#[test]
fn physical_grammar_reservation_precedes_corrupt_cursor_replay() {
    let record = generated_record();
    let raw = record.encode().unwrap();
    let exact = decode_bytes(&record, &raw);
    assert_eq!(Record::decode(&raw, exact).unwrap().encode().unwrap(), raw);
    assert_eq!(
        Record::decode(&raw, exact - 1).err().unwrap(),
        "search complete program reservation exceeds independent decode-byte admission"
    );
    let batch_at = HEADER + SPEC_BYTES;
    let mut corrupt = raw.clone();
    // Corrupt the actual before-cursor magic, not the outer record or batch header.
    *corrupt.get_mut(batch_at + 64).unwrap() ^= 1;
    assert_eq!(Record::decode(&corrupt, BYTES).err().unwrap(), "Cursor");
    assert_eq!(
        Record::decode_bounded(&corrupt, BYTES, record.spec.nodes - 1)
            .err()
            .unwrap(),
        "search grammar replay exceeds independent node admission"
    );
    for programs in [1_000_000, u64::MAX] {
        let mut huge = corrupt.clone();
        let at = HEADER + 104 + CURSOR_BYTES;
        huge.get_mut(at..at + 8)
            .unwrap()
            .copy_from_slice(&programs.to_le_bytes());
        assert_eq!(
            Record::decode(&huge, BYTES).err().unwrap(),
            "search complete program reservation exceeds independent decode-byte admission"
        );
    }
}

#[test]
fn independent_declaration_header_admission_precedes_any_grammar_decoding() {
    let record = generated_record();
    let raw = record.encode().unwrap();
    assert_eq!(
        Record::declaration(&raw, raw.len() as u64).unwrap(),
        record.spec
    );
    let truncated = raw.get(..HEADER + SPEC_BYTES - 1).unwrap();
    assert_eq!(
        Record::declaration(truncated, BYTES).unwrap_err(),
        "search record header or byte admission refused"
    );
    assert_eq!(
        Record::declaration(&raw, raw.len() as u64 - 1).unwrap_err(),
        "search record header or byte admission refused"
    );
    let mut foreign = raw;
    *foreign.first_mut().unwrap() ^= 1;
    assert_eq!(
        Record::declaration(&foreign, BYTES).unwrap_err(),
        "search record header or byte admission refused"
    );
}

#[test]
fn every_byte_mutation_is_refused_or_remains_an_exact_canonical_observation() {
    let record = completed_record();
    let raw = record.encode().unwrap();
    let binding = record.binding().unwrap();
    let spec = record.spec.identity();
    let mut refused = 0;
    let mut changed_observations = 0;
    for at in 0..raw.len() {
        let mut changed = raw.clone();
        *changed.get_mut(at).unwrap() ^= 1;
        match Record::decode(&changed, BYTES) {
            Err(_) => refused += 1,
            Ok(decoded) => {
                assert_eq!(
                    decoded.encode().unwrap(),
                    changed,
                    "noncanonical accepted offset {at}"
                );
                if at >= HEADER {
                    assert!(
                        decoded.binding().unwrap() != binding || decoded.spec.identity() != spec,
                        "immutable source/plan/batch mutation lost from identity at {at}"
                    );
                } else {
                    // These outer observation fields are authenticated by the
                    // journal's seal, not fabricated cryptographic codec checks.
                    assert!(
                        decoded.previous != record.previous
                            || decoded.campaign != record.campaign
                            || decoded.summaries != record.summaries
                            || decoded.phase != record.phase
                            || decoded.ordinal != record.ordinal
                            || decoded.reason != record.reason,
                        "accepted mutation did not change its declared observation at {at}"
                    );
                }
                changed_observations += 1;
            }
        }
    }
    assert!(
        refused > 1024,
        "padding and immutable bindings must actually refuse"
    );
    assert!(
        changed_observations > 0,
        "payload codec is not a cryptographic journal seal"
    );
    assert_eq!(refused + changed_observations, raw.len());
}

#[test]
fn refusal_utf8_padding_and_predecessor_parts_are_exact() {
    let mut refused = generated_record();
    refused.phase = 2;
    refused.reason = "r".repeat(1024);
    refused.previous = Some((7, hash(b"generated preceding receipt")));
    let raw = refused.encode().unwrap();
    assert_eq!(Record::decode(&raw, BYTES).unwrap().reason.len(), 1024);
    refused.reason.push('x');
    assert_eq!(
        refused.validate().unwrap_err(),
        "search phase, reason or grammar allowance differs"
    );
    let mut invalid_utf8 = raw.clone();
    *invalid_utf8.get_mut(152 + 8 * 168).unwrap() = 0xff;
    assert!(
        Record::decode(&invalid_utf8, BYTES)
            .err()
            .unwrap()
            .contains("utf-8")
    );
    let reserved = generated_record().encode().unwrap();
    let mut padding = reserved.clone();
    *padding.get_mut(HEADER - 1).unwrap() = 1;
    assert_eq!(
        Record::decode(&padding, BYTES).err().unwrap(),
        "search reason padding differs"
    );
    let mut pin_only = reserved.clone();
    *pin_only.get_mut(16).unwrap() = 1;
    assert_eq!(
        Record::decode(&pin_only, BYTES).err().unwrap(),
        "search predecessor is partial"
    );
    assert_eq!(
        Record::predecessor(&pin_only).unwrap_err(),
        "search predecessor is partial"
    );
    let mut ordinal_only = reserved;
    *ordinal_only.get_mut(8).unwrap() = 1;
    assert_eq!(
        Record::decode(&ordinal_only, BYTES).err().unwrap(),
        "search predecessor pin absent"
    );
    assert_eq!(
        Record::predecessor(&ordinal_only).unwrap_err(),
        "search predecessor is partial"
    );
}

#[test]
fn frozen_source_policy_program_limits_and_allocation_cannot_be_substituted() {
    let original = generated_record();
    for change in 0..7 {
        let mut record = duplicate(&original);
        match change {
            0 => record.spec.sources.swap(0, 1),
            1 => record.spec.policy = hash(b"foreign generated policy"),
            2 => record.spec.bytes += 1,
            3 => record.spec.records += 1,
            4 => record.spec.alpha -= 1,
            5 => record.spec.programs += 1,
            _ => record.spec.nodes += 1,
        }
        assert!(record.validate().is_err(), "binding change {change}");
    }
    let mut record = duplicate(&original);
    record.batch =
        Batch::prepare(Cursor::new(&[40, 41]).unwrap(), 0, 0, record.spec.budget()).unwrap();
    assert_eq!(
        record.validate().unwrap_err(),
        "search batch qualification plan differs from its frozen declaration"
    );
    let mut completed = completed_record();
    completed.ordinal = 1;
    assert_eq!(
        completed.validate().unwrap_err(),
        "search rung counts, binding or exact allocation differ"
    );
    let mut foreign_campaign = completed_record();
    foreign_campaign.campaign.identity = hash(b"foreign generated campaign");
    assert_eq!(
        foreign_campaign.validate().unwrap_err(),
        "search completion names a foreign qualification campaign"
    );
}

#[test]
fn missing_overflowed_or_reassigned_rung_results_never_count_as_completion() {
    let original = completed_record();
    for rung in 0..8 {
        for change in 0..6 {
            let mut record = duplicate(&original);
            let summary = record.summaries.get_mut(rung).unwrap();
            match change {
                0 => summary.child.identity = [0; 32],
                1 => summary.child.pin = [0; 32],
                2 => summary.projection = [0; 32],
                3 => summary.count += 1,
                4 => summary.counts = [u64::MAX, 1, 0, 0],
                _ => {
                    summary.allocation = runner::search_allocation_v1::allocate(
                        0,
                        ((rung + 1) % 8) as u64,
                        record.spec.alpha,
                    )
                    .unwrap()
                    .digest();
                }
            }
            assert!(record.validate().is_err(), "rung {rung}, change {change}");
        }
    }
    let mut missing_reason = generated_record();
    missing_reason.phase = 2;
    assert!(missing_reason.validate().is_err());
    let mut premature = completed_record();
    premature.phase = 0;
    assert_eq!(
        premature.validate().unwrap_err(),
        "unfinished search batch carries completed results"
    );
    // A valid identity without the actual completion pin is not a completion.
    let mut no_campaign_pin = completed_record();
    no_campaign_pin.campaign.pin = [0; 32];
    assert_eq!(
        no_campaign_pin.validate().unwrap_err(),
        "search completion names a foreign qualification campaign"
    );
    // Each orphan summary must refuse independently of the campaign guard.
    for rung in 0..8 {
        for phase in [0, 2] {
            let mut orphan = generated_record();
            orphan.phase = phase;
            if phase == 2 {
                orphan.reason = "generated refusal with orphan child".into();
            }
            *orphan.summaries.get_mut(rung).unwrap() = *original.summaries.get(rung).unwrap();
            assert_eq!(
                orphan.validate().unwrap_err(),
                "unfinished search batch carries completed results"
            );
        }
    }
    premature.phase = 2;
    premature.reason = "generated refusal".into();
    assert_eq!(
        premature.validate().unwrap_err(),
        "unfinished search batch carries completed results"
    );
}

#[test]
fn node_only_progress_carries_no_invented_qualification_or_results() {
    let mut record = generated_record();
    record.spec.nodes = 1;
    record.spec.initial = Cursor::new(&[30]).unwrap().initial_descriptor();
    let first = Batch::prepare(record.spec.cursor().unwrap(), 0, 0, record.spec.budget()).unwrap();
    let second = Batch::prepare(
        first.next_cursor(),
        first.work(),
        first.cumulative_programs().unwrap(),
        record.spec.budget(),
    )
    .unwrap();
    assert_eq!(first.programs().len(), 1);
    assert!(second.programs().is_empty());
    record.batch = second;
    record.plan.clear();
    for phase in [0, 1, 2] {
        record.phase = phase;
        record.reason = if phase == 2 {
            "generated node refusal".into()
        } else {
            String::new()
        };
        let raw = record.encode().unwrap();
        assert_eq!(Record::decode(&raw, BYTES).unwrap().encode().unwrap(), raw);
    }
    let exact_bytes = record.encode().unwrap().len() as u64 + 96;
    record.spec.bytes = exact_bytes;
    assert_eq!(record.encode().unwrap().len() as u64 + 96, exact_bytes);
    record.spec.bytes -= 1;
    assert_eq!(
        record.encode().unwrap_err(),
        "search record exceeds complete byte admission"
    );
    record.spec.bytes = BYTES;
    record.plan.push(0);
    assert_eq!(
        record.validate().unwrap_err(),
        "node-only search batch carries invented qualification evidence"
    );
    record.plan.clear();
    record.campaign = completed_record().campaign;
    assert_eq!(
        record.validate().unwrap_err(),
        "node-only search batch carries invented qualification evidence"
    );
    record.campaign = Summary::EMPTY.child;
    for rung in 0..8 {
        *record.summaries.get_mut(rung).unwrap() = *completed_record().summaries.get(rung).unwrap();
        assert_eq!(
            record.validate().unwrap_err(),
            "node-only search batch carries invented qualification evidence"
        );
        *record.summaries.get_mut(rung).unwrap() = Summary::EMPTY;
    }
}

#[test]
fn ordered_transitions_require_prework_reservation_and_preserve_failed_slot_on_retry() {
    use crate::boolean_search_reader::transition;
    let reservation = generated_record();
    transition(None, &reservation).unwrap();
    let complete = completed_record();
    assert_eq!(
        transition(None, &complete).unwrap_err(),
        "search first record is not its initial pre-work reservation"
    );
    let mut prior_at_genesis = duplicate(&reservation);
    prior_at_genesis.previous = Some((1, hash(b"generated foreign origin")));
    assert!(transition(None, &prior_at_genesis).is_err());
    let mut skipped = duplicate(&reservation);
    skipped.ordinal = 1;
    assert!(transition(None, &skipped).is_err());
    let mut failed = duplicate(&reservation);
    failed.phase = 2;
    failed.reason = "generated transient child failure".into();
    transition(Some(&reservation), &failed).unwrap();
    transition(Some(&failed), &failed).unwrap();
    transition(Some(&failed), &complete).unwrap();
    assert_eq!(
        transition(Some(&failed), &reservation).unwrap_err(),
        "search reassigned an unfinished reservation"
    );
    assert!(transition(Some(&failed), &skipped).is_err());
    transition(Some(&reservation), &complete).unwrap();
    assert!(transition(Some(&complete), &complete).is_err());
    assert!(transition(Some(&complete), &failed).is_err());

    let mut next = generated_record();
    next.ordinal = 1;
    next.previous = Some((2, hash(b"generated complete predecessor")));
    next.batch = Batch::prepare(
        complete.batch.next_cursor(),
        complete.batch.work(),
        complete.batch.cumulative_programs().unwrap(),
        complete.spec.budget(),
    )
    .unwrap();
    next.plan = generated_plan(next.batch.programs());
    transition(Some(&complete), &next).unwrap();
    let mut reassigned = duplicate(&next);
    reassigned.ordinal = failed.ordinal;
    reassigned.phase = 2;
    reassigned.reason = "generated replacement of an already failed batch".into();
    reassigned.validate().unwrap();
    assert_eq!(
        transition(Some(&failed), &reassigned).unwrap_err(),
        "search reassigned an unfinished reservation"
    );
    let mut wrong_order = duplicate(&next);
    wrong_order.ordinal = 2;
    assert!(transition(Some(&complete), &wrong_order).is_err());
    let mut wrong_work = duplicate(&next);
    wrong_work.batch = Batch::prepare(
        complete.batch.next_cursor(),
        complete.batch.work() + 1,
        complete.batch.cumulative_programs().unwrap(),
        complete.spec.budget(),
    )
    .unwrap();
    assert!(transition(Some(&complete), &wrong_work).is_err());
    let mut changed_declaration = duplicate(&next);
    changed_declaration.spec.initial = Cursor::new(&[30, 31, 40]).unwrap().initial_descriptor();
    assert_eq!(
        transition(Some(&complete), &changed_declaration).unwrap_err(),
        "search changed its immutable declaration"
    );
}

#[test]
fn legacy_projection_spec_has_pinned_original_bytes_and_digest() {
    // Independent fixed V1 wire image was hashed with the retained 198270
    // vocab/core libraries. These generated declarations authorize no work.
    let spec = Spec {
        rungs: RungScope::ALL,
        projection_rule: ProjectionRule::LegacyV1,
        sources: [[1; 32], [2; 32]],
        policy: [3; 32],
        initial: Cursor::new(&[30]).unwrap().initial_descriptor(),
        programs: 2,
        nodes: 100,
        bytes: BYTES,
        records: RECORDS,
        alpha: 50_000,
    };
    let raw = spec.encode();
    assert_eq!(raw.len(), 3230);
    assert_eq!(raw.get(..8).unwrap(), b"BRBQSS01");
    assert_eq!(
        crate::identity_hex(&spec.identity()),
        "c2df305bd71e299749f8c0d5c289fa3507ac5c86ec178397dff25715ff63d8eb"
    );
    assert_eq!(Spec::decode(&raw).unwrap().encode(), raw);
    assert_eq!(ProjectionRule::LegacyV1.version(), 1);
    assert_eq!(
        ProjectionRule::LegacyV1.summary_domain(),
        b"brutex-search-wide-projection-v1\0"
    );
    assert_eq!(ProjectionRule::SharedCeilingsV2.version(), 2);
    assert_eq!(
        ProjectionRule::SharedCeilingsV2.summary_domain(),
        b"brutex-search-wide-projection-v2\0"
    );
}

#[test]
fn legacy_records_round_trip_exactly_and_new_rule_changes_only_versioned_identity() {
    for completed in [false, true] {
        let mut record = if completed {
            completed_record()
        } else {
            generated_record()
        };
        let v2 = record.encode().unwrap();
        let v2_identity = record.spec.identity();
        let v2_binding = record.binding().unwrap();
        let mut legacy = v2.clone();
        legacy.get_mut(..8).unwrap().copy_from_slice(b"BRBQSR01");
        legacy
            .get_mut(HEADER..HEADER + 8)
            .unwrap()
            .copy_from_slice(b"BRBQSS01");
        record.spec.projection_rule = ProjectionRule::LegacyV1;
        assert_eq!(record.encode().unwrap(), legacy);
        assert_eq!(
            Record::declaration(&legacy, BYTES).unwrap().projection_rule,
            ProjectionRule::LegacyV1
        );
        let decoded = Record::decode(&legacy, BYTES).unwrap();
        assert_eq!(decoded.encode().unwrap(), legacy);
        assert_eq!(decoded.plan, record.plan);
        assert_eq!(decoded.summaries, record.summaries);
        assert_ne!(decoded.spec.identity(), v2_identity);
        assert_ne!(decoded.binding().unwrap(), v2_binding);
        assert_eq!(v2.len(), legacy.len(), "existing strides do not change");
        record.spec.projection_rule = ProjectionRule::SharedCeilingsV2;
        assert_eq!(record.encode().unwrap(), v2);
        assert_eq!(Record::decode(&v2, BYTES).unwrap().encode().unwrap(), v2);
    }
}

#[test]
fn mixed_or_unknown_projection_headers_refuse_before_inner_replay() {
    for (outer, inner) in [
        (*b"BRBQSR01", *b"BRBQSS02"),
        (*b"BRBQSR02", *b"BRBQSS01"),
        (*b"BRBQSR02", *b"BRBQSS03"),
    ] {
        let mut raw = generated_record().encode().unwrap();
        raw.get_mut(..8).unwrap().copy_from_slice(&outer);
        raw.get_mut(HEADER..HEADER + 8)
            .unwrap()
            .copy_from_slice(&inner);
        // Also corrupt the before cursor: version refusal must precede replay.
        *raw.get_mut(HEADER + SPEC_BYTES + 64).unwrap() ^= 1;
        assert_eq!(
            Record::declaration(&raw, BYTES).unwrap_err(),
            "search record and declaration projection versions differ"
        );
        assert_eq!(
            Record::decode(&raw, BYTES).err().unwrap(),
            "search record and declaration projection versions differ"
        );
    }
    let mut raw = generated_record().encode().unwrap();
    raw.get_mut(..8).unwrap().copy_from_slice(b"BRBQSR05");
    assert_eq!(
        Record::decode(&raw, BYTES).err().unwrap(),
        "search record header or byte admission refused"
    );
    let mut spec = generated_record().spec.encode();
    spec.get_mut(..8).unwrap().copy_from_slice(b"BRBQSS05");
    assert_eq!(
        Spec::decode(&spec).unwrap_err(),
        "search declaration format differs"
    );
}

#[test]
fn projection_rule_cannot_change_inside_an_existing_search_history() {
    use crate::boolean_search_reader::transition;
    for rule in [ProjectionRule::LegacyV1, ProjectionRule::SharedCeilingsV2] {
        let mut before = generated_record();
        before.spec.projection_rule = rule;
        let mut after = completed_record();
        after.spec.projection_rule = rule;
        transition(Some(&before), &after).unwrap();
        after.spec.projection_rule = match rule {
            ProjectionRule::LegacyV1 => ProjectionRule::SharedCeilingsV2,
            ProjectionRule::SharedCeilingsV2
            | ProjectionRule::ScopedSharedCeilingsV3
            | ProjectionRule::IndexConsistencyV4 => ProjectionRule::LegacyV1,
        };
        after.validate().unwrap();
        assert_eq!(
            transition(Some(&before), &after).unwrap_err(),
            "search changed its immutable declaration"
        );
    }
}

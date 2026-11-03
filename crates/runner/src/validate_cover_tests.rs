//! BASE-COV-LIBS-LS2: the anchored walk's refusal conversions and accessors,
//! and every seal and provenance cross-wire its V2 and V3 reconciliations
//! refuse, each reached by a copy of the cached fixture with one fact moved.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the exception every test module in this workspace takes: a fixture that does \
              not hold must fail loudly, fold positions come from the fixture itself, and a \
              cross-wire table reads as one list."
)]

use super::tests::{
    anchored_admission_fixture_v2, anchored_search_fixture_v3, anchored_search_fixture_v4,
};
use super::{
    AnchoredAdmissionFoldV2, AnchoredAdmissionValidationRefusalV2, AnchoredAdmissionValidationV2,
    AnchoredSearchValidationRefusalV3, AnchoredSearchValidationRefusalV4,
    AnchoredSearchValidationV3, hash_validation_family_v2, hash_validation_family_v3,
    hash_walk_facts_v2, hash_walk_facts_v3,
};
use crate::exit_grid_policy::ExitGridErrorV1;
use costs::fill::Direction;
use vocab::ConditionMask;

/// Which refusal a cross-wired copy must answer.
#[derive(Clone, Copy, Debug)]
enum Expect {
    /// The whole-walk seal no longer reproduces.
    Seal,
    /// The first decided fold's provenance disagrees.
    Decided,
    /// The first undecided fold's provenance disagrees.
    Undecided,
}

/// One cross-wire: what it moves, the move, and the refusal it must answer.
type Crosswire<T> = (&'static str, fn(&mut T, usize, usize), Expect);

/// One edit of a visible fold, with what it breaks.
type VisibleEdit = (&'static str, fn(&mut super::FoldResult));

/// One edit of a visible fold, with whether the candidate family (true) or
/// the provenance (false) must refuse it.
type FoldEdit = (&'static str, fn(&mut super::FoldResult), bool);

/// One trace of a decided fold put back on an undecided one, with its name.
type Trace = (&'static str, fn(&mut super::FoldResult, &super::FoldResult));

/// The first decided and the first undecided fold of a fixture.
fn decided_and_undecided(folds: &[AnchoredAdmissionFoldV2]) -> (usize, usize) {
    (
        folds
            .iter()
            .position(|fold| fold.choice.is_some())
            .expect("the fixture decides a fold"),
        folds
            .iter()
            .position(|fold| fold.choice.is_none())
            .expect("the fixture leaves a fold undecided"),
    )
}

/// The seal and per-fold cross-wires V2 and V3 share, for one opaque type and
/// its own family and walk reseal (a moved PROOF field is resealed, so the
/// per-fold comparison and not the digest is what must refuse it).
macro_rules! crosswires {
    ($opaque:ty, $family:path, $walk:path) => {{
        fn reseal(value: &mut $opaque) {
            value.validation_family_digest = $family(&value.folds);
            value.walk_facts_digest = $walk(
                value.validation_policy_digest,
                value.validation_family_digest,
                &value.folds,
            );
        }
        let table: [Crosswire<$opaque>; 29] = [
            (
                "a refused visible walk",
                |v, _, _| v.validated.refused = Some("refused".to_owned()),
                Expect::Seal,
            ),
            (
                "no fold at all",
                |v, _, _| {
                    v.folds.clear();
                    v.validated.folds.clear();
                },
                Expect::Seal,
            ),
            (
                "one more visible fold than proofs",
                |v, _, _| {
                    let extra = v.validated.folds[0].clone();
                    v.validated.folds.push(extra);
                },
                Expect::Seal,
            ),
            (
                "a policy digest that does not reseal",
                |v, _, _| v.validation_policy_digest[0] ^= 1,
                Expect::Seal,
            ),
            (
                "a family digest that does not reseal",
                |v, _, _| v.validation_family_digest[0] ^= 1,
                Expect::Seal,
            ),
            (
                "a walk digest that does not reseal",
                |v, _, _| v.walk_facts_digest[0] ^= 1,
                Expect::Seal,
            ),
            (
                "a resealed proof at another position",
                |v, d, _| {
                    v.folds[d].index += 1;
                    reseal(v);
                },
                Expect::Decided,
            ),
            (
                "a visible fold at another position",
                |v, d, _| v.validated.folds[d].index += 1,
                Expect::Decided,
            ),
            (
                "a visible training window one bar longer",
                |v, d, _| v.validated.folds[d].train_bars += 1,
                Expect::Decided,
            ),
            (
                "a resealed proof with a second training span",
                |v, d, _| {
                    v.folds[d].train_second_end = v.folds[d].train_second_start + 1;
                    reseal(v);
                },
                Expect::Decided,
            ),
            (
                "a visible test window one bar longer",
                |v, d, _| v.validated.folds[d].test_bars += 1,
                Expect::Decided,
            ),
            (
                "one more visible purged bar",
                |v, d, _| v.validated.folds[d].purged += 1,
                Expect::Decided,
            ),
            (
                "one more visible considered candidate",
                |v, d, _| v.validated.folds[d].considered += 1,
                Expect::Decided,
            ),
            (
                "one more visible priced candidate",
                |v, d, _| v.validated.folds[d].priced += 1,
                Expect::Decided,
            ),
            (
                "one more visible in-sample trade",
                |v, d, _| v.validated.folds[d].in_sample.trades += 1,
                Expect::Decided,
            ),
            (
                "one more visible out-of-sample trade",
                |v, d, _| v.validated.folds[d].out_of_sample.trades += 1,
                Expect::Decided,
            ),
            (
                "a chosen exit total that is not the choice's",
                |v, d, _| {
                    let total = v.validated.folds[d].chosen_exit_total.expect("decided");
                    v.validated.folds[d].chosen_exit_total = Some(total + 1);
                },
                Expect::Decided,
            ),
            (
                "an in-sample cell that is not the choice's",
                |v, d, _| {
                    let at = usize::try_from(v.folds[d].choice.expect("decided").ordinal)
                        .expect("an index");
                    v.validated.folds[d].in_sample_all[at] += 1;
                },
                Expect::Decided,
            ),
            (
                "an out-of-sample cell that is not the choice's",
                |v, d, _| {
                    let at = usize::try_from(v.folds[d].choice.expect("decided").ordinal)
                        .expect("an index");
                    v.validated.folds[d].out_of_sample_all[at] += 1;
                },
                Expect::Decided,
            ),
            (
                "a better in-sample candidate than the one chosen",
                |v, d, _| {
                    let best = v.folds[d].choice.expect("decided").in_sample_pessimistic;
                    v.validated.folds[d].in_sample_all.push(best + 1);
                },
                Expect::Decided,
            ),
            (
                "an undecided fold that names a combination",
                |v, _, u| v.validated.folds[u].chosen = Some(ConditionMask::ZERO),
                Expect::Undecided,
            ),
            (
                "an undecided fold that names a side",
                |v, _, u| v.validated.folds[u].chosen_side = Some(Direction::Long),
                Expect::Undecided,
            ),
            (
                "an undecided fold that names an exit",
                |v, _, u| v.validated.folds[u].chosen_exit = Some(crate::grid::Chosen::default()),
                Expect::Undecided,
            ),
            (
                "an undecided fold with an exit total",
                |v, _, u| v.validated.folds[u].chosen_exit_total = Some(0),
                Expect::Undecided,
            ),
            (
                "an undecided fold with an out-of-sample exit",
                |v, _, u| v.validated.folds[u].out_of_sample_exit = Some(0),
                Expect::Undecided,
            ),
            (
                "no visible side for a decided fold",
                |v, d, _| v.validated.folds[d].chosen_side = None,
                Expect::Decided,
            ),
            (
                // Bit 383 is past every live position, so no choice holds it.
                "a visible combination that is not the choice's",
                |v, d, _| {
                    let mask = v.folds[d].choice.expect("decided").mask;
                    v.validated.folds[d].chosen = Some(mask.with_bit(383));
                },
                Expect::Decided,
            ),
            (
                // No grid has a stop rung at the largest index.
                "a visible exit that is not the choice's",
                |v, d, _| {
                    let exit = v.folds[d].choice.expect("decided").exit;
                    v.validated.folds[d].chosen_exit = Some(crate::grid::Chosen {
                        stop: Some(usize::MAX),
                        ..exit
                    });
                },
                Expect::Decided,
            ),
            (
                "a visible out-of-sample exit that is not the choice's",
                |v, d, _| {
                    let total = v.folds[d]
                        .choice
                        .expect("decided")
                        .out_of_sample_pessimistic;
                    v.validated.folds[d].out_of_sample_exit = Some(total + 1);
                },
                Expect::Decided,
            ),
        ];
        table
    }};
}

/// EVERY REFUSAL OF THE ADMISSION WALK KEEPS ITS NAME AND ITS FOLD across the
/// conversion the V3 search door applies to it.
#[test]
fn a_v2_refusal_converts_to_the_same_v3_refusal() {
    let why = "upstream".to_owned();
    for (from, to) in [
        (
            AnchoredAdmissionValidationRefusalV2::UpstreamRefused(why.clone()),
            AnchoredSearchValidationRefusalV3::UpstreamRefused(why),
        ),
        (
            AnchoredAdmissionValidationRefusalV2::Empty,
            AnchoredSearchValidationRefusalV3::Empty,
        ),
        (
            AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs,
            AnchoredSearchValidationRefusalV3::ZeroResolvedRungs,
        ),
        (
            AnchoredAdmissionValidationRefusalV2::StableWidth,
            AnchoredSearchValidationRefusalV3::StableWidth,
        ),
        (
            AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold: 3 },
            AnchoredSearchValidationRefusalV3::CandidateFamilyMismatch { fold: 3 },
        ),
        (
            AnchoredAdmissionValidationRefusalV2::IncompleteChosenOos { fold: 4 },
            AnchoredSearchValidationRefusalV3::IncompleteChosenOos { fold: 4 },
        ),
        (
            AnchoredAdmissionValidationRefusalV2::ExitRungMismatch { fold: 5 },
            AnchoredSearchValidationRefusalV3::ExitRungMismatch { fold: 5 },
        ),
        (
            AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold: usize::MAX },
            AnchoredSearchValidationRefusalV3::ProvenanceMismatch { fold: usize::MAX },
        ),
        (
            AnchoredAdmissionValidationRefusalV2::AggregateOosOverflow,
            AnchoredSearchValidationRefusalV3::AggregateOosOverflow,
        ),
        (
            AnchoredAdmissionValidationRefusalV2::SealMismatch,
            AnchoredSearchValidationRefusalV3::SealMismatch,
        ),
    ] {
        assert_eq!(AnchoredSearchValidationRefusalV3::from(from), to);
    }
    let grid = ExitGridErrorV1::MissingSeriesIdentity("feed");
    assert_eq!(
        AnchoredSearchValidationRefusalV4::from(grid.clone()),
        AnchoredSearchValidationRefusalV4::ExitGrid(grid)
    );
}

/// THE V4 WALK READS BACK WHAT IT ISSUED: its visible folds and the full
/// grid identity its projection carries.
#[test]
fn the_v4_walk_reads_back_its_folds_and_grid_identity() {
    let walk = anchored_search_fixture_v4();
    assert!(!walk.validated().folds.is_empty());
    assert_eq!(walk.validated(), &walk.validated);
    let projection = walk
        .search_authority_projection()
        .expect("the fixture reconciles");
    assert_eq!(projection.grid_identity().digest(), walk.full_grid_digest);
    assert_ne!(walk.full_grid_digest, [0; 32]);
}

/// Every cross-wire of `table` is refused by `project` with the refusal
/// `expect` names.
fn assert_crosswires<T: Clone, E: PartialEq + std::fmt::Debug>(
    base: &T,
    folds: &[AnchoredAdmissionFoldV2],
    table: &[Crosswire<T>],
    project: impl Fn(&T) -> Result<(), E>,
    refusal: impl Fn(Expect, usize, usize) -> E,
) {
    assert_eq!(project(base), Ok(()));
    let (decided, undecided) = decided_and_undecided(folds);
    for (what, tamper, expect) in table {
        let mut moved = base.clone();
        tamper(&mut moved, decided, undecided);
        assert_eq!(
            project(&moved),
            Err(refusal(*expect, decided, undecided)),
            "{what}"
        );
    }
}

/// EVERY SEAL AND PROVENANCE CROSS-WIRE OF THE V2 ADMISSION WALK IS REFUSED,
/// each by the fold it names; an undecided fold may name nothing.
#[test]
fn every_v2_seal_and_provenance_crosswire_is_refused_by_name() {
    let base = anchored_admission_fixture_v2();
    let table = crosswires!(
        AnchoredAdmissionValidationV2,
        hash_validation_family_v2,
        hash_walk_facts_v2
    );
    assert_crosswires(
        &base,
        &base.folds,
        &table,
        |value| value.search_authority_projection().map(|_| ()),
        |expect, decided, undecided| match expect {
            Expect::Seal => AnchoredAdmissionValidationRefusalV2::SealMismatch,
            Expect::Decided => {
                AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold: decided }
            }
            Expect::Undecided => {
                AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold: undecided }
            }
        },
    );
}

/// EVERY SEAL AND PROVENANCE CROSS-WIRE OF THE V3 SEARCH WALK IS REFUSED, and
/// so are a zero rung count and a side rule or execution mode it never ran.
#[test]
fn every_v3_seal_and_provenance_crosswire_is_refused_by_name() {
    let base = anchored_search_fixture_v3();
    let table = crosswires!(
        AnchoredSearchValidationV3,
        hash_validation_family_v3,
        hash_walk_facts_v3
    );
    assert_crosswires(
        &base,
        &base.folds,
        &table,
        |value| value.search_authority_projection().map(|_| ()),
        |expect, decided, undecided| match expect {
            Expect::Seal => AnchoredSearchValidationRefusalV3::SealMismatch,
            Expect::Decided => {
                AnchoredSearchValidationRefusalV3::ProvenanceMismatch { fold: decided }
            }
            Expect::Undecided => {
                AnchoredSearchValidationRefusalV3::ProvenanceMismatch { fold: undecided }
            }
        },
    );
    let mut zero = base.clone();
    zero.policy.resolved_rungs = 0;
    assert_eq!(
        zero.search_authority_projection().map(|_| ()),
        Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs)
    );
    let mut rule = base.clone();
    rule.policy.training_edge_side_rule = rule.policy.training_edge_side_rule.wrapping_add(1);
    let mut mode = base;
    mode.policy.execution_mode = mode.policy.execution_mode.wrapping_add(1);
    for (what, moved) in [("side rule", rule), ("execution mode", mode)] {
        assert_eq!(
            moved.search_authority_projection().map(|_| ()),
            Err(AnchoredSearchValidationRefusalV3::SealMismatch),
            "{what}"
        );
    }
}

/// The V4 fixture with its family and walk digests recomputed, so a moved
/// PROOF field reaches the per-fold comparison instead of the seal.
fn reseal_v4(value: &mut super::AnchoredSearchValidationV4) {
    value.validation_family_digest =
        super::hash_validation_family_v4(&value.folds).expect("the family hashes");
    value.walk_facts_digest = super::hash_walk_facts_v4(
        value.validation_policy_digest,
        value.full_grid_digest,
        value.validation_family_digest,
        &value.folds,
    );
}

/// The decided fold's choice, and its candidate ordinal.
fn choice_v4(value: &mut super::AnchoredSearchValidationV4) -> &mut super::AnchoredSearchChoiceV4 {
    value
        .folds
        .first_mut()
        .and_then(|fold| fold.choice.as_mut())
        .expect("the V4 fixture decides its fold")
}

/// One V4 cross-wire: what it moves, the move, and whether the whole-walk
/// seal (true) or fold 0's provenance (false) must refuse it.
type CrosswireV4 = (
    &'static str,
    fn(&mut super::AnchoredSearchValidationV4),
    bool,
);

/// EVERY SEAL, FOLD, CANDIDATE AND CHOICE CROSS-WIRE OF THE V4 SEARCH WALK IS
/// REFUSED, each by name: the walk's own seal, the fold's window and counts,
/// the candidate vectors against the visible ones, the sealed choice against
/// its own candidate, and the visible choice against the sealed one.
#[test]
fn every_v4_seal_fold_candidate_and_choice_crosswire_is_refused_by_name() {
    let base = anchored_search_fixture_v4();
    assert!(base.search_authority_projection().is_ok());
    let table: [CrosswireV4; 41] = [
        (
            "an unknown candidate order",
            |v| v.policy.semantic_order = v.policy.semantic_order.wrapping_add(1),
            true,
        ),
        (
            "one policy digest for both sides",
            |v| v.policy.full_long_policy_digest = v.policy.full_short_policy_digest,
            true,
        ),
        (
            "one resolution digest for both sides",
            |v| v.policy.full_long_resolution_digest = v.policy.full_short_resolution_digest,
            true,
        ),
        (
            "a refused visible walk",
            |v| v.validated.refused = Some("refused".to_owned()),
            true,
        ),
        (
            "no fold at all",
            |v| {
                v.folds.clear();
                v.validated.folds.clear();
            },
            true,
        ),
        (
            "one more visible fold than proofs",
            |v| {
                let extra = v.validated.folds[0].clone();
                v.validated.folds.push(extra);
            },
            true,
        ),
        (
            "a resealed proof at another position",
            |v| {
                v.folds[0].index += 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a visible fold at another position",
            |v| v.validated.folds[0].index += 1,
            false,
        ),
        (
            "a resealed training window that does not start at bar 0",
            |v| {
                v.folds[0].train_start = 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a visible training window one bar longer",
            |v| v.validated.folds[0].train_bars += 1,
            false,
        ),
        (
            "a visible test window one bar longer",
            |v| v.validated.folds[0].test_bars += 1,
            false,
        ),
        (
            "one more visible purged bar",
            |v| v.validated.folds[0].purged += 1,
            false,
        ),
        (
            "one more visible considered mask",
            |v| v.validated.folds[0].considered += 1,
            false,
        ),
        (
            "one more visible priced mask",
            |v| v.validated.folds[0].priced += 1,
            false,
        ),
        (
            "a visible halt",
            |v| {
                v.validated.folds[0].halted =
                    engine::Ladder::with_min_hits(1).refused_on_memory(1).halted;
            },
            false,
        ),
        (
            "a resealed selection count that is not the candidates'",
            |v| {
                v.folds[0].selected_candidates += 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "one more visible in-sample cell than candidates",
            |v| v.validated.folds[0].in_sample_all.push(0),
            false,
        ),
        (
            "one more visible out-of-sample cell than candidates",
            |v| v.validated.folds[0].out_of_sample_all.push(0),
            false,
        ),
        (
            "a visible in-sample summary that is not the proof's",
            |v| v.validated.folds[0].in_sample.trades += 1,
            false,
        ),
        (
            "a visible out-of-sample summary that is not the proof's",
            |v| v.validated.folds[0].out_of_sample.trades += 1,
            false,
        ),
        (
            "a visible in-sample cell that is not its candidate's",
            |v| v.validated.folds[0].in_sample_all[0] += 1,
            false,
        ),
        (
            "a sealed choice of another combination",
            |v| {
                let choice = choice_v4(v);
                choice.mask = choice.mask.with_bit(383);
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice on another side than its candidate",
            |v| {
                let at = usize::try_from(choice_v4(v).ordinal).expect("an index");
                choice_v4(v).side = Direction::Short;
                v.folds[0].candidates[at].side = Direction::Long;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice of another coordinate",
            |v| {
                choice_v4(v).coordinate.stop = Some(usize::MAX);
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another resolution",
            |v| {
                choice_v4(v).resolution_digest[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another evaluation",
            |v| {
                choice_v4(v).evaluation_digest[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another selection",
            |v| {
                choice_v4(v).selection_digest[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another training run",
            |v| {
                choice_v4(v).training_run_id[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another replay",
            |v| {
                choice_v4(v).replay_digest[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another out-of-sample run",
            |v| {
                choice_v4(v).oos_run_id[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another training cell",
            |v| {
                choice_v4(v).training_cell_digest[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another out-of-sample cell",
            |v| {
                choice_v4(v).oos_cell_digest[0] ^= 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a sealed choice with another in-sample total",
            |v| {
                choice_v4(v).in_sample_pessimistic += 1;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a visible combination that is not the choice's",
            |v| {
                let mask = choice_v4(v).mask;
                v.validated.folds[0].chosen = Some(mask.with_bit(383));
            },
            false,
        ),
        (
            "no visible side for a decided fold",
            |v| v.validated.folds[0].chosen_side = None,
            false,
        ),
        (
            "no visible exit for a decided fold",
            |v| v.validated.folds[0].chosen_exit = None,
            false,
        ),
        (
            "a visible out-of-sample exit that is not the choice's",
            |v| {
                let total = choice_v4(v).out_of_sample_pessimistic;
                v.validated.folds[0].out_of_sample_exit = Some(total + 1);
            },
            false,
        ),
        (
            "a resealed better candidate than the one chosen",
            |v| {
                let fold = &mut v.folds[0];
                let mut better = fold.candidates[0];
                better.in_sample_pessimistic = fold
                    .candidates
                    .iter()
                    .map(|candidate| candidate.in_sample_pessimistic)
                    .max()
                    .expect("candidates")
                    + 1;
                fold.candidates.push(better);
                fold.selected_candidates += 1;
                let visible = &mut v.validated.folds[0];
                visible.in_sample_all.push(better.in_sample_pessimistic);
                visible.out_of_sample_all.push(
                    better
                        .out_of_sample_pessimistic
                        .expect("an out-of-sample total"),
                );
                reseal_v4(v);
            },
            false,
        ),
        (
            "a resealed choice past the last candidate",
            |v| {
                choice_v4(v).ordinal = u64::MAX;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a resealed undecided fold that kept its candidates",
            |v| {
                v.folds[0].choice = None;
                reseal_v4(v);
            },
            false,
        ),
        (
            "a resealed undecided fold whose visible fold still chose",
            |v| {
                let fold = &mut v.folds[0];
                fold.choice = None;
                fold.candidates.clear();
                fold.selected_candidates = 0;
                v.validated.folds[0].in_sample_all.clear();
                v.validated.folds[0].out_of_sample_all.clear();
                reseal_v4(v);
            },
            false,
        ),
    ];
    for (what, tamper, seal) in table {
        let mut moved = base.clone();
        tamper(&mut moved);
        let want = if seal {
            AnchoredSearchValidationRefusalV4::SealMismatch
        } else {
            AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold: 0 }
        };
        assert_eq!(
            moved.search_authority_projection().map(|_| ()),
            Err(want),
            "{what}"
        );
    }
}

/// AN UNDECIDED V4 FOLD MAY NAME NOTHING: each visible trace of a choice is
/// refused on its own, and the fold that names nothing reconciles, counted as
/// undecided.
#[test]
fn an_undecided_v4_fold_is_refused_for_each_visible_trace_of_a_choice() {
    let mut undecided = anchored_search_fixture_v4();
    let fold = &mut undecided.folds[0];
    fold.choice = None;
    fold.candidates.clear();
    fold.selected_candidates = 0;
    let visible = &mut undecided.validated.folds[0];
    visible.in_sample_all.clear();
    visible.out_of_sample_all.clear();
    let decided = visible.clone();
    visible.chosen = None;
    visible.chosen_side = None;
    visible.chosen_exit = None;
    visible.chosen_exit_total = None;
    visible.out_of_sample_exit = None;
    reseal_v4(&mut undecided);
    // Each trace of the decided fold's visible choice, put back on its own.
    let traces: [Trace; 5] = [
        ("a combination", |v, d| v.chosen = d.chosen),
        ("a side", |v, d| v.chosen_side = d.chosen_side),
        ("an exit", |v, d| v.chosen_exit = d.chosen_exit),
        ("an exit total", |v, d| {
            v.chosen_exit_total = d.chosen_exit_total;
        }),
        ("an out-of-sample exit", |v, d| {
            v.out_of_sample_exit = d.out_of_sample_exit;
        }),
    ];
    for (trace, restore) in traces {
        let mut named = undecided.clone();
        restore(&mut named.validated.folds[0], &decided);
        assert_eq!(
            named.search_authority_projection().map(|_| ()),
            Err(AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold: 0 }),
            "an undecided fold that still names {trace}"
        );
    }
    let projection = undecided
        .search_authority_projection()
        .expect("a fold that names nothing reconciles");
    assert_eq!(projection.decided_folds(), 0);
    assert_eq!(projection.fold_count(), 1);
}

/// A decided fold's visible record for the fixture candidate (in-sample 7)
/// with an out-of-sample total of 3.
fn decided_visible() -> super::FoldResult {
    let candidate = super::tests::admission_candidate_v2();
    super::FoldResult {
        considered: 1,
        priced: 1,
        chosen: Some(candidate.mask),
        chosen_side: Some(Direction::Long),
        chosen_exit: Some(candidate.pick.rungs),
        chosen_exit_total: Some(candidate.pessimistic),
        out_of_sample_exit: Some(3),
        in_sample_all: vec![candidate.pessimistic],
        out_of_sample_all: vec![3],
        ..super::FoldResult::default()
    }
}

/// The same fold, undecided.
fn undecided_visible() -> super::FoldResult {
    super::FoldResult {
        chosen: None,
        chosen_side: None,
        chosen_exit: None,
        chosen_exit_total: None,
        out_of_sample_exit: None,
        ..decided_visible()
    }
}

const OOS: [super::CandidateOosV2; 1] = [super::CandidateOosV2 {
    pessimistic: Some(3),
}];

/// The fold observed by a fresh Admission V2 capture over the fixture
/// candidate.
fn observe_v2(
    visible: &super::FoldResult,
    oos: &[super::CandidateOosV2],
    chosen: Option<usize>,
) -> Result<(), AnchoredAdmissionValidationRefusalV2> {
    super::tests::admission_capture_v2().observe_fold(
        &super::tests::admission_split_v2(),
        visible,
        &[super::tests::admission_candidate_v2()],
        oos,
        chosen,
    )
}

/// A CAPTURED FOLD REFUSES EVERY FAMILY AND PROVENANCE CROSS-WIRE by its
/// fold: a halt, unpriced or unaligned candidates, a choice past the last
/// candidate, a visible choice that is not the sealed one, and an undecided
/// fold that names any part of a choice.
#[test]
fn a_captured_fold_refuses_every_family_and_provenance_crosswire() {
    assert_eq!(observe_v2(&decided_visible(), &OOS, Some(0)), Ok(()));
    assert_eq!(observe_v2(&undecided_visible(), &OOS, None), Ok(()));
    let family = AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold: 0 };
    let provenance = AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold: 0 };
    // Each edit, and whether the candidate family (true) or the provenance
    // (false) refuses it.
    let decided: [FoldEdit; 10] = [
        (
            "a halted fold",
            |v| v.halted = engine::Ladder::with_min_hits(1).refused_on_memory(1).halted,
            true,
        ),
        (
            "a candidate considered and not priced",
            |v| v.considered = 2,
            true,
        ),
        (
            "no in-sample cell for the candidate",
            |v| v.in_sample_all.clear(),
            true,
        ),
        (
            "no out-of-sample cell for the candidate",
            |v| v.out_of_sample_all.clear(),
            true,
        ),
        (
            "a visible combination that is not the candidate's",
            |v| v.chosen = v.chosen.map(|mask| mask.with_bit(383)),
            false,
        ),
        ("no visible side", |v| v.chosen_side = None, false),
        ("no visible exit", |v| v.chosen_exit = None, false),
        (
            "a visible exit total that is not the candidate's",
            |v| v.chosen_exit_total = Some(8),
            false,
        ),
        (
            "a visible out-of-sample exit that is not the outcome's",
            |v| v.out_of_sample_exit = Some(4),
            false,
        ),
        (
            "a visible in-sample cell that is not the candidate's",
            |v| v.in_sample_all = vec![8],
            false,
        ),
    ];
    for (what, tamper, is_family) in decided {
        let mut visible = decided_visible();
        tamper(&mut visible);
        let want = if is_family { &family } else { &provenance };
        assert_eq!(
            observe_v2(&visible, &OOS, Some(0)),
            Err(want.clone()),
            "{what}"
        );
    }
    assert_eq!(
        observe_v2(&decided_visible(), &OOS, Some(1)),
        Err(provenance.clone()),
        "a choice past the last candidate"
    );
    assert_eq!(
        observe_v2(&decided_visible(), &[], Some(0)),
        Err(family.clone()),
        "no out-of-sample row for the candidate"
    );
    // The candidate hash refuses unaligned rows before the fold is captured,
    // so the capture's own alignment arm is read directly.
    assert_eq!(
        super::capture_anchored_fold(
            &super::tests::admission_split_v2(),
            &decided_visible(),
            &[super::tests::admission_candidate_v2()],
            &[],
            Some(0),
            [0; 32],
        ),
        Err(family)
    );
    let undecided: [VisibleEdit; 5] = [
        ("a combination", |v| v.chosen = Some(ConditionMask::ZERO)),
        ("a side", |v| v.chosen_side = Some(Direction::Long)),
        ("an exit", |v| {
            v.chosen_exit = Some(crate::grid::Chosen::default());
        }),
        ("an exit total", |v| v.chosen_exit_total = Some(7)),
        ("an out-of-sample exit", |v| v.out_of_sample_exit = Some(3)),
    ];
    for (what, tamper) in undecided {
        let mut visible = undecided_visible();
        tamper(&mut visible);
        assert_eq!(
            observe_v2(&visible, &OOS, None),
            Err(provenance.clone()),
            "an undecided fold that names {what}"
        );
    }
}

/// THE SEALED CHOICE MUST BE THE VISIBLE ONE AND ITS OWN CANDIDATE'S, and the
/// best: each comparison refuses on its own, read directly because the
/// capture always builds the choice from that candidate.
#[test]
fn a_choice_that_is_not_its_candidates_or_not_the_best_does_not_match() {
    let candidate = super::tests::admission_candidate_v2();
    let choice = super::AnchoredAdmissionChoiceV2 {
        ordinal: 0,
        mask: candidate.mask,
        side: candidate.side,
        exit: candidate.pick.rungs,
        exit_values: super::AnchoredAdmissionExitValuesV2::default(),
        in_sample_pessimistic: candidate.pessimistic,
        out_of_sample_pessimistic: 3,
    };
    let scored = [super::tests::admission_candidate_v2()];
    assert!(super::choice_matches_visible_v2(
        &choice,
        &decided_visible(),
        &scored
    ));
    // A visible fold that agrees with each moved choice, so only the
    // candidate comparison can disagree.
    let other_mask = candidate.mask.with_bit(383);
    let other_exit = crate::grid::Chosen {
        stop: Some(usize::MAX),
        ..candidate.pick.rungs
    };
    for (what, moved, visible) in [
        (
            "another combination",
            super::AnchoredAdmissionChoiceV2 {
                mask: other_mask,
                ..choice
            },
            super::FoldResult {
                chosen: Some(other_mask),
                ..decided_visible()
            },
        ),
        (
            "another side",
            super::AnchoredAdmissionChoiceV2 {
                side: Direction::Short,
                ..choice
            },
            super::FoldResult {
                chosen_side: Some(Direction::Short),
                ..decided_visible()
            },
        ),
        (
            "another exit",
            super::AnchoredAdmissionChoiceV2 {
                exit: other_exit,
                ..choice
            },
            super::FoldResult {
                chosen_exit: Some(other_exit),
                ..decided_visible()
            },
        ),
        (
            "another in-sample total",
            super::AnchoredAdmissionChoiceV2 {
                in_sample_pessimistic: 8,
                ..choice
            },
            super::FoldResult {
                chosen_exit_total: Some(8),
                in_sample_all: vec![8],
                ..decided_visible()
            },
        ),
        (
            "a candidate that does not exist",
            super::AnchoredAdmissionChoiceV2 {
                ordinal: 1,
                ..choice
            },
            super::FoldResult {
                in_sample_all: vec![7, 7],
                ..decided_visible()
            },
        ),
    ] {
        assert!(
            !super::choice_matches_visible_v2(&moved, &visible, &scored),
            "{what}"
        );
    }
    // COMBINED: the visible fold and the choice agree, and a later candidate
    // scored better than the one chosen.
    let better = super::Scored {
        pessimistic: 9,
        ..super::tests::admission_candidate_v2()
    };
    let two = [super::tests::admission_candidate_v2(), better];
    let visible = super::FoldResult {
        considered: 2,
        priced: 2,
        in_sample_all: vec![7, 9],
        out_of_sample_all: vec![3, 3],
        ..decided_visible()
    };
    assert!(!super::choice_matches_visible_v2(&choice, &visible, &two));
}

/// A CAPTURE THAT REFUSES A FOLD RETAINS THE REFUSAL, for either door, and a
/// finished capture answers it before anything else it was given.
#[test]
fn a_refused_fold_is_retained_and_answered_by_finish() {
    let mut halted = decided_visible();
    halted.halted = engine::Ladder::with_min_hits(1).refused_on_memory(1).halted;
    let split = super::tests::admission_split_v2();
    let candidate = [super::tests::admission_candidate_v2()];
    let mut v2 = super::tests::admission_capture_v2();
    assert!(
        !super::AnchoredCapture::AdmissionV2(&mut v2).observe_fold_and_retain(
            &split,
            &halted,
            &candidate,
            &OOS,
            Some(0)
        )
    );
    let family = AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold: 0 };
    assert_eq!(v2.refusal, Some(family.clone()));
    assert_eq!(v2.finish(super::Validated::default()), Err(family));
    let sweeper = super::tests::sweeper();
    let v3_capture = || {
        super::AnchoredSearchCaptureV3::new(2, super::tests::h(1), 1, &sweeper, 1, 60_000_000)
            .expect("a valid V3 capture policy")
    };
    let mut v3 = v3_capture();
    assert!(
        !super::AnchoredCapture::SearchV3(&mut v3).observe_fold_and_retain(
            &split,
            &halted,
            &candidate,
            &OOS,
            Some(0)
        )
    );
    let family = AnchoredSearchValidationRefusalV3::CandidateFamilyMismatch { fold: 0 };
    assert_eq!(v3.refusal, Some(family.clone()));
    assert_eq!(v3.finish(super::Validated::default()), Err(family.clone()));
    // The V3 candidate hash refuses unaligned candidate rows on its own.
    assert_eq!(
        v3_capture().observe_fold(&split, &decided_visible(), &candidate, &[], Some(0)),
        Err(family)
    );
    // A fresh capture answers an upstream refusal, an empty walk, and a walk
    // whose visible folds are not the ones it captured.
    let refused = || super::Validated {
        refused: Some("upstream".to_owned()),
        ..super::Validated::default()
    };
    let one_fold = || super::Validated {
        folds: vec![super::FoldResult::default()],
        refused: None,
    };
    assert_eq!(
        super::tests::admission_capture_v2().finish(refused()),
        Err(AnchoredAdmissionValidationRefusalV2::UpstreamRefused(
            "upstream".to_owned()
        ))
    );
    assert_eq!(
        super::tests::admission_capture_v2().finish(super::Validated::default()),
        Err(AnchoredAdmissionValidationRefusalV2::Empty)
    );
    assert_eq!(
        super::tests::admission_capture_v2().finish(one_fold()),
        Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
    );
    assert_eq!(
        v3_capture().finish(refused()),
        Err(AnchoredSearchValidationRefusalV3::UpstreamRefused(
            "upstream".to_owned()
        ))
    );
    assert_eq!(
        v3_capture().finish(super::Validated::default()),
        Err(AnchoredSearchValidationRefusalV3::Empty)
    );
    assert_eq!(
        v3_capture().finish(one_fold()),
        Err(AnchoredSearchValidationRefusalV3::SealMismatch)
    );
    // Zero resolved rungs is refused by both capture policies.
    assert_eq!(
        super::AnchoredAdmissionCaptureV2::new(
            2,
            super::tests::h(1),
            1,
            Direction::Long,
            &sweeper,
            0,
            60_000_000
        )
        .map(|_| ()),
        Err(AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs)
    );
    assert_eq!(
        super::AnchoredSearchCaptureV3::new(2, super::tests::h(1), 1, &sweeper, 0, 60_000_000)
            .map(|_| ()),
        Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs)
    );
}

/// A TRAILING TAKE-PROFIT READS ITS ARM ON THE TARGET LADDER AND ITS TRAIL ON
/// THE TRAIL LADDER, and an index past either ladder is refused by its fold.
#[test]
fn a_trailing_take_profit_reads_its_two_rungs_and_refuses_one_past_the_ladder() {
    let ladder =
        |rungs| crate::excursion::Ladder::new(rungs).expect("an increasing positive ladder");
    let pick = |arm, trail| super::ExitPick {
        rungs: crate::grid::Chosen {
            ttp: Some(crate::grid::Ttp { arm, trail }),
            ..crate::grid::Chosen::default()
        },
        stops: ladder(vec![10]),
        targets: ladder(vec![20, 40]),
        trails: ladder(vec![5]),
    };
    let values = super::selected_exit_values_v2(&pick(1, 0), 4).expect("both rungs exist");
    assert_eq!((values.ttp_arm, values.ttp_trail), (Some(40), Some(5)));
    assert_eq!((values.stop, values.target, values.tsl), (None, None, None));
    for (arm, trail) in [(2, 0), (0, 1), (usize::MAX, usize::MAX)] {
        assert_eq!(
            super::selected_exit_values_v2(&pick(arm, trail), 4).map(|_| ()),
            Err(AnchoredAdmissionValidationRefusalV2::ExitRungMismatch { fold: 4 }),
            "{arm} {trail}"
        );
    }
}

/// EACH OF THE EIGHT SIGNAL LENGTHS HAS ITS CANONICAL LABEL, and no other
/// length has one.
#[test]
fn each_signal_length_names_its_label_and_no_other_length_does() {
    const MINUTE: i64 = 60_000_000;
    for (minutes, label) in [
        (1, "1min"),
        (2, "2min"),
        (3, "3min"),
        (5, "5min"),
        (10, "10min"),
        (15, "15min"),
        (30, "30min"),
        (60, "60min"),
    ] {
        assert_eq!(super::signal_timeframe_v4(minutes * MINUTE), Some(label));
    }
    for length in [
        0,
        -MINUTE,
        4 * MINUTE,
        MINUTE + 1,
        1440 * MINUTE,
        i64::MAX,
        i64::MIN,
    ] {
        assert_eq!(super::signal_timeframe_v4(length), None, "{length}");
    }
}

/// A plain signal column folded from `bars`.
fn built(bars: &[indicators::Candle]) -> indicators::column::Column {
    indicators::column::Column::build(bars, &mut super::tests::evaluator())
}

/// THE V4 DOOR AUTHENTICATES ITS SIGNAL COLUMN'S SHAPE: no signal, signal
/// bars out of order, a column already projected onto fills, and a column
/// folded from fewer bars than the series are each refused.
#[test]
fn a_signal_column_that_does_not_cover_its_series_is_refused_by_shape() {
    let bars = crate::synthetic::sessions(4);
    let column = built(&bars);
    assert_eq!(
        super::authenticate_signal_column_shape_v4(&bars, &column),
        Ok(())
    );
    let refused = Err(AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
        "the prepared signal column does not exactly cover its ordered signal series",
    ));
    assert_eq!(
        super::authenticate_signal_column_shape_v4(&[], &built(&[])),
        refused
    );
    let mut repeated = bars.clone();
    repeated[1].ts_micros = repeated[0].ts_micros;
    assert_eq!(
        super::authenticate_signal_column_shape_v4(&repeated, &column),
        refused
    );
    let width = column.len();
    let (fill, _) = column
        .reproject(&vec![None; width], width)
        .expect("a parallel map");
    assert_eq!(
        super::authenticate_signal_column_shape_v4(&bars, &fill),
        refused
    );
    let shorter = built(&bars[..bars.len() - 1]);
    assert_eq!(
        super::authenticate_signal_column_shape_v4(&bars, &shorter),
        refused
    );
}

/// A PREPARED PREFIX MUST BE THE RETAINED FULL COLUMN'S OWN CAUSAL PREFIX:
/// a prefix shorter than the one before, a cursor already past the full
/// column, a full column shorter than the prefix, and a rebuilt prefix whose
/// sources, bits, evaluator policy or acceptance alone differ from the full
/// column's are each refused by name.
#[test]
fn a_prepared_prefix_that_is_not_the_full_columns_prefix_is_refused() {
    let bars = crate::synthetic::sessions(7);
    let full = built(&bars);
    let first = *full
        .sources()
        .first()
        .expect("seven sessions warm the evaluator");
    let cut = first + 20;
    let prefix = &bars[..cut];
    let mismatch = AnchoredSearchValidationRefusalV4::SignalSourceMismatch;
    let mut cursor = super::PreparedSignalPrefixCursor::default();
    assert_eq!(cursor.authenticate(prefix, &built(prefix), &full), Ok(()));
    assert!(cursor.end > 0, "the prefix holds warm rows");
    let shorter = &bars[..cut - 1];
    assert_eq!(
        cursor.authenticate(shorter, &built(shorter), &full),
        Err(mismatch(
            "prepared signal-prefix boundaries are not monotonic"
        ))
    );
    let mut past = super::PreparedSignalPrefixCursor {
        end: full.sources().len() + 1,
        last_signal_bars: None,
    };
    assert_eq!(
        past.authenticate(prefix, &built(prefix), &full),
        Err(mismatch(
            "prepared signal-prefix boundaries are not monotonic"
        ))
    );
    let short_full = built(shorter);
    assert_eq!(
        super::PreparedSignalPrefixCursor::default().authenticate(
            prefix,
            &built(prefix),
            &short_full
        ),
        Err(mismatch(
            "the retained full signal acceptance is shorter than a causal prefix"
        ))
    );
    let differs = Err(mismatch(
        "a rebuilt causal signal prefix differs from the retained full prepared column",
    ));
    let fresh = |prefix: &[indicators::Candle], prepared: &indicators::column::Column| {
        super::PreparedSignalPrefixCursor::default().authenticate(prefix, prepared, &full)
    };
    let honest = built(prefix);
    // A corrupt warm bar drops its row: the sources differ.
    let mut corrupt = prefix.to_vec();
    corrupt[first + 5].high = corrupt[first + 5].low - 1;
    let dropped = built(&corrupt);
    assert!(!dropped.sources().contains(&(first + 5)));
    assert_eq!(fresh(&corrupt, &dropped), differs);
    // Cleared rows keep every source and lose their bits.
    let mut cleared = built(prefix);
    cleared.clear_before(cut);
    assert_eq!(cleared.sources(), honest.sources());
    assert_ne!(cleared.bits(), honest.bits());
    assert_eq!(fresh(prefix, &cleared), differs);
    // Another calendar is another evaluator policy with the same truth here:
    // no synthetic day is one of the charter's non-regular days.
    let other = indicators::column::Column::build(
        prefix,
        &mut indicators::evaluator::Evaluator::with_calendar(
            indicators::evaluator::Widths::pinned().expect("pinned widths are valid"),
            indicators::vwap::Availability::Absent,
            indicators::pattern::Thresholds::CLASSICAL,
            indicators::evaluator::Calendar::all_regular(),
        ),
    );
    assert_eq!(
        (other.sources(), other.bits()),
        (honest.sources(), honest.bits())
    );
    assert_ne!(
        other.evaluation_spec_token(),
        honest.evaluation_spec_token()
    );
    assert_eq!(fresh(prefix, &other), differs);
    // A corrupt bar before the evaluator warms changes only the acceptance.
    let mut cold = bars[..first].to_vec();
    let last = cold.len() - 1;
    cold[last].high = cold[last].low - 1;
    let cold_column = built(&cold);
    assert!(cold_column.is_empty(), "no row before the evaluator warms");
    assert_eq!(fresh(&cold, &cold_column), differs);
}

/// A FOLD THAT CANNOT BE PROJECTED IS REFUSED BY NAME, never replaced: a
/// signal length of zero cannot align, a column with no evaluator cannot be
/// re-evaluated over execution bars, an OOS boundary past its series, and a
/// projected row before the OOS execution boundary.
#[test]
fn a_fold_that_cannot_be_projected_is_refused_by_name() {
    const MINUTE: i64 = 60_000_000;
    let bars = crate::synthetic::sessions(6);
    let column = built(&bars);
    assert!(!column.is_empty(), "six sessions warm the evaluator");
    assert!(super::project_fold(&bars, &column, &bars, MINUTE).is_ok());
    assert!(super::project_fold(&bars, &column, &bars, 0).is_err());
    let blank = indicators::column::Column::default();
    assert!(super::project_fold(&bars, &blank, &bars, MINUTE).is_err());
    assert!(super::project_oos_fold(&bars, &column, &bars, MINUTE, 0, 0).is_ok());
    for (what, signal_at, execution_at, length) in [
        (
            "a signal boundary past the series",
            bars.len() + 1,
            0,
            MINUTE,
        ),
        (
            "an execution boundary past the series",
            0,
            bars.len() + 1,
            MINUTE,
        ),
        ("a signal length of zero", 0, 0, 0),
        (
            "every projected row before the boundary",
            0,
            bars.len(),
            MINUTE,
        ),
    ] {
        assert!(
            super::project_oos_fold(&bars, &column, &bars, length, signal_at, execution_at)
                .is_err(),
            "{what}"
        );
    }
    assert!(super::project_oos_fold(&bars, &blank, &bars, MINUTE, 0, 0).is_err());
}

/// THE V3 DOOR REFUSES ZERO RESOLVED RUNGS BEFORE IT RUNS ANY SEARCH, and the
/// V4 door refuses an execution series with a stamp off the minute grid.
#[test]
fn the_search_doors_refuse_zero_rungs_and_a_coarse_execution_path() {
    let bars = crate::synthetic::sessions(2);
    let called = std::cell::Cell::new(false);
    let mut builder = |slice: &[indicators::Candle]| {
        called.set(true);
        Ok(built(slice))
    };
    assert_eq!(
        super::walk_forward_projected_prepared_anchored_search_v3(
            &bars,
            super::ExecutionSeries {
                bars: &bars,
                signal_length_micros: 60_000_000,
            },
            super::tests::h(15),
            3,
            &super::tests::sweeper(),
            &mut builder,
            0,
        )
        .map(|_| ()),
        Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs)
    );
    assert!(!called.get(), "no fold is built for a refused policy");
    let instrument = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .expect("NSE-NIFTY is an allowed index");
    let resolve = |side, bars: &[indicators::Candle]| {
        let series = crate::exit_grid_policy::ExecutionSeriesV1::new(
            &instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            bars,
        )
        .expect("complete identity");
        super::tests::exact_grid_policy_v4(side)
            .resolve_attested(series)
            .expect("the grid resolves")
    };
    let long = resolve(crate::excursion::Side::Long, &bars);
    let short = resolve(crate::excursion::Side::Short, &bars);
    // A bar stamped half a minute off the grid: a whole missing minute is a
    // legal gap, an off-grid stamp is not.
    let mut coarse = bars.clone();
    let middle = coarse.len() / 2;
    coarse[middle].ts_micros += 30_000_000;
    let execution = crate::exit_grid_policy::ExecutionSeriesV1::new(
        &instrument,
        "test-feed",
        "test-commit",
        [0xA5; 32],
        &coarse,
    )
    .expect("complete identity");
    assert_eq!(
        super::walk_forward_projected_prepared_anchored_search_v4(
            &bars,
            &built(&bars),
            execution,
            60_000_000,
            super::tests::h(15),
            1,
            &super::tests::sweeper(),
            &mut builder,
            &long,
            &short,
        )
        .map(|_| ()),
        Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "execution is not a non-empty exact one-minute path"
        ))
    );
}

/// The V4 grid pair resolved over `bars` under one identity, both sides.
fn grid_pair_v4(
    instrument: &brutex_core::instrument::InstrumentKey,
    feed: &str,
    commit: &str,
    calendar: [u8; 32],
    bars: &[indicators::Candle],
) -> (
    crate::exit_grid_policy::ResolvedExitGridV1,
    crate::exit_grid_policy::ResolvedExitGridV1,
) {
    let resolve = |side| {
        let series = crate::exit_grid_policy::ExecutionSeriesV1::new(
            instrument, feed, commit, calendar, bars,
        )
        .expect("complete identity");
        super::tests::exact_grid_policy_v4(side)
            .resolve_attested(series)
            .expect("the grid resolves")
    };
    (
        resolve(crate::excursion::Side::Long),
        resolve(crate::excursion::Side::Short),
    )
}

/// THE V4 GRID PAIR IS ONE SOURCE SEEN FROM TWO SIDES: a Short in the Short
/// slot, and a pair whose instrument, feed, commit or calendar differ between
/// its sides, or from the attested series, is refused by name.
#[test]
fn a_grid_pair_of_two_sources_or_the_wrong_side_is_refused_by_name() {
    use brutex_core::instrument::{Exchange, InstrumentKey};
    let bars = crate::synthetic::sessions(2);
    let nifty = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("an allowed index");
    let bank = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").expect("an allowed index");
    let series = |instrument| {
        crate::exit_grid_policy::ExecutionSeriesV1::new(
            instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &bars,
        )
        .expect("complete identity")
    };
    let (long, short) = grid_pair_v4(&nifty, "test-feed", "test-commit", [0xA5; 32], &bars);
    let mut moved = bars.clone();
    moved[10].close = moved[10].low;
    assert_ne!(moved, bars);
    assert!(super::authenticate_grid_pair_v4(series(&nifty), &long, &short, false).is_ok());
    assert_eq!(
        super::authenticate_grid_pair_v4(series(&nifty), &long, &long, false),
        Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "Long/Short resolutions are swapped or carry the wrong side"
        ))
    );
    assert_eq!(
        super::authenticate_grid_pair_v4(series(&nifty), &short, &short, false),
        Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "Long/Short resolutions are swapped or carry the wrong side"
        ))
    );
    let differs = Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
        "Long/Short source or training identity differs",
    ));
    for (what, other) in [
        (
            "instrument",
            grid_pair_v4(&bank, "test-feed", "test-commit", [0xA5; 32], &bars).1,
        ),
        (
            "feed",
            grid_pair_v4(&nifty, "other-feed", "test-commit", [0xA5; 32], &bars).1,
        ),
        (
            "commit",
            grid_pair_v4(&nifty, "test-feed", "other-commit", [0xA5; 32], &bars).1,
        ),
        (
            "calendar",
            grid_pair_v4(&nifty, "test-feed", "test-commit", [0x5A; 32], &bars).1,
        ),
        (
            "training bars",
            grid_pair_v4(&nifty, "test-feed", "test-commit", [0xA5; 32], &moved).1,
        ),
    ] {
        assert_eq!(
            super::authenticate_grid_pair_v4(series(&nifty), &long, &other, false),
            differs,
            "{what}"
        );
    }
    assert_eq!(
        super::authenticate_grid_pair_v4(series(&bank), &long, &short, false),
        differs,
        "a pair over another series than the attested one"
    );
}

/// A TORN OR RESEALED GRID PAIR IS REFUSED BY NAME: a side whose digest no
/// longer reproduces, a resealed side whose training span is not its training
/// digest's, and a resealed side whose levels are not what its policy
/// resolves from the attested series.
#[test]
fn a_torn_or_resealed_grid_pair_is_refused_by_name() {
    type Resolved = crate::exit_grid_policy::ResolvedExitGridV1;
    use crate::exit_grid_policy::resolution_tamper::{torn, with_training_span, with_wider_stops};
    let bars = crate::synthetic::sessions(2);
    let nifty = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .expect("an allowed index");
    let series = || {
        crate::exit_grid_policy::ExecutionSeriesV1::new(
            &nifty,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &bars,
        )
        .expect("complete identity")
    };
    let (long, short) = grid_pair_v4(&nifty, "test-feed", "test-commit", [0xA5; 32], &bars);
    let pair = |long: &Resolved, short: &Resolved| {
        super::authenticate_grid_pair_v4(series(), long, short, false)
    };
    assert!(pair(&long, &short).is_ok());
    let torn_pair = Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
        "one resolution or policy digest is torn",
    ));
    assert_eq!(pair(&torn(&long), &short), torn_pair, "long");
    assert_eq!(pair(&long, &torn(&short)), torn_pair, "short");
    let differs = Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
        "Long/Short source or training identity differs",
    ));
    let (count, first, last) = (
        short.training_bars(),
        short.training_first_ts_micros(),
        short.training_last_ts_micros(),
    );
    for (what, span) in [
        ("bars", (count - 1, first, last)),
        ("first", (count, first + 60_000_000, last)),
        ("last", (count, first, last - 60_000_000)),
    ] {
        assert_eq!(
            pair(&long, &with_training_span(&short, span)),
            differs,
            "{what}"
        );
    }
    let stale = Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
        "retained resolution does not exactly match the attested execution series",
    ));
    assert_eq!(pair(&with_wider_stops(&long), &short), stale, "long");
    assert_eq!(pair(&long, &with_wider_stops(&short)), stale, "short");
}

/// THE V4 WALK REFUSES BEFORE IT EVALUATES ANY GRID: a span it cannot split
/// (zero splits), a fold whose search halted at its ceiling, and a fold whose
/// training execution is empty, because the execution series starts after
/// the first test window opens, is refused when its grid is re-resolved.
#[test]
fn the_v4_walk_refuses_no_fold_a_halted_search_and_an_empty_training_execution() {
    let bars = crate::synthetic::sessions(16);
    let column = built(&bars);
    let nifty = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .expect("an allowed index");
    let execution_of = |bars| {
        crate::exit_grid_policy::ExecutionSeriesV1::new(
            &nifty,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            bars,
        )
        .expect("complete identity")
    };
    let (long, short) = grid_pair_v4(&nifty, "test-feed", "test-commit", [0xA5; 32], &bars);
    let complete = crate::Sweeper::new(engine::Ladder::with_min_hits(2_200).with_ceiling(20_000));
    let mut builder = |slice: &[indicators::Candle]| Ok(built(slice));
    assert_eq!(
        super::walk_forward_projected_prepared_anchored_search_v4(
            &bars,
            &column,
            execution_of(&bars),
            60_000_000,
            super::tests::h(15),
            0,
            &complete,
            &mut builder,
            &long,
            &short,
        )
        .map(|_| ()),
        Err(AnchoredSearchValidationRefusalV4::Empty)
    );
    let halting = crate::Sweeper::new(engine::Ladder::with_min_hits(1).with_ceiling(1));
    assert_eq!(
        super::walk_forward_projected_prepared_anchored_search_v4(
            &bars,
            &column,
            execution_of(&bars),
            60_000_000,
            super::tests::h(15),
            1,
            &halting,
            &mut builder,
            &long,
            &short,
        )
        .map(|_| ()),
        Err(AnchoredSearchValidationRefusalV4::IncompleteSearch { fold: 0 })
    );
    let late = &bars[bars.len() - 2 * crate::synthetic::BARS_PER_SESSION..];
    let (late_long, late_short) =
        grid_pair_v4(&nifty, "test-feed", "test-commit", [0xA5; 32], late);
    assert_eq!(
        super::walk_forward_projected_prepared_anchored_search_v4(
            &bars,
            &column,
            execution_of(late),
            60_000_000,
            super::tests::h(15),
            1,
            &complete,
            &mut builder,
            &late_long,
            &late_short,
        )
        .map(|_| ()),
        Err(AnchoredSearchValidationRefusalV4::ExitGrid(
            ExitGridErrorV1::EmptyExecutionSeries
        ))
    );
}

/// A FOLD WITH NO TEST WINDOW IS REFUSED, not walked: with one split over
/// sixteen sessions the fold's test window starts one horizon after its
/// training boundary, and a horizon that spans the rest of the series leaves
/// it empty. Refused as `DiscontiguousTraining` for that fold, before any search.
#[test]
fn a_fold_whose_horizon_spans_the_rest_of_the_series_is_refused() {
    let bars = crate::synthetic::sessions(16);
    let column = built(&bars);
    let nifty = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .expect("an allowed index");
    let execution = crate::exit_grid_policy::ExecutionSeriesV1::new(
        &nifty,
        "test-feed",
        "test-commit",
        [0xA5; 32],
        &bars,
    )
    .expect("complete identity");
    let (long, short) = grid_pair_v4(&nifty, "test-feed", "test-commit", [0xA5; 32], &bars);
    let complete = crate::Sweeper::new(engine::Ladder::with_min_hits(2_200).with_ceiling(20_000));
    let halfway = u32::try_from(bars.len() / 2).expect("a small series");
    assert_eq!(
        crate::split::anchored_folds(bars.len(), super::tests::h(halfway), 1)
            .first()
            .map(|fold| fold.test.is_empty()),
        Some(true),
        "the premise: one split, and a horizon of half the series"
    );
    let mut builder = |slice: &[indicators::Candle]| Ok(built(slice));
    assert_eq!(
        super::walk_forward_projected_prepared_anchored_search_v4(
            &bars,
            &column,
            execution,
            60_000_000,
            super::tests::h(halfway),
            1,
            &complete,
            &mut builder,
            &long,
            &short,
        )
        .map(|_| ()),
        Err(AnchoredSearchValidationRefusalV4::DiscontiguousTraining { fold: 0 })
    );
}

/// THE LEGACY WALK REFUSES A RUNG COUNT ITS ENVIRONMENT CANNOT GIVE, before any
/// fold: with `BRUTEX_GRID_RUNGS=0` the walk returns no fold and a refusal
/// that names the variable. The process environment is set only for a child
/// run of this same test binary (the variable is never written in this
/// process, which would need `unsafe`); the child runs this test alone and
/// asserts the refusal, and the parent asserts that the child passed.
#[test]
fn a_zero_rung_environment_refuses_the_legacy_walk_by_name() {
    const CHILD: &str = "BRUTEX_LS2_ZERO_RUNG_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let refused = super::walk_forward_shaped(
            &[],
            super::tests::h(15),
            3,
            Direction::Long,
            &super::tests::sweeper(),
            &mut super::tests::evaluator,
            super::Shape::Anchored,
            super::TradesOnTheseBars::Yes,
        );
        assert!(refused.folds.is_empty());
        assert!(
            refused
                .refused
                .as_deref()
                .is_some_and(|why| why.starts_with("BRUTEX_GRID_RUNGS is \"0\"")),
            "{:?}",
            refused.refused
        );
        return;
    }
    // Captured, not inherited: the child prints its own `test result` line,
    // which would otherwise be read as one more test of this run.
    let child = std::process::Command::new(std::env::current_exe().expect("this test binary"))
        .args([
            "--exact",
            "validate::cover_tests::a_zero_rung_environment_refuses_the_legacy_walk_by_name",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .env("BRUTEX_GRID_RUNGS", "0")
        .output()
        .expect("the test binary re-runs");
    assert!(
        child.status.success(),
        "the child run refused as named: {}\n{}",
        child.status,
        String::from_utf8_lossy(&child.stdout)
    );
}

/// BASE-COV-LIBS-LS2: A SIGNAL TIMEFRAME WITH NO POSITIVE DURATION IS REFUSED
/// BEFORE ANY FOLD, by name: its exact close cannot be mapped onto the
/// one-minute execution path, for a zero, a negative and the most negative
/// length alike.
#[test]
fn a_signal_timeframe_without_positive_duration_is_refused_before_any_fold() {
    let bars = crate::synthetic::sessions(2);
    for signal_length_micros in [0, -60_000_000, i64::MIN] {
        let refused = super::walk_forward_projected_with_rungs(
            &bars,
            super::ExecutionSeries {
                bars: &bars,
                signal_length_micros,
            },
            super::tests::h(15),
            3,
            Direction::Long,
            &super::tests::sweeper(),
            &mut super::tests::evaluator,
            super::Shape::Anchored,
            super::FoldRungs::Fixed(0),
            &|_| {},
        );
        assert!(refused.folds.is_empty(), "{signal_length_micros}");
        assert_eq!(
            refused.refused.as_deref(),
            Some(
                "the signal timeframe has no positive duration, so its exact close cannot be mapped"
            ),
            "{signal_length_micros}"
        );
    }
}

/// BASE-COV-LIBS-LS2: A FOLD WHOSE TEST WINDOW THE HORIZON PUSHES PAST THE
/// SERIES IS SKIPPED, never judged on bars that do not exist. Over 750 bars
/// in three blocks of 250, a 300-bar horizon leaves the first fold a test
/// window of 550..750 and puts the second fold's start at the series end:
/// the walk is attempted (no refusal) and reports the first fold alone.
#[test]
fn a_fold_whose_test_window_starts_past_the_series_is_skipped() {
    let bars = crate::synthetic::sessions(2);
    assert_eq!(bars.len(), 750);
    let horizon = super::tests::h(300);
    let windows = super::Shape::Anchored.folds(bars.len(), horizon, 2);
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].test, 550..750);
    assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());
    let walked = super::walk_forward_shaped_with_rungs(
        &bars,
        horizon,
        2,
        Direction::Long,
        &super::tests::sweeper(),
        &mut super::tests::evaluator,
        super::Shape::Anchored,
        super::TradesOnTheseBars::Yes,
        0,
    );
    assert_eq!(walked.refused, None);
    assert_eq!(walked.folds.len(), 1);
    assert_eq!(walked.folds[0].index, 0);
    assert_eq!(walked.folds[0].test_bars, 200);
}

//! Bounded exhaustive evidence; this is not a claim about all possible inputs.

#![allow(
    clippy::expect_used,
    reason = "bounded fixture conversions must succeed"
)]

use std::collections::{BTreeMap, BTreeSet};

use engine::{Ladder, Why};
use vocab::ConditionMask;

// Every word, the 63/64 and 127/128 boundaries, the highest live position,
// and the exact implication above_s2 (82) -> above_s3 (84).
const POSITIONS: [u32; 11] = [0, 63, 64, 82, 84, 127, 128, 192, 274, 320, 369];
const INVALID: [u32; 5] = [6, 235, 370, 384, u32::MAX];

type Answers = BTreeMap<Vec<u32>, u64>;

fn exact_support(rows: &[ConditionMask], positions: &[u32]) -> u64 {
    // Deliberately does not call hits, support, set_positions, the engine's
    // prefix join, or the implication implementation under test.
    u64::try_from(
        rows.iter()
            .filter(|row| positions.iter().all(|position| row.get(*position)))
            .count(),
    )
    .expect("fixture rows fit u64")
}

fn exhaustive(rows: &[ConditionMask], threshold: u64) -> Answers {
    let total = u64::try_from(rows.len()).expect("fixture rows fit u64");
    let eligible: Vec<u32> = POSITIONS
        .into_iter()
        .filter(|position| {
            let support = exact_support(rows, &[*position]);
            support > 0 && support < total && support >= threshold.max(1)
        })
        .collect();
    let mut expected = Answers::new();
    for subset in 1_usize..(1_usize << eligible.len()) {
        let positions: Vec<u32> = eligible
            .iter()
            .enumerate()
            .filter(|(index, _)| subset & (1_usize << index) != 0)
            .map(|(_, position)| *position)
            .collect();
        if positions.contains(&82) && positions.contains(&84) {
            continue;
        }
        let support = exact_support(rows, &positions);
        if support >= threshold.max(1) {
            assert!(expected.insert(positions, support).is_none());
        }
    }
    expected
}

fn rows(count: usize, shape: usize) -> Vec<ConditionMask> {
    let mut state = 0xa076_1d64_78bd_642f_u64;
    (0..count)
        .map(|row| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let mut mask = ConditionMask::ZERO;
            for (index, position) in POSITIONS.into_iter().enumerate() {
                let held = match shape {
                    0 => row + 1 < count,
                    1 => state & (1_u64 << index) != 0,
                    _ => position == 0 || (position != 63 && (row + index) % 3 == 0),
                };
                if held {
                    mask = mask.with_bit(position);
                }
            }
            // Keep the production implication true in the synthetic evidence.
            if mask.get(82) {
                mask = mask.with_bit(84);
            }
            mask
        })
        .collect()
}

fn offers() -> [Vec<u32>; 3] {
    let normal = POSITIONS.to_vec();
    let mut repeated = normal.clone();
    repeated.reverse();
    repeated.extend(POSITIONS);
    let mut malformed = normal.clone();
    malformed.rotate_left(4);
    malformed.extend(INVALID);
    malformed.extend(INVALID);
    [normal, repeated, malformed]
}

fn assert_offer_accounting(
    sweep: &engine::Sweep,
    bars: &[ConditionMask],
    offered: &[u32],
    threshold: u64,
) {
    let total = u64::try_from(bars.len()).expect("fixture count fits u64");
    let distinct: BTreeSet<u32> = offered.iter().copied().collect();
    let first = sweep.levels.first().expect("k=1 is reported");
    assert_eq!(
        first.generated,
        u64::try_from(offered.len()).expect("fixture")
    );
    assert_eq!(
        first.duplicates,
        u64::try_from(offered.len() - distinct.len()).expect("fixture")
    );
    let mut excluded = BTreeMap::new();
    let mut infrequent = 0_u64;
    for position in distinct {
        if INVALID.contains(&position) {
            assert!(excluded.insert(position, (None, Why::NotLive)).is_none());
        } else {
            let support = exact_support(bars, &[position]);
            let reason = if support == 0 {
                Some(Why::AlwaysFalse)
            } else if support == total {
                Some(Why::AlwaysTrue)
            } else {
                None
            };
            if let Some(reason) = reason {
                assert!(excluded.insert(position, (Some(support), reason)).is_none());
            } else if support < threshold.max(1) {
                infrequent += 1;
            }
        }
    }
    let observed: BTreeMap<_, _> = sweep
        .excluded
        .iter()
        .map(|entry| (entry.position, (entry.support, entry.reason)))
        .collect();
    assert_eq!(sweep.excluded.len(), observed.len());
    assert_eq!(observed, excluded);
    assert_eq!(
        first.excluded,
        u64::try_from(excluded.len()).expect("fixture")
    );
    assert_eq!(first.infrequent, infrequent);
    assert_eq!(first.pruned, 0);
    assert!(
        sweep
            .levels
            .iter()
            .skip(1)
            .all(|level| level.duplicates == 0 && level.excluded == 0)
    );
}

#[test]
fn complete_ladders_equal_independent_exhaustive_subsets_across_all_six_words() {
    for position in POSITIONS {
        assert!(vocab::table::is_live(
            u16::try_from(position).expect("fixture position fits u16")
        ));
    }
    let mut cells = 0_usize;
    let mut compared_answers = 0_usize;
    let mut deepest = 0_usize;
    for count in [0_usize, 1, 2, 7, 63, 64, 65] {
        for shape in 0..3 {
            let bars = rows(count, shape);
            let total = u64::try_from(count).expect("fixture count fits u64");
            for threshold in [0, 1, 2, total, total + 1, u64::MAX] {
                let expected = exhaustive(&bars, threshold);
                for offered in offers() {
                    cells += 1;
                    let sweep = Ladder::with_min_hits(threshold)
                        .with_support_lanes(1)
                        .walk(&bars, &offered);
                    assert!(
                        sweep.completed(),
                        "count={count}, shape={shape}, threshold={threshold}"
                    );
                    assert_eq!(sweep.bars, total);
                    assert_eq!(sweep.min_hits, threshold.max(1));
                    let mut actual = Answers::new();
                    for item in sweep.all_frequent() {
                        let positions: Vec<u32> = (0..ConditionMask::BITS)
                            .filter(|position| item.mask.get(*position))
                            .collect();
                        assert!(actual.insert(positions, item.hits).is_none());
                    }
                    assert_eq!(
                        actual, expected,
                        "count={count}, shape={shape}, threshold={threshold}"
                    );
                    compared_answers += actual.len();
                    deepest = deepest.max(sweep.depth());
                    assert!(sweep.levels.iter().all(engine::Frontier::reconciles));
                    assert!(
                        sweep
                            .levels
                            .last()
                            .expect("k=1 is reported")
                            .frequent
                            .is_empty()
                    );

                    assert_offer_accounting(&sweep, &bars, &offered, threshold);
                }
            }
        }
    }
    assert_eq!(cells, 378);
    assert!(
        compared_answers > 10_000,
        "the finite oracle must exercise nonempty frontiers"
    );
    assert_eq!(
        deepest, 10,
        "the full/empty fixture must reach the informative maximum"
    );
}

#[test]
fn a_maximum_requested_lane_count_is_a_scheduling_bound_and_preserves_the_answer() {
    let masks = rows(7, 0);
    let one = Ladder::with_min_hits(1).with_support_lanes(1);
    let maximum = Ladder::with_min_hits(1).with_support_lanes(usize::MAX);
    let available = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    assert!(maximum.support_lanes() >= 1);
    assert!(maximum.support_lanes() <= available);
    assert_eq!(
        maximum.walk(&masks, &POSITIONS),
        one.walk(&masks, &POSITIONS)
    );
}

#[test]
fn a_setup_memory_refusal_is_neither_extinction_nor_an_invented_measurement() {
    let refused = Ladder::with_min_hits(7).refused_on_memory(123);
    assert!(!refused.completed());
    assert_eq!(refused.bars, 123);
    assert_eq!(refused.min_hits, 7);
    assert!(refused.levels.is_empty());
    assert!(refused.excluded.is_empty());
    let halt = refused.halted.expect("memory refusal is explicit");
    assert_eq!(halt.breach, engine::Breach::Memory);
    assert_eq!(halt.k, 0);
    assert_eq!(halt.candidates, 0);
    assert_eq!(halt.pairs, 0);
}

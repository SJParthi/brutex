//! Generated numeric evidence only; no market data and no profit claim.
//!
//! Every equality below is against the three SEPARATE public procedures, which
//! this change leaves untouched, on the same inputs: the shared walk must
//! reproduce each receipt field for field and bit for bit.
#![expect(clippy::unwrap_used, reason = "bounded generated fixture assertions")]
use super::super::{DEFAULT_BLOCK, white_reality_check_receipt_v1};
use super::super::{Rng, romano_wolf_adjusted_p_values_v1, spa_receipt_v1};
use super::*;

/// Noise around zero, deterministic per seed.
fn noise(periods: usize, seed: u64) -> Vec<i64> {
    let mut rng = Rng::new(seed);
    (0..periods)
        .map(|_| i64::try_from(rng.next_u64() % 200).unwrap() - 100)
        .collect()
}

/// The same noise with a fixed edge added to every period.
fn edged(periods: usize, seed: u64, edge: i64) -> Vec<i64> {
    noise(periods, seed)
        .into_iter()
        .map(|value| value + edge)
        .collect()
}

/// Positions of every row that is not one repeated value, the rows
/// `bootstrap_zero_v2` hands Romano--Wolf.
fn variable_rows(returns: &[Vec<i64>]) -> Vec<usize> {
    returns
        .iter()
        .enumerate()
        .filter(|(_, row)| row.iter().any(|value| Some(value) != row.first()))
        .map(|(position, _)| position)
        .collect()
}

type Separate = (
    Option<WhiteRealityCheckReceiptV1>,
    Option<SpaReceiptV1>,
    Option<RomanoWolfAdjustedReceiptV1>,
);

/// The three separate procedures, called as the index-stop route called them:
/// White and SPA on the whole family, Romano--Wolf on the named rows copied.
fn separate(
    returns: &[Vec<i64>],
    rows: &[usize],
    draws: usize,
    seed: u64,
    block: usize,
) -> Separate {
    let named: Vec<Vec<i64>> = rows
        .iter()
        .map(|&row| returns.get(row).unwrap().clone())
        .collect();
    (
        white_reality_check_receipt_v1(returns, draws, seed, block),
        spa_receipt_v1(returns, draws, seed, block),
        if rows.is_empty() {
            None
        } else {
            romano_wolf_adjusted_p_values_v1(&named, draws, seed, block)
        },
    )
}

/// The shared walk, asserted equal to the separate procedures bit for bit.
fn identical(
    returns: &[Vec<i64>],
    rows: &[usize],
    draws: usize,
    seed: u64,
    block: usize,
) -> FamilyTestsV1 {
    let shared = family_tests_v1(returns, rows, draws, seed, block).unwrap();
    let (white, spa, romano_wolf) = separate(returns, rows, draws, seed, block);
    let at = format!("draws={draws} seed={seed} block={block} rows={rows:?}");
    assert_eq!(Some(shared.white()), white, "White differs at {at}");
    assert_eq!(Some(shared.spa()), spa, "SPA differs at {at}");
    assert_eq!(
        shared.romano_wolf(),
        romano_wolf.as_ref(),
        "Romano-Wolf differs at {at}"
    );
    shared
}

#[test]
fn ties_zero_variance_zero_rows_and_hopeless_rows_match_the_separate_procedures() {
    let leader = edged(64, 1, 15);
    let returns = vec![
        leader.clone(),
        noise(64, 2),
        // An exact duplicate: a tie in every statistic, broken by position.
        leader,
        // An exact zero session vector: White and SPA only.
        vec![0; 64],
        // A nonzero constant: SPA's conservative zero contribution.
        vec![7; 64],
        // Far below Hansen's gate, so SPA leaves it uncentred.
        edged(64, 3, -80),
        noise(64, 4),
    ];
    let rows = variable_rows(&returns);
    assert_eq!(rows, vec![0, 1, 2, 5, 6]);
    for block in [1, 2, 7, DEFAULT_BLOCK] {
        for draws in [1, 2, 37] {
            for seed in [0, 11] {
                let shared = identical(&returns, &rows, draws, seed, block);
                let romano_wolf = shared.romano_wolf().unwrap();
                assert_eq!(
                    romano_wolf
                        .candidate(0)
                        .unwrap()
                        .observed_statistic()
                        .to_bits(),
                    romano_wolf
                        .candidate(2)
                        .unwrap()
                        .observed_statistic()
                        .to_bits(),
                    "the fixture must carry an exact observed-statistic tie"
                );
            }
        }
    }
}

#[test]
fn the_shortest_samples_keep_every_strategy_in_hansens_recentring() {
    // Two and three periods: `ln ln n` is undefined or negative there, so the
    // gate keeps every strategy -- the other arm of the gate expression.
    for returns in [
        vec![vec![1, 3], vec![4, -2], vec![0, 0], vec![-5, -5]],
        vec![
            vec![1, 3, 2],
            vec![4, -2, 9],
            vec![0, 0, 0],
            vec![-9, -8, -12],
        ],
    ] {
        let rows = variable_rows(&returns);
        for draws in [1, 5, 50] {
            for block in [1, 2] {
                identical(&returns, &rows, draws, 5, block);
            }
        }
    }
}

#[test]
fn an_empty_row_list_runs_no_romano_wolf_and_still_matches_white_and_spa() {
    for returns in [
        vec![vec![0; 8], vec![0; 8]],
        vec![noise(40, 9), vec![0; 40]],
    ] {
        let shared = identical(&returns, &[], 23, 3, 4);
        assert!(shared.romano_wolf().is_none());
    }
}

#[test]
fn named_rows_in_any_order_or_repeated_are_the_romano_wolf_family() {
    let returns = vec![noise(48, 20), edged(48, 21, 25), vec![0; 48], noise(48, 22)];
    for rows in [vec![3, 0], vec![1, 3, 1], vec![0, 1, 3]] {
        identical(&returns, &rows, 41, 8, 3);
    }
}

#[test]
fn pure_noise_exercises_both_sides_of_every_comparison() {
    let returns: Vec<Vec<i64>> = (0..6).map(|seed| noise(80, seed)).collect();
    let rows = variable_rows(&returns);
    let draws = 200;
    let shared = identical(&returns, &rows, draws, 13, DEFAULT_BLOCK);
    for matched in [
        shared.white().matched_or_exceeded(),
        shared.spa().matched_or_exceeded(),
    ] {
        assert!(
            matched > 0 && matched < draws,
            "a vacuous fixture: {matched} of {draws} draws matched"
        );
    }
    let romano_wolf = shared.romano_wolf().unwrap();
    assert!(
        (0..rows.len()).any(|position| {
            let strict = romano_wolf
                .candidate(position)
                .unwrap()
                .strict_exceedances();
            strict > 0 && strict < draws
        }),
        "no Romano-Wolf rank was exceeded on some draws and not on others"
    );
}

/// A larger family with every row kind the index-stop route produces.
fn larger_family() -> Vec<Vec<i64>> {
    let mut returns: Vec<Vec<i64>> = (0_u64..40)
        .map(|strategy| match strategy % 8 {
            0 => vec![0; 150],
            1 => edged(150, strategy, 35),
            2 => edged(150, strategy, -60),
            3 => noise(150, strategy)
                .into_iter()
                .map(|value| value * 9)
                .collect(),
            4 => vec![3; 150],
            _ => noise(150, strategy),
        })
        .collect();
    returns.push(returns.get(1).unwrap().clone());
    returns
}

#[test]
fn a_larger_family_matches_across_more_draws_than_one_chunk_holds() {
    let returns = larger_family();
    let rows = variable_rows(&returns);
    let draws = MAX_CHUNK_DRAWS + 904;
    assert!(
        chunk_draws(150) < draws,
        "the walk must cross a chunk boundary"
    );
    identical(&returns, &rows, draws, 97, DEFAULT_BLOCK);
}

#[test]
fn one_thread_and_many_threads_agree_bit_for_bit() {
    let returns = larger_family();
    let rows = variable_rows(&returns);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| family_tests_v1(&returns, &rows, 700, 31, 5))
            .unwrap()
    };
    let one = run(1);
    assert_eq!(one, run(4));
    assert_eq!(one, run(7));
    assert_eq!(one, family_tests_v1(&returns, &rows, 700, 31, 5).unwrap());
}

#[test]
fn neither_the_chunk_size_nor_the_task_split_moves_a_count() {
    let returns = larger_family();
    let rows = variable_rows(&returns);
    let (draws, seed, block) = (97, 43, 3);
    let (periods, stats) = exact_family_test_inputs_v1(&returns, draws, block).unwrap();
    let plan = Stepdown::new(&returns, &rows, draws, seed, block).unwrap();
    let family = Family::new(&returns, &stats, periods, Some(&plan));
    let reference = count(&family, periods, draws, seed, block, MAX_CHUNK_DRAWS).unwrap();
    assert_eq!(reference.strict.len(), rows.len());
    // Zero is clamped to one draw per chunk; 5 does not divide TASK_DRAWS.
    for chunk in [0, 1, 2, 5, TASK_DRAWS, 96, 97, 98] {
        assert_eq!(
            count(&family, periods, draws, seed, block, chunk).unwrap(),
            reference,
            "chunk {chunk}"
        );
    }
}

#[test]
fn a_generation_step_holds_at_most_its_index_budget() {
    assert_eq!(chunk_draws(0), MAX_CHUNK_DRAWS);
    assert_eq!(chunk_draws(1), MAX_CHUNK_DRAWS);
    assert_eq!(chunk_draws(225), MAX_CHUNK_DRAWS);
    assert_eq!(
        chunk_draws(4_096),
        INDEX_CHUNK_BYTES / (4_096 * size_of::<usize>())
    );
    assert_eq!(chunk_draws(1 << 20), 1);
    assert_eq!(chunk_draws(usize::MAX), 1);
    for periods in [225, 4_096, 1 << 20] {
        assert!(chunk_draws(periods) * periods * size_of::<usize>() <= INDEX_CHUNK_BYTES);
    }
}

/// One refusal case: family, named rows, draws, block and the expected refusal.
type Case = (
    Vec<Vec<i64>>,
    Vec<usize>,
    usize,
    usize,
    FamilyTestsRefusalV1,
);

/// Every refusal, with the procedure the separate calls would name first.
fn refusal_cases() -> Vec<Case> {
    let variable = noise(8, 1);
    vec![
        // A named row outside the family.
        (vec![vec![1, 2]], vec![1], 5, 1, FamilyTestsRefusalV1::Rows),
        // Romano--Wolf refuses a point mass it cannot studentize ...
        (
            vec![vec![7; 8], variable.clone()],
            vec![0],
            5,
            1,
            FamilyTestsRefusalV1::RomanoWolf,
        ),
        // ... and a row whose f64 values collapse to one number.
        (
            vec![vec![i64::MAX, i64::MAX - 1]],
            vec![0],
            5,
            1,
            FamilyTestsRefusalV1::RomanoWolf,
        ),
        // Romano--Wolf is called first, so it names a shape both refuse.
        (
            vec![variable.clone()],
            vec![0],
            0,
            1,
            FamilyTestsRefusalV1::RomanoWolf,
        ),
        (
            vec![variable.clone()],
            vec![0],
            5,
            0,
            FamilyTestsRefusalV1::RomanoWolf,
        ),
        (
            vec![variable.clone()],
            vec![0],
            usize::MAX,
            1,
            FamilyTestsRefusalV1::RomanoWolf,
        ),
        (
            vec![vec![1], vec![2]],
            vec![0],
            5,
            1,
            FamilyTestsRefusalV1::RomanoWolf,
        ),
        (
            vec![vec![1, 2, 3], vec![4, 5]],
            vec![0, 1],
            5,
            1,
            FamilyTestsRefusalV1::RomanoWolf,
        ),
        // Romano--Wolf accepts its rows; White refuses the whole family.
        (
            vec![vec![1, 2, 3], vec![4, 5]],
            vec![0],
            5,
            1,
            FamilyTestsRefusalV1::White,
        ),
        // Without named rows White is the first procedure to refuse.
        (vec![], vec![], 5, 1, FamilyTestsRefusalV1::White),
        (vec![vec![]], vec![], 5, 1, FamilyTestsRefusalV1::White),
        (vec![vec![1]], vec![], 5, 1, FamilyTestsRefusalV1::White),
        (
            vec![variable.clone()],
            vec![],
            0,
            1,
            FamilyTestsRefusalV1::White,
        ),
        (
            vec![variable.clone()],
            vec![],
            5,
            0,
            FamilyTestsRefusalV1::White,
        ),
        (
            vec![variable],
            vec![],
            usize::MAX,
            1,
            FamilyTestsRefusalV1::White,
        ),
    ]
}

#[test]
fn refusals_name_the_first_separate_procedure_that_refuses() {
    for (returns, rows, draws, block, expected) in refusal_cases() {
        assert_eq!(
            family_tests_v1(&returns, &rows, draws, 3, block),
            Err(expected),
            "{returns:?} rows={rows:?} draws={draws} block={block}"
        );
        // The separate procedure named by the refusal refuses too.
        let (white, _, romano_wolf) = if expected == FamilyTestsRefusalV1::Rows {
            (None, None, None)
        } else {
            separate(&returns, &rows, draws, 3, block)
        };
        match expected {
            FamilyTestsRefusalV1::RomanoWolf => assert!(romano_wolf.is_none()),
            FamilyTestsRefusalV1::White => assert!(white.is_none()),
            _ => assert_eq!(expected, FamilyTestsRefusalV1::Rows),
        }
    }
}

/// A family whose every comparison hits, for driving each count to its limit.
fn saturated(returns: &[Vec<i64>]) -> (Family<'_>, usize) {
    let rows = variable_rows(returns);
    let (periods, stats) = exact_family_test_inputs_v1(returns, 3, 1).unwrap();
    let plan = Stepdown::new(returns, &rows, 3, 0, 1).unwrap();
    let mut family = Family::new(returns, &stats, periods, Some(&plan));
    family.white_observed = f64::NEG_INFINITY;
    family.spa_observed = f64::NEG_INFINITY;
    for rung in &mut family.walk {
        rung.observed = f64::NEG_INFINITY;
    }
    (family, periods)
}

#[test]
fn a_count_that_cannot_grow_or_a_rank_naming_no_lane_refuses_the_walk() {
    let returns = vec![noise(6, 1), noise(6, 2)];
    let (family, periods) = saturated(&returns);
    let index: Vec<usize> = (0..periods).collect();
    let mut resampled = vec![0.0; family.lanes.len()];

    let mut tally = Tally::new(family.walk.len());
    assert_eq!(
        family.accumulate(&index, &mut resampled, &mut tally),
        Some(())
    );
    assert_eq!(
        tally,
        Tally {
            white: 1,
            spa: 1,
            strict: vec![1, 1]
        }
    );

    for full in [
        Tally {
            white: usize::MAX,
            spa: 0,
            strict: vec![0, 0],
        },
        Tally {
            white: 0,
            spa: usize::MAX,
            strict: vec![0, 0],
        },
        Tally {
            white: 0,
            spa: 0,
            strict: vec![usize::MAX, 0],
        },
    ] {
        let mut tally = full.clone();
        assert_eq!(family.accumulate(&index, &mut resampled, &mut tally), None);
        let mut total = full;
        assert_eq!(
            total.absorb(&Tally {
                white: 1,
                spa: 1,
                strict: vec![1, 1]
            }),
            None
        );
    }

    let mut orphan = saturated(&returns).0;
    orphan.walk.push(Rung {
        position: 2,
        lane: 99,
        observed: 0.0,
        mean: 0.0,
        standard_error: 1.0,
    });
    assert_eq!(orphan.count_draws(std::slice::from_ref(&index)), None);
    assert_eq!(count(&orphan, periods, 3, 0, 1, MAX_CHUNK_DRAWS), None);
}

#[test]
fn a_receipt_is_refused_rather_than_built_from_counts_that_do_not_fit_its_ranks() {
    let returns = vec![noise(12, 5), noise(12, 6), noise(12, 7)];
    let rows = variable_rows(&returns);
    let plan = Stepdown::new(&returns, &rows, 9, 0, 2).unwrap();
    assert!(plan.receipt(&[0, 0, 0], 9, 12, 0, 2).is_some());
    // Too few counts leave a rank unfilled.
    assert!(plan.receipt(&[0, 0], 9, 12, 0, 2).is_none());
    // A count whose `+1` numerator cannot be represented.
    assert!(plan.receipt(&[usize::MAX, 0, 0], 9, 12, 0, 2).is_none());
    // A rank naming no position.
    let mut stray = Stepdown::new(&returns, &rows, 9, 0, 2).unwrap();
    let last = stray.walk.last_mut().unwrap();
    last.position = 99;
    assert!(stray.receipt(&[0, 0, 0], 9, 12, 0, 2).is_none());
}

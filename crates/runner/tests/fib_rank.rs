//! FIB-2 PART B: the mover rank (D-6322, D-6323). Every test here is named in
//! `docs/04-invariants.md` beside the invariant it proves. The oracles — a
//! std sort and a bitonic sorting network over the same order — live here
//! only, never in the shipping code. Synthetic prices only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]

use std::cmp::Ordering;

use runner::fib::rank::{
    Census, MEMBER_REFUSALS, MemberRefusal, MemberState, MoverRank, RankRefusal, RankWidth,
};

/// A deterministic generator (splitmix64).
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> i64 {
        i64::try_from(self.next() % n).unwrap()
    }
}

/// The 09:15 minute's close, as a synthetic instant.
const OPEN: i64 = 1_000_000;

/// The rank fold over one day of `minutes`, with `previous` closes.
fn day(previous: &[Option<i64>]) -> MoverRank {
    let mut rank = MoverRank::new(previous.len()).unwrap();
    rank.begin_day(previous).unwrap();
    rank
}

/// Every member's (gainer, loser) rank.
fn ranks(rank: &MoverRank) -> Vec<(u16, u16)> {
    (0..rank.pool())
        .map(|slot| {
            let slot = u16::try_from(slot).unwrap();
            (rank.gainer_rank(slot), rank.loser_rank(slot))
        })
        .collect()
}

/// The exact order of two members' moves: `(c1 − p1) x p2` against
/// `(c2 − p2) x p1`.
fn exact(a: (i64, i64), b: (i64, i64)) -> Ordering {
    let (ca, pa) = (i128::from(a.0), i128::from(a.1));
    let (cb, pb) = (i128::from(b.0), i128::from(b.1));
    ((ca - pa) * pb).cmp(&((cb - pb) * pa))
}

/// The oracle: gainers by exact move descending, losers ascending, each tie
/// by slot, from `(slot, close, previous)` of the ranked members.
fn std_sort(members: &[(u16, i64, i64)]) -> (Vec<u16>, Vec<u16>) {
    let mut gainers: Vec<_> = members.iter().filter(|m| m.1 > m.2).copied().collect();
    let mut losers: Vec<_> = members.iter().filter(|m| m.1 < m.2).copied().collect();
    gainers.sort_by(|a, b| exact((b.1, b.2), (a.1, a.2)).then(a.0.cmp(&b.0)));
    losers.sort_by(|a, b| exact((a.1, a.2), (b.1, b.2)).then(a.0.cmp(&b.0)));
    (
        gainers.iter().map(|m| m.0).collect(),
        losers.iter().map(|m| m.0).collect(),
    )
}

/// The second oracle: a bitonic sorting network over the next power of two,
/// padded with sentinels that sort last and never rank.
fn network(members: &[(u16, i64, i64)], descending: bool) -> Vec<u16> {
    let side: Vec<Option<(u16, i64, i64)>> = members
        .iter()
        .filter(|m| if descending { m.1 > m.2 } else { m.1 < m.2 })
        .copied()
        .map(Some)
        .collect();
    let width = side.len().max(1).next_power_of_two();
    let mut lanes = side;
    lanes.resize(width, None);
    let before = |a: &Option<(u16, i64, i64)>, b: &Option<(u16, i64, i64)>| -> bool {
        match (a, b) {
            (Some(a), Some(b)) => {
                let order = exact((a.1, a.2), (b.1, b.2));
                let order = if descending { order.reverse() } else { order };
                order.then(a.0.cmp(&b.0)) != Ordering::Greater
            }
            (Some(_) | None, None) => true,
            (None, Some(_)) => false,
        }
    };
    let mut k = 2;
    while k <= width {
        let mut j = k / 2;
        while j > 0 {
            for i in 0..width {
                let l = i ^ j;
                if l > i {
                    let up = i & k == 0;
                    let ordered = before(&lanes[i], &lanes[l]);
                    if ordered != up {
                        lanes.swap(i, l);
                    }
                }
            }
            j /= 2;
        }
        k *= 2;
    }
    lanes.into_iter().flatten().map(|m| m.0).collect()
}

#[test]
fn fifty_stocks_with_identical_moves_rank_by_slot() {
    let previous = vec![Some(100_000); 50];
    let mut rank = day(&previous);
    let closes = vec![Some(101_000); 50];
    let census = rank.minute(OPEN, &closes).unwrap();
    assert_eq!((census.gainers, census.losers, census.ranked), (50, 0, 50));
    let expected: Vec<u16> = (0..50).collect();
    assert_eq!(rank.gainers(), &expected[..]);
    for slot in 0..50 {
        assert_eq!(rank.gainer_rank(slot), slot + 1);
        assert_eq!(rank.loser_rank(slot), 0);
        assert_eq!(rank.move_bp(slot), Some(100));
    }
    // Losers too: identical drops rank by slot.
    let census = rank.minute(OPEN + 60, &vec![Some(99_000); 50]).unwrap();
    assert_eq!((census.gainers, census.losers), (0, 50));
    assert_eq!(rank.losers(), &expected[..]);
    assert_eq!(rank.loser_rank(49), 50);
}

#[test]
fn a_member_missing_all_morning_that_then_appears() {
    let mut rank = day(&[Some(50_000), Some(60_000), Some(70_000)]);
    for minute in 0..120 {
        let census = rank
            .minute(OPEN + minute * 60, &[Some(50_500), None, Some(69_000)])
            .unwrap();
        assert_eq!(census.missing, 1, "minute {minute}");
        assert_eq!(rank.state(1), Some(MemberState::Missing));
        assert_eq!((rank.gainer_rank(1), rank.loser_rank(1)), (0, 0));
        assert_eq!(rank.move_bp(1), None);
    }
    let census = rank
        .minute(OPEN + 120 * 60, &[Some(50_500), Some(63_000), Some(69_000)])
        .unwrap();
    assert_eq!(census.missing, 0);
    assert_eq!(rank.state(1), Some(MemberState::Fresh));
    assert_eq!(rank.gainer_rank(1), 1, "500 bp ahead of 100 bp");
    assert_eq!(rank.gainer_rank(0), 2);
    assert_eq!(rank.loser_rank(2), 1);
    assert_eq!(rank.totals().missing, 120);
    assert_eq!(
        (rank.totals().minutes, rank.totals().ranked),
        (121, 120 * 2 + 3),
        "every minute folded, every ranked member-minute counted"
    );
    assert_eq!(rank.state(3), None, "outside the pool");
}

#[test]
fn a_stale_member_is_ranked_on_its_last_close_and_counted() {
    let mut rank = day(&[Some(10_000), Some(10_000)]);
    rank.minute(OPEN, &[Some(10_300), Some(10_100)]).unwrap();
    let census = rank.minute(OPEN + 60, &[None, Some(10_200)]).unwrap();
    assert_eq!(census.stale, 1);
    assert_eq!(census.ranked, 2);
    assert_eq!(rank.state(0), Some(MemberState::Stale));
    assert_eq!(rank.move_bp(0), Some(300), "the carried close");
    assert_eq!(rank.gainer_rank(0), 1);
    let census = rank.minute(OPEN + 120, &[None, None]).unwrap();
    assert_eq!(census.stale, 2);
    assert_eq!(rank.totals().stale, 3);
}

#[test]
fn pools_of_1_2_3_200_256_257_and_512_match_both_oracles() {
    let mut mix = Mix(41);
    for pool in [1_usize, 2, 3, 200, 256, 257, 512] {
        let previous: Vec<Option<i64>> = (0..pool)
            .map(|_| Some(1_000 + mix.below(9_000_000)))
            .collect();
        let mut rank = MoverRank::new(pool).unwrap();
        rank.begin_day(&previous).unwrap();
        assert_eq!(
            rank.width(),
            if pool <= 255 {
                RankWidth::U8
            } else {
                RankWidth::U16
            }
        );
        for minute in 0..5 {
            let closes: Vec<Option<i64>> = previous
                .iter()
                .map(|p| {
                    let p = p.unwrap();
                    Some(p + mix.below(1_001) - 500)
                })
                .collect();
            rank.minute(OPEN + minute * 60, &closes).unwrap();
            let members: Vec<(u16, i64, i64)> = (0..pool)
                .map(|s| (s as u16, closes[s].unwrap(), previous[s].unwrap()))
                .collect();
            let (gainers, losers) = std_sort(&members);
            assert_eq!(rank.gainers(), &gainers[..], "pool {pool}");
            assert_eq!(rank.losers(), &losers[..], "pool {pool}");
            assert_eq!(network(&members, true), gainers, "pool {pool}");
            assert_eq!(network(&members, false), losers, "pool {pool}");
            let ranked = gainers.len() + losers.len();
            assert!(ranked <= pool, "the padding never ranks");
            let max = ranks(&rank)
                .iter()
                .map(|(g, l)| (*g).max(*l))
                .max()
                .unwrap();
            assert!(usize::from(max) <= pool);
        }
    }
}

/// FM-F009: with 257 gainers a `u8` rank would wrap 256 to 0 (unranked); the
/// run's width is chosen from its largest pool and the 257th gainer is 257.
#[test]
fn pool_257_all_gainers_ranks_257() {
    let previous = vec![Some(200_000); 257];
    let mut rank = day(&previous);
    let closes: Vec<Option<i64>> = (0..257).map(|s| Some(200_001 + 257 - s)).collect();
    rank.minute(OPEN, &closes).unwrap();
    assert_eq!(rank.width(), RankWidth::U16);
    assert_eq!(rank.gainer_rank(0), 1);
    assert_eq!(rank.gainer_rank(255), 256);
    assert_eq!(rank.gainer_rank(256), 257);
    let mut out = Vec::new();
    rank.width().put(257, &mut out).unwrap();
    assert_eq!(out, [1, 1]);
    assert_eq!(
        RankWidth::U8.put(256, &mut out),
        Err(RankRefusal::RankWidthExceeded {
            rank: 256,
            width: RankWidth::U8
        }),
        "refused by name, never wrapped"
    );
    RankWidth::U8.put(255, &mut out).unwrap();
    assert_eq!(out, [1, 1, 255]);
    assert_eq!(RankWidth::of_pool(255), Ok(RankWidth::U8));
    assert_eq!(RankWidth::of_pool(256), Ok(RankWidth::U16));
    assert_eq!(RankWidth::of_pool(65_535), Ok(RankWidth::U16));
    assert_eq!(
        RankWidth::of_pool(65_536),
        Err(RankRefusal::RankWidthExceeded {
            rank: 65_536,
            width: RankWidth::U16
        })
    );
    assert_eq!(
        MoverRank::new(65_536).unwrap_err().name(),
        "rank_width_exceeded"
    );
    assert_eq!((RankWidth::U8.name(), RankWidth::U16.name()), ("u8", "u16"));
    assert_eq!(RankWidth::U8.max_rank(), 255);
    assert_eq!(RankWidth::U16.max_rank(), u16::MAX);
}

#[test]
fn a_move_of_zero_is_on_neither_side() {
    let mut rank = day(&[Some(40_000), Some(40_000), Some(40_000)]);
    let census = rank
        .minute(OPEN, &[Some(40_000), Some(40_010), Some(39_990)])
        .unwrap();
    assert_eq!(
        census,
        Census {
            minute: OPEN,
            pool: 3,
            ranked: 2,
            gainers: 1,
            losers: 1,
            flat: 1,
            missing: 0,
            stale: 0,
            refused: 0,
        }
    );
    assert_eq!((rank.gainer_rank(0), rank.loser_rank(0)), (0, 0));
    assert_eq!(rank.move_bp(0), Some(0));
    assert_eq!(rank.totals().flat, 1);
}

/// FM-F019: moves of 0.4 bp and 0.6 bp are both gainers (the side is the
/// sign in paisa), 0.6 first, though both `move_bp` are 0.
#[test]
fn moves_of_0_4_bp_and_0_6_bp_are_both_gainers_0_6_first() {
    let mut rank = day(&[Some(1_000_000), Some(1_000_000), Some(1_000_000)]);
    rank.minute(OPEN, &[Some(1_000_040), Some(1_000_060), Some(999_960)])
        .unwrap();
    assert_eq!(rank.move_bp(0), Some(0));
    assert_eq!(rank.move_bp(1), Some(0));
    assert_eq!(rank.gainers(), &[1, 0]);
    assert_eq!(rank.losers(), &[2]);
    assert_eq!(rank.loser_rank(2), 1, "-0.4 bp is a loser");
    // An exact tie inside one move_bp run goes to the slot.
    let mut tie = day(&[Some(300_000), Some(600_000), Some(150_000)]);
    tie.minute(OPEN, &[Some(300_003), Some(600_006), Some(150_001)])
        .unwrap();
    assert_eq!(
        tie.gainers(),
        &[0, 1, 2],
        "1 bp = 1 bp by slot, then 0.66 bp"
    );
}

/// FM-F021: a previous close of 1 and a close of 300,000 does not fit `i32`
/// basis points: that member is refused for the minute, the others unchanged.
#[test]
fn a_move_out_of_range_refuses_only_its_member() {
    let previous = [Some(1), Some(50_000), Some(50_000)];
    let mut rank = day(&previous);
    let census = rank
        .minute(OPEN, &[Some(300_000), Some(50_500), Some(49_000)])
        .unwrap();
    assert_eq!(
        rank.state(0),
        Some(MemberState::Refused(MemberRefusal::MoveOutOfRange))
    );
    assert_eq!(census.refused, 1);
    assert_eq!(rank.gainers(), &[1]);
    assert_eq!(rank.losers(), &[2]);
    let mut without = day(&[Some(1_000), Some(50_000), Some(50_000)]);
    without
        .minute(OPEN, &[Some(1_000), Some(50_500), Some(49_000)])
        .unwrap();
    assert_eq!(
        &ranks(&rank)[1..],
        &ranks(&without)[1..],
        "the others unchanged"
    );
    assert_eq!(
        rank.totals().refused[MemberRefusal::MoveOutOfRange as usize],
        1
    );
    // A move that just fits is ranked.
    let mut edge = day(&[Some(1)]);
    edge.minute(OPEN, &[Some(214_749)]).unwrap();
    assert_eq!(edge.move_bp(0), Some(2_147_480_000));
}

#[test]
fn bad_prices_are_refused_by_name_and_counted() {
    let previous = [None, Some(0), Some(-5), Some(10_000), Some(10_000)];
    let mut rank = day(&previous);
    let census = rank
        .minute(OPEN, &[Some(1), Some(1), Some(1), Some(0), Some(10_100)])
        .unwrap();
    assert_eq!(census.refused, 4);
    assert_eq!(
        rank.state(0),
        Some(MemberState::Refused(MemberRefusal::NoPreviousClose))
    );
    assert_eq!(
        rank.state(1),
        Some(MemberState::Refused(
            MemberRefusal::PreviousCloseNotPositive
        ))
    );
    assert_eq!(
        rank.state(2),
        Some(MemberState::Refused(
            MemberRefusal::PreviousCloseNotPositive
        ))
    );
    assert_eq!(
        rank.state(3),
        Some(MemberState::Refused(MemberRefusal::CloseNotPositive))
    );
    // The bad print keeps no close: next minute the member is missing...
    rank.minute(OPEN + 60, &[None, None, None, None, None])
        .unwrap();
    assert_eq!(rank.state(3), Some(MemberState::Missing));
    // ...and after a good close, a later bad print keeps the good one.
    rank.minute(OPEN + 120, &[None, None, None, Some(10_200), None])
        .unwrap();
    rank.minute(OPEN + 180, &[None, None, None, Some(-1), None])
        .unwrap();
    assert_eq!(
        rank.state(3),
        Some(MemberState::Refused(MemberRefusal::CloseNotPositive))
    );
    rank.minute(OPEN + 240, &[None, None, None, None, None])
        .unwrap();
    assert_eq!(rank.state(3), Some(MemberState::Stale));
    assert_eq!(rank.move_bp(3), Some(200));
    assert_eq!(
        rank.totals().refused,
        [5, 10, 2, 0],
        "five minutes, by reason"
    );
    let names: Vec<&str> = MemberRefusal::ALL.iter().map(|r| r.name()).collect();
    assert_eq!(
        names,
        [
            "no_previous_close",
            "previous_close_not_positive",
            "close_not_positive",
            "move_out_of_range"
        ]
    );
    assert_eq!(MEMBER_REFUSALS, 4);
}

#[test]
fn a_stock_turns_from_gainer_to_loser_intraday() {
    let mut rank = day(&[Some(80_000), Some(80_000)]);
    rank.minute(OPEN, &[Some(80_800), Some(80_100)]).unwrap();
    assert_eq!((rank.gainer_rank(0), rank.loser_rank(0)), (1, 0));
    rank.minute(OPEN + 60, &[Some(80_000), Some(80_100)])
        .unwrap();
    assert_eq!((rank.gainer_rank(0), rank.loser_rank(0)), (0, 0), "flat");
    rank.minute(OPEN + 120, &[Some(79_000), Some(80_100)])
        .unwrap();
    assert_eq!((rank.gainer_rank(0), rank.loser_rank(0)), (0, 1));
    assert_eq!(rank.gainer_rank(1), 1);
}

/// The rank at minute m is a fold of minutes 0..=m only: changing any later
/// minute leaves it unchanged (`CLAUDE.md` §3 rule 7).
#[test]
fn the_rank_at_minute_m_is_unchanged_when_any_later_minute_changes() {
    let mut mix = Mix(3);
    let pool = 40;
    let previous: Vec<Option<i64>> = (0..pool)
        .map(|_| Some(10_000 + mix.below(90_000)))
        .collect();
    let minutes: Vec<Vec<Option<i64>>> = (0..30)
        .map(|_| {
            previous
                .iter()
                .map(|p| (mix.below(4) != 0).then(|| p.unwrap() + mix.below(201) - 100))
                .collect()
        })
        .collect();
    let run = |minutes: &[Vec<Option<i64>>]| -> Vec<Vec<(u16, u16)>> {
        let mut rank = day(&previous);
        minutes
            .iter()
            .enumerate()
            .map(|(m, closes)| {
                rank.minute(OPEN + i64::try_from(m).unwrap() * 60, closes)
                    .unwrap();
                ranks(&rank)
            })
            .collect()
    };
    let base = run(&minutes);
    for m in [0, 7, 15, 28] {
        let mut changed = minutes.clone();
        for later in changed.iter_mut().skip(m + 1) {
            for close in later.iter_mut() {
                *close = Some(10_000 + mix.below(90_000));
            }
        }
        let other = run(&changed);
        assert_eq!(&other[..=m], &base[..=m], "minute {m}");
    }
}

/// FM-F010: the radix order equals a std sort and a bitonic network on
/// 10,000 seeded random minutes, ties and stale and missing members included.
#[test]
fn radix_equals_std_sort_and_network() {
    let mut mix = Mix(10_000);
    let mut rank = MoverRank::new(64).unwrap();
    let mut previous: Vec<Option<i64>> = Vec::new();
    let mut last: Vec<Option<i64>> = Vec::new();
    for minute in 0..10_000_i64 {
        if minute % 375 == 0 {
            let pool = 1 + usize::try_from(mix.below(64)).unwrap();
            previous = (0..pool)
                .map(|_| Some(1_000 * (1 + mix.below(50))))
                .collect();
            last = vec![None; pool];
            rank.begin_day(&previous).unwrap();
        }
        let pool = rank.pool();
        let closes: Vec<Option<i64>> = (0..pool)
            .map(|_| (mix.below(5) != 0).then(|| 1_000 * (1 + mix.below(50)) + mix.below(3) - 1))
            .collect();
        for (held, close) in last.iter_mut().zip(&closes) {
            if close.is_some() {
                *held = *close;
            }
        }
        rank.minute(OPEN + minute * 60, &closes).unwrap();
        let members: Vec<(u16, i64, i64)> = (0..pool)
            .filter_map(|s| last[s].map(|c| (s as u16, c, previous[s].unwrap())))
            .collect();
        let (gainers, losers) = std_sort(&members);
        assert_eq!(rank.gainers(), &gainers[..], "minute {minute}");
        assert_eq!(rank.losers(), &losers[..], "minute {minute}");
        if minute % 97 == 0 {
            assert_eq!(network(&members, true), gainers);
            assert_eq!(network(&members, false), losers);
        }
    }
}

#[test]
fn the_fold_is_deterministic() {
    let replay = || {
        let mut mix = Mix(77);
        let previous: Vec<Option<i64>> = (0..100).map(|_| Some(5_000 + mix.below(5_000))).collect();
        let mut rank = day(&previous);
        let mut seen = Vec::new();
        for minute in 0..50 {
            let closes: Vec<Option<i64>> = previous
                .iter()
                .map(|p| (mix.below(3) != 0).then(|| p.unwrap() + mix.below(11) - 5))
                .collect();
            seen.push(rank.minute(OPEN + minute * 60, &closes).unwrap());
            seen.push(rank.census().unwrap());
        }
        (seen, ranks(&rank), rank.totals(), rank.work())
    };
    assert_eq!(replay(), replay());
}

#[test]
fn the_work_per_minute_is_the_same_at_every_minute_of_a_day() {
    let previous: Vec<Option<i64>> = (0..128).map(|s| Some(10_000 + s)).collect();
    let mut rank = day(&previous);
    let mut works = Vec::new();
    for minute in 0..20_i64 {
        // Distinct moves every minute, so no tie run adds comparisons.
        let closes: Vec<Option<i64>> = (0..128)
            .map(|s| Some(10_000 + s + (s * 7 + minute * 13) % 255 - 127))
            .collect();
        let before = rank.work();
        rank.minute(OPEN + minute * 60, &closes).unwrap();
        works.push(rank.work() - before);
    }
    let first = works[0];
    let spread = works.iter().map(|w| w.abs_diff(first)).max().unwrap();
    assert!(spread <= 128 * 8, "{works:?}");
}

#[test]
fn every_misuse_is_refused_by_name_and_changes_nothing() {
    assert_eq!(MoverRank::new(0), Err(RankRefusal::PoolEmpty));
    let mut rank = MoverRank::new(3).unwrap();
    assert_eq!(rank.max_pool(), 3);
    assert_eq!(rank.minute(OPEN, &[]), Err(RankRefusal::NoDay));
    assert_eq!(rank.begin_day(&[]), Err(RankRefusal::PoolEmpty));
    assert_eq!(
        rank.begin_day(&[Some(1); 4]),
        Err(RankRefusal::PoolOverRunMaximum {
            pool: 4,
            max_pool: 3
        })
    );
    rank.begin_day(&[Some(100), Some(200)]).unwrap();
    assert_eq!(rank.census(), None);
    assert_eq!(
        rank.minute(OPEN, &[Some(1)]),
        Err(RankRefusal::ClosesNotPool { closes: 1, pool: 2 })
    );
    rank.minute(OPEN, &[Some(101), Some(199)]).unwrap();
    let before = rank.clone();
    assert_eq!(
        rank.minute(OPEN, &[Some(102), Some(198)]),
        Err(RankRefusal::MinuteOutOfOrder {
            minute: OPEN,
            last: OPEN
        })
    );
    assert_eq!(
        rank.minute(OPEN - 60, &[Some(102), Some(198)]),
        Err(RankRefusal::MinuteOutOfOrder {
            minute: OPEN - 60,
            last: OPEN
        })
    );
    assert_eq!(rank, before);
    // A new day restarts the minutes and clears the ranks.
    rank.begin_day(&[Some(100), Some(200), Some(300)]).unwrap();
    assert_eq!(ranks(&rank), [(0, 0), (0, 0), (0, 0)]);
    rank.minute(OPEN, &[None, None, None]).unwrap();
    assert_eq!(rank.census().unwrap().missing, 3);
    let names: Vec<&str> = [
        RankRefusal::RankWidthExceeded {
            rank: 0,
            width: RankWidth::U8,
        },
        RankRefusal::PoolEmpty,
        RankRefusal::PoolOverRunMaximum {
            pool: 0,
            max_pool: 0,
        },
        RankRefusal::NoDay,
        RankRefusal::ClosesNotPool { closes: 0, pool: 0 },
        RankRefusal::MinuteOutOfOrder { minute: 0, last: 0 },
    ]
    .iter()
    .map(RankRefusal::name)
    .collect();
    assert_eq!(
        names,
        [
            "rank_width_exceeded",
            "pool_empty",
            "pool_over_run_maximum",
            "no_day",
            "closes_not_pool",
            "minute_out_of_order"
        ]
    );
    let error: &dyn std::error::Error = &RankRefusal::NoDay;
    assert_eq!(error.to_string(), "mover rank refused (no_day): NoDay");
    assert_eq!(rank.gainer_rank(9), 0);
    assert_eq!(rank.loser_rank(9), 0);
    assert_eq!(rank.move_bp(9), None);
    assert_eq!(MEMBER_REFUSALS, 4);
}

/// FM-F005 (D-6314): the rank's exact share of its side's ranked count is
/// stamped beside the raw rank; an unranked member has none.
#[test]
fn the_rank_share_is_exact_and_absent_when_unranked() {
    let mut rank = day(&[Some(1_000), Some(1_000), Some(1_000), Some(1_000)]);
    rank.minute(OPEN, &[Some(1_030), Some(1_010), Some(990), None])
        .unwrap();
    let share = |slot| rank.rank_share(slot).map(|r| (r.num(), r.den()));
    assert_eq!(share(0), Some((1, 2)));
    assert_eq!(share(1), Some((2, 2)));
    assert_eq!(share(2), Some((1, 1)));
    assert_eq!(share(3), None, "missing");
    assert_eq!(share(9), None, "outside the pool");
}

/// FM-F005: decision ranks beyond the first learning window's widest rank are
/// counted per year, as information; a year out of order is refused by name.
#[test]
fn ranks_beyond_the_first_window_are_counted_per_year() {
    use runner::fib::rank::BeyondW0;
    let mut beyond = BeyondW0::new(150);
    for (year, rank) in [(2019, 3), (2019, 151), (2019, 0), (2020, 150), (2020, 260)] {
        beyond.record(year, rank).unwrap();
    }
    assert_eq!(beyond.years(), &[(2019, 2, 1), (2020, 2, 1)]);
    assert_eq!(
        beyond.record(2019, 5),
        Err(RankRefusal::YearOutOfOrder {
            year: 2019,
            last: 2020
        })
    );
    assert_eq!(
        RankRefusal::YearOutOfOrder { year: 0, last: 0 }.name(),
        "year_out_of_order"
    );
    assert_eq!(beyond.years().len(), 2, "nothing counted");
}

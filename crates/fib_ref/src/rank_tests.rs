use super::*;

fn mv(sym: u32, p: i64, c: i64) -> Mover {
    Mover {
        sym,
        p,
        c,
        stale_min: 0,
    }
}

fn find(f: &[Ranked], sym: u32) -> Option<Ranked> {
    f.iter().copied().find(|r| r.sym == sym)
}

#[test]
fn competition_ranks_share_ties_and_skip() {
    // moves: +10%, +5%, +5%, 0, -2%, -2%, -9%
    let pool = [
        mv(1, 110, 100),
        mv(2, 105, 100),
        mv(3, 210, 200),
        mv(4, 50, 50),
        mv(5, 98, 100),
        mv(6, 49, 50),
        mv(7, 91, 100),
    ];
    let f = frame(&pool, None);
    let g: Vec<u32> = f.iter().map(|r| r.gainer).collect();
    let l: Vec<u32> = f.iter().map(|r| r.loser).collect();
    assert_eq!(g, [1, 2, 2, 4, 5, 5, 7]);
    assert_eq!(l, [7, 5, 5, 4, 2, 2, 1]);
    // the edge tie: top 2 holds three stocks
    let top2: Vec<u32> = f
        .iter()
        .filter(|r| r.in_top_long(2, MoverSign::RankOnly))
        .map(|r| r.sym)
        .collect();
    assert_eq!(top2, [1, 2, 3]);
    let bottom2: Vec<u32> = f
        .iter()
        .filter(|r| r.in_top_short(2, MoverSign::RankOnly))
        .map(|r| r.sym)
        .collect();
    assert_eq!(bottom2, [5, 6, 7]);
}

#[test]
fn the_sign_reading_differs_only_on_a_falling_market() {
    let pool = [mv(1, 99, 100), mv(2, 98, 100), mv(3, 100, 100)];
    let f = frame(&pool, None);
    assert!(find(&f, 1).is_some_and(|r| r.in_top_long(10, MoverSign::RankOnly)));
    assert!(find(&f, 1).is_some_and(|r| !r.in_top_long(10, MoverSign::SignedOnly)));
    assert!(find(&f, 3).is_some_and(|r| !r.in_top_short(10, MoverSign::SignedOnly)));
    assert!(find(&f, 3).is_some_and(|r| r.in_top_short(10, MoverSign::RankOnly)));
    assert!(find(&f, 2).is_some_and(|r| r.in_top_short(1, MoverSign::SignedOnly)));
    assert_eq!(find(&f, 3).map(|r| r.sign), Some(0));
}

#[test]
fn exact_cross_multiplication_separates_near_moves() {
    // 1/3 against 333/1000: equal as floats to 3 digits, not exactly
    let a = mv(1, 4, 3);
    let b = mv(2, 1333, 1000);
    assert_eq!(cmp_move(&a, &b), Ordering::Greater);
    let c = mv(3, 2_000_000, 1_500_000); // exactly 1/3
    assert_eq!(cmp_move(&a, &c), Ordering::Equal);
    // the largest paise the store holds do not overflow
    let big = mv(4, 2 * (i64::MAX / 4), i64::MAX / 4);
    let small = mv(5, 2, 1);
    assert_eq!(cmp_move(&big, &small), Ordering::Equal);
}

#[test]
fn stale_and_invalid_stocks_leave_the_pool() {
    let pool = [
        Mover {
            stale_min: 30,
            ..mv(1, 120, 100)
        },
        mv(2, 105, 100),
        mv(3, 105, 0), // no previous close
    ];
    let carried = frame(&pool, None);
    assert_eq!(carried.len(), 2);
    assert!(find(&carried, 1).is_some_and(|r| r.stale && r.gainer == 1));
    let limited = frame(&pool, Some(15));
    assert_eq!(limited.len(), 1);
    assert_eq!(find(&limited, 2).map(|r| r.gainer), Some(1));
    assert!(frame(&[], None).is_empty());
    assert_eq!(frame(&pool, Some(30)).len(), 2);
}

#[test]
fn a_single_stock_is_first_both_ways() {
    let f = frame(&[mv(9, 100, 100)], None);
    assert_eq!((f[0].gainer, f[0].loser), (1, 1));
    let all_equal = frame(&[mv(1, 1, 1), mv(2, 2, 2), mv(3, 3, 3)], None);
    assert!(all_equal.iter().all(|r| r.gainer == 1 && r.loser == 1));
}

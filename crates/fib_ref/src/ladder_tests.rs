use super::*;
use crate::clock::OPEN;

pub(crate) fn candle(i: u32, o: i64, h: i64, l: i64, c: i64) -> Candle {
    Candle {
        i,
        first_minute: i,
        last_minute: i,
        close_t: OPEN + 60 * (i + 1),
        o,
        h,
        l,
        c,
        v: 1,
        incomplete: false,
    }
}

fn fold_all(cs: &[Candle], ties: ExtremeTies, taint: TaintScope) -> Ladder {
    let mut l = Ladder::EMPTY;
    for c in cs {
        l.fold(c, ties, taint);
    }
    l
}

#[test]
fn the_first_candle_holds_both_extremes() {
    let l = fold_all(
        &[candle(0, 100, 110, 90, 105)],
        ExtremeTies::Strict,
        TaintScope::AnyFolded,
    );
    assert_eq!(l.leg, Leg::Undetermined);
    assert!(!l.usable());
    assert_eq!((l.hi, l.lo, l.range()), (110, 90, 20));
    assert!(!Ladder::EMPTY.usable());
    assert_eq!(Ladder::EMPTY.rung_milli(0), None);
}

#[test]
fn direction_follows_the_later_one_sided_extreme() {
    let cs = [
        candle(0, 100, 110, 90, 105),
        candle(1, 105, 112, 100, 111), // new high -> Up
        candle(2, 111, 111, 95, 96),   // inside -> stays Up
        candle(3, 96, 97, 85, 86),     // new low -> Down
        candle(4, 86, 120, 80, 119),   // both -> Undetermined
        candle(5, 119, 125, 110, 124), // new high -> Up
    ];
    let ladders = running(&cs, ExtremeTies::Strict, TaintScope::AnyFolded);
    let legs: Vec<Leg> = ladders.iter().map(|l| l.leg).collect();
    assert_eq!(
        legs,
        [
            Leg::Undetermined,
            Leg::Undetermined,
            Leg::Up,
            Leg::Up,
            Leg::Down,
            Leg::Undetermined,
            Leg::Up
        ]
    );
    assert_eq!(ladders[6].hi_ix, 5);
    assert_eq!(ladders[6].lo_ix, 4);
    assert!(ladders[6].usable());
}

#[test]
fn equal_extremes_move_only_under_the_latest_reading() {
    let cs = [
        candle(0, 100, 110, 90, 105),
        candle(1, 105, 112, 100, 111), // Up
        candle(2, 111, 111, 90, 95),   // equal low
    ];
    let strict = fold_all(&cs, ExtremeTies::Strict, TaintScope::AnyFolded);
    assert_eq!(strict.leg, Leg::Up);
    assert_eq!(strict.lo_ix, 0);
    let latest = fold_all(&cs, ExtremeTies::Latest, TaintScope::AnyFolded);
    assert_eq!(latest.leg, Leg::Down);
    assert_eq!(latest.lo_ix, 2);
    // an equal high under Latest moves the high's index too
    let more = [cs[0], cs[1], candle(2, 111, 112, 101, 105)];
    assert_eq!(
        fold_all(&more, ExtremeTies::Latest, TaintScope::AnyFolded).hi_ix,
        2
    );
    assert_eq!(
        fold_all(&more, ExtremeTies::Strict, TaintScope::AnyFolded).hi_ix,
        1
    );
}

#[test]
fn taint_scopes_differ_on_an_inside_hole() {
    let mut hole = candle(2, 105, 108, 101, 106); // inside candle with a hole
    hole.incomplete = true;
    let cs = [
        candle(0, 100, 110, 90, 105),
        candle(1, 105, 112, 100, 111),
        hole,
    ];
    assert!(fold_all(&cs, ExtremeTies::Strict, TaintScope::AnyFolded).tainted);
    assert!(!fold_all(&cs, ExtremeTies::Strict, TaintScope::ExtremeCandles).tainted);
    let mut extreme_hole = candle(2, 111, 130, 101, 129);
    extreme_hole.incomplete = true;
    let cs2 = [cs[0], cs[1], extreme_hole];
    assert!(fold_all(&cs2, ExtremeTies::Strict, TaintScope::ExtremeCandles).tainted);
    // a first candle with a hole taints from the start
    let mut first = candle(0, 1, 2, 0, 1);
    first.incomplete = true;
    assert!(fold_all(&[first], ExtremeTies::Strict, TaintScope::ExtremeCandles).tainted);
}

#[test]
fn rungs_on_both_legs_match_the_kit_chart() {
    // MAZDOCK's A1 ladder 2,075.00 - 2,093.90 on an Up leg (BRIEF fixture).
    let up = Ladder {
        hi: 209_390,
        lo: 207_500,
        hi_ix: 4,
        lo_ix: 1,
        leg: Leg::Up,
        tainted: false,
        folded: 5,
    };
    assert_eq!(up.range(), 1890);
    assert_eq!(up.rung_milli(0), Some(209_390_000));
    assert_eq!(up.rung_milli(1000), Some(207_500_000));
    assert_eq!(up.rung_milli(786), Some(209_390_000 - 786 * 1890));
    assert_eq!(up.rung_milli(500), Some(208_445_000));
    assert_eq!(up.rung_milli(1272), Some(207_500_000 + 1272 * 1890));
    assert!(up.rung_milli(1272) > up.rung_milli(0));
    let down = Ladder {
        leg: Leg::Down,
        ..up
    };
    assert_eq!(down.rung_milli(0), Some(207_500_000));
    assert_eq!(down.rung_milli(1000), Some(209_390_000));
    assert_eq!(down.rung_milli(382), Some(207_500_000 + 382 * 1890));
    assert_eq!(down.rung_milli(1618), Some(209_390_000 - 1618 * 1890));
    assert!(down.rung_milli(1618) < down.rung_milli(0));
    let und = Ladder {
        leg: Leg::Undetermined,
        ..up
    };
    assert_eq!(und.rung_milli(500), None);
}

#[test]
fn rung_beyond_walks_price_order_over_all_twelve() {
    let up = Ladder {
        hi: 209_390,
        lo: 207_500,
        hi_ix: 4,
        lo_ix: 1,
        leg: Leg::Up,
        tainted: false,
        folded: 5,
    };
    // from 0.786 one rung up is 0.618 ("went to 0.618"), two is 0.5
    assert_eq!(up.rung_beyond(786, 1, true), up.rung_milli(618));
    assert_eq!(up.rung_beyond(786, 2, true), up.rung_milli(500));
    assert_eq!(up.rung_beyond(786, 5, true), up.rung_milli(0));
    assert_eq!(up.rung_beyond(786, 6, true), up.rung_milli(1272));
    assert_eq!(up.rung_beyond(786, 1, false), up.rung_milli(1000));
    assert_eq!(up.rung_beyond(1000, 1, false), None);
    assert_eq!(up.rung_beyond(3000, 1, true), None);
    assert_eq!(up.rung_beyond(786, 0, true), up.rung_milli(786));
    assert_eq!(up.rung_beyond(786, u32::MAX, true), None);
    assert_eq!(up.rung_beyond(1, 1, true), None); // not a rung
    let und = Ladder {
        leg: Leg::Undetermined,
        ..up
    };
    assert_eq!(und.rung_beyond(786, 1, true), None);
}

#[test]
fn relations_mirror_for_the_short() {
    let level = 100_000; // Rs 1.00 in milli-paise of 100 paise
    assert_eq!(relate(101, 99, level, 0, true), Relation::Touch);
    assert_eq!(relate(100, 100, level, 0, true), Relation::Touch);
    assert_eq!(relate(105, 101, level, 0, true), Relation::Clear);
    assert_eq!(relate(99, 95, level, 0, true), Relation::Through);
    assert_eq!(relate(105, 101, level, 0, false), Relation::Through);
    assert_eq!(relate(99, 95, level, 0, false), Relation::Clear);
    // a tolerance of one paisa turns a near miss into a touch
    assert_eq!(relate(105, 101, level, 1_000, true), Relation::Touch);
    assert_eq!(relate(99, 95, level, 1_000, false), Relation::Touch);
    assert_eq!(relate(105, 101, level, 999, true), Relation::Clear);
    assert_eq!(Relation::Touch.name(), "touch");
    assert_eq!(Relation::Clear.name(), "clear");
    assert_eq!(Relation::Through.name(), "through");
    assert_eq!(Leg::Up.name(), "up");
    assert_eq!(Leg::Down.name(), "down");
    assert_eq!(Leg::Undetermined.name(), "undetermined");
}

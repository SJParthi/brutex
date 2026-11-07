use super::*;
use crate::clock::{OPEN, SESSION_END};

fn sec(t: u32, l: i64, h: i64, v: i64) -> Sec {
    Sec {
        t,
        o: l,
        h,
        l,
        c: h,
        v,
    }
}

fn plan(side: Side, decided_at: u32, amount: i64, stop: i64, exit: Exit) -> Plan {
    Plan {
        side,
        decided_at,
        amount,
        stop,
        exit,
        lambda: 14,
        share_ppm: 100_000,
        step: 5,
        session_end: SESSION_END,
        data_start: OPEN,
    }
}

const T0: u32 = OPEN + 600; // 09:25:00

#[test]
fn protection_limits_are_never_kinder() {
    assert_eq!(buy_limit(10_000), 10_300);
    assert_eq!(buy_limit(10_001), 10_301); // 10,301.03 floored
    assert_eq!(sell_limit(10_001), 9_701); // 9,700.97 ceiled
    assert_eq!(buy_limit(5_000), 5_150);
    assert_eq!(sell_limit(5_000), 4_850);
    assert_eq!(buy_limit(5_001), 5_151);
    assert_eq!(sell_limit(5_001), 4_851);
}

#[test]
fn latency_volume_cap_and_worst_print() {
    // decision at T0; the first traded second at or after T0 + 14 starts it
    let secs = vec![
        sec(T0 + 5, 9_990, 10_000, 1_000),
        sec(T0 + 15, 10_000, 10_010, 300),   // fills 30 at 10,010
        sec(T0 + 16, 10_005, 10_020, 0),     // no trade: nothing
        sec(T0 + 17, 10_010, 10_015, 690),   // fills 69 at 10,015 (99 of 100 shares)
        sec(T0 + 18, 10_020, 10_030, 50),    // fills 1 of 5 allowed: done
        sec(T0 + 40, 10_100, 10_200, 1_000), // target 10,150 prints; sells 100 at 10,100
    ];
    let p = plan(Side::Long, T0, 1_000_000, 9_900, Exit::Target(10_150));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!(w.status, Status::Ok);
    assert_eq!(w.start, Some(T0 + 15));
    assert_eq!(w.sizing_px, Some(10_000));
    assert_eq!(w.qty, 100);
    let f: Vec<(u32, i64, i64, u8)> = w
        .fills
        .iter()
        .map(|f| (f.t, f.qty, f.px, f.order))
        .collect();
    assert_eq!(
        f,
        [
            (T0 + 15, 30, 10_010, 0),
            (T0 + 17, 69, 10_015, 0),
            (T0 + 18, 1, 10_030, 0),
            (T0 + 40, 100, 10_100, 1)
        ]
    );
    assert_eq!(w.reason, Reason::Target);
    assert_eq!(w.flags & rule::VOLUME_WALKED, rule::VOLUME_WALKED);
    assert_eq!(w.filled(), 100);
    assert_eq!(
        (w.first_fill(), w.last_fill()),
        (Some(T0 + 15), Some(T0 + 40))
    );
}

#[test]
fn stop_and_target_in_one_second_take_the_stop() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_000, 10_000),
        sec(T0 + 20, 9_800, 10_300, 10_000),
    ];
    let p = plan(Side::Long, T0, 1_000_000, 9_900, Exit::Target(10_200));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!(w.reason, Reason::Stop);
    assert_eq!(w.fills.last().map(|f| (f.px, f.qty)), Some((9_800, 100)));
    assert_eq!(w.flag_names(), "-");
}

#[test]
fn a_stop_through_at_the_first_fill_and_the_within_second_readings() {
    // the first fill second prints through the stop
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 9_850, 10_000, 2_000),
        sec(T0 + 15, 9_840, 9_870, 2_000),
    ];
    let p = plan(Side::Long, T0, 1_000_000, 9_900, Exit::Target(10_500));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_ne!(w.flags & rule::STOP_THROUGH_AT_FILL, 0);
    // entry 100 at 10,000 (budget 200), then the stop sells 100 at 9,850 in the same second
    let f: Vec<(u32, i64, i64, u8)> = w
        .fills
        .iter()
        .map(|f| (f.t, f.qty, f.px, f.order))
        .collect();
    assert_eq!(f, [(T0 + 14, 100, 10_000, 0), (T0 + 14, 100, 9_850, 1)]);
    let x = walk(&secs, &p, WithinSecond::ExitFirst, RestingAfterCut::Acts);
    let f: Vec<(u32, i64, i64, u8)> = x
        .fills
        .iter()
        .map(|f| (f.t, f.qty, f.px, f.order))
        .collect();
    assert_eq!(f, [(T0 + 14, 100, 10_000, 0), (T0 + 15, 100, 9_840, 1)]);
}

#[test]
fn the_short_mirrors_every_price() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 9_990, 10_010, 10_000),
        sec(T0 + 30, 9_700, 9_760, 10_000),
    ];
    let p = plan(Side::Short, T0, 1_000_000, 10_100, Exit::Target(9_750));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    // a short sells at the low and buys back at the high
    let f: Vec<(i64, i64, u8)> = w.fills.iter().map(|f| (f.qty, f.px, f.order)).collect();
    assert_eq!(f, [(100, 9_990, 0), (100, 9_760, 1)]);
    assert_eq!(w.reason, Reason::Target);
}

#[test]
fn refusals_before_the_start() {
    let secs = vec![sec(T0 + 14, 10_000, 10_000, 10)];
    let p = plan(Side::Long, T0, 1_000_000, 9_900, Exit::StopOnly);
    assert_eq!(
        walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts).status,
        Status::NoSizingPrice
    );
    let late = Plan {
        data_start: T0 + 100,
        ..p
    };
    assert_eq!(
        walk(
            &secs,
            &late,
            WithinSecond::EntryFirst,
            RestingAfterCut::Acts
        )
        .status,
        Status::NoFillDataWindow
    );
    let secs2 = vec![
        sec(T0, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_000, 10),
    ];
    let tiny = Plan { amount: 9_999, ..p };
    assert_eq!(
        walk(
            &secs2,
            &tiny,
            WithinSecond::EntryFirst,
            RestingAfterCut::Acts
        )
        .status,
        Status::PriceAbovePosition
    );
    let none: Vec<Sec> = Vec::new();
    let w = walk(&none, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!(
        (w.status, w.reason, w.start),
        (Status::Ok, Reason::NoFill, None)
    );
}

#[test]
fn the_1508_and_1510_clocks() {
    // an entry at 15:07:50 fills; a market exit decided at 15:07:50 would
    // start at 15:08:04: rejected; the broker squares off at 15:10
    let d = CUT - 24; // 15:07:36
    let secs = vec![
        sec(d, 10_000, 10_000, 10),
        sec(d + 14, 10_000, 10_000, 10_000),
        sec(CUT + 4, 10_050, 10_060, 1_000),
        sec(SQUARE_OFF + 30, 9_950, 10_000, 1_000),
        sec(SQUARE_OFF + 60, 9_900, 9_990, 1_000),
    ];
    let p = plan(Side::Long, d, 1_000_000, 9_000, Exit::MarketAt(CUT - 10));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_ne!(w.flags & rule::CUT_REJECTED, 0);
    assert_eq!(w.reason, Reason::SquareOff);
    assert_eq!(
        w.fills.last().map(|f| (f.t, f.px, f.order)),
        Some((SQUARE_OFF, 9_900, 2))
    );
    // no traded second from 15:10: the worst since the entry, flagged
    let secs2 = vec![
        sec(d, 10_000, 10_000, 10),
        sec(d + 14, 9_990, 10_000, 10_000),
        sec(CUT + 4, 9_980, 10_060, 1_000),
    ];
    let w2 = walk(&secs2, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_ne!(w2.flags & rule::UNCLOSABLE, 0);
    assert_eq!(w2.fills.last().map(|f| f.px), Some(9_980));
    // an entry starting at or after 15:08:00 is cancelled before it starts
    let p3 = plan(Side::Long, CUT - 10, 1_000_000, 9_000, Exit::StopOnly);
    let secs3 = vec![
        sec(CUT - 20, 10_000, 10_000, 10),
        sec(CUT + 4, 10_000, 10_000, 1_000),
    ];
    let w3 = walk(&secs3, &p3, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!((w3.reason, w3.entry_end), (Reason::NoFill, Some(CUT)));
    // a resting stop at 15:09 acts, or is rejected under the other reading
    let secs4 = vec![
        sec(d, 10_000, 10_000, 10),
        sec(d + 14, 10_000, 10_000, 10_000),
        sec(CUT + 60, 9_750, 9_790, 10_000),
        sec(SQUARE_OFF + 5, 9_600, 9_650, 1_000),
    ];
    let p4 = plan(Side::Long, d, 1_000_000, 9_800, Exit::StopOnly);
    let acts = walk(&secs4, &p4, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!(
        (acts.reason, acts.fills.last().map(|f| f.px)),
        (Reason::Stop, Some(9_750))
    );
    let rej = walk(
        &secs4,
        &p4,
        WithinSecond::EntryFirst,
        RestingAfterCut::Rejected,
    );
    assert_eq!(
        (rej.reason, rej.fills.last().map(|f| f.px)),
        (Reason::SquareOff, Some(9_600))
    );
}

#[test]
fn xd_moves_the_stop_to_the_entry_after_latency_and_xe_trails() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_002, 10_000), // entry 99 at 10,002 (10 lakh / 10,000 = 100? sized 100 at 10,002 avg)
        sec(T0 + 20, 10_090, 10_110, 10_000), // T1 10,100 prints: move decided at +21, effective >= +35
        sec(T0 + 30, 9_990, 10_000, 10_000),  // the old stop 9,900 still rests: not hit
        sec(T0 + 36, 9_995, 10_005, 10_000),  // the new stop 10,000 is in force: low 9,995 hits it
    ];
    let p = plan(
        Side::Long,
        T0,
        1_000_000,
        9_900,
        Exit::MoveAtBreak {
            trigger: 10_100,
            then_target: 10_400,
        },
    );
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!(w.moves, [(T0 + 36, 10_000)]);
    assert_eq!(w.reason, Reason::Stop);
    assert_eq!(w.fills.last().map(|f| f.px), Some(9_995));
    // XE: armed at 10,100, trailing by 50
    let p2 = plan(
        Side::Long,
        T0,
        1_000_000,
        9_900,
        Exit::Trail {
            arm: 10_100,
            distance: 50,
        },
    );
    let secs2 = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_000, 10_000),
        sec(T0 + 20, 10_100, 10_150, 10_000), // armed, best 10,150: stop 10,100 from +35
        sec(T0 + 25, 10_150, 10_200, 10_000), // best 10,200: stop 10,150 from +40
        sec(T0 + 38, 10_120, 10_160, 10_000), // stop 10,100 in force: not hit (10,120)
        sec(T0 + 41, 10_140, 10_160, 10_000), // stop 10,150 in force: hit at 10,140
    ];
    let w2 = walk(&secs2, &p2, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!(w2.moves, [(T0 + 38, 10_100), (T0 + 41, 10_150)]);
    assert_eq!(
        (w2.reason, w2.fills.last().map(|f| f.px)),
        (Reason::Stop, Some(10_140))
    );
}

#[test]
fn protection_blocks_a_far_print_and_a_part_filled_entry_exits_its_part() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_400, 500), // high beyond 10,300: no fill
        sec(T0 + 15, 10_000, 10_010, 500), // fills 50
        sec(T0 + 16, 9_800, 9_850, 500),   // prints through the stop
        sec(T0 + 17, 9_780, 9_800, 2_000),
    ];
    let p = plan(Side::Long, T0, 1_000_000, 9_900, Exit::StopOnly);
    // entry first: the stop second fills the entry whole before the stop
    // reads it; the exit has no budget left there and sells next second
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_ne!(w.flags & rule::PROTECTION_LIMITED, 0);
    assert_eq!(w.flags & rule::PARTIAL, 0);
    assert_eq!(w.entry_end, None);
    let f: Vec<(u32, i64, i64, u8)> = w
        .fills
        .iter()
        .map(|f| (f.t, f.qty, f.px, f.order))
        .collect();
    assert_eq!(
        f,
        [
            (T0 + 15, 50, 10_010, 0),
            (T0 + 16, 50, 9_850, 0),
            (T0 + 17, 100, 9_780, 1)
        ]
    );
    // exit first: the stop reads the 50 held, cancels the remainder
    let x = walk(&secs, &p, WithinSecond::ExitFirst, RestingAfterCut::Acts);
    assert_ne!(x.flags & rule::PARTIAL, 0);
    assert_eq!(x.entry_end, Some(T0 + 16));
    let f: Vec<(u32, i64, i64, u8)> = x
        .fills
        .iter()
        .map(|f| (f.t, f.qty, f.px, f.order))
        .collect();
    assert_eq!(f, [(T0 + 15, 50, 10_010, 0), (T0 + 16, 50, 9_800, 1)]);
    assert!(x.flag_names().contains("partial"));
}

#[test]
fn a_market_exit_before_any_fill_cancels_the_entry() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_000, 5),
        sec(T0 + 60, 10_000, 10_000, 5),
    ];
    let p = plan(Side::Long, T0, 1_000_000, 9_000, Exit::MarketAt(T0 + 30));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!((w.reason, w.entry_end), (Reason::NoFill, Some(T0 + 60)));
    assert!(w.fills.is_empty());
    for r in [
        Reason::NoFill,
        Reason::Stop,
        Reason::Target,
        Reason::Market,
        Reason::SquareOff,
    ] {
        assert!(!r.name().is_empty());
    }
}

#[test]
fn after_a_rejected_exit_the_resting_stop_still_reads_each_second() {
    let d = CUT - 24;
    let secs = vec![
        sec(d, 10_000, 10_000, 10),
        sec(d + 14, 10_000, 10_000, 10_000),
        sec(CUT + 4, 10_050, 10_060, 1_000), // the market exit would start here: rejected
        sec(CUT + 30, 10_040, 10_050, 1_000), // read again: the market exit is done
        sec(SQUARE_OFF + 1, 10_000, 10_010, 1_000),
        sec(SQUARE_OFF + 2, 9_990, 10_000, 0), // a quiet second: no price for the square-off
    ];
    let p = plan(Side::Long, d, 1_000_000, 9_000, Exit::MarketAt(CUT - 10));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_ne!(w.flags & rule::CUT_REJECTED, 0);
    assert_eq!(w.fills.last().map(|f| (f.order, f.px)), Some((2, 10_000)));
}

#[test]
fn a_trail_step_that_does_not_raise_the_stop_is_not_sent() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_000, 10_000),
        sec(T0 + 20, 10_100, 10_150, 10_000), // armed: 10,150 - 300 = 9,850 is below the stop 9,900
        sec(T0 + 25, 10_150, 10_260, 10_000), // 9,960 raises it
        sec(T0 + 60, 9_955, 9_970, 10_000),   // hit at 9,955 (inside the 9,953 limit)
    ];
    let p = plan(
        Side::Long,
        T0,
        1_000_000,
        9_900,
        Exit::Trail {
            arm: 10_100,
            distance: 300,
        },
    );
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!(w.moves, [(T0 + 60, 9_960)]);
    assert_eq!(
        (w.reason, w.fills.last().map(|f| f.px)),
        (Reason::Stop, Some(9_955))
    );
}

#[test]
fn a_zero_fill_cancel_ends_the_walk_before_later_seconds() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_000, 5), // budget 0
        sec(T0 + 50, 10_000, 10_000, 5), // the market exit cancels the entry here
        sec(T0 + 51, 10_000, 10_000, 10_000),
    ];
    let p = plan(Side::Long, T0, 1_000_000, 9_000, Exit::MarketAt(T0 + 30));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!((w.reason, w.entry_end), (Reason::NoFill, Some(T0 + 50)));
    assert!(w.fills.is_empty());
}

#[test]
fn an_exit_second_beyond_protection_fills_nothing() {
    let secs = vec![
        sec(T0 - 1, 10_000, 10_000, 10),
        sec(T0 + 14, 10_000, 10_000, 10_000),
        sec(T0 + 20, 9_650, 9_850, 10_000), // stop: the exit starts, LTP 10,000, limit 9,700; 9,650 beyond it
        sec(T0 + 21, 9_750, 9_800, 10_000), // inside: sells here
    ];
    let p = plan(Side::Long, T0, 1_000_000, 9_900, Exit::StopOnly);
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_ne!(w.flags & rule::PROTECTION_LIMITED, 0);
    assert_eq!(w.fills.last().map(|f| (f.t, f.px)), Some((T0 + 21, 9_750)));
}

#[test]
fn the_1508_cut_and_a_market_decision_in_the_same_second() {
    // the entry has filled nothing when 15:08:00 cancels it, in the very
    // second the market exit falls due: the cancel stands at 15:08:00
    let d = CUT - 20;
    let secs = vec![
        sec(d, 10_000, 10_000, 10),
        sec(d + 14, 10_000, 10_000, 5),
        sec(CUT, 10_000, 10_000, 5),
    ];
    let p = plan(Side::Long, d, 1_000_000, 9_000, Exit::MarketAt(CUT - 14));
    let w = walk(&secs, &p, WithinSecond::EntryFirst, RestingAfterCut::Acts);
    assert_eq!((w.reason, w.entry_end), (Reason::NoFill, Some(CUT)));
    assert_ne!(w.flags & rule::ENTRY_CUT, 0);
}

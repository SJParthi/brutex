use super::*;
use crate::bars::fold;
use crate::clock::{OPEN, hms};
use crate::e1::tests::minute;

pub(crate) fn seed(side: Side) -> E2Params {
    E2Params {
        side,
        rung: 786,
        tol_milli: 0,
        bounce_m: 1,
        clause: TouchClause::Eq(2),
        mode: LadderMode::Running,
        close_beyond_ends: false,
        reading: Reading::Close,
        stop_offset: 5,
        t1_offset: 5,
        xj_offset: 0,
    }
}

/// A day on the MAZDOCK shape: an up leg lo 10,000 hi 10,500 (R 500),
/// 0.786 at 10,107, 0.618 at 10,191; first touch at 4, bounce to 0.618
/// at 6, return at 8.
pub(crate) fn touch_day() -> Vec<Minute> {
    vec![
        minute(0, 10_050, 10_100, 10_000, 10_090), // low 10,000
        minute(1, 10_090, 10_300, 10_080, 10_290),
        minute(2, 10_290, 10_500, 10_280, 10_450), // high 10,500: Up
        minute(3, 10_450, 10_460, 10_150, 10_160), // clear above 10,107
        minute(4, 10_160, 10_170, 10_100, 10_140), // touches 10,107: n = 1
        minute(5, 10_140, 10_180, 10_120, 10_175), // clear, bounce 10,180
        minute(6, 10_175, 10_200, 10_150, 10_195), // clear, bounce 10,200 >= 10,191: passed
        minute(7, 10_195, 10_196, 10_130, 10_135), // clear, still away
        minute(8, 10_135, 10_140, 10_105, 10_130), // touches again: return n = 2
        minute(9, 10_130, 10_160, 10_120, 10_150),
    ]
}

#[test]
fn the_second_touch_is_counted_and_entered_on_its_close() {
    let day = touch_day();
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let feed = SecondFeed {
        minutes: &day,
        secs: &[],
        step: 5,
        data_start: OPEN,
    };
    let out = e2(
        &candles,
        &top,
        &feed,
        &seed(Side::Long),
        &Readings::HEADLINE,
    );
    let events: Vec<(u32, &str, u32)> = out
        .trace
        .iter()
        .filter(|t| !t.event.is_empty())
        .map(|t| (t.candle, t.event, t.n))
        .collect();
    assert_eq!(events, [(4, "first_touch", 1), (8, "return", 2)]);
    assert_eq!(out.trace[8].gap, Some(3));
    assert!(out.trace[6].passed);
    assert_eq!(out.trace[6].bounce, Some(10_200));
    let sigs: Vec<(u32, u32, Status)> = out
        .signals
        .iter()
        .map(|s| (s.candle, s.n, s.status))
        .collect();
    assert_eq!(sigs, [(4, 1, Status::TouchNumber), (8, 2, Status::Ok)]);
    let s = out.signals[1];
    assert_eq!(s.t.map(hms), Some("09:24:00".to_owned()));
    let lv = s
        .levels
        .map(|l| (l.level_milli, l.pattern_extreme, l.stop_milli, l.xj_milli));
    assert_eq!(lv, Some((10_107_000, 10_100, 10_095_000, Some(10_200_000))));
}

#[test]
fn the_short_mirror_counts_the_same() {
    let day: Vec<Minute> = touch_day()
        .iter()
        .map(|m| Minute {
            o: 30_000 - m.o,
            h: 30_000 - m.l,
            l: 30_000 - m.h,
            c: 30_000 - m.c,
            g: m.g.map(|g| 30_000 - g),
            ..*m
        })
        .collect();
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let feed = SecondFeed {
        minutes: &day,
        secs: &[],
        step: 5,
        data_start: OPEN,
    };
    let out = e2(
        &candles,
        &top,
        &feed,
        &seed(Side::Short),
        &Readings::HEADLINE,
    );
    let ok: Vec<&E2Signal> = out
        .signals
        .iter()
        .filter(|s| s.status == Status::Ok)
        .collect();
    assert_eq!(ok.len(), 1);
    let lv = ok[0]
        .levels
        .map(|l| (l.level_milli, l.pattern_extreme, l.stop_milli));
    assert_eq!(
        lv,
        Some((
            30_000_000 - 10_107_000,
            30_000 - 10_100,
            (30_000 - 10_095) * 1000
        ))
    );
    assert_eq!(out.trace[8].event, "return");
}

#[test]
fn a_hole_ends_the_count_and_a_through_candle_wipes_the_bounce() {
    let mut day = touch_day();
    day[7].held = false;
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let feed = SecondFeed {
        minutes: &day,
        secs: &[],
        step: 5,
        data_start: OPEN,
    };
    let out = e2(
        &candles,
        &top,
        &feed,
        &seed(Side::Long),
        &Readings::HEADLINE,
    );
    assert_eq!(out.trace[7].event, "ended");
    // the headline taint leaves no usable ladder after the hole
    assert_eq!(
        (out.trace[8].phase, out.trace[8].level_milli),
        (Phase::Idle, None)
    );
    assert!(out.signals.iter().all(|s| s.n != 2));
    // a through candle wipes the bounce: no return at 9
    let mut thru = touch_day();
    thru[7] = minute(7, 10_195, 10_196, 10_050, 10_060); // whole candle? no: high above
    thru[7] = minute(7, 10_100, 10_104, 10_050, 10_060); // entirely below 10,107
    let candles = fold(&thru, 1);
    let out = e2(
        &candles,
        &top,
        &SecondFeed {
            minutes: &thru,
            ..feed
        },
        &seed(Side::Long),
        &Readings::HEADLINE,
    );
    assert_eq!(out.trace[7].rel, Some(Relation::Through));
    assert_eq!(out.trace[7].bounce, None);
    assert_eq!(out.trace[8].event, "merge");
}

#[test]
fn the_retouch_readings_differ_on_the_gap() {
    // touch at 4, a re-touch at 6 before the bounce passes, a pass at 8,
    // a return at 10
    let day = vec![
        minute(0, 10_050, 10_100, 10_000, 10_090),
        minute(1, 10_090, 10_300, 10_080, 10_290),
        minute(2, 10_290, 10_500, 10_280, 10_450),
        minute(3, 10_450, 10_460, 10_150, 10_160),
        minute(4, 10_160, 10_170, 10_100, 10_140), // n = 1
        minute(5, 10_140, 10_150, 10_120, 10_145), // clear, bounce 10,150
        minute(6, 10_145, 10_150, 10_104, 10_120), // re-touch, not passed
        minute(7, 10_120, 10_160, 10_110, 10_155), // clear
        minute(8, 10_155, 10_210, 10_150, 10_205), // passes 10,191
        minute(9, 10_205, 10_206, 10_150, 10_160),
        minute(10, 10_160, 10_170, 10_101, 10_150), // return
    ];
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let feed = SecondFeed {
        minutes: &day,
        secs: &[],
        step: 5,
        data_start: OPEN,
    };
    let merge = e2(
        &candles,
        &top,
        &feed,
        &seed(Side::Long),
        &Readings::HEADLINE,
    );
    assert_eq!(merge.trace[10].gap, Some(3));
    assert_eq!(merge.trace[6].event, "merge");
    let mut r = Readings::HEADLINE;
    r.retouch = Retouch::Ignore;
    let ignore = e2(&candles, &top, &feed, &seed(Side::Long), &r);
    assert_eq!(ignore.trace[10].gap, Some(5));
    assert_eq!(ignore.trace[6].event, "");
    // the pattern extreme differs too: 10,101 both, but 10,104 joins only on merge
    assert_eq!(
        merge.signals[1].levels.map(|l| l.pattern_extreme),
        Some(10_100)
    );
}

#[test]
fn the_second_reading_triggers_inside_the_return_candle() {
    let day = touch_day();
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    // seconds of minutes 7 and 8: price falls to 10,107 at 09:23:35
    let secs: Vec<Sec> = (0..120)
        .map(|k| {
            let t = OPEN + 7 * 60 + k;
            let px = if k == 95 { 10_105 } else { 10_150 };
            Sec {
                t,
                o: px,
                h: px,
                l: px,
                c: px,
                v: 10,
            }
        })
        .collect();
    let feed = SecondFeed {
        minutes: &day,
        secs: &secs,
        step: 5,
        data_start: OPEN,
    };
    let p = E2Params {
        reading: Reading::Second,
        ..seed(Side::Long)
    };
    let out = e2(&candles, &top, &feed, &p, &Readings::HEADLINE);
    let s: Vec<&E2Signal> = out.signals.iter().collect();
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!(s[0].armed_at.map(hms), Some("09:22:00".to_owned()));
    assert_eq!(s[0].trigger, Some(10_105)); // 10,107 rounded down to the step
    assert_eq!(s[0].trigger_s.map(hms), Some("09:23:35".to_owned()));
    assert_eq!(s[0].t.map(hms), Some("09:23:36".to_owned()));
    assert_eq!(s[0].end, Some((OPEN + 9 * 60, "touch_candle")));
    assert_eq!(s[0].status, Status::Ok);
    assert_eq!(s[0].n, 2);
    // the resting reading is refused by name
    let p = E2Params {
        reading: Reading::Resting,
        ..seed(Side::Long)
    };
    let out = e2(&candles, &top, &feed, &p, &Readings::HEADLINE);
    assert!(
        out.signals
            .iter()
            .all(|s| s.status == Status::TouchOrderUnsourced)
    );
}

fn run(day: &[Minute], secs: &[Sec], top: &[bool], p: &E2Params, r: &Readings) -> E2Out {
    let candles = fold(day, 1);
    let feed = SecondFeed {
        minutes: day,
        secs,
        step: 5,
        data_start: OPEN,
    };
    e2(&candles, top, &feed, p, r)
}

/// One second at each listed offset from 09:15:00 at that price; volume 10.
fn prints(list: &[(u32, i64)]) -> Vec<Sec> {
    list.iter()
        .map(|&(off, px)| Sec {
            t: OPEN + off,
            o: px,
            h: px,
            l: px,
            c: px,
            v: 10,
        })
        .collect()
}

#[test]
fn a_second_hole_after_the_count_ended_changes_nothing() {
    let mut day = touch_day();
    day[6].held = false;
    day[7].held = false;
    let top = vec![true; day.len()];
    let mut r = Readings::HEADLINE;
    r.taint = crate::ladder::TaintScope::ExtremeCandles; // keep the ladder usable past the holes
    let out = run(&day, &[], &top, &seed(Side::Long), &r);
    assert_eq!(out.trace[6].event, "ended");
    assert_eq!((out.trace[7].phase, out.trace[7].event), (Phase::Ended, ""));
    // an ended book still reads each complete candle's relation
    assert_eq!(out.trace[8].phase, Phase::Ended);
    assert!(out.trace[8].rel.is_some());
}

#[test]
fn a_bounce_rung_that_does_not_exist_never_passes() {
    let day = touch_day();
    let top = vec![true; day.len()];
    let p = E2Params {
        bounce_m: u32::MAX,
        ..seed(Side::Long)
    };
    let out = run(&day, &[], &top, &p, &Readings::HEADLINE);
    assert!(out.trace.iter().all(|t| !t.passed));
    assert!(out.trace.iter().all(|t| t.event != "return"));
}

#[test]
fn s29b_both_ways_and_on_a_through_candle() {
    let day = touch_day();
    let top = vec![true; day.len()];
    let p = E2Params {
        close_beyond_ends: true,
        ..seed(Side::Long)
    };
    // no close below 0.786 on the standard day: the count goes on
    let out = run(&day, &[], &top, &p, &Readings::HEADLINE);
    assert_eq!(out.trace[8].event, "return");
    // a through candle whose close sits below the level ends the count by name
    let mut thru = touch_day();
    thru[7] = minute(7, 10_100, 10_104, 10_050, 10_060);
    let out = run(&thru, &[], &top, &p, &Readings::HEADLINE);
    assert_eq!(
        (out.trace[7].event, out.trace[7].phase),
        ("ended", Phase::Ended)
    );
}

#[test]
fn an_outside_bar_return_leaves_no_rung_target() {
    let mut day = touch_day();
    // the return is an outside bar: a new low and a new high in one candle
    day[8] = minute(8, 10_135, 10_600, 9_900, 10_130);
    let top = vec![true; day.len()];
    let out = run(&day, &[], &top, &seed(Side::Long), &Readings::HEADLINE);
    let s = out.signals.iter().find(|s| s.n == 2).copied();
    assert_eq!(
        s.and_then(|s| s.levels).map(|l| (l.t2_milli, l.t3_milli)),
        Some((None, None))
    );
    // the judging ladder still prices both
    let mut r = Readings::HEADLINE;
    r.target_ladder = TargetLadder::Judging;
    let out = run(&day, &[], &top, &seed(Side::Long), &r);
    let s = out.signals.iter().find(|s| s.n == 2).copied();
    assert!(
        s.and_then(|s| s.levels)
            .is_some_and(|l| l.t2_milli.is_some())
    );
}

#[test]
fn a_return_after_1507_59_is_after_the_last_entry() {
    // 355 flat minutes inside the day's coming range, then the touch day
    let mut day: Vec<Minute> = (0..355)
        .map(|m| minute(m, 10_250, 10_250, 10_250, 10_250))
        .collect();
    for (k, mut m) in touch_day().into_iter().enumerate() {
        m.m = 355 + u32::try_from(k).unwrap_or(0);
        day.push(m);
    }
    let top = vec![true; day.len()];
    let out = run(&day, &[], &top, &seed(Side::Long), &Readings::HEADLINE);
    let s: Vec<(u32, Status)> = out.signals.iter().map(|s| (s.n, s.status)).collect();
    assert!(s.contains(&(2, Status::AfterLastEntry)), "{s:?}");
}

#[test]
fn e2s_refusals_and_its_ends() {
    let day = touch_day();
    let top = vec![true; day.len()];
    let p = E2Params {
        reading: Reading::Second,
        ..seed(Side::Long)
    };
    let r = Readings::HEADLINE;
    // armed at 09:22:00 (candle 6); a quiet second at the level is skipped,
    // the next print reaching 10,105 triggers
    let mut secs = prints(&[
        (7 * 60 + 20, 10_150),
        (7 * 60 + 30, 10_100),
        (7 * 60 + 40, 10_104),
    ]);
    secs[1].v = 0;
    let out = run(&day, &secs, &top, &p, &r);
    assert_eq!(out.signals[0].trigger_s, Some(OPEN + 7 * 60 + 40));
    // out of top N at the arming candle
    let mut off = top.clone();
    off[6] = false;
    let out = run(&day, &secs, &off, &p, &r);
    assert_eq!(out.signals[0].status, Status::NotInTopN);
    // the judging ladder for the second reading's targets
    let mut j = r;
    j.target_ladder = TargetLadder::Judging;
    let out = run(&day, &secs, &top, &p, &j);
    assert!(out.signals[0].levels.is_some());
    // a data start after the arming close + 14 s
    let feed = SecondFeed {
        minutes: &day,
        secs: &secs,
        step: 5,
        data_start: OPEN + 3_600,
    };
    let out = e2(&fold(&day, 1), &top, &feed, &p, &r);
    assert_eq!(out.signals[0].status, Status::NoFillDataWindow);
    // a hole after the arming ends it at the hole minute's close
    let mut holed = day.clone();
    holed[7].held = false;
    let out = run(&holed, &[], &top, &p, &r);
    assert_eq!(out.signals[0].end, Some((OPEN + 8 * 60, "hole")));
    // a through candle after the arming ends it at its close
    let mut thru = day.clone();
    thru[7] = minute(7, 10_100, 10_104, 10_050, 10_060);
    let out = run(&thru, &[], &top, &p, &r);
    assert_eq!(out.signals[0].end, Some((OPEN + 8 * 60, "through")));
    // a new high after the arming (a clear candle) changes the signature: the
    // arming ends at that candle's close
    let mut high = day.clone();
    high[7] = minute(7, 10_195, 10_600, 10_150, 10_590);
    let out = run(&high, &[], &top, &p, &r);
    assert_eq!(out.signals[0].end, Some((OPEN + 8 * 60, "signature")));
    // a bounce that passes on a new high arms and ends at once
    let mut hi = day.clone();
    hi[6] = minute(6, 10_175, 10_600, 10_150, 10_195);
    let out = run(&hi, &[], &top, &E2Params { bounce_m: 6, ..p }, &r);
    assert!(out.signals.iter().all(|s| s.trigger_s.is_none()));
}

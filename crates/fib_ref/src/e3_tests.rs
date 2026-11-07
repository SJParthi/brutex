use super::*;
use crate::bars::fold;
use crate::clock::hms;
use crate::e1::tests::minute;

fn seed(side: Side, d: u32) -> E3Params {
    E3Params {
        side,
        d,
        reading: E3Reading::Close,
        clause: BreakClause::First,
        stop_offset: 5,
        t1_offset: 5,
    }
}

/// A range of 2 minutes (10,000 - 10,100), then a break at minute 4, a
/// fall back inside at 5 and a second break at 6.
fn orb_day() -> Vec<Minute> {
    vec![
        minute(0, 10_050, 10_100, 10_000, 10_060),
        minute(1, 10_060, 10_080, 10_020, 10_070),
        minute(2, 10_070, 10_095, 10_050, 10_090), // inside
        minute(3, 10_090, 10_100, 10_080, 10_100), // at H: not above
        minute(4, 10_100, 10_130, 10_095, 10_120), // first break
        minute(5, 10_120, 10_125, 10_090, 10_095), // back inside
        minute(6, 10_095, 10_140, 10_090, 10_135), // second break
        minute(7, 10_135, 10_150, 10_130, 10_145),
    ]
}

#[test]
fn the_range_and_its_refusals() {
    let day = orb_day();
    let r = range(&day, 2);
    assert_eq!((r.h, r.l, r.status), (10_100, 10_000, Status::Ok));
    assert_eq!(hms(r.close_t), "09:17:00");
    let mut holed = day.clone();
    holed[1].held = false;
    assert_eq!(range(&holed, 2).status, Status::OrbTainted);
    assert_eq!(range(&holed, 1).status, Status::Ok);
    let mut padded = day.clone();
    padded[0].padded = true; // padded with a GDFL trade
    assert_eq!(range(&padded, 1).status, Status::OrbTainted);
    let flat = vec![minute(0, 100, 100, 100, 100)];
    assert_eq!(range(&flat, 1).status, Status::OrbNoWidth);
    assert_eq!(range(&flat, 2).status, Status::OrbTainted); // not enough data
    assert_eq!(range(&day, 0).d, 1);
    assert_eq!(DURATIONS.len(), 8);
}

#[test]
fn every_fresh_cross_is_a_break_numbered_by_its_ordinal() {
    let day = orb_day();
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let feed = SecondFeed {
        minutes: &day,
        secs: &[],
        step: 5,
        data_start: OPEN,
    };
    let out = e3(
        &candles,
        &top,
        &feed,
        &seed(Side::Long, 2),
        &Readings::HEADLINE,
    );
    let got: Vec<(u32, u32, Status)> = out
        .events
        .iter()
        .map(|e| (e.candle, e.ordinal, e.status))
        .collect();
    assert_eq!(got, [(4, 1, Status::Ok), (6, 2, Status::BreakOrdinal)]);
    let lv = out.events[0]
        .levels
        .map(|l| (l.pattern_extreme, l.stop_milli, l.t1_milli));
    assert_eq!(lv, Some((10_000, 9_995_000, 10_135_000)));
    let any = E3Params {
        clause: BreakClause::Any,
        ..seed(Side::Long, 2)
    };
    let out = e3(&candles, &top, &feed, &any, &Readings::HEADLINE);
    assert!(out.events.iter().all(|e| e.status == Status::Ok));
    assert!(BreakClause::Second.passes(2) && !BreakClause::Second.passes(3));
    assert!(BreakClause::SecondOrLater.passes(3) && !BreakClause::SecondOrLater.passes(1));
    assert_eq!(BreakClause::Any.name(), "any");
}

#[test]
fn no_record_inside_the_range_and_the_short_mirror() {
    let day = orb_day();
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let feed = SecondFeed {
        minutes: &day,
        secs: &[],
        step: 5,
        data_start: OPEN,
    };
    // d = 5: the range is minutes 0..4 (H 10,130); the first break is at 6
    let out = e3(
        &candles,
        &top,
        &feed,
        &seed(Side::Long, 5),
        &Readings::HEADLINE,
    );
    assert_eq!(out.events.iter().map(|e| e.candle).collect::<Vec<_>>(), [6]);
    // the mirrored day breaks the mirrored low
    let mirrored: Vec<Minute> = day
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
    let mc = fold(&mirrored, 1);
    let mf = SecondFeed {
        minutes: &mirrored,
        ..feed
    };
    let out = e3(&mc, &top, &mf, &seed(Side::Short, 2), &Readings::HEADLINE);
    assert_eq!(
        out.events
            .iter()
            .map(|e| (e.candle, e.ordinal))
            .collect::<Vec<_>>(),
        [(4, 1), (6, 2)]
    );
    assert_eq!(
        out.events[0].levels.map(|l| l.stop_milli),
        Some((30_000 - 9_995) * 1000)
    );
    let tainted = SecondFeed {
        minutes: &[],
        ..feed
    };
    assert!(
        e3(
            &candles,
            &top,
            &tainted,
            &seed(Side::Long, 2),
            &Readings::HEADLINE
        )
        .events
        .is_empty()
    );
}

#[test]
fn the_second_reading_triggers_after_the_blind_14_seconds() {
    let day = orb_day();
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    // during minute 4 a print above 10,100 at 09:19:10 (inside the 14 s
    // after the 09:19:00 close: unseen) and at 09:19:20
    let secs: Vec<Sec> = (0..60)
        .map(|k| {
            let t = OPEN + 4 * 60 + k;
            let px = if k == 10 || k >= 20 { 10_110 } else { 10_090 };
            Sec {
                t,
                o: px,
                h: px,
                l: px,
                c: px,
                v: 5,
            }
        })
        .collect();
    let feed = SecondFeed {
        minutes: &day,
        secs: &secs,
        step: 5,
        data_start: OPEN,
    };
    let p = E3Params {
        reading: E3Reading::Second,
        ..seed(Side::Long, 2)
    };
    let out = e3(&candles, &top, &feed, &p, &Readings::HEADLINE);
    let hit: Vec<(u32, Option<u32>)> = out.events.iter().map(|e| (e.candle, e.trigger_s)).collect();
    assert_eq!(hit, [(3, Some(OPEN + 4 * 60 + 20))]);
    assert_eq!(out.events[0].t, Some(OPEN + 4 * 60 + 21));
    assert_eq!(out.events[0].trigger, Some(10_100));
    // the boundary reading lets the candle closing at 09:17:00 arm
    let mut r = Readings::HEADLINE;
    r.orb_boundary = OrbBoundary::Arms;
    let secs2: Vec<Sec> = (0..60)
        .map(|k| {
            let t = OPEN + 2 * 60 + k;
            let px = if k >= 30 { 10_110 } else { 10_090 };
            Sec {
                t,
                o: px,
                h: px,
                l: px,
                c: px,
                v: 5,
            }
        })
        .collect();
    let f2 = SecondFeed {
        secs: &secs2,
        ..feed
    };
    let head = e3(&candles, &top, &f2, &p, &Readings::HEADLINE);
    assert!(head.events.is_empty());
    let arms = e3(&candles, &top, &f2, &p, &r);
    assert_eq!(
        arms.events.iter().map(|e| e.candle).collect::<Vec<_>>(),
        [1]
    );
    // a late data start and a minute with no GDFL price refuse by name
    let late = SecondFeed {
        data_start: OPEN + 3600,
        ..feed
    };
    assert!(
        e3(&candles, &top, &late, &p, &Readings::HEADLINE)
            .events
            .iter()
            .all(|e| e.status == Status::NoFillDataWindow)
    );
    let mut quiet = day.clone();
    for m in &mut quiet[2..] {
        m.g = None;
    }
    let qf = SecondFeed {
        minutes: &quiet,
        ..feed
    };
    assert!(
        e3(&candles, &top, &qf, &p, &Readings::HEADLINE)
            .events
            .iter()
            .all(|e| e.status == Status::NoTriggerRatio)
    );
}

fn run3(day: &[Minute], secs: &[Sec], top: &[bool], p: &E3Params, r: &Readings) -> E3Out {
    let candles = fold(day, 1);
    let feed = SecondFeed {
        minutes: day,
        secs,
        step: 5,
        data_start: OPEN,
    };
    e3(&candles, top, &feed, p, r)
}

#[test]
fn e3c_clauses_and_refusals_by_name() {
    let day = orb_day();
    let r = Readings::HEADLINE;
    let p = seed(Side::Long, 2);
    // the boundary reading lets the 09:17 candle in, where nothing breaks
    let mut arms = r;
    arms.orb_boundary = OrbBoundary::Arms;
    let out = run3(&day, &[], &[true; 8], &p, &arms);
    assert_eq!(
        out.events.iter().map(|e| e.candle).collect::<Vec<_>>(),
        [4, 6]
    );
    // out of top N at the break
    let mut off = vec![true; 8];
    off[4] = false;
    let out = run3(&day, &[], &off, &p, &r);
    assert_eq!(out.events[0].status, Status::NotInTopN);
    // an incomplete break candle
    let mut holed = day.clone();
    holed[4].padded = true; // padded with a GDFL trade: a hole
    let out = run3(&holed, &[], &[true; 8], &p, &r);
    assert_eq!(out.events[0].status, Status::IncompleteCandle);
    // the judging ladder for the rung targets
    let mut j = r;
    j.target_ladder = TargetLadder::Judging;
    let out = run3(&day, &[], &[true; 8], &p, &j);
    assert!(out.events[0].levels.is_some());
}

#[test]
fn a_break_after_1507_59_is_after_the_last_entry() {
    let mut day: Vec<Minute> = (0..375)
        .map(|m| minute(m, 10_050, 10_060, 10_040, 10_050))
        .collect();
    day[0] = minute(0, 10_050, 10_100, 10_000, 10_060);
    day[370] = minute(370, 10_050, 10_130, 10_045, 10_120); // breaks 10,100 at 15:26
    let top = vec![true; 375];
    let out = run3(&day, &[], &top, &seed(Side::Long, 1), &Readings::HEADLINE);
    assert_eq!(
        out.events
            .iter()
            .map(|e| (e.candle, e.status))
            .collect::<Vec<_>>(),
        [(370, Status::AfterLastEntry)]
    );
    // and on the second reading, armed at 15:25, triggered at 15:25:30
    let secs: Vec<Sec> = (0..60u32)
        .map(|k| {
            let px = if k >= 30 { 10_110 } else { 10_050 };
            Sec {
                t: OPEN + 370 * 60 + k,
                o: px,
                h: px,
                l: px,
                c: px,
                v: 5,
            }
        })
        .collect();
    let p = E3Params {
        reading: E3Reading::Second,
        ..seed(Side::Long, 1)
    };
    let out = run3(&day, &secs, &top, &p, &Readings::HEADLINE);
    assert_eq!(
        out.events.iter().map(|e| e.status).collect::<Vec<_>>(),
        [Status::AfterLastEntry]
    );
}

#[test]
fn e3s_skips_an_incomplete_arming_candle_and_a_quiet_second() {
    let day = orb_day();
    let p = E3Params {
        reading: E3Reading::Second,
        ..seed(Side::Long, 2)
    };
    let r = Readings::HEADLINE;
    let secs: Vec<Sec> = (0..60u32)
        .map(|k| {
            let px = if k == 20 || k >= 25 { 10_110 } else { 10_090 };
            Sec {
                t: OPEN + 4 * 60 + k,
                o: px,
                h: px,
                l: px,
                c: px,
                v: if k == 20 { 0 } else { 5 },
            }
        })
        .collect();
    let out = run3(&day, &secs, &[true; 8], &p, &r);
    assert_eq!(out.events[0].trigger_s, Some(OPEN + 4 * 60 + 25));
    // out of top N at the arming candle
    let mut off = vec![true; 8];
    off[3] = false;
    let out = run3(&day, &secs, &off, &p, &r);
    assert_eq!(out.events[0].status, Status::NotInTopN);
    // an incomplete arming candle arms nothing
    let mut holed = day.clone();
    holed[3].held = false;
    let out = run3(&holed, &secs, &[true; 8], &p, &r);
    assert!(out.events.iter().all(|e| e.candle != 3));
}

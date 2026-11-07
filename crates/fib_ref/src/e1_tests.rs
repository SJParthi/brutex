use super::*;
use crate::bars::fold;
use crate::clock::{OPEN, hms};

pub(crate) fn seed(side: Side, k: u32) -> E1Params {
    E1Params {
        side,
        k,
        rung: 382,
        holding: Holding::AllLows,
        mode: LadderMode::Running,
        max_height_ppm: None,
        colour: true,
        stop_offset: 5,
        t1_offset: 5,
    }
}

pub(crate) fn minute(m: u32, o: i64, h: i64, l: i64, c: i64) -> Minute {
    Minute {
        m,
        o,
        h,
        l,
        c,
        v: 100,
        held: true,
        padded: false,
        g: Some(c),
    }
}

/// The operator's flag on a synthetic session: a push to a new high at
/// minute 3, a two-candle box holding above 0.382, a green break at 6.
pub(crate) fn flag_day() -> Vec<Minute> {
    vec![
        minute(0, 10_000, 10_020, 9_950, 10_010),
        minute(1, 10_010, 10_080, 10_000, 10_070),
        minute(2, 10_070, 10_150, 10_060, 10_140),
        minute(3, 10_140, 10_200, 10_130, 10_190), // push extreme 10,200
        minute(4, 10_190, 10_195, 10_160, 10_170), // box
        minute(5, 10_170, 10_185, 10_165, 10_180), // box
        minute(6, 10_180, 10_210, 10_175, 10_205), // green break above 10,195
        minute(7, 10_205, 10_230, 10_200, 10_225),
    ]
}

fn mirror_day(day: &[Minute]) -> Vec<Minute> {
    // a real short day: prices reflected about 20,000 paise
    day.iter()
        .map(|m| Minute {
            o: 20_000 - m.o,
            h: 20_000 - m.l,
            l: 20_000 - m.h,
            c: 20_000 - m.c,
            g: m.g.map(|g| 20_000 - g),
            ..*m
        })
        .collect()
}

#[test]
fn the_operators_flag_gives_exactly_one_signal_at_the_break_close() {
    let candles = fold(&flag_day(), 1);
    let top = vec![true; candles.len()];
    let events = e1c(&candles, &top, &seed(Side::Long, 2), &Readings::HEADLINE);
    let ok: Vec<&E1Event> = events.iter().filter(|e| e.status == Status::Ok).collect();
    assert_eq!(ok.len(), 1, "{events:?}");
    let e = ok[0];
    assert_eq!(e.candle, 6);
    assert_eq!(hms(e.t), "09:22:00");
    assert_eq!(
        (e.ext, e.box_first, e.box_high, e.box_low),
        (3, 4, 10_195, 10_160)
    );
    let lv = e.levels.unwrap_or(Levels {
        pattern_extreme: 0,
        stop_milli: 0,
        t1_milli: 0,
        t2_milli: None,
        t3_milli: None,
    });
    assert_eq!(lv.stop_milli, 10_155_000);
    assert_eq!(lv.t1_milli, 10_215_000); // new high 10,210 + 5
    // after fold: lo 9,950 hi 10,210, R 260: 1.272 -> 9,950 + 330.72
    assert_eq!(lv.t2_milli, Some(9_950_000 + 1272 * 260));
    assert_eq!(lv.t3_milli, Some(9_950_000 + 1618 * 260));
}

#[test]
fn its_short_mirror_gives_the_mirrored_signal() {
    let candles = fold(&mirror_day(&flag_day()), 1);
    let top = vec![true; candles.len()];
    let events = e1c(&candles, &top, &seed(Side::Short, 2), &Readings::HEADLINE);
    let ok: Vec<&E1Event> = events.iter().filter(|e| e.status == Status::Ok).collect();
    assert_eq!(ok.len(), 1, "{events:?}");
    let e = ok[0];
    assert_eq!(e.candle, 6);
    assert_eq!((e.box_high, e.box_low), (20_000 - 10_195, 20_000 - 10_160));
    let lv = e.levels.map(|l| (l.stop_milli, l.t1_milli));
    assert_eq!(lv, Some((9_845_000, 9_785_000)));
}

#[test]
#[allow(clippy::too_many_lines, reason = "every clause, refused in order")]
fn clauses_refuse_by_name_in_their_order() {
    let day = flag_day();
    let p = seed(Side::Long, 2);
    let r = Readings::HEADLINE;
    let top_off = vec![false; 8];
    let candles = fold(&day, 1);
    let s = e1c(&candles, &top_off, &p, &r);
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::NotInTopN)
    );
    // a red break candle
    let mut red = day.clone();
    red[6] = minute(6, 10_209, 10_212, 10_175, 10_200);
    let top = vec![true; 8];
    let s = e1c(&fold(&red, 1), &top, &p, &r);
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Colour)
    );
    // colour off accepts it
    let s = e1c(&fold(&red, 1), &top, &E1Params { colour: false, ..p }, &r);
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Ok)
    );
    // a tight clause the box fails: height 35 of R 250 is 140,000 ppm
    let s = e1c(
        &candles,
        &top,
        &E1Params {
            max_height_ppm: Some(100_000),
            ..p
        },
        &r,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Height)
    );
    let s = e1c(
        &candles,
        &top,
        &E1Params {
            max_height_ppm: Some(140_000),
            ..p
        },
        &r,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Ok)
    );
    // a deep box below 0.382 fails the holding rule
    let mut deep = day.clone();
    deep[4] = minute(4, 10_190, 10_195, 10_050, 10_170);
    let s = e1c(&fold(&deep, 1), &top, &p, &r);
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Holding)
    );
    let s = e1c(
        &fold(&deep, 1),
        &top,
        &E1Params {
            holding: Holding::Off,
            ..p
        },
        &r,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Ok)
    );
    let s = e1c(
        &fold(&deep, 1),
        &top,
        &E1Params {
            holding: Holding::AllCloses,
            ..p
        },
        &r,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Ok)
    );
    let s = e1c(
        &fold(&deep, 1),
        &top,
        &E1Params {
            holding: Holding::NoCloseBelowFirstLow,
            ..p
        },
        &r,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Ok)
    );
    // frozen mode reads the ladder at the box's first candle
    let s = e1c(
        &candles,
        &top,
        &E1Params {
            mode: LadderMode::Frozen,
            ..p
        },
        &r,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Ok)
    );
}

#[test]
fn an_incomplete_candle_voids_the_scope_and_a_tainted_ladder_refuses() {
    let mut day = flag_day();
    day[5].held = false; // a Kite hole inside the box
    let candles = fold(&day, 1);
    let top = vec![true; 8];
    let s = e1c(&candles, &top, &seed(Side::Long, 2), &Readings::HEADLINE);
    assert!(
        s.iter()
            .any(|e| e.candle == 5 && e.status == Status::IncompleteCandle && e.levels.is_none())
    );
    assert!(s.iter().all(|e| e.status != Status::Ok));
    // under the extreme-candles taint a hole before the push leaves the
    // break signalling; under the headline taint it is refused by name
    let mut early = flag_day();
    early[1].held = false;
    let s = e1c(
        &fold(&early, 1),
        &top,
        &seed(Side::Long, 2),
        &Readings::HEADLINE,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::TaintedLadder)
    );
    let mut r = Readings::HEADLINE;
    r.taint = crate::ladder::TaintScope::ExtremeCandles;
    let s = e1c(&fold(&early, 1), &top, &seed(Side::Long, 2), &r);
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::Ok)
    );
    // an incomplete break candle
    let mut bad_break = flag_day();
    bad_break[6].padded = true; // padded with a GDFL trade
    let s = e1c(
        &fold(&bad_break, 1),
        &top,
        &seed(Side::Long, 2),
        &Readings::HEADLINE,
    );
    assert_eq!(
        s.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::IncompleteCandle)
    );
}

#[test]
fn readings_after_a_break_and_of_the_box_anchor_differ() {
    // a break at 6 without a new high, then a second box [7, 8] broken at
    // 9 by a close equal to the session high (not a new high)
    let mut day: Vec<Minute> = flag_day().into_iter().take(6).collect();
    day.push(minute(6, 10_180, 10_198, 10_175, 10_197));
    day.push(minute(7, 10_197, 10_199, 10_180, 10_185));
    day.push(minute(8, 10_185, 10_196, 10_182, 10_190));
    day.push(minute(9, 10_190, 10_200, 10_186, 10_200));
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let breaks = |r: &Readings| -> Vec<u32> {
        e1c(&candles, &top, &seed(Side::Long, 2), r)
            .iter()
            .filter(|e| e.status == Status::Ok)
            .map(|e| e.candle)
            .collect()
    };
    assert_eq!(breaks(&Readings::HEADLINE), [6]);
    let mut r = Readings::HEADLINE;
    r.after_break = AfterBreak::NextBox;
    assert_eq!(breaks(&r), [6, 9]);
    // anchored against sliding: no break on the candle after the box,
    // then a close above the anchored box but not the slid one
    let mut slow: Vec<Minute> = flag_day().into_iter().take(6).collect();
    slow.push(minute(6, 10_180, 10_199, 10_170, 10_175)); // red, no break
    slow.push(minute(7, 10_175, 10_198, 10_172, 10_197)); // > 10,195, < 10,199
    let candles = fold(&slow, 1);
    let top = vec![true; candles.len()];
    let ok = |r: &Readings| -> Vec<(u32, u32)> {
        e1c(&candles, &top, &seed(Side::Long, 2), r)
            .iter()
            .filter(|e| e.status == Status::Ok)
            .map(|e| (e.candle, e.box_first))
            .collect()
    };
    assert!(ok(&Readings::HEADLINE).is_empty());
    r = Readings::HEADLINE;
    r.box_anchor = BoxAnchor::Anchored;
    assert_eq!(ok(&r), [(7, 4)]);
    assert_eq!(box_for(5, 4, 2, BoxAnchor::Sliding), None);
    assert_eq!(box_for(6, 4, 2, BoxAnchor::Sliding), Some((4, 5)));
    assert_eq!(box_for(9, 4, 2, BoxAnchor::Sliding), Some((7, 8)));
    assert_eq!(box_for(9, 4, 2, BoxAnchor::Anchored), Some((4, 5)));
    assert_eq!(box_for(9, 4, 0, BoxAnchor::Sliding), Some((8, 8)));
}

#[test]
fn the_second_reading_triggers_on_the_first_second_beyond() {
    let day = flag_day();
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    // seconds of minute 6 (09:21): a print above 10,195 at 09:21:20
    let secs: Vec<Sec> = (0..60)
        .map(|k| {
            let t = OPEN + 6 * 60 + k;
            let px = if k >= 20 { 10_200 } else { 10_185 };
            Sec {
                t,
                o: px,
                h: px,
                l: px,
                c: px,
                v: 50,
            }
        })
        .collect();
    let feed = SecondFeed {
        minutes: &day,
        secs: &secs,
        step: 5,
        data_start: OPEN,
    };
    let ev = e1s(
        &candles,
        &top,
        &feed,
        &seed(Side::Long, 2),
        &Readings::HEADLINE,
    );
    let hit = ev.iter().find(|e| e.t.is_some()).copied();
    assert_eq!(
        hit.and_then(|e| e.trigger_s).map(hms),
        Some("09:21:20".to_owned())
    );
    assert_eq!(hit.and_then(|e| e.t).map(hms), Some("09:21:21".to_owned()));
    assert_eq!(hit.map(|e| e.candle), Some(5));
    assert_eq!(hit.and_then(|e| e.trigger), Some(10_195));
    assert_eq!(hit.map(|e| e.status), Some(Status::Ok)); // the arming candle 5 is green
    // data start after the arming close + 14 s refuses by name
    let late = SecondFeed {
        data_start: OPEN + 23 * 60,
        ..feed
    };
    let ev = e1s(
        &candles,
        &top,
        &late,
        &seed(Side::Long, 2),
        &Readings::HEADLINE,
    );
    assert!(ev.iter().any(|e| e.status == Status::NoFillDataWindow));
    // no GDFL price in R1's minute refuses the trigger by name
    let mut quiet = day.clone();
    for m in &mut quiet {
        m.g = None;
    }
    let feed2 = SecondFeed {
        minutes: &quiet,
        ..feed
    };
    let ev = e1s(
        &candles,
        &top,
        &feed2,
        &seed(Side::Long, 2),
        &Readings::HEADLINE,
    );
    assert!(ev.iter().all(|e| e.status == Status::NoTriggerRatio));
}

/// Seconds over minutes `from..to`: price `base` except prints of `hit` at
/// the listed offsets from 09:15:00 (one share each, volume 50).
fn secs_with(from: u32, to: u32, base: i64, hits: &[(u32, i64)]) -> Vec<Sec> {
    (OPEN + 60 * from..OPEN + 60 * to)
        .map(|t| {
            let px = hits
                .iter()
                .find(|(at, _)| *at == t - OPEN)
                .map_or(base, |(_, p)| *p);
            Sec {
                t,
                o: px,
                h: px,
                l: px,
                c: px,
                v: 50,
            }
        })
        .collect()
}

fn e1s_run(
    day: &[Minute],
    secs: &[Sec],
    top: &[bool],
    p: &E1Params,
    r: &Readings,
) -> Vec<E1sEvent> {
    let candles = fold(day, 1);
    let feed = SecondFeed {
        minutes: day,
        secs,
        step: 5,
        data_start: OPEN,
    };
    e1s(&candles, top, &feed, p, r)
}

#[test]
fn e1s_clauses_at_the_trigger_refuse_by_name() {
    let day = flag_day();
    let secs = secs_with(6, 8, 10_185, &[(6 * 60 + 20, 10_200)]);
    let top = vec![true; 8];
    let p = seed(Side::Long, 2);
    let r = Readings::HEADLINE;
    // out of top N at the arming candle
    let mut off = top.clone();
    off[5] = false;
    let ev = e1s_run(&day, &secs, &off, &p, &r);
    assert_eq!(
        ev.iter().find(|e| e.t.is_some()).map(|e| e.status),
        Some(Status::NotInTopN)
    );
    // colour clause off on a red arming candle
    let mut red = day.clone();
    red[5] = minute(5, 10_185, 10_190, 10_165, 10_170);
    let ev = e1s_run(&red, &secs, &top, &E1Params { colour: false, ..p }, &r);
    assert_eq!(
        ev.iter().find(|e| e.t.is_some()).map(|e| e.status),
        Some(Status::Ok)
    );
    let ev = e1s_run(&red, &secs, &top, &p, &r);
    assert_eq!(
        ev.iter().find(|e| e.t.is_some()).map(|e| e.status),
        Some(Status::Colour)
    );
    // a hole before the push taints the ladder: refused by name at the trigger
    // (with no holding rule: the seed rule already needs a usable ladder)
    let mut early = day.clone();
    early[1].held = false;
    let ev = e1s_run(
        &early,
        &secs,
        &top,
        &E1Params {
            holding: Holding::Off,
            ..p
        },
        &r,
    );
    assert_eq!(
        ev.iter().find(|e| e.t.is_some()).map(|e| e.status),
        Some(Status::TaintedLadder)
    );
    // the push candle holds both extremes: the ladder stays Undetermined
    let mut both = day.clone();
    both[3] = minute(3, 10_140, 10_200, 9_900, 10_190);
    let ev = e1s_run(
        &both,
        &secs,
        &top,
        &E1Params {
            holding: Holding::Off,
            ..p
        },
        &r,
    );
    assert_eq!(
        ev.iter().find(|e| e.t.is_some()).map(|e| e.status),
        Some(Status::LadderUndetermined)
    );
    let candles = fold(&both, 1);
    let c = e1c(&candles, &top, &p, &r);
    assert_eq!(
        c.iter().find(|e| e.candle == 6).map(|e| e.status),
        Some(Status::LadderUndetermined)
    );
}

#[test]
fn e1s_scope_rules_a_hole_a_consumed_box_an_anchored_box_and_a_failed_box() {
    let p = seed(Side::Long, 2);
    let r = Readings::HEADLINE;
    // after the trigger (09:21:20) no new high comes: the scope stays consumed
    let mut day: Vec<Minute> = flag_day().into_iter().take(6).collect();
    day.push(minute(6, 10_180, 10_198, 10_175, 10_190)); // no new high
    day.push(minute(7, 10_190, 10_196, 10_180, 10_185));
    day.push(minute(8, 10_185, 10_195, 10_180, 10_190));
    let top = vec![true; day.len()];
    let secs = secs_with(6, 9, 10_185, &[(6 * 60 + 20, 10_198)]);
    let ev = e1s_run(&day, &secs, &top, &p, &r);
    assert_eq!(ev.iter().filter(|e| e.t.is_some()).count(), 1);
    // re-armed on the next box instead: armings continue after the trigger
    let mut nb = r;
    nb.after_break = AfterBreak::NextBox;
    let more = e1s_run(&day, &secs, &top, &p, &nb);
    assert!(more.len() >= ev.len());
    // a hole inside the box voids the arming
    let mut holed = flag_day();
    holed[5].held = false;
    let secs2 = secs_with(6, 8, 10_185, &[(6 * 60 + 20, 10_200)]);
    let ev = e1s_run(&holed, &secs2, &[true; 8], &p, &r);
    assert!(ev.iter().all(|e| e.candle != 5));
    // the anchored box arms only once its K candles are complete
    let mut an = r;
    an.box_anchor = BoxAnchor::Anchored;
    let ev = e1s_run(&flag_day(), &secs2, &[true; 8], &p, &an);
    assert_eq!(
        ev.iter()
            .find(|e| e.t.is_some())
            .map(|e| (e.candle, e.status)),
        Some((5, Status::Ok))
    );
    // a box below the holding level never arms
    let mut deep = flag_day();
    deep[4] = minute(4, 10_190, 10_195, 10_000, 10_170);
    let ev = e1s_run(&deep, &secs2, &[true; 8], &p, &r);
    assert!(ev.iter().all(|e| e.t.is_none()));
    // an untraded second in the window is skipped, not read
    let mut quiet = secs2.clone();
    if let Some(s) = quiet.iter_mut().find(|s| s.t == OPEN + 6 * 60 + 15) {
        s.v = 0;
        s.h = 10_300;
    }
    let ev = e1s_run(&flag_day(), &quiet, &[true; 8], &p, &r);
    assert_eq!(
        ev.iter().find_map(|e| e.trigger_s),
        Some(OPEN + 6 * 60 + 20)
    );
}

#[test]
fn e1c_a_hole_after_a_consumed_scope_writes_no_void() {
    let mut day = flag_day();
    day.push(minute(8, 10_225, 10_228, 10_215, 10_220));
    day.push(minute(9, 10_220, 10_224, 10_212, 10_218));
    day[9].held = false; // the scope was consumed by the break at 6? no: 6 made a new high
    let candles = fold(&day, 1);
    let top = vec![true; candles.len()];
    let r = Readings::HEADLINE;
    let ev = e1c(&candles, &top, &seed(Side::Long, 2), &r);
    // the break at 6 made a new high (re-armed); 7 made another; the hole at 9 voids that scope once
    assert_eq!(
        ev.iter()
            .filter(|e| e.status == Status::IncompleteCandle)
            .count(),
        1
    );
    // a second hole right after finds the scope already void: no second void
    let mut two = day.clone();
    two.push(minute(10, 10_218, 10_220, 10_210, 10_215));
    two[10].held = false;
    let candles = fold(&two, 1);
    let top = vec![true; candles.len()];
    let ev = e1c(&candles, &top, &seed(Side::Long, 2), &r);
    assert_eq!(
        ev.iter()
            .filter(|e| e.status == Status::IncompleteCandle)
            .count(),
        1
    );
}

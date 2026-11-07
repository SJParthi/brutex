use super::*;
use crate::clock::hms;

fn m(m: u32, o: i64, h: i64, l: i64, c: i64) -> Minute {
    Minute {
        m,
        o,
        h,
        l,
        c,
        v: 10,
        held: true,
        padded: false,
        g: Some(c),
    }
}

fn day(n: u32) -> Vec<Minute> {
    (0..n)
        .map(|i| {
            let base = 10_000 + i64::from(i) * 5;
            m(i, base, base + 10, base - 10, base + 5)
        })
        .collect()
}

#[test]
fn hole_and_r1_follow_the_three_minute_kinds() {
    let held = m(0, 1, 2, 0, 1);
    assert!(!held.hole());
    assert_eq!(held.r1(), Some((1, 1)));
    let missing = Minute {
        held: false,
        ..held
    };
    assert!(missing.hole());
    assert_eq!(missing.r1(), None);
    let padded_traded = Minute {
        padded: true,
        ..held
    };
    assert!(padded_traded.hole());
    assert_eq!(padded_traded.r1(), None);
    let padded_quiet = Minute {
        padded: true,
        g: None,
        ..held
    };
    assert!(!padded_quiet.hole());
    assert_eq!(padded_quiet.r1(), None);
    let no_gdfl = Minute { g: None, ..held };
    assert!(!no_gdfl.hole());
    assert_eq!(no_gdfl.r1(), None);
}

#[test]
fn fold_forms_whole_candles_on_the_0915_grid() {
    let minutes = day(375);
    for tf in TIMEFRAMES {
        let candles = fold(&minutes, tf);
        let expect = 375_u32.div_ceil(tf);
        assert_eq!(u32::try_from(candles.len()).ok(), Some(expect), "tf {tf}");
        let last = candles.last().copied();
        assert_eq!(last.map(|c| c.close_t), Some(CANDLE_END), "tf {tf}");
        assert_eq!(candles[0].close_t, OPEN + 60 * tf.min(375));
    }
    let five = fold(&minutes, 5);
    assert_eq!(five[0].o, minutes[0].o);
    assert_eq!(five[0].h, minutes[4].h);
    assert_eq!(five[0].l, minutes[0].l);
    assert_eq!(five[0].c, minutes[4].c);
    assert_eq!(five[0].v, 50);
    assert_eq!(hms(five[1].close_t), "09:25:00");
    let sixty = fold(&minutes, 60);
    assert_eq!(sixty.len(), 7);
    assert_eq!(hms(sixty[6].close_t), "15:30:00");
    assert_eq!(sixty[6].first_minute, 360);
    assert_eq!(sixty[6].last_minute, 374);
}

#[test]
fn fold_stops_at_the_data_end_and_marks_holes() {
    let mut minutes = day(12);
    // a partial last candle is not formed
    assert_eq!(fold(&minutes, 5).len(), 2);
    assert_eq!(fold(&minutes, 0).len(), 12); // tf 0 reads as 1
    assert!(fold(&[], 5).is_empty());
    minutes[6].held = false;
    let candles = fold(&minutes, 5);
    assert!(!candles[0].incomplete);
    assert!(candles[1].incomplete);
    // the missing minute does not price the candle
    assert_eq!(candles[1].h, minutes[9].h);
    minutes[2].padded = true; // padded with a GDFL trade: a hole
    assert!(fold(&minutes, 5)[0].incomplete);
    minutes[2].g = None; // padded, nobody traded: held
    assert!(!fold(&minutes, 5)[0].incomplete);
}

#[test]
fn a_candle_with_no_held_minute_is_flat_and_incomplete() {
    let mut minutes = day(4);
    for minute in &mut minutes[2..4] {
        minute.held = false;
    }
    let candles = fold(&minutes, 2);
    assert!(candles[1].incomplete);
    assert_eq!(candles[1].o, candles[0].c);
    assert_eq!(candles[1].h, candles[0].c);
    let all_missing: Vec<Minute> = day(2)
        .into_iter()
        .map(|x| Minute { held: false, ..x })
        .collect();
    let flat = fold(&all_missing, 2);
    assert_eq!((flat[0].o, flat[0].c), (0, 0));
}

#[test]
fn colours_are_strict() {
    let c = Candle {
        i: 0,
        first_minute: 0,
        last_minute: 0,
        close_t: OPEN + 60,
        o: 10,
        h: 12,
        l: 9,
        c: 11,
        v: 1,
        incomplete: false,
    };
    assert!(c.green() && !c.red());
    let r = Candle { c: 9, ..c };
    assert!(r.red() && !r.green());
    let flat = Candle { c: 10, ..c };
    assert!(!flat.red() && !flat.green());
}

#[test]
fn minute_and_second_lookups() {
    assert_eq!(last_minute_closed_by(OPEN), None);
    assert_eq!(last_minute_closed_by(OPEN + 59), None);
    assert_eq!(last_minute_closed_by(OPEN + 60), Some(0));
    assert_eq!(last_minute_closed_by(OPEN + 119), Some(0));
    assert_eq!(last_minute_closed_by(CANDLE_END), Some(374));
    assert_eq!(last_minute_closed_by(CANDLE_END + 500), Some(374));
    let secs = [
        Sec {
            t: 10,
            o: 5,
            h: 5,
            l: 5,
            c: 5,
            v: 0,
        },
        Sec {
            t: 12,
            o: 6,
            h: 7,
            l: 6,
            c: 7,
            v: 3,
        },
        Sec {
            t: 15,
            o: 8,
            h: 8,
            l: 8,
            c: 8,
            v: 0,
        },
        Sec {
            t: 20,
            o: 9,
            h: 9,
            l: 9,
            c: 9,
            v: 1,
        },
    ];
    assert_eq!(first_at_or_after(&secs, 0), 0);
    assert_eq!(first_at_or_after(&secs, 13), 2);
    assert_eq!(first_at_or_after(&secs, 99), 4);
    assert_eq!(first_traded_at_or_after(&secs, 0).map(|s| s.t), Some(12));
    assert_eq!(first_traded_at_or_after(&secs, 13).map(|s| s.t), Some(20));
    assert_eq!(first_traded_at_or_after(&secs, 21), None);
    assert_eq!(last_print_before(&secs, 12), None);
    assert_eq!(last_print_before(&secs, 13), Some(7));
    assert_eq!(last_print_before(&secs, 20), Some(7));
    assert_eq!(last_print_before(&secs, 21), Some(9));
    assert!(!secs[0].traded() && secs[1].traded());
}

#[test]
fn the_clock_grid_starts_with_a_part_candle_to_the_first_multiple() {
    use crate::model::CandleGrid;
    let day: Vec<Minute> = (0..375).map(|k| m(k, 100, 101, 99, 100)).collect();
    let closes = |tf: u32, g: CandleGrid| -> Vec<String> {
        fold_on(&day, tf, g)
            .iter()
            .map(|c| hms(c.close_t))
            .collect()
    };
    // the grids agree where 09:15 (minute 555 of the day) is on the clock grid
    for tf in [1, 3, 5, 15] {
        assert_eq!(fold_on(&day, tf, CandleGrid::Clock), fold(&day, tf), "{tf}");
    }
    let hours = [
        "10:00:00", "11:00:00", "12:00:00", "13:00:00", "14:00:00", "15:00:00",
    ];
    let mut clock60: Vec<String> = hours.iter().map(|s| (*s).to_owned()).collect();
    clock60.push("15:30:00".to_owned());
    assert_eq!(closes(60, CandleGrid::Clock), clock60);
    assert_eq!(
        closes(60, CandleGrid::Session),
        [
            "10:15:00", "11:15:00", "12:15:00", "13:15:00", "14:15:00", "15:15:00", "15:30:00"
        ]
    );
    // 555 is odd: the first two-minute clock candle is 09:15 alone
    let c2 = fold_on(&day, 2, CandleGrid::Clock);
    assert_eq!(
        (c2[0].first_minute, c2[0].last_minute, hms(c2[0].close_t)),
        (0, 0, "09:16:00".to_owned())
    );
    assert_eq!((c2.len(), fold(&day, 2).len()), (188, 188));
    assert_eq!(closes(10, CandleGrid::Clock)[..2], ["09:20:00", "09:30:00"]);
    assert_eq!(closes(30, CandleGrid::Clock)[0], "09:30:00");
    assert_eq!(closes(30, CandleGrid::Session)[0], "09:45:00");
}

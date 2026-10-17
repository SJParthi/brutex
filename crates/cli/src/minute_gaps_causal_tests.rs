//! Causal row admission, minute-gap policy 4 (lookahead-oos-1,
//! g-zerodha-universe-census-3).
//!
//! Every bar here is invented: constant paisa prices no index or share ever
//! printed, on the exchange calendar's own session minutes.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;

const MINUTE: i64 = MINUTE_MICROS;

/// One invented session: its IST day, the minute ranges the feed lost, and the
/// minute it stopped at, if it stopped early.
type DaySpec = (i64, Vec<std::ops::RangeInclusive<i64>>, Option<i64>);

/// A named [`DaySpec`].
type Case = (
    &'static str,
    i64,
    Vec<std::ops::RangeInclusive<i64>>,
    Option<i64>,
);

fn day_of(year: u16, month: u8, date: u8) -> i64 {
    i64::from(
        pull::session::Day::new(year, month, date)
            .expect("date")
            .days_from_epoch(),
    )
}

/// The open of minute `minute_of_day` IST on `day`.
fn stamp(day: i64, minute_of_day: i64) -> i64 {
    day * DAY_MICROS + minute_of_day * MINUTE - indicators::IST_OFFSET_MICROS
}

fn candle(ts_micros: i64) -> Candle {
    Candle {
        ts_micros,
        open: 4_242,
        high: 4_343,
        low: 4_141,
        close: 4_242,
        volume: 7,
        open_interest: i64::MIN,
    }
}

/// Every session minute the calendar holds for `day`, in order.
fn session_minutes(day: i64) -> Vec<i64> {
    let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
        return Vec::new();
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| i64::from(window.from)..=i64::from(window.to))
        .collect()
}

/// A day's minutes as the vendor served them: every session minute except
/// `missing`, and nothing at or after `stop` (minute of day).
fn served(day: i64, missing: &[std::ops::RangeInclusive<i64>], stop: Option<i64>) -> Vec<Candle> {
    session_minutes(day)
        .into_iter()
        .filter(|minute| !missing.iter().any(|range| range.contains(minute)))
        .filter(|minute| stop.is_none_or(|stop| *minute < stop))
        .map(|minute| candle(stamp(day, minute)))
        .collect()
}

/// The vendor's own signal bars at `length` minutes: one per bucket of the
/// calendar's session, holes or not, stamped at the bucket's open.
fn buckets(day: i64, length: i64) -> Vec<Candle> {
    let minutes = session_minutes(day);
    let (Some(&first), Some(&last)) = (minutes.first(), minutes.last()) else {
        return Vec::new();
    };
    (0..=(last - first) / length)
        .map(|k| first + k * length)
        .map(|open| candle(stamp(day, open)))
        .collect()
}

/// A month-shaped series of `days`, each with its own holes: (minutes,
/// signal at `length`).
fn series(days: &[DaySpec], length: i64) -> (Vec<Candle>, Vec<Candle>) {
    let mut minutes = Vec::new();
    let mut signal = Vec::new();
    // In time order, as a stored month is.
    let mut ordered: Vec<&DaySpec> = days.iter().collect();
    ordered.sort_by_key(|entry| entry.0);
    for (day, missing, stop) in ordered {
        let served = served(*day, missing, *stop);
        if length == 1 {
            signal.extend(served.iter().copied());
        } else {
            signal.extend(buckets(*day, length));
        }
        minutes.extend(served);
    }
    (minutes, signal)
}

fn admitted(signal: &[Candle], minutes: &[Candle], length: i64) -> Vec<Candle> {
    let cuts = days_with_minute_holes(signal, minutes, length * MINUTE, |day| {
        crate::stored::nse_session_close_minute(day)
    });
    withhold(signal, &cuts).0
}

fn hhmm(hour: i64, minute: i64) -> i64 {
    hour * 60 + minute
}

/// The days the prefix property is checked on: every shape a cut can take.
fn prefix_property_cases() -> Vec<Case> {
    vec![
        (
            "hole at 10:51",
            day_of(2024, 4, 23),
            vec![hhmm(10, 51)..=hhmm(10, 56)],
            None,
        ),
        (
            "hole at 14:02",
            day_of(2024, 4, 24),
            vec![hhmm(14, 2)..=hhmm(14, 2)],
            None,
        ),
        (
            "two holes",
            day_of(2024, 4, 25),
            vec![hhmm(10, 51)..=hhmm(10, 52), hhmm(14, 2)..=hhmm(14, 3)],
            None,
        ),
        (
            "missing closing minutes",
            day_of(2024, 4, 26),
            Vec::new(),
            Some(hhmm(15, 25)),
        ),
        (
            "listing shape",
            day_of(2024, 4, 29),
            vec![hhmm(9, 16)..=hhmm(9, 39)],
            None,
        ),
        (
            "hole in the last minute",
            day_of(2024, 4, 30),
            vec![hhmm(15, 28)..=hhmm(15, 28)],
            None,
        ),
        (
            "special Saturday 2024-01-20",
            day_of(2024, 1, 20),
            vec![hhmm(14, 2)..=hhmm(14, 2)],
            None,
        ),
        (
            "budget Saturday 2020-02-01",
            day_of(2020, 2, 1),
            vec![hhmm(14, 2)..=hhmm(14, 2)],
            None,
        ),
        (
            "hole at 09:15 alone",
            day_of(2024, 5, 2),
            vec![hhmm(9, 15)..=hhmm(9, 15)],
            None,
        ),
        (
            "hole at 09:15..09:20",
            day_of(2024, 5, 3),
            vec![hhmm(9, 15)..=hhmm(9, 20)],
            None,
        ),
        ("halt 2020-03-13", day_of(2020, 3, 13), Vec::new(), None),
        ("halt 2020-03-23", day_of(2020, 3, 23), Vec::new(), None),
    ]
}

/// THE PREFIX PROPERTY. For every cut `t` on a 15-minute grid, the rows at or
/// before `t` admitted from the whole month equal those admitted from the
/// month truncated at `t` (signal bars through `t`, minutes through the close
/// of the bar at `t`): a row's existence never depends on a later minute.
///
/// Under policy 3 a holed day lost every row, so its 09:15..10:45 rows were
/// admitted from the truncated month and missing from the whole one: this
/// fails on the 10:51 hole, the 14:02 hole, the missing closing minute, the
/// listing shape, both Saturdays and both halts.
#[test]
fn a_row_before_a_later_hole_is_admitted() {
    let cases = prefix_property_cases();
    for (label, day, missing, stop) in &cases {
        assert!(
            !session_minutes(*day).is_empty(),
            "premise: {label} is a session"
        );
        // The session BEFORE it, clean, so every cut also compares a whole
        // earlier day's rows and a rung that admits nothing on the holed day
        // (15min across a 09:22 halt) still compares something.
        let quiet = (1..10)
            .map(|back| *day - back)
            .find(|candidate| !session_minutes(*candidate).is_empty())
            .expect("a session within ten days before");
        let days = vec![(quiet, Vec::new(), None), (*day, missing.clone(), *stop)];
        for length in [1_i64, 2, 5, 15] {
            let (minutes, signal) = series(&days, length);
            let whole = admitted(&signal, &minutes, length);
            let mut checked = 0_usize;
            for t in (0..)
                .map(|k| stamp(*day, hhmm(9, 15) + 15 * k))
                .take_while(|t| *t <= stamp(*day, hhmm(15, 30)))
            {
                let pre_signal: Vec<Candle> = signal
                    .iter()
                    .copied()
                    .filter(|bar| bar.ts_micros <= t)
                    .collect();
                let pre_minutes: Vec<Candle> = minutes
                    .iter()
                    .copied()
                    .filter(|bar| bar.ts_micros <= t + (length - 1) * MINUTE)
                    .collect();
                let early = admitted(&pre_signal, &pre_minutes, length);
                let late: Vec<Candle> = whole
                    .iter()
                    .copied()
                    .filter(|bar| bar.ts_micros <= t)
                    .collect();
                assert_eq!(
                    early.len(),
                    late.len(),
                    "{label} at {length}min, cut {}: a row's existence depends on later minutes",
                    (t + indicators::IST_OFFSET_MICROS).rem_euclid(DAY_MICROS) / MINUTE
                );
                assert_eq!(early, late, "{label} at {length}min");
                checked += early.len();
            }
            assert!(checked > 0, "{label} at {length}min compared rows");
        }
    }
}

/// From the FIRST hole on, every row of the day is withheld -- between two
/// holes and after the second too -- and every row wholly before it is
/// admitted; the quiet day around it is untouched.
#[test]
fn rows_from_the_first_hole_on_stay_withheld() {
    let quiet = day_of(2024, 4, 22);
    let holed = day_of(2024, 4, 25);
    let days = vec![
        (quiet, Vec::new(), None),
        (
            holed,
            vec![hhmm(10, 51)..=hhmm(10, 52), hhmm(14, 2)..=hhmm(14, 3)],
            None,
        ),
    ];
    for length in [1_i64, 5, 15] {
        let (minutes, signal) = series(&days, length);
        let cuts = days_with_minute_holes(&signal, &minutes, length * MINUTE, |day| {
            crate::stored::nse_session_close_minute(day)
        });
        assert_eq!(days_of(&cuts), vec![holed], "{length}min");
        assert_eq!(
            cuts[0].at_micros,
            stamp(holed, hhmm(10, 51)),
            "cut at the FIRST hole"
        );
        assert_eq!(cuts[0].cause, CutCause::Hole);
        let (kept, removed) = withhold(&signal, &cuts);
        for bar in &signal {
            let on_holed = indicators::ist_day(bar.ts_micros) == holed;
            let wholly_before = bar.ts_micros + length * MINUTE <= stamp(holed, hhmm(10, 51));
            assert_eq!(
                kept.contains(bar),
                !on_holed || wholly_before,
                "{length}min bar at {}",
                (bar.ts_micros + indicators::IST_OFFSET_MICROS).rem_euclid(DAY_MICROS) / MINUTE
            );
        }
        let exclusion = GapExclusion::signal_only(&cuts, &signal);
        let cut = exclusion.cuts()[0];
        assert_eq!(cut.withheld, removed);
        let on_day = signal
            .iter()
            .filter(|b| indicators::ist_day(b.ts_micros) == holed)
            .count();
        assert_eq!(
            cut.admitted + cut.withheld,
            u64::try_from(on_day).expect("count")
        );
        assert!(exclusion.day_names()[0].starts_with("2024-04-25 from 10:51 (minute hole): "));
    }
    // 15min: 09:15..10:30 admitted (six buckets), 10:45 holds the hole.
    let (minutes, signal) = series(&days, 15);
    let cuts = days_with_minute_holes(&signal, &minutes, 15 * MINUTE, |day| {
        crate::stored::nse_session_close_minute(day)
    });
    assert_eq!(
        GapExclusion::signal_only(&cuts, &signal).cuts()[0].admitted,
        6
    );
}

/// The extremes: an empty month, every session holed, a hole right after the
/// open (the whole day goes on a coarse rung), a hole in the last minute, a
/// holed first and last session of the month.
#[test]
fn the_causal_rule_at_its_extremes() {
    assert!(days_with_minute_holes(&[], &[], 15 * MINUTE, |_| None).is_empty());
    assert_eq!(withhold(&[], &[]), (Vec::new(), 0));

    let first = day_of(2024, 4, 1);
    let last = day_of(2024, 4, 30);
    let every: Vec<_> = [first, day_of(2024, 4, 2), last]
        .into_iter()
        .map(|day| (day, vec![hhmm(9, 16)..=hhmm(9, 16)], None))
        .collect();
    let (minutes, signal) = series(&every, 15);
    let cuts = days_with_minute_holes(&signal, &minutes, 15 * MINUTE, |day| {
        crate::stored::nse_session_close_minute(day)
    });
    assert_eq!(days_of(&cuts), vec![first, day_of(2024, 4, 2), last]);
    assert!(
        withhold(&signal, &cuts).0.is_empty(),
        "a 09:16 hole takes every 15min row"
    );
    // On 1min the 09:15 row of each day survives.
    let (minutes, signal) = series(&every, 1);
    let cuts = days_with_minute_holes(&signal, &minutes, MINUTE, |_| None);
    let kept = withhold(&signal, &cuts).0;
    assert_eq!(kept.len(), 3);
    assert!(kept.iter().all(|bar| {
        (bar.ts_micros + indicators::IST_OFFSET_MICROS).rem_euclid(DAY_MICROS) / MINUTE
            == hhmm(9, 15)
    }));

    // The last minute (15:29) absent: the 15:15 bucket's closing minute is
    // demanded and missing, so only it goes; on 1min nothing demands it.
    let tail = vec![(last, Vec::new(), Some(hhmm(15, 29)))];
    let (minutes, signal) = series(&tail, 15);
    let cuts = days_with_minute_holes(&signal, &minutes, 15 * MINUTE, |day| {
        crate::stored::nse_session_close_minute(day)
    });
    assert_eq!(cuts.len(), 1);
    assert_eq!(cuts[0].cause, CutCause::ClosingMinute);
    assert_eq!(withhold(&signal, &cuts).1, 1, "only the 15:15 bucket");
    let (minutes, signal) = series(&tail, 1);
    assert!(days_with_minute_holes(&signal, &minutes, MINUTE, |_| None).is_empty());

    // A day cut twice by the retry keeps the earlier cut, and refuses a
    // later or equal one (the recurrence the retry loops refuse on).
    let mut held = vec![Cut::new(
        first,
        stamp(first, hhmm(11, 0)),
        CutCause::Hole,
        15 * MINUTE,
    )];
    assert!(!tighten(
        &mut held,
        Cut::new(
            first,
            stamp(first, hhmm(12, 0)),
            CutCause::Unsourceable,
            15 * MINUTE
        )
    ));
    assert!(tighten(
        &mut held,
        Cut::new(
            first,
            stamp(first, hhmm(10, 0)),
            CutCause::Unsourceable,
            15 * MINUTE
        )
    ));
    assert!(tighten(
        &mut held,
        Cut::new(
            last,
            stamp(last, hhmm(10, 0)),
            CutCause::Unsourceable,
            15 * MINUTE
        )
    ));
    assert!(tighten(
        &mut held,
        Cut::new(
            first - 1,
            stamp(first - 1, hhmm(10, 0)),
            CutCause::Unsourceable,
            15 * MINUTE
        )
    ));
    assert_eq!(days_of(&held), vec![first - 1, first, last]);
    assert_eq!(held[1].at_micros, stamp(first, hhmm(10, 0)));
}

/// A RECORDED HALT WITHHOLDS FROM ITS START, whatever the feed served in it.
///
/// Invented index minutes, flat and complete (375 a day, no hole) through the
/// 2020-03-13 and 2020-03-23 `HALTS` windows. Every bucket that reaches past
/// the halt's first whole minute is withheld on every rung; every bucket
/// wholly before it is admitted; the exclusion names the halt.
#[test]
fn a_recorded_halt_withholds_from_its_start() {
    for (day, halt_from, expected) in [
        // 09:22 is the first whole halted minute on 2020-03-13.
        (
            day_of(2020, 3, 13),
            hhmm(9, 22),
            [(1, 7), (2, 3), (5, 1), (15, 0)],
        ),
        // 09:59 on 2020-03-23.
        (
            day_of(2020, 3, 23),
            hhmm(9, 59),
            [(1, 44), (2, 22), (5, 8), (15, 2)],
        ),
    ] {
        let halt = pull::calendar::halt_on(day).expect("a recorded halt");
        assert_eq!(i64::from(halt.halted.from), halt_from, "premise: HALTS");
        assert_eq!(served(day, &[], None).len(), 375, "premise: no minute hole");
        for (length, before) in expected {
            let (minutes, signal) = series(&[(day, Vec::new(), None)], length);
            let cuts = days_with_minute_holes(&signal, &minutes, length * MINUTE, |day| {
                crate::stored::nse_session_close_minute(day)
            });
            assert_eq!(days_of(&cuts), vec![day], "{length}min");
            assert_eq!(cuts[0].cause, CutCause::Halt);
            assert_eq!(cuts[0].at_micros, stamp(day, halt_from));
            let kept = withhold(&signal, &cuts).0;
            assert_eq!(
                kept.len(),
                before,
                "{length}min rows wholly before the halt"
            );
            assert!(
                kept.iter()
                    .all(|bar| bar.ts_micros + length * MINUTE <= stamp(day, halt_from))
            );
            let exclusion = GapExclusion::one_series(&cuts, &signal);
            assert!(
                exclusion.day_names()[0].contains("(halt)"),
                "{:?}",
                exclusion.day_names()
            );
        }
    }
    // A day with no recorded halt and no hole is not cut.
    let quiet = day_of(2020, 3, 12);
    let (minutes, signal) = series(&[(quiet, Vec::new(), None)], 15);
    assert!(
        days_with_minute_holes(&signal, &minutes, 15 * MINUTE, |day| {
            crate::stored::nse_session_close_minute(day)
        })
        .is_empty()
    );
}

/// The causal rule has its own policy number, and the identity binds each
/// day's cut: two runs that cut one day at different instants, or a run that
/// withheld nothing and one that cut, never share a data term; `sweep-stored`'s
/// params bind version 4, not version 3.
#[test]
fn the_causal_rule_has_its_own_policy_version() {
    assert_eq!(MINUTE_GAP_POLICY, 4);
    let day = day_of(2024, 4, 23);
    let early = Cut::new(day, stamp(day, hhmm(10, 51)), CutCause::Hole, 15 * MINUTE);
    let later = Cut::new(day, stamp(day, hhmm(14, 2)), CutCause::Hole, 15 * MINUTE);
    let halt = Cut::new(day, stamp(day, hhmm(10, 51)), CutCause::Halt, 15 * MINUTE);
    let digest = [9_u8; 32];
    assert_ne!(
        bind_withheld(digest, &[early]),
        bind_withheld(digest, &[later])
    );
    assert_ne!(
        bind_withheld(digest, &[early]),
        bind_withheld(digest, &[halt])
    );
    assert_ne!(bind_withheld(digest, &[early]), digest);
    assert_eq!(bind_withheld(digest, &[]), digest);
    let ladder = engine::Ladder::with_min_hits(5);
    let v4 = crate::stored_month_params(ladder, Some(&GapExclusion::none()));
    let v3 = runner::identity::Params::of(ladder).with_policy(&[3]);
    assert_ne!(v4, v3);
    assert_eq!(v4, runner::identity::Params::of(ladder).with_policy(&[4]));
}

/// The admission rule is one O(1) probe per bar however many days are cut, and
/// the indicator fold's `(day, from)` pairs say the same thing.
#[test]
fn the_admission_probe_and_the_fold_pairs_agree() {
    let day = day_of(2024, 4, 23);
    let cut = Cut::new(day, stamp(day, hhmm(10, 51)), CutCause::Hole, 15 * MINUTE);
    let admission = Admission::of(&[cut]);
    assert!(admission.admits(stamp(day, hhmm(10, 30))));
    assert!(!admission.admits(stamp(day, hhmm(10, 45))));
    assert!(!admission.admits(stamp(day, hhmm(15, 15))));
    assert!(admission.admits(stamp(day + 1, hhmm(10, 45))));
    assert!(Admission::of(&[]).admits(stamp(day, hhmm(10, 45))));
    assert_eq!(Admission::pairs(&[cut]), vec![(day, cut.from_micros)]);
    assert!(cut.withholds(stamp(day, hhmm(10, 45))));
    assert!(!cut.withholds(stamp(day, hhmm(10, 30))));
    assert_eq!(cut.clock(), "10:51");
}

/// A HOLE AT THE OPEN WITHHOLDS THE WHOLE DAY, on every rung (AH-01 repair
/// round 1). The interior walk only compared two bars of one day, so a day
/// whose 09:15 minute -- or 09:15..09:20 -- the feed lost had no cut at all:
/// its first bar's step came from the previous night, and every coarse
/// bucket's closing minute was held. Policy 4 then admitted every row after
/// the hole. The calendar's first session minute is now the hole.
#[test]
fn a_hole_at_the_open_withholds_the_whole_day() {
    let quiet = day_of(2024, 5, 2);
    let holed = day_of(2024, 5, 3);
    for missing in [hhmm(9, 15)..=hhmm(9, 15), hhmm(9, 15)..=hhmm(9, 20)] {
        let days = vec![
            (quiet, Vec::new(), None),
            (holed, vec![missing.clone()], None),
        ];
        for length in [1_i64, 2, 5, 15] {
            let (minutes, signal) = series(&days, length);
            let cuts = days_with_minute_holes(&signal, &minutes, length * MINUTE, |day| {
                crate::stored::nse_session_close_minute(day)
            });
            assert_eq!(days_of(&cuts), vec![holed], "{length}min {missing:?}");
            assert_eq!(
                cuts[0].at_micros,
                stamp(holed, hhmm(9, 15)),
                "cut AT the open"
            );
            assert_eq!(cuts[0].cause, CutCause::Hole);
            let (kept, removed) = withhold(&signal, &cuts);
            assert!(
                kept.iter()
                    .all(|bar| indicators::ist_day(bar.ts_micros) == quiet),
                "{length}min {missing:?}: no row of the holed day is admitted"
            );
            let on_holed = signal
                .iter()
                .filter(|bar| indicators::ist_day(bar.ts_micros) == holed)
                .count();
            assert!(on_holed > 0, "premise: the holed day has rows");
            assert_eq!(removed, u64::try_from(on_holed).expect("count"));
            assert_eq!(
                kept.len(),
                signal.len() - on_holed,
                "the quiet day is whole"
            );
            assert_eq!(
                GapExclusion::signal_only(&cuts, &signal).day_names(),
                vec![format!(
                    "2024-05-03 from 09:15 (minute hole): 0 admitted before, {on_holed} withheld"
                )]
            );
        }
    }
    // The interior gap list names the day too.
    let (minutes, _) = series(&[(holed, vec![hhmm(9, 15)..=hhmm(9, 15)], None)], 1);
    assert_eq!(days_with_interior_gaps(&minutes), vec![holed]);
}

/// A TWO-WINDOW SESSION IS NOT HOLED BETWEEN ITS WINDOWS. 2024-03-02 traded
/// 09:15-09:59 and 11:30-12:29 (the calendar's own record); the old walk
/// called 10:00 a hole and withheld the second window. A hole INSIDE the
/// second window still cuts there, and a minute outside every window (a
/// stray 09:08) is no hole.
#[test]
fn a_two_window_session_is_not_holed_between_its_windows() {
    let day = day_of(2024, 3, 2);
    let kind = pull::calendar::kind_of(day);
    assert!(
        matches!(kind, pull::calendar::DayKind::Open(_)),
        "premise: 2024-03-02 is a session"
    );
    let pull::calendar::DayKind::Open(session) = kind else {
        return;
    };
    assert_eq!(session.count, 2, "premise: two windows");
    let minutes = served(day, &[], None);
    assert_eq!(minutes.len(), 105, "premise: 45 + 60 minutes");
    assert!(days_with_minute_holes(&minutes, &minutes, MINUTE, |_| None).is_empty());
    // The vendor's 15min buckets of each window.
    let signal: Vec<Candle> = [hhmm(9, 15), hhmm(9, 30), hhmm(9, 45)]
        .into_iter()
        .chain((0..4).map(|k| hhmm(11, 30) + 15 * k))
        .map(|open| candle(stamp(day, open)))
        .collect();
    assert!(
        days_with_minute_holes(&signal, &minutes, 15 * MINUTE, |day| {
            crate::stored::nse_session_close_minute(day)
        })
        .is_empty()
    );
    // A hole at 11:40 cuts at 11:40: 45 + 10 one-minute rows before it.
    let holed = served(day, &[hhmm(11, 40)..=hhmm(11, 40)], None);
    let cuts = days_with_minute_holes(&holed, &holed, MINUTE, |_| None);
    assert_eq!(cuts.len(), 1);
    assert_eq!(cuts[0].at_micros, stamp(day, hhmm(11, 40)));
    assert_eq!(withhold(&holed, &cuts).0.len(), 55);
    // A stray pre-open minute is outside every window.
    let mut stray = vec![candle(stamp(day, hhmm(9, 8)))];
    stray.extend(minutes.iter().copied());
    assert!(days_with_interior_gaps(&stray).is_empty());
}

/// A day the calendar cannot describe (here a Sunday) has no known first
/// minute, so it gets no leading cut, and its interior rule is the old one:
/// any same-day step longer than a minute is a hole at the minute after.
#[test]
fn a_day_the_calendar_cannot_describe_keeps_the_calendar_free_rule() {
    let sunday = day_of(2024, 4, 21);
    assert_eq!(
        pull::calendar::kind_of(sunday),
        pull::calendar::DayKind::Closed,
        "premise"
    );
    let whole: Vec<Candle> = [hhmm(10, 0), hhmm(10, 1)]
        .into_iter()
        .map(|minute| candle(stamp(sunday, minute)))
        .collect();
    assert!(days_with_interior_gaps(&whole).is_empty());
    let mut gapped = whole.clone();
    gapped.push(candle(stamp(sunday, hhmm(10, 5))));
    let cuts = days_with_minute_holes(&gapped, &gapped, MINUTE, |_| None);
    assert_eq!(days_of(&cuts), vec![sunday]);
    assert_eq!(cuts[0].at_micros, stamp(sunday, hhmm(10, 2)));
}

/// A HOLE AT A HALT'S START IS NAMED THE HALT. A feed that lost its minutes
/// from 2020-03-13's first whole halted minute ties the interior hole with
/// the halt; the tie named it a minute hole, sending the operator to the
/// vendor for an exchange event.
#[test]
fn a_hole_at_a_halt_start_is_named_the_halt() {
    let day = day_of(2020, 3, 13);
    let halt = pull::calendar::halt_on(day).expect("a recorded halt");
    let from = i64::from(halt.halted.from);
    let minutes = served(day, &[from..=from + 30], None);
    for length in [1_i64, 15] {
        let signal = if length == 1 {
            minutes.clone()
        } else {
            buckets(day, length)
        };
        let cuts = days_with_minute_holes(&signal, &minutes, length * MINUTE, |day| {
            crate::stored::nse_session_close_minute(day)
        });
        assert_eq!(cuts.len(), 1, "{length}min");
        assert_eq!(cuts[0].at_micros, stamp(day, from));
        assert_eq!(cuts[0].cause, CutCause::Halt, "{length}min");
    }
}

/// A HOLE AT THE OPEN OUTSIDE THE CALENDAR'S RANGE withholds the whole day
/// too. `pull::calendar::kind_of` answers `Unmeasured` before 2019-12-02 and
/// after 2026-09-04, and Zerodha's `1min` reaches 2018-01 and 2026-09-30; a
/// traded day there is held to the regular session, so a missing 09:15 is
/// still a hole at 09:15 on every rung, and a whole regular session is not.
#[test]
fn a_hole_at_the_open_outside_the_calendar_withholds_the_whole_day() {
    let regular: Vec<i64> = (hhmm(9, 15)..=hhmm(15, 29)).collect();
    let close = |_day: i64| u16::try_from(hhmm(15, 29)).ok();
    for holed in [day_of(2019, 6, 4), day_of(2026, 9, 15)] {
        let quiet = holed - 1;
        for day in [quiet, holed] {
            assert_eq!(
                pull::calendar::kind_of(day),
                pull::calendar::DayKind::Unmeasured,
                "premise: outside the calendar's range"
            );
        }
        for missing in [hhmm(9, 15)..=hhmm(9, 15), hhmm(9, 15)..=hhmm(9, 20)] {
            let mut minutes: Vec<Candle> = regular
                .iter()
                .map(|minute| candle(stamp(quiet, *minute)))
                .collect();
            minutes.extend(
                regular
                    .iter()
                    .filter(|minute| !missing.contains(minute))
                    .map(|minute| candle(stamp(holed, *minute))),
            );
            for length in [1_i64, 2, 5, 15] {
                let signal: Vec<Candle> = if length == 1 {
                    minutes.clone()
                } else {
                    let step = usize::try_from(length).expect("a rung");
                    [quiet, holed]
                        .into_iter()
                        .flat_map(|day| {
                            regular
                                .iter()
                                .step_by(step)
                                .map(move |minute| candle(stamp(day, *minute)))
                        })
                        .collect()
                };
                let quiet_rows = signal
                    .iter()
                    .filter(|bar| indicators::ist_day(bar.ts_micros) == quiet)
                    .count();
                let cuts = days_with_minute_holes(&signal, &minutes, length * MINUTE, close);
                assert_eq!(days_of(&cuts), vec![holed], "{length}min {missing:?}");
                assert_eq!(cuts[0].at_micros, stamp(holed, hhmm(9, 15)), "cut AT 09:15");
                assert_eq!(cuts[0].cause, CutCause::Hole);
                let (kept, removed) = withhold(&signal, &cuts);
                assert_eq!(
                    kept.len(),
                    quiet_rows,
                    "{length}min: only the whole day is kept"
                );
                assert!(
                    kept.iter()
                        .all(|bar| indicators::ist_day(bar.ts_micros) == quiet)
                );
                assert_eq!(
                    removed,
                    u64::try_from(signal.len() - quiet_rows).expect("count"),
                    "{length}min: every row of the holed day is withheld"
                );
            }
        }
    }
}

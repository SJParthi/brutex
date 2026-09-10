//! Generated daily return observations only; these are not market performance.
#![expect(
    clippy::unwrap_used,
    reason = "named invariant assertions over finite fixtures"
)]

use super::*;
use brutex_core::instrument::{Exchange, InstrumentKey};

fn day(year: u16, month: u8, day: u8) -> i64 {
    i64::from(Day::new(year, month, day).unwrap().days_from_epoch())
}
fn family(symbol: &str) -> ResearchFamilyV1 {
    ResearchFamilyV1::new(InstrumentKey::index(Exchange::Nse, symbol).unwrap()).unwrap()
}
fn span(first: i64, last: i64) -> Vec<Session> {
    (first..=last)
        .filter(|&day| eligibility_of(day) == Eligibility::Eligible)
        .map(|day| Session {
            day,
            pessimistic_paisa: 100,
            trades: 1,
        })
        .collect()
}
fn run(first: i64, last: i64, rows: &[Session]) -> Evaluation {
    evaluate(Policy::V1, family("NIFTY"), first, last, rows, true)
}
fn round_trip(result: &Evaluation) {
    let raw = result.canonical_bytes();
    let decoded = Evaluation::decode(&raw, Some(&result.weeks)).unwrap();
    assert_eq!(&decoded, result);
    assert_eq!(decoded.digest(), result.digest());
    assert_eq!(
        Evaluation::decode(&raw, None).unwrap().canonical_bytes(),
        raw
    );
    for week in &result.weeks {
        assert_eq!(Week::decode(&week.canonical_bytes()).unwrap(), *week);
    }
    assert_eq!(
        Summary::decode(&result.summary.canonical_bytes()).unwrap(),
        result.summary
    );
}

#[test]
fn every_five_day_win_loss_flat_no_trade_sequence_obeys_the_declared_policy() {
    let first = day(2025, 1, 6);
    let last = day(2025, 1, 10);
    assert_eq!(span(first, last).len(), 5);
    for code in 0..4_usize.pow(5) {
        let mut choice = code;
        let mut wins = 0;
        let mut losses = 0;
        let mut flats = 0;
        let mut no_trades = 0;
        let mut streak = 0;
        let mut maximum = 0;
        let mut rows = span(first, last);
        for row in &mut rows {
            match choice % 4 {
                0 => {
                    wins += 1;
                    streak = 0;
                    row.trades = 7;
                }
                1 => {
                    losses += 1;
                    streak += 1;
                    maximum = maximum.max(streak);
                    row.pessimistic_paisa = -100;
                    row.trades = 4;
                }
                2 => {
                    flats += 1;
                    row.pessimistic_paisa = 0;
                    row.trades = 3;
                }
                _ => {
                    no_trades += 1;
                    row.pessimistic_paisa = 0;
                    row.trades = 0;
                }
            }
            choice /= 4;
        }
        let result = run(first, last, &rows);
        let expected = if wins >= 3 && losses <= 2 && maximum <= 2 {
            Outcome::Passed
        } else {
            Outcome::Failed
        };
        assert_eq!(result.outcome, expected, "sequence {code}");
        assert_eq!(result.summary.eligible_days, 5);
        assert_eq!(result.summary.observed_days, 5);
        assert_eq!(result.summary.winning_days, wins);
        assert_eq!(result.summary.losing_days, losses);
        assert_eq!(result.summary.zero_days, flats);
        assert_eq!(result.summary.no_trade_days, no_trades);
        assert_eq!(result.summary.longest_losing_streak, maximum);
        assert_eq!(result.summary.complete_weeks, 1);
        assert_eq!(
            result.summary.pessimistic_paisa,
            (i64::try_from(wins).unwrap() - i64::try_from(losses).unwrap()) * 100
        );
        assert_eq!(
            result.reasons & Reason::WinningDayRatio.mask() != 0,
            wins < 3
        );
        assert_eq!(
            result.reasons & Reason::WeeklyLosses.mask() != 0,
            losses > 2
        );
        round_trip(&result);
    }
}

#[test]
fn every_missing_position_refuses_instead_of_manufacturing_a_zero_day() {
    let first = day(2025, 1, 6);
    let last = day(2025, 1, 10);
    for mask in 1..32 {
        let rows: Vec<_> = span(first, last)
            .into_iter()
            .enumerate()
            .filter_map(|(index, row)| (mask & (1 << index) == 0).then_some(row))
            .collect();
        let result = run(first, last, &rows);
        assert_eq!(result.outcome, Outcome::Refused);
        assert_ne!(result.reasons & Reason::MissingSession.mask(), 0);
        assert_eq!(result.summary.missing_days, 5 - rows.len() as u64);
        assert_eq!(result.summary.observed_days, rows.len() as u64);
        assert_eq!(result.summary.no_trade_days, 0);
        assert_eq!(result.summary.zero_days, 0);
        assert_eq!(result.summary.passing_weeks, 0);
        assert_eq!(result.weeks.first().unwrap().outcome, Outcome::Refused);
        round_trip(&result);
    }
}

#[test]
fn losses_cross_week_boundaries_even_when_both_individual_weeks_pass() {
    let first = day(2025, 1, 6);
    let last = day(2025, 1, 17);
    let mut rows = span(first, last);
    assert_eq!(rows.len(), 10);
    for index in [3, 4, 5] {
        rows.get_mut(index).unwrap().pessimistic_paisa = -100;
    }
    let result = run(first, last, &rows);
    assert_eq!(result.summary.passing_weeks, 2);
    assert_eq!(result.summary.winning_days, 7);
    assert_eq!(result.summary.longest_losing_streak, 3);
    assert_eq!(result.outcome, Outcome::Failed);
    assert_eq!(result.reasons, Reason::LosingDayStreak.mask());
    assert_eq!(result.first_issue_day, Some(day(2025, 1, 13)));
    round_trip(&result);
}

#[test]
fn flat_and_no_trade_days_preserve_a_loss_streak_and_only_a_win_resets_it() {
    let first = day(2025, 1, 6);
    let last = day(2025, 1, 17);
    let mut rows = span(first, last);
    for index in [3, 4, 7] {
        rows.get_mut(index).unwrap().pessimistic_paisa = -100;
    }
    rows.get_mut(5).unwrap().pessimistic_paisa = 0;
    rows.get_mut(5).unwrap().trades = 0;
    rows.get_mut(6).unwrap().pessimistic_paisa = 0;
    rows.get_mut(6).unwrap().trades = 2;
    let result = run(first, last, &rows);
    assert_eq!(result.summary.longest_losing_streak, 3);
    assert_eq!(result.summary.no_trade_days, 1);
    assert_eq!(result.summary.zero_days, 1);
    assert_eq!(result.first_issue_day, Some(day(2025, 1, 15)));
    rows.get_mut(6).unwrap().pessimistic_paisa = 1;
    let reset = run(first, last, &rows);
    assert_eq!(reset.summary.longest_losing_streak, 2);
    assert_eq!(reset.reasons & Reason::LosingDayStreak.mask(), 0);
    assert_eq!(reset.outcome, Outcome::Passed);
    round_trip(&result);
    round_trip(&reset);
}

#[test]
fn long_loss_runs_are_counted_exactly_across_holidays_and_year_boundaries() {
    let first = day(2024, 12, 23);
    let last = day(2025, 1, 17);
    let mut rows = span(first, last);
    for row in &mut rows {
        row.pessimistic_paisa = -1;
    }
    let result = run(first, last, &rows);
    assert_eq!(result.summary.longest_losing_streak, rows.len() as u64);
    assert_eq!(result.summary.losing_days, rows.len() as u64);
    assert!(result.summary.short_weeks > 0);
    assert_eq!(result.outcome, Outcome::Failed);
    round_trip(&result);
    let first = day(2024, 12, 30);
    let last = day(2025, 1, 10);
    let result = run(first, last, &span(first, last));
    assert_eq!(result.outcome, Outcome::Passed);
    assert_eq!(
        result
            .weeks
            .iter()
            .map(|week| week.monday)
            .collect::<Vec<_>>(),
        [day(2024, 12, 30), day(2025, 1, 6)]
    );
    let leap_first = day(2024, 2, 26);
    let leap_last = day(2024, 3, 1);
    let leap = run(leap_first, leap_last, &span(leap_first, leap_last));
    assert_eq!(leap.summary.observed_days, 5);
    assert_eq!(leap.outcome, Outcome::Passed);
}

#[test]
fn short_and_partial_weeks_never_pass_the_weekly_test_vacuously() {
    for (first, last, short, partial) in [
        (day(2025, 3, 10), day(2025, 3, 14), 1, 0),
        (day(2025, 1, 6), day(2025, 1, 8), 0, 1),
        (day(2025, 1, 7), day(2025, 1, 10), 0, 1),
        (day(2025, 1, 11), day(2025, 1, 12), 0, 1),
    ] {
        let rows = span(first, last);
        let result = run(first, last, &rows);
        assert_eq!(result.outcome, Outcome::Unmeasured);
        assert_ne!(result.reasons & Reason::NoCompleteWeek.mask(), 0);
        assert_eq!(result.summary.short_weeks, short);
        assert_eq!(result.summary.partial_weeks, partial);
        assert_eq!(result.summary.complete_weeks, 0);
        assert_eq!(result.summary.passing_weeks, 0);
        assert!(
            result
                .weeks
                .iter()
                .all(|week| week.outcome == Outcome::NotApplicable)
        );
        round_trip(&result);
    }
}

#[test]
fn existing_excluded_sessions_and_real_weekend_sessions_remain_distinct() {
    let first = day(2024, 2, 26);
    let last = day(2024, 3, 3);
    assert_eq!(eligibility_of(day(2024, 3, 2)), Eligibility::Excluded);
    let result = run(first, last, &span(first, last));
    assert_eq!(result.outcome, Outcome::Passed);
    assert_eq!(result.summary.excluded_days, 1);
    assert_eq!(result.summary.missing_days, 0);
    assert_eq!(result.summary.weekend_sessions, 0);
    round_trip(&result);
    let first = day(2025, 1, 27);
    let last = day(2025, 2, 7);
    let saturday = day(2025, 2, 1);
    assert_eq!(eligibility_of(saturday), Eligibility::Eligible);
    let mut rows = span(first, last);
    for row in &mut rows {
        if [saturday, day(2025, 2, 3), day(2025, 2, 4)].contains(&row.day) {
            row.pessimistic_paisa = -1;
        }
    }
    let result = run(first, last, &rows);
    assert_eq!(result.summary.weekend_sessions, 1);
    assert_eq!(result.summary.eligible_days, 11);
    assert_eq!(result.summary.passing_weeks, 2);
    assert_eq!(result.summary.longest_losing_streak, 3);
    assert_eq!(result.reasons, Reason::LosingDayStreak.mask());
    round_trip(&result);
}

#[test]
fn missing_calendar_authority_stays_unmeasured_even_when_rows_are_offered() {
    let first = pull::calendar::LAST_DAY + 1;
    let last = first + 6;
    let rows = [Session {
        day: first,
        pessimistic_paisa: 100,
        trades: 1,
    }];
    let result = run(first, last, &rows);
    assert_eq!(result.outcome, Outcome::Unmeasured);
    assert_eq!(result.summary.unmeasured_days, 7);
    assert_eq!(result.summary.eligible_days, 0);
    assert_eq!(result.summary.no_trade_days, 0);
    assert_eq!(result.summary.observed_days, 0);
    assert_eq!(result.sessions_digest, session_digest(&rows));
    assert_ne!(result.reasons & Reason::UnmeasuredCalendar.mask(), 0);
    round_trip(&result);
}

#[test]
fn malformed_order_counts_returns_spans_and_unexpected_sessions_refuse() {
    let first = day(2025, 1, 6);
    let last = day(2025, 1, 12);
    let valid = span(first, last);
    let mut invalid_sets = Vec::new();
    let mut duplicate = valid.clone();
    duplicate.insert(1, *duplicate.first().unwrap());
    invalid_sets.push(duplicate);
    let mut reversed = valid.clone();
    reversed.reverse();
    invalid_sets.push(reversed);
    let mut outside = valid.clone();
    outside.first_mut().unwrap().day = i64::MIN;
    invalid_sets.push(outside);
    let mut empty_trade = valid.clone();
    empty_trade.first_mut().unwrap().trades = 0;
    invalid_sets.push(empty_trade);
    let mut closed = valid.clone();
    closed.push(Session {
        day: last,
        pessimistic_paisa: 0,
        trades: 0,
    });
    invalid_sets.push(closed);
    let excessive = vec![*valid.first().unwrap(); 8];
    invalid_sets.push(excessive);
    for rows in invalid_sets {
        let result = run(first, last, &rows);
        assert_eq!(result.outcome, Outcome::Refused);
        assert_eq!(result.summary, Summary::default());
        assert_eq!(result.sessions_digest, session_digest(&rows));
        round_trip(&result);
    }
    for (first, last) in [(last, first), (-1, 0), (0, i64::MAX)] {
        let result = run(first, last, &[]);
        assert_eq!(result.outcome, Outcome::Refused);
        assert_eq!(result.reasons, Reason::InvalidSpan.mask());
        round_trip(&result);
    }
    let excluded = day(2024, 3, 2);
    let result = run(
        excluded,
        excluded,
        &[Session {
            day: excluded,
            pessimistic_paisa: 0,
            trades: 0,
        }],
    );
    assert_eq!(result.outcome, Outcome::Refused);
    assert_eq!(result.reasons, Reason::UnexpectedSession.mask());
}

#[test]
fn count_and_both_paisa_overflow_directions_refuse_before_partial_day_updates() {
    let first = day(2025, 1, 6);
    for (left, right, trades) in [(i64::MAX, 1, 1), (i64::MIN, -1, 1), (1, 1, u64::MAX)] {
        let rows = [
            Session {
                day: first,
                pessimistic_paisa: left,
                trades,
            },
            Session {
                day: first + 1,
                pessimistic_paisa: right,
                trades: 1,
            },
        ];
        let result = run(first, first + 1, &rows);
        assert_eq!(result.outcome, Outcome::Refused);
        assert_ne!(result.reasons & Reason::ArithmeticOverflow.mask(), 0);
        assert_eq!(result.summary.observed_days, 1);
        assert_eq!(result.summary.calendar_days, 1);
        assert_eq!(result.first_issue_day, Some(first + 1));
        round_trip(&result);
    }
    // A previous week's negative return can keep the total representable while
    // this week's independent subtotal overflows. Neither may wrap.
    let last = day(2025, 1, 17);
    let mut rows = span(first, last);
    for row in &mut rows {
        row.pessimistic_paisa = 0;
    }
    rows.first_mut().unwrap().pessimistic_paisa = -2;
    rows.get_mut(5).unwrap().pessimistic_paisa = i64::MAX;
    rows.get_mut(6).unwrap().pessimistic_paisa = 1;
    let result = run(first, last, &rows);
    assert_eq!(result.outcome, Outcome::Refused);
    assert_ne!(result.reasons & Reason::ArithmeticOverflow.mask(), 0);
    assert_ne!(result.reasons & Reason::WeeklyWins.mask(), 0);
    // The failed first week's Monday remains the first reported issue even
    // after a later overflow changes the overall outcome to Refused.
    assert_eq!(result.first_issue_day, Some(first));
    assert_eq!(result.summary.pessimistic_paisa, i64::MAX - 2);
    round_trip(&result);

    // Keep the first week's subtotal at -2 but make its four winning days
    // satisfy the weekly rule, isolating the next week's subtotal overflow.
    for (row, paisa) in rows.iter_mut().zip([-6, 1, 1, 1, 1]) {
        row.pessimistic_paisa = paisa;
    }
    let result = run(first, last, &rows);
    assert_eq!(result.outcome, Outcome::Refused);
    assert_eq!(result.reasons, Reason::ArithmeticOverflow.mask());
    assert_eq!(result.first_issue_day, Some(day(2025, 1, 14)));
    assert_eq!(result.summary.pessimistic_paisa, i64::MAX - 2);
    assert_eq!(result.summary.passing_weeks, 1);
    round_trip(&result);
}

#[test]
fn policy_is_index_only_and_week_retention_does_not_change_identity() {
    let first = day(2025, 1, 6);
    let last = day(2025, 1, 17);
    let rows = span(first, last);
    for symbol in ["NIFTY", "BANKNIFTY"] {
        let retained = evaluate(Policy::V1, family(symbol), first, last, &rows, true);
        let summary_only = evaluate(Policy::V1, family(symbol), first, last, &rows, false);
        assert_eq!(retained.outcome, Outcome::Passed);
        assert_eq!(retained.canonical_bytes(), summary_only.canonical_bytes());
        assert_eq!(retained.digest(), summary_only.digest());
        assert!(summary_only.weeks.is_empty());
        assert_eq!(retained.weeks.len(), 2);
        assert_eq!(retained.sessions_digest, session_digest(&rows));
        round_trip(&retained);
    }
    let cash =
        ResearchFamilyV1::new(InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap()).unwrap();
    let result = evaluate(Policy::V1, cash, first, last, &rows, true);
    assert_eq!(result.outcome, Outcome::NotApplicable);
    assert_eq!(result.summary, Summary::default());
    assert_eq!(result.reasons, 0);
    round_trip(&result);
}

#[test]
fn exact_versioned_codecs_refuse_truncation_changed_rules_and_forged_week_claims() {
    let policy = Policy::V1.canonical_bytes();
    assert_eq!(Policy::decode(&policy).unwrap(), Policy::V1);
    for index in 0..policy.len() {
        let mut changed = policy;
        *changed.get_mut(index).unwrap() ^= 1;
        assert!(Policy::decode(&changed).is_err());
    }
    let first = day(2025, 1, 6);
    let last = day(2025, 1, 17);
    let result = run(first, last, &span(first, last));
    let bytes = result.canonical_bytes();
    for end in 0..bytes.len() {
        assert!(Evaluation::decode(bytes.get(..end).unwrap(), None).is_err());
    }
    let mut reversed = result.weeks.clone();
    reversed.reverse();
    assert!(Evaluation::decode(&bytes, Some(&reversed)).is_err());
    let mut altered = result.weeks.clone();
    altered.first_mut().unwrap().pessimistic_paisa += 1;
    assert!(Evaluation::decode(&bytes, Some(&altered)).is_err());
    let mut missing = result.weeks.clone();
    missing.pop();
    assert!(Evaluation::decode(&bytes, Some(&missing)).is_err());
    let partial = run(first, first + 2, &span(first, first + 2));
    let mut forged = partial.clone();
    forged.outcome = Outcome::Passed;
    forged.reasons = 0;
    assert!(Evaluation::decode(&forged.canonical_bytes(), None).is_err());
    let mut week = *partial.weeks.first().unwrap();
    week.kind = WeekKind::Complete;
    week.outcome = Outcome::Passed;
    assert!(Week::decode(&week.canonical_bytes()).is_err());
    let invalid_zero = Session {
        day: first,
        pessimistic_paisa: 1,
        trades: 0,
    };
    assert!(Session::decode(&invalid_zero.canonical_bytes()).is_err());
    for outcome in [
        Outcome::Passed,
        Outcome::Failed,
        Outcome::Unmeasured,
        Outcome::Refused,
        Outcome::NotApplicable,
    ] {
        assert!(!outcome.as_str().is_empty());
    }
}

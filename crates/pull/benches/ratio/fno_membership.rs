//! C-PIT-01 (PIT-1, D-5164): the point-in-time F&O membership's lookups,
//! flat at 1x, 10x and 100x the held days. `members`, `pool_size` and
//! `is_member` on a known day, and `members` on a sealed refused session
//! inside the span and on a day before it, are each one subtraction into a
//! fixed-stride day index and one slice, row or bit; none walks the days or
//! the months. The harness, `crates/pull/benches/ratio.rs`, fails on a ratio
//! above its ceiling and prints p50, p99 and max beside each.
//!
//! Every input is invented here and nothing is written: the months are folded
//! in memory (`State::fold`, `MonthTable`) and served by `Membership::of`. The
//! share names (`ZZQ..`) are not NSE symbols and the strike is no price.

use std::hint::black_box;

use pull::fno_membership::{DayInput, DayStatus, Membership, MonthTable, State, evidence};
use pull::gdfl_nfo::DAY_FORM_FROM_DAYS;
use pull::session::Day;

use super::{cost_ps, ratio, refuse, spread};

/// Shares listed on every known day.
const SHARES: u32 = 40;
/// Calls per timed repetition.
const REPS: u32 = 20_000;
/// The IST calendar's months the folds run over start here (the decoder
/// settles an expiry against that calendar, so the days stay inside it).
const FIRST: (u16, u8) = (2018, 1);
/// Month abbreviations as the vendor's tickers spell them.
const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// The `i`th invented share: `ZZQ` and two letters.
fn share(i: u32) -> String {
    let letter = |n: u32| char::from(b'A' + u8::try_from(n % 26).unwrap_or(0));
    format!("ZZQ{}{}", letter(i / 26), letter(i))
}

/// `(year, month)` plus `n` months.
fn plus(year: u16, month: u8, n: u32) -> (u16, u8) {
    let at = u32::from(year) * 12 + u32::from(month - 1) + n;
    (
        u16::try_from(at / 12).unwrap_or(year),
        u8::try_from(at % 12 + 1).unwrap_or(month),
    )
}

/// Every share's one contract on `day`, the next month's, 500 CE, spelled in
/// the form the decoder reads on that trade day (`pull::gdfl_nfo`, FORMAT.md
/// §6): before `DAY_FORM_FROM_DAYS` (2019-02-01) a share option is read only
/// in the monthly form `YYMON` (the month's last Thursday session), from it
/// only in the day form `DDMONYY`, here the 28th or the weekday before it.
/// A day-form name before the switch reads as no contract at all, which made
/// every 2018 day of the first draft of this bench a day with no share option.
fn tickers(day: Day) -> Vec<String> {
    let (year, month) = plus(day.year(), day.month(), 1);
    let name = MONTHS
        .get(usize::from(month - 1))
        .unwrap_or_else(|| refuse("a month name"));
    let stem = if day.days_from_epoch() < DAY_FORM_FROM_DAYS {
        format!("{:02}{name}500CE", year % 100)
    } else {
        let the_28th = Day::new(year, month, 28).unwrap_or_else(|_| refuse("an expiry day"));
        // Epoch day 0 was a Thursday: `(epoch + 3) % 7` is 0 on a Monday, 5 on
        // a Saturday and 6 on a Sunday, which step back one and two days.
        let weekday = (the_28th.days_from_epoch() + 3) % 7;
        let dd = 28 - u8::try_from(weekday.saturating_sub(4)).unwrap_or(0);
        format!("{dd:02}{name}{:02}500CE", year % 100)
    };
    (0..SHARES).map(|i| format!("{}{stem}", share(i))).collect()
}

/// `months` months folded from [`FIRST`], the 15th of each a session with no
/// day file (a sealed refused record), served without files.
fn membership(months: u32) -> Membership {
    let (year, month) = FIRST;
    let mut day = Day::new(year, month, 1).unwrap_or_else(|_| refuse("a first day"));
    let (last_year, last_month) = plus(year, month, months - 1);
    let last = Day::new(last_year, last_month, 1)
        .unwrap_or_else(|_| refuse("the final month"))
        .end_of_month();
    let mut state = State::default();
    let mut tables: Vec<MonthTable> = Vec::new();
    let mut open: Option<MonthTable> = None;
    while day <= last {
        let listed = day.day() != 15;
        let input = if listed {
            let names = tickers(day);
            DayInput::Listed(evidence(day, names.iter().map(String::as_str)))
        } else {
            DayInput::Unknown(DayStatus::NoSource, None)
        };
        if open
            .as_ref()
            .is_some_and(|table| table.month() != (day.year(), day.month()))
        {
            tables.extend(open.take());
        }
        let table = open.get_or_insert_with(|| MonthTable::begin(day, &state));
        let record = state
            .fold(day, input)
            .unwrap_or_else(|why| refuse(&format!("a fold: {why}")));
        // Every listed day must be a known record holding every share: a day
        // the decoder read no share option on is named here with its words,
        // never left to surface as a chain refusal with no day in it.
        if listed && (record.row.status != DayStatus::Known || u32::from(record.row.pool) != SHARES)
        {
            refuse(&format!(
                "{day} was listed and folded as {:?} with a pool of {}: {:?}",
                record.row.status, record.row.pool, record.refused_sample
            ));
        }
        table
            .push(record, &state)
            .unwrap_or_else(|why| refuse(&format!("a push: {why}")));
        day = day.succ().unwrap_or_else(|_| refuse("a next day"));
    }
    tables.extend(open);
    Membership::of(tables).unwrap_or_else(|why| refuse(&format!("the chain: {why}")))
}

/// One timed lookup: the membership, the day, a share name; a count the
/// optimiser cannot discard.
type Lookup = fn(&Membership, Day, &str) -> usize;

/// C-PIT-01: each lookup at 1x, 10x and 100x the held days (1, 10 and 100
/// months, 2018-01 on).
pub(crate) fn fno_membership_lookup_is_flat() -> bool {
    let sizes = [membership(1), membership(10), membership(100)];
    let known = Day::new(2018, 1, 10).unwrap_or_else(|_| refuse("a known day"));
    let refused = Day::new(2018, 1, 15).unwrap_or_else(|_| refuse("a refused day"));
    let before = Day::new(2017, 12, 31).unwrap_or_else(|_| refuse("a day before"));
    let probe = share(1);
    for held in &sizes {
        if held.pool_size(known) != Ok(u16::try_from(SHARES).unwrap_or(0))
            || held.members(refused).is_ok()
            || held.members(before).is_ok()
            || held.is_member(&probe, known) != Ok(true)
        {
            refuse("the invented membership does not answer as built");
        }
    }
    let mut ok = true;
    let lookups: [(&str, Day, Lookup); 4] = [
        ("members(day), known", known, |held, day, _| {
            held.members(day).map_or(0, <[_]>::len)
        }),
        ("is_member(name, day), known", known, |held, day, name| {
            usize::from(held.is_member(name, day) == Ok(true))
        }),
        (
            "members(day), refused inside the span",
            refused,
            |held, day, _| usize::from(held.members(day).is_err()),
        ),
        ("members(day), before the span", before, |held, day, _| {
            usize::from(held.members(day).is_err())
        }),
    ];
    for (label, day, lookup) in lookups {
        let cost = |held: &Membership| {
            cost_ps(REPS, || {
                lookup(black_box(held), black_box(day), black_box(&probe))
            })
        };
        let base = cost(&sizes[0]);
        ok &= ratio(&format!("C-PIT-01 {label}, 10x"), base, cost(&sizes[1]));
        ok &= ratio(&format!("C-PIT-01 {label}, 100x"), base, cost(&sizes[2]));
        spread(&format!("C-PIT-01 {label}, 100x"), REPS, || {
            black_box(lookup(black_box(&sizes[2]), black_box(day), &probe));
        });
    }
    ok
}

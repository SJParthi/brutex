#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): what `fno_membership` answers that no
//! test had read (Route B on `c9d653d5f`): a day's evidence by its day and
//! shares, a refusal sample at its bound, the fold's span and counts, a late
//! spelling's listing lag, a month's largest pool and a record offered out of
//! its month's order. Every ticker is invented (`ZZ...`).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

fn d(year: u16, month: u8, day: u8) -> Day {
    Day::new(year, month, day).unwrap()
}

/// Three contract keys of the March 2024 expiry under `share`.
fn keys(share: &str) -> Vec<String> {
    ["28MAR24500CE", "28MAR24520CE", "28MAR24480PE"]
        .iter()
        .map(|key| format!("{share}{key}"))
        .collect()
}

fn symbol(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

/// A day's evidence names its day and its shares in name order, says which
/// share it lists, counts every name it refused and keeps exactly the first
/// [`REFUSAL_SAMPLE`] of them (one past the bound), and an empty listing
/// holds nothing.
#[test]
fn a_days_evidence_names_its_shares_and_samples_its_refusals() {
    let on = d(2024, 3, 5);
    let mut tickers = keys("ZZBETA");
    tickers.extend(keys("ZZALPHA"));
    tickers.extend((0..=REFUSAL_SAMPLE).map(|i| format!("not a ticker {i}")));
    let held = evidence(on, tickers.iter().map(String::as_str));
    assert_eq!(held.day(), on);
    assert_eq!(
        held.shares().map(Symbol::as_str).collect::<Vec<_>>(),
        ["ZZALPHA", "ZZBETA"]
    );
    assert!(held.lists(&symbol("ZZALPHA")));
    assert!(!held.lists(&symbol("ZZGAMMA")));
    assert_eq!(held.names, 7 + u32::try_from(REFUSAL_SAMPLE).unwrap());
    assert_eq!(held.refused, u32::try_from(REFUSAL_SAMPLE).unwrap() + 1);
    assert_eq!(held.refused_sample.len(), REFUSAL_SAMPLE);
    assert!(
        held.refused_sample[0].starts_with("not a ticker 0: "),
        "{:?}",
        held.refused_sample
    );
    let empty = evidence(on, std::iter::empty());
    assert_eq!((empty.names, empty.shares().count()), (0, 0));
}

/// Two days folded and pushed into their month: the fold's span runs from
/// the first known day to the last, it holds two spellings in two series,
/// the share first listed a day after its expiry's series began is one day
/// late, the month's largest pool is two and it measured no handover; a
/// record of the next day pushed into another month's table, or into its own
/// month's table out of order, is refused by name (extremes at once: a month
/// begun on its first trading day and a table with no row yet).
#[test]
fn the_fold_says_its_span_and_counts_and_a_month_refuses_a_record_out_of_order() {
    let (monday, tuesday, wednesday) = (d(2024, 3, 4), d(2024, 3, 5), d(2024, 3, 6));
    let mut state = State::default();
    let mut month = MonthTable::begin(monday, &state);
    let alpha = keys("ZZALPHA");
    let mut both = keys("ZZALPHA");
    both.extend(keys("ZZBETA"));
    for (day, tickers) in [(monday, &alpha), (tuesday, &both)] {
        let held = evidence(day, tickers.iter().map(String::as_str));
        let record = state.fold(day, DayInput::Listed(held)).unwrap();
        month.push(record, &state).unwrap();
    }
    assert_eq!(
        (state.known_from(), state.last()),
        (Some(monday), Some(tuesday))
    );
    assert_eq!((state.spellings(), state.series()), (2, 2));
    let lags: Vec<(String, u32)> = (0..2)
        .map(|id| {
            let spelling = state.spelling(id).unwrap();
            (
                spelling.name.as_str().to_owned(),
                spelling.listing_lag_days(),
            )
        })
        .collect();
    assert_eq!(lags, [("ZZALPHA".to_owned(), 0), ("ZZBETA".to_owned(), 1)]);
    assert_eq!(month.max_pool(), 2);
    assert!(month.handovers().is_empty());
    let late = state
        .fold(
            wednesday,
            DayInput::Listed(evidence(wednesday, both.iter().map(String::as_str))),
        )
        .unwrap();
    let mut april = MonthTable::begin(d(2024, 4, 1), &state);
    let refused = april.push(late.clone(), &state).unwrap_err();
    let MembershipError::Corrupt { why, .. } = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(
        why,
        "2024-03-06: a record out of its month's order was offered"
    );
    let mut again = MonthTable::begin(monday, &state);
    assert!(again.push(late, &state).is_err(), "Wednesday is not Monday");
}

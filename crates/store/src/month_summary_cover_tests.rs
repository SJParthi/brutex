#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
//! BASE-COV-LIBS: the month summary's refusal words and sentences, February
//! in every kind of year, its day bounds, and `describes` against a header
//! that differs in each number it compares. Every price is invented.

use super::*;
use crate::format::OI_NULL;

const SYMBOL: u32 = 41;

fn may() -> YearMonth {
    YearMonth::new(2031, 5).unwrap()
}

fn bar(ts_micros: i64, price: i64) -> Bar {
    Bar {
        ts_micros,
        open: price,
        high: price + 10,
        low: price - 10,
        close: price + 1,
        volume: 2,
        open_interest: OI_NULL,
    }
}

/// 09:15 IST on day `day` of May 2031, plus `minute` minutes.
fn at(day: i64, minute: i64) -> i64 {
    ((first_day_of(may()) + day - 1) * 86_400 + 33_300 - 19_800 + minute * 60) * 1_000_000
}

fn summary_of(timeframe: u32, bars: &[Bar]) -> MonthSummary {
    let mut summary = MonthSummary::new(SYMBOL, timeframe, may());
    for bar in bars {
        summary.fold(bar);
    }
    summary
}

fn header_of(summary: &MonthSummary) -> Header {
    Header {
        generation: summary.generation,
        n_valid: summary.n_valid,
        first_ts_micros: summary.first_ts_micros,
        last_ts_micros: summary.last_ts_micros,
        ..Header::genesis(summary.symbol_id, summary.timeframe_secs, 0)
    }
}

#[test]
fn every_refusal_has_its_word_and_its_sentence() {
    let cases = [
        (
            Refused::BelowThreshold,
            "below_threshold",
            "a month under 1,024 bars keeps no summary",
        ),
        (
            Refused::Version(9),
            "version",
            "summary version 9 is not one this build reads",
        ),
        (Refused::Shape("a bad day"), "shape", "shape: a bad day"),
        (
            Refused::NotThisMonth,
            "not_this_month",
            "the summary names another symbol, rung or month",
        ),
        (
            Refused::Stale {
                summary: (1, 2),
                header: (3, 4),
            },
            "stale",
            "generation 1 with 2 bars; the month commits generation 3 with 4",
        ),
    ];
    for (refused, word, said) in cases {
        assert_eq!(refused.reason(), word);
        let text = refused.to_string();
        assert!(text.starts_with(&format!("{word}: ")), "{text}");
        assert!(text.contains(said), "{text}");
    }
}

#[test]
fn february_has_its_days_in_every_kind_of_year() {
    let feb = |year: u16| days_in(YearMonth::new(year, 2).unwrap());
    assert_eq!(feb(2023), 28);
    assert_eq!(feb(2024), 29);
    assert_eq!(feb(2000), 29);
    assert_eq!(feb(2100), 28);
    assert_eq!(days_in(YearMonth::new(2024, 4).unwrap()), 30);
    assert_eq!(days_in(YearMonth::new(2024, 12).unwrap()), 31);
}

#[test]
fn a_summary_spans_its_months_days_and_keeps_no_second_outside_them() {
    let summary = summary_of(1, &[bar(at(2, 0), 70_000), bar(at(2, 1), 70_010)]);
    assert_eq!(summary.first_day(), first_day_of(may()));
    assert_eq!(summary.last_day(), first_day_of(may()) + 30);
    assert_eq!(
        summary.seconds_on(summary.first_day() - 1),
        Some(Vec::new())
    );
    assert_eq!(summary.seconds_on(summary.last_day() + 1), Some(Vec::new()));
    assert_eq!(summary.seconds_on(i64::MIN), Some(Vec::new()));
    assert_eq!(summary.seconds_on(summary.first_day()), Some(Vec::new()));
    assert_eq!(
        summary.seconds_on(summary.first_day() + 1).map(|s| s.len()),
        Some(2)
    );
    let minute = summary_of(60, &[bar(at(2, 0), 70_000)]);
    assert_eq!(minute.seconds_on(minute.first_day() + 1), None);
}

#[test]
fn a_summary_describes_only_the_header_it_was_built_for() {
    let rows = [bar(at(3, 0), 70_000), bar(at(3, 1), 70_010)];
    let header = Header {
        generation: 4,
        n_valid: 2,
        first_ts_micros: rows[0].ts_micros,
        last_ts_micros: rows[1].ts_micros,
        ..Header::genesis(SYMBOL, 60, 0)
    };
    let summary = MonthSummary::build(&header, may(), &rows);
    assert_eq!(summary.describes(&header, may()), Ok(()));
    let rung = Header {
        timeframe_secs: 300,
        ..header
    };
    assert_eq!(summary.describes(&rung, may()), Err(Refused::NotThisMonth));
    let symbol = Header {
        symbol_id: SYMBOL + 1,
        ..header
    };
    assert_eq!(
        summary.describes(&symbol, may()),
        Err(Refused::NotThisMonth)
    );
    assert_eq!(
        summary.describes(&header, YearMonth::new(2031, 6).unwrap()),
        Err(Refused::NotThisMonth)
    );
    let stale = Err(Refused::Stale {
        summary: (summary.generation, summary.n_valid),
        header: (summary.generation, summary.n_valid),
    });
    let first = Header {
        first_ts_micros: header.first_ts_micros - 60_000_000,
        ..header
    };
    assert_eq!(summary.describes(&first, may()), stale);
    let last = Header {
        last_ts_micros: header.last_ts_micros + 60_000_000,
        ..header
    };
    assert_eq!(summary.describes(&last, may()), stale);
    // Two numbers off at once: still one refusal, naming both sides.
    let both = Header {
        generation: header.generation + 1,
        n_valid: header.n_valid + 1,
        ..header
    };
    assert_eq!(
        summary.describes(&both, may()),
        Err(Refused::Stale {
            summary: (summary.generation, summary.n_valid),
            header: (summary.generation + 1, summary.n_valid + 1),
        })
    );
    // An empty month: its stamps are not compared.
    let empty = summary_of(60, &[]);
    let nothing = Header {
        first_ts_micros: 1,
        last_ts_micros: 2,
        ..header_of(&empty)
    };
    assert_eq!(empty.describes(&nothing, may()), Ok(()));
}

//! Did it work in EVERY period, or only on average?
//!
//! # The question a total cannot answer
//!
//! `runner::grid::Cell` is a fold: totals, counts, and two maxima over every
//! trade. It answers *"what did this make over eighty-one months"* and *"how far
//! did the worst trade run"*, and both are exactly right.
//!
//! It cannot answer *"was it profitable in every one of those years"*, and the
//! difference is not small. Two combinations:
//!
//! | year | A | B |
//! |---|---|---|
//! | 2020 | +40,000 | +5,600 |
//! | 2021 | −3,000 | +5,800 |
//! | 2022 | −4,000 | +5,500 |
//! | 2023–26 | −20,000 | +22,100 |
//! | **total** | **+13,000** | **+39,000** |
//!
//! A found one regime in 2020 and has been dying since. B works. They have the
//! same sign on the total, similar t-statistics, and the fold cannot separate
//! them. Nothing in this repository could, before this module.
//!
//! # Six grains, because a year is not enough either
//!
//! "Positive in seven years" is seven observations. A combination can be
//! positive in every year while losing money in eighteen months out of eighty
//! and making it back in two. So the same rows are bucketed by day, week, month,
//! quarter, half and year, and the report gives the share of each that was
//! positive — six views of one series, each catching what the coarser ones hide.
//!
//! # Everything is derived from the timestamps
//!
//! `runner::grid::TradeRow` carries the entry bar's stamp, so no bucket boundary
//! is a constant here: a month is a month because the calendar says so, and the
//! first and last buckets are whatever the data spans. Nothing is padded to a
//! round number and no empty bucket is invented — a period with no trades is not
//! a losing period, it is a period the combination did not fire in, and the
//! report distinguishes them.

use runner::grid::TradeRow;

/// Microseconds in a day.
const MICROS_PER_DAY: i64 = 86_400 * 1_000_000;

/// India Standard Time, +05:30 from UTC.
///
/// Applied BEFORE any division, for the reason `indicators::weekday_bit` gives:
/// a bar at 09:15 IST is 03:45 UTC the same day, and dividing first files every
/// morning bar before 05:30 IST into the previous day.
const IST_OFFSET_MICROS: i64 = 19_800 * 1_000_000;

/// How finely to slice the series.
///
/// Ordered coarsest to finest so a report reads down toward detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grain {
    /// Calendar year.
    Year,
    /// Six months: January–June, July–December.
    Half,
    /// Three months.
    Quarter,
    /// Calendar month.
    Month,
    /// Seven days from the epoch's own Thursday — see [`Grain::bucket`].
    Week,
    /// One IST trading day.
    Day,
}

/// Every grain, coarsest first, for a report that walks them all.
pub const GRAINS: [Grain; 6] = [
    Grain::Year,
    Grain::Half,
    Grain::Quarter,
    Grain::Month,
    Grain::Week,
    Grain::Day,
];

impl Grain {
    /// The word a report prints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Year => "year",
            Self::Half => "half",
            Self::Quarter => "quarter",
            Self::Month => "month",
            Self::Week => "week",
            Self::Day => "day",
        }
    }

    /// Which bucket a timestamp falls in.
    ///
    /// # The number is an identifier, not a date
    ///
    /// Buckets only ever need to be COMPARED — same bucket or not — so this
    /// returns a monotone key rather than a calendar value. That keeps the
    /// civil-calendar arithmetic to one place, [`year_month`], and keeps every
    /// other grain a division.
    ///
    /// `Week` divides days by seven from the epoch, which was a Thursday. That
    /// makes a "week" Thursday-to-Wednesday rather than Monday-to-Sunday, and
    /// the shift is stated rather than corrected: a week boundary that does not
    /// line up with a weekend still partitions the series evenly, which is all a
    /// consistency count needs. Correcting it would put a second calendar
    /// convention in a module that has one.
    #[must_use]
    pub fn bucket(self, ts_micros: i64) -> i64 {
        let day = ts_micros
            .saturating_add(IST_OFFSET_MICROS)
            .div_euclid(MICROS_PER_DAY);
        match self {
            Self::Day => day,
            Self::Week => day.div_euclid(7),
            Self::Month | Self::Quarter | Self::Half | Self::Year => {
                let (y, m) = year_month(day);
                // Months since year zero, then divided into the grain. One
                // expression rather than four, so a boundary cannot be right at
                // one grain and wrong at another.
                let months = y.saturating_mul(12).saturating_add(i64::from(m) - 1);
                match self {
                    Self::Month => months,
                    Self::Quarter => months.div_euclid(3),
                    Self::Half => months.div_euclid(6),
                    _ => months.div_euclid(12),
                }
            }
        }
    }
}

/// The civil year and month a day-since-epoch falls in.
///
/// Howard Hinnant's `civil_from_days`, which is exact over the whole `i64` range
/// and needs no lookup table. Written out rather than pulled in: a date
/// dependency for one function would be a dependency `CLAUDE.md` §2 has to
/// justify, and this is twelve lines.
fn year_month(days_since_epoch: i64) -> (i64, u32) {
    let z = days_since_epoch.saturating_add(719_468);
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, u32::try_from(m).unwrap_or(1))
}

/// One period's worth of trades.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bucket {
    /// The grain's own key, so buckets sort and compare without a date.
    pub key: i64,
    /// Round trips that opened in this period.
    pub trades: u64,
    /// How many made money under the pessimistic reading.
    pub wins: u64,
    /// Total paisa per unit, pessimistic.
    pub net: i64,
    /// The furthest ANY trade in this period ran against entry, in ppm.
    ///
    /// A maximum and not a mean, for the reason `Cell::worst_mae` is: a stop is
    /// placed once and every trade must survive it.
    pub worst_adverse: i64,
    /// The same, in PAISA at each trade's own entry price.
    ///
    /// This is the figure an operator's rule is actually compared against. A
    /// rule of "no trade beyond ten points" converted once at a span-wide
    /// reference means ten points at one end of the span and seventeen at the
    /// other; measured per trade it means ten points everywhere.
    pub worst_adverse_paisa: i64,
}

/// What the buckets say, at one grain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stability {
    /// Every period that held at least one trade, in order.
    ///
    /// **Empty periods are absent rather than zero.** A quarter with no trades
    /// is not a losing quarter; it is a quarter the combination did not fire in,
    /// and counting it as a loss would punish selectivity — which is the whole
    /// property being searched for.
    pub buckets: Vec<Bucket>,
}

impl Stability {
    /// Periods whose pessimistic total was above zero.
    #[must_use]
    pub fn positive(&self) -> usize {
        self.buckets.iter().filter(|b| b.net > 0).count()
    }

    /// The share of periods that were positive, in basis points.
    ///
    /// `0` when there are no periods, which a caller reads as "no evidence"
    /// rather than as "never positive" — the two are different and a rule that
    /// conflated them would reject a combination for having no trades in a
    /// grain it was never given.
    #[must_use]
    pub fn positive_share_bp(&self) -> i64 {
        let n = i64::try_from(self.buckets.len()).unwrap_or(i64::MAX);
        if n == 0 {
            return 0;
        }
        i64::try_from(self.positive())
            .unwrap_or(i64::MAX)
            .saturating_mul(10_000)
            / n
    }

    /// The worst single period's total.
    #[must_use]
    pub fn worst_period(&self) -> i64 {
        self.buckets.iter().map(|b| b.net).min().unwrap_or(0)
    }

    /// The furthest any trade ran against entry, over every period.
    ///
    /// Equal to `Cell::worst_mae` by construction — the same trades, the same
    /// maximum — and computed here so a per-period table can be read without
    /// the fold beside it.
    #[must_use]
    pub fn worst_adverse(&self) -> i64 {
        self.buckets
            .iter()
            .map(|b| b.worst_adverse)
            .max()
            .unwrap_or(0)
    }
}

/// Bucket one variant's trades at one grain.
///
/// # Cost
///
/// One pass over the rows. They arrive in entry order because
/// `runner::grid` walks candidates in order, so the bucket key only ever
/// increases and a new bucket is opened by comparing against the last —
/// **O(trades)**, no sort and no map.
///
/// That ordering is an assumption about the caller, so it is not trusted: a key
/// that goes BACKWARDS opens a new bucket rather than merging into an earlier
/// one, which keeps the count honest instead of silently producing a bucket that
/// spans two periods.
#[must_use]
pub fn at(rows: &[TradeRow], grain: Grain) -> Stability {
    let mut buckets: Vec<Bucket> = Vec::new();
    for row in rows {
        let key = grain.bucket(row.ts_micros);
        let fresh = buckets.last().is_none_or(|b| b.key != key);
        if fresh {
            buckets.push(Bucket {
                key,
                ..Bucket::default()
            });
        }
        let Some(b) = buckets.last_mut() else {
            continue;
        };
        b.trades = b.trades.saturating_add(1);
        if row.pnl > 0 {
            b.wins = b.wins.saturating_add(1);
        }
        b.net = b.net.saturating_add(row.pnl);
        if row.adverse > b.worst_adverse {
            b.worst_adverse = row.adverse;
        }
        if row.adverse_paisa > b.worst_adverse_paisa {
            b.worst_adverse_paisa = row.adverse_paisa;
        }
    }
    Stability { buckets }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes"
)]
mod tests {
    use super::{Grain, at, year_month};
    use runner::grid::TradeRow;

    /// Days since the epoch for a civil date, so fixtures read as dates.
    fn day(y: i64, m: u32, d: u32) -> i64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = i64::from((m + 9) % 12);
        let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// A trade opening at 09:15 IST on a given date.
    fn trade(y: i64, m: u32, d: u32, pnl: i64) -> TradeRow {
        TradeRow {
            // 09:15 IST is 03:45 UTC.
            ts_micros: (day(y, m, d) * 86_400 + 3 * 3600 + 45 * 60) * 1_000_000,
            pnl,
            adverse: 0,
            adverse_paisa: 0,
        }
    }

    /// THE CIVIL CALENDAR IS EXACT, INCLUDING THE CASES THAT USUALLY ARE NOT.
    ///
    /// Month arithmetic is where date code fails quietly, and a bucket assigned
    /// to the wrong month makes a consistency count wrong without making it
    /// look wrong. Leap day, leap century, both year boundaries and the epoch.
    #[test]
    fn the_calendar_is_right_on_every_case_that_usually_breaks_it() {
        for (y, m, d) in [
            (2020_i64, 1_u32, 1_u32),
            (2019, 12, 31),
            (2024, 2, 29),
            (2000, 2, 29),
            (1970, 1, 1),
            (2026, 8, 23),
        ] {
            assert_eq!(year_month(day(y, m, d)), (y, m), "{y}-{m:02}-{d:02}");
        }
    }

    /// EVERY GRAIN PARTITIONS, AND COARSER NEVER SPLITS WHAT FINER JOINS.
    ///
    /// The property that makes six grains meaningful: two trades in the same
    /// month are in the same quarter, half and year. A grain that violated it
    /// would report more periods at a coarser slice than a finer one, which is
    /// arithmetically impossible and would mean a boundary is wrong.
    #[test]
    fn a_coarser_grain_never_separates_what_a_finer_one_joins() {
        let rows: Vec<TradeRow> = vec![
            trade(2020, 1, 2, 10),
            trade(2020, 1, 3, 10),
            trade(2020, 3, 31, 10),
            trade(2020, 4, 1, 10),
            trade(2020, 12, 31, 10),
            trade(2021, 1, 1, 10),
        ];
        let counts: Vec<usize> = [
            Grain::Year,
            Grain::Half,
            Grain::Quarter,
            Grain::Month,
            Grain::Day,
        ]
        .iter()
        .map(|&g| at(&rows, g).buckets.len())
        .collect();
        for pair in counts.windows(2) {
            let [coarse, fine] = pair else { continue };
            assert!(
                coarse <= fine,
                "a coarser grain produced MORE periods than a finer one: \
                 {counts:?}"
            );
        }
        // And the specific boundaries, so the test is not satisfied by every
        // grain collapsing to one bucket.
        assert_eq!(at(&rows, Grain::Year).buckets.len(), 2, "2020 and 2021");
        assert_eq!(at(&rows, Grain::Quarter).buckets.len(), 4, "Q1 Q2 Q4 Q1");
        assert_eq!(at(&rows, Grain::Month).buckets.len(), 5);
    }

    /// A GOOD TOTAL AND A GOOD EVERY-YEAR ARE DIFFERENT, AND THIS IS THE PROOF.
    ///
    /// Both series below make money. One made it all in the first year and has
    /// lost since; the other earned it evenly. Their TOTALS are close and their
    /// per-year records are opposite, which is the entire reason this module
    /// exists.
    #[test]
    fn a_combination_that_died_after_one_good_year_is_separable_from_one_that_did_not() {
        let died = vec![
            trade(2020, 6, 1, 40_000),
            trade(2021, 6, 1, -3_000),
            trade(2022, 6, 1, -4_000),
            trade(2023, 6, 1, -20_000),
        ];
        let steady = vec![
            trade(2020, 6, 1, 3_500),
            trade(2021, 6, 1, 3_400),
            trade(2022, 6, 1, 3_300),
            trade(2023, 6, 1, 3_200),
        ];
        let (a, b) = (at(&died, Grain::Year), at(&steady, Grain::Year));

        // The totals do not separate them -- both are positive.
        let total = |s: &super::Stability| s.buckets.iter().map(|x| x.net).sum::<i64>();
        assert!(total(&a) > 0 && total(&b) > 0, "both make money overall");

        // The per-year record does, completely.
        assert_eq!(a.positive_share_bp(), 2_500, "one year in four");
        assert_eq!(b.positive_share_bp(), 10_000, "every year");
        assert_eq!(a.worst_period(), -20_000);
        assert_eq!(b.worst_period(), 3_200);
    }

    /// AN EMPTY PERIOD IS ABSENT, NOT A LOSING ONE.
    ///
    /// A quarter with no trades is a quarter the combination did not fire in.
    /// Counting it as a loss would punish selectivity, which is the property
    /// being searched for -- the rarest combination fires least often.
    #[test]
    fn a_period_with_no_trades_is_not_counted_as_a_loss() {
        let sparse = vec![trade(2020, 1, 2, 100), trade(2023, 1, 2, 100)];
        let s = at(&sparse, Grain::Year);
        assert_eq!(s.buckets.len(), 2, "2021 and 2022 are absent, not zero");
        assert_eq!(
            s.positive_share_bp(),
            10_000,
            "both periods it traded in were positive"
        );
    }

    /// NO ROWS IS NO EVIDENCE, AND NOT A FAILING SCORE.
    #[test]
    fn an_empty_series_reports_no_evidence_rather_than_never_positive() {
        let s = at(&[], Grain::Year);
        assert_eq!(s.buckets.len(), 0);
        assert_eq!(s.positive_share_bp(), 0);
        assert_eq!(s.worst_period(), 0);
        assert_eq!(s.worst_adverse(), 0);
    }

    /// THE WORST ADVERSE IS A MAXIMUM OVER EVERY PERIOD, NOT A MEAN.
    ///
    /// The figure that decides whether a stop is survivable is the worst single
    /// excursion, and a per-period table that averaged it would report a stop as
    /// holding when one trade in ten thousand ran through it.
    #[test]
    fn the_worst_adverse_is_the_maximum_across_every_period() {
        let rows = vec![
            TradeRow {
                ts_micros: trade(2020, 1, 2, 0).ts_micros,
                pnl: 10,
                adverse: 40,
                adverse_paisa: 40,
            },
            TradeRow {
                ts_micros: trade(2021, 1, 2, 0).ts_micros,
                pnl: 10,
                adverse: 900,
                adverse_paisa: 900,
            },
            TradeRow {
                ts_micros: trade(2022, 1, 2, 0).ts_micros,
                pnl: 10,
                adverse: 60,
                adverse_paisa: 60,
            },
        ];
        let s = at(&rows, Grain::Year);
        assert_eq!(s.worst_adverse(), 900, "the maximum, not the mean of 333");
    }
}

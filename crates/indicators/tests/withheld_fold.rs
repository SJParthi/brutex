//! p11num-1, D-1781: a withheld day is FOLDED and not SWEPT.
//!
//! The repro shape the finding ran: fourteen sessions of 375 one-minute bars
//! on a deterministic random walk, with one interior minute missing on the
//! eleventh. Before D-1781 every stored door removed that day from the slice
//! before the fold, so the days after it were computed on day ten spliced onto
//! day twelve. These tests pin the replacement: the withheld day is stepped
//! through the evaluator and only its rows are left out, so every swept row is
//! byte-identical to the same bar's row in a run that folded the whole series.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a failing fixture must name its premise"
)]

use indicators::Candle;
use indicators::anchored::{AnchoredEvaluator, DailyEligibility, DailyReference};
use indicators::column::{AnchoredColumn, Column};
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use vocab::ConditionMask;

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
/// 09:15 IST in UTC microseconds past midnight.
const OPEN: i64 = 225 * MINUTE;
const FIRST_DAY: i64 = 27_000;
const SESSIONS: i64 = 14;
const BARS_PER_SESSION: i64 = 375;
/// The eleventh session, so three sessions (1,125 bars) follow it, as in the
/// finding.
const HOLED_DAY: i64 = FIRST_DAY + 10;
/// The interior minute the vendor never served on the holed day.
const HOLE: i64 = 150;

/// The trend families the finding counted: EMA20/200 (0-5) and ATR and
/// `SuperTrend` (64-65).
const TREND_BITS: [u32; 8] = [0, 1, 2, 3, 4, 5, 64, 65];

/// A deterministic xorshift walk. No `rand`: the same inputs give the same
/// bars on every machine (`CLAUDE.md` §3 rule 5).
fn walk() -> Vec<Candle> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        i64::try_from(state % 4_001).expect("small") - 2_000
    };
    let mut close: i64 = 2_500_000;
    let mut bars = Vec::new();
    for day in FIRST_DAY..FIRST_DAY + SESSIONS {
        for minute in 0..BARS_PER_SESSION {
            let open = close;
            close = (close + next()).max(1_000_000);
            let wick = next().abs() / 4;
            let bar = Candle {
                ts_micros: day * DAY + OPEN + minute * MINUTE,
                open,
                high: open.max(close) + wick,
                low: open.min(close) - wick,
                close,
                volume: 0,
                open_interest: i64::MIN,
            };
            if day == HOLED_DAY && minute == HOLE {
                continue;
            }
            bars.push(bar);
        }
    }
    bars
}

fn evaluator() -> Evaluator {
    Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

fn day_of(bar: &Candle) -> i64 {
    indicators::ist_day(bar.ts_micros)
}

/// The rows of `whole` whose bar is not on the withheld day, with each source
/// renumbered to the kept slice, which is the column a withholding fold must
/// return.
fn kept_rows_of(
    whole: &Column,
    bars: &[Candle],
) -> (Vec<ConditionMask>, Vec<ConditionMask>, Vec<usize>) {
    let held = bars.iter().filter(|bar| day_of(bar) == HOLED_DAY).count();
    let first_held = bars
        .iter()
        .position(|bar| day_of(bar) == HOLED_DAY)
        .expect("the holed day is in the series");
    let (mut bits, mut known, mut sources) = (Vec::new(), Vec::new(), Vec::new());
    for ((mask, available), &source) in whole.bits().iter().zip(whole.known()).zip(whole.sources())
    {
        if day_of(&bars[source]) == HOLED_DAY {
            continue;
        }
        bits.push(*mask);
        known.push(*available);
        sources.push(if source > first_held {
            source - held
        } else {
            source
        });
    }
    (bits, known, sources)
}

/// How many rows after the held day the spliced column differs from the
/// withholding fold on a trend bit, and how many rows follow the held day.
/// Before it, the two must agree row for row. Prints the per-bit counts.
fn splice_differences(withheld: &Column, spliced: &Column, kept: &[Candle]) -> (usize, usize) {
    let after = kept
        .iter()
        .position(|bar| day_of(bar) > HOLED_DAY)
        .expect("sessions follow the held day");
    let mut differing_rows = 0_usize;
    let mut per_bit = [0_usize; TREND_BITS.len()];
    let spliced_at = |source: usize| {
        spliced
            .sources()
            .iter()
            .position(|&s| s == source)
            .map(|row| spliced.bits()[row])
    };
    let mut after_rows = 0_usize;
    for (row, &source) in withheld.sources().iter().enumerate() {
        if source < after {
            assert_eq!(
                spliced_at(source),
                Some(withheld.bits()[row]),
                "before the held day the two runs agree"
            );
            continue;
        }
        after_rows += 1;
        let Some(old) = spliced_at(source) else {
            continue;
        };
        let new = withheld.bits()[row];
        let mut differs = false;
        for (slot, &bit) in TREND_BITS.iter().enumerate() {
            if old.get(bit) != new.get(bit) {
                per_bit[slot] += 1;
                differs = true;
            }
        }
        differing_rows += usize::from(differs);
    }
    eprintln!(
        "p11num-1: {differing_rows} of {after_rows} rows after the held day differ on trend bits; per bit {:?}",
        TREND_BITS.iter().zip(per_bit).collect::<Vec<_>>()
    );
    (differing_rows, after_rows)
}

/// **A withheld day's state reaches every later row, and its own rows are
/// not swept.** p11num-1, D-1781.
#[test]
fn a_withheld_day_is_folded_so_later_rows_equal_the_whole_fold_and_its_own_rows_are_not_swept() {
    let bars = walk();
    assert_eq!(
        bars.len(),
        usize::try_from(SESSIONS * BARS_PER_SESSION - 1).expect("small"),
        "premise: fourteen sessions with one interior minute missing"
    );
    let held = bars.iter().filter(|bar| day_of(bar) == HOLED_DAY).count();
    assert_eq!(held, 374, "premise: the holed day keeps its other minutes");
    let kept: Vec<Candle> = bars
        .iter()
        .copied()
        .filter(|bar| day_of(bar) != HOLED_DAY)
        .collect();

    // A: the whole series folded and swept. B: folded whole, D withheld.
    // C: what every stored door did before D-1781 -- the day cut first.
    let whole = Column::build(&bars, &mut evaluator());
    let (withheld, folded_without_row) =
        Column::build_withholding(&bars, &mut evaluator(), &[HOLED_DAY]);
    let spliced = Column::build(&kept, &mut evaluator());

    assert_eq!(
        folded_without_row,
        u64::try_from(held).expect("small"),
        "every bar of the withheld day was folded and given no row"
    );
    let (bits, known, sources) = kept_rows_of(&whole, &bars);
    assert_eq!(
        withheld.bits(),
        bits.as_slice(),
        "every swept mask is the whole fold's"
    );
    assert_eq!(
        withheld.known(),
        known.as_slice(),
        "and so is every availability mask"
    );
    assert_eq!(
        withheld.sources(),
        sources.as_slice(),
        "sources index the kept slice"
    );
    for &source in withheld.sources() {
        assert_ne!(
            day_of(&kept[source]),
            HOLED_DAY,
            "no row is on the withheld day"
        );
    }
    assert_eq!(
        withheld.census().offered,
        u64::try_from(kept.len()).expect("small")
    );
    assert!(withheld.census().reconciles());
    assert!(
        withheld.sources().len() > 1_000,
        "premise: the fixture is warm"
    );

    // THE DEFECT, MEASURED. The spliced column is indexed by the same kept
    // slice; on the rows after the held day it must differ from the fold.
    let (differing_rows, after_rows) = splice_differences(&withheld, &spliced, &kept);
    assert_eq!(
        after_rows, 1_125,
        "premise: three sessions of 375 bars follow"
    );
    assert!(
        differing_rows > 0,
        "the splice must be visible on this fixture, or the test proves nothing"
    );
}

/// **Withholding nothing is the ordinary build, byte for byte.**
#[test]
fn withholding_no_day_is_the_ordinary_build() {
    let bars = walk();
    let (column, withheld) = Column::build_withholding(&bars, &mut evaluator(), &[]);
    assert_eq!(withheld, 0);
    assert_eq!(column, Column::build(&bars, &mut evaluator()));
    // A day the series does not hold withholds nothing and changes nothing.
    let (absent, none) = Column::build_withholding(&bars, &mut evaluator(), &[FIRST_DAY - 1]);
    assert_eq!(none, 0);
    assert_eq!(absent, column);
}

fn references() -> Vec<DailyReference> {
    (FIRST_DAY - 5..FIRST_DAY + SESSIONS)
        .map(|day| {
            DailyReference::new(
                Candle {
                    ts_micros: day * DAY + OPEN,
                    open: 2_500_000,
                    high: 2_510_000,
                    low: 2_490_000,
                    close: 2_500_500,
                    volume: 0,
                    open_interest: i64::MIN,
                },
                DailyEligibility::Eligible,
            )
            .expect("an eligible generated daily record")
        })
        .collect()
}

fn anchored(references: &[DailyReference]) -> AnchoredEvaluator<'_> {
    AnchoredEvaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
        references,
    )
    .expect("ordered references")
}

/// **The anchored door the stored sweep uses folds the same way.**
#[test]
fn the_anchored_withholding_build_equals_the_whole_anchored_fold_on_every_kept_row() {
    let bars = walk();
    let references = references();
    let whole = AnchoredColumn::build_required(&bars, &mut anchored(&references))
        .expect("every day has a prior record");
    let mut evaluator = anchored(&references);
    let withheld = AnchoredColumn::build_required_withholding(&bars, &mut evaluator, &[HOLED_DAY])
        .expect("every day has a prior record");
    assert_eq!(withheld.withheld_bars(), 374);
    assert_eq!(whole.withheld_bars(), 0);
    let (bits, known, sources) = kept_rows_of(whole.column(), &bars);
    assert_eq!(withheld.column().bits(), bits.as_slice());
    assert_eq!(withheld.column().known(), known.as_slice());
    assert_eq!(withheld.column().sources(), sources.as_slice());
    // The withheld day's bars consumed references, so the daily census is
    // the whole fold's.
    assert_eq!(withheld.reference_census(), whole.reference_census());
    assert!(withheld.reference_census().reconciles());
    // The evaluator committed: it ends where the whole fold ends.
    assert_eq!(
        evaluator.reference_census(),
        whole.reference_census(),
        "a successful build commits the evaluator"
    );
}

/// **A withheld bar with no prior daily record still refuses the build**,
/// because its state would reach every later row. The evaluator is left
/// exactly as it was handed in.
#[test]
fn a_withheld_bar_without_a_daily_reference_refuses_and_commits_nothing() {
    let bars = walk();
    // References begin only after the first session, so session one -- even
    // when withheld -- has no prior record.
    let late: Vec<DailyReference> = references().into_iter().skip(5).collect();
    let mut evaluator = anchored(&late);
    let before = evaluator.reference_census();
    let refused = AnchoredColumn::build_required_withholding(&bars, &mut evaluator, &[FIRST_DAY])
        .expect_err("the withheld first session had no prior record");
    assert_eq!(refused.signal_days, 1);
    assert_eq!(refused.signal_bars, 375);
    assert_eq!(evaluator.reference_census(), before, "nothing committed");
}

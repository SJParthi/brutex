//! ATTACK `fold`, round 1: timeframe derivation from one-minute bars.
//!
//! Every rung above one minute is re-derived from minutes by `pull::fold`. This
//! file attacks that derivation against a NAIVE REFERENCE fold written here
//! from the table in `fold`'s own module doc (open first, high max, low min,
//! close last, volume summed, open interest the last that carried one), over
//! 100,000 seeded random sessions, plus the ladder's associativity, the grid's
//! alignment to 09:15 IST, the closing stubs, the open-interest null sentinel,
//! overflow, ordering, day/month/year/leap boundaries, exceptional sessions,
//! and the per-input-bar cost of the fold at 10^3..10^6 bars.
//!
//! The PRNG is a hand-written splitmix64 with a fixed seed, so every rerun is
//! byte-identical (`CLAUDE.md` §3 rule 5).

// A test that asserts nothing is banned; one that cannot fail loudly is one
// that asserts nothing.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use pull::calendar::{DayKind, Session as CalendarSession};
use pull::fold::{Bucket, FoldError, complete_minutes, fold, fold_from_bars};
use pull::session::{Day, IST_OFFSET_SECS, IstMoment};
use pull::vendor::{SessionKind, Venue};
use store::format::Bar;

const MINUTE_US: i64 = 60_000_000;
const DAY_US: i64 = 86_400_000_000;
const IST_US: i64 = IST_OFFSET_SECS * 1_000_000;
/// The rungs the store files and the ladder derives, plus the day.
const WIDTHS: [u32; 9] = [60, 120, 180, 300, 600, 900, 1_800, 3_600, 86_400];

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, per_thousand: u64) -> bool {
        self.below(1_000) < per_thousand
    }
}

/// IST midnight of an epoch day, as UTC micros.
fn midnight(day: i64) -> i64 {
    day * DAY_US - IST_US
}

fn day_of(d: Day) -> i64 {
    i64::from(d.days_from_epoch())
}

fn bar(ts: i64, rng: &mut SplitMix) -> Bar {
    // Paisa integers, high >= open/close >= low by construction.
    let a = i64::try_from(rng.below(5_000_000)).unwrap() + 1;
    let b = i64::try_from(rng.below(5_000_000)).unwrap() + 1;
    let up = i64::try_from(rng.below(10_000)).unwrap();
    let down = i64::try_from(rng.below(10_000)).unwrap();
    Bar {
        ts_micros: ts,
        open: a,
        high: a.max(b) + up,
        low: (a.min(b) - down).max(1),
        close: b,
        volume: i64::try_from(rng.below(1_000_000)).unwrap(),
        open_interest: if rng.chance(300) {
            i64::MIN
        } else {
            i64::try_from(rng.below(1_000_000_000)).unwrap()
        },
    }
}

fn plain(ts: i64) -> Bar {
    Bar {
        ts_micros: ts,
        open: 100,
        high: 100,
        low: 100,
        close: 100,
        volume: 1,
        open_interest: i64::MIN,
    }
}

/// THE REFERENCE. Written from `fold`'s module-doc table, not from its code:
/// the bucket of an intraday width is counted from 09:15 IST of a FIXED
/// reference day (2025-07-01) rather than from the epoch, and the day bucket
/// from that day's IST midnight. Sums are taken in i128 so an overflow the
/// fold must refuse is visible here as a value that does not fit.
fn reference(bars: &[Bar], secs: u32) -> Option<Vec<Bar>> {
    let reference_day = 20_270_i64; // 2025-07-01
    let anchor = if secs >= 86_400 {
        midnight(reference_day)
    } else {
        midnight(reference_day) + 555 * MINUTE_US
    };
    let width = i64::from(secs) * 1_000_000;
    let mut out: Vec<Bar> = Vec::new();
    let mut volume: i128 = 0;
    for b in bars {
        let start = anchor + (b.ts_micros - anchor).div_euclid(width) * width;
        let same = out.last().is_some_and(|last| last.ts_micros == start);
        if same {
            let last = out.last_mut().unwrap();
            last.high = last.high.max(b.high);
            last.low = last.low.min(b.low);
            last.close = b.close;
            volume += i128::from(b.volume);
            last.volume = i64::try_from(volume).ok()?;
            if b.open_interest != i64::MIN {
                last.open_interest = b.open_interest;
            }
        } else {
            volume = i128::from(b.volume);
            out.push(Bar {
                ts_micros: start,
                ..*b
            });
        }
    }
    Some(out)
}

/// One random "input": one or two IST days of minutes with random holes, the
/// occasional pre-open or post-close minute, and day pairs that straddle a
/// month or a year boundary at a raised rate.
fn random_input(rng: &mut SplitMix, first_day: i64, span: u64) -> Vec<Bar> {
    let mut days = vec![first_day + i64::try_from(rng.below(span)).unwrap()];
    if rng.chance(250) {
        let next = days[0] + 1 + i64::try_from(rng.below(3)).unwrap();
        days.push(next);
    }
    let mut out = Vec::new();
    for day in days {
        let keep = 200 + rng.below(800); // per-mille of minutes kept
        let from: i64 = if rng.chance(100) { 540 } else { 555 };
        let to: i64 = if rng.chance(100) { 940 } else { 930 };
        // A short random slice keeps 100,000 sessions affordable in a debug
        // build; a tenth of the sessions are whole.
        let (lo, hi) = if rng.chance(100) {
            (from, to)
        } else {
            let lo = from + i64::try_from(rng.below(u64::try_from(to - from).unwrap())).unwrap();
            let len = 1 + i64::try_from(rng.below(90)).unwrap();
            (lo, (lo + len).min(to))
        };
        for m in lo..hi {
            if rng.below(1_000) < keep {
                out.push(bar(midnight(day) + m * MINUTE_US, rng));
            }
        }
    }
    out
}

fn widths() -> Vec<Bucket> {
    WIDTHS
        .iter()
        .map(|s| Bucket::of_secs(*s).unwrap())
        .collect()
}

/// Every (source, target) pair on the ladder where the target is a whole
/// multiple of the source AND, for the day, the source grid lands on IST
/// midnight (the source width divides 555 minutes). The four day pairs that
/// fail the second test are asserted refused by name here.
fn ladder_pairs() -> Vec<(u32, u32)> {
    let mut pairs = Vec::new();
    let mut misaligned = Vec::new();
    for s in WIDTHS {
        for t in WIDTHS {
            if t > s && t % s == 0 {
                if t == 86_400 && 33_300 % s != 0 {
                    misaligned.push(s);
                    assert_eq!(
                        fold_from_bars(&[], Bucket::DAY, Bucket::of_secs(s).unwrap()),
                        Err(FoldError::GridMisaligned {
                            want_secs: t,
                            source_secs: s
                        })
                    );
                } else {
                    pairs.push((s, t));
                }
            }
        }
    }
    assert_eq!(misaligned, vec![120, 600, 1_800, 3_600]);
    pairs
}

/// 100,000 random sessions: `fold` equals the naive reference at a random
/// rung, and the two-step ladder fold equals the direct fold at a random pair.
/// A further 2,000 sessions are checked at EVERY rung and EVERY pair.
#[test]
fn random_sessions_fold_equals_the_naive_reference_and_the_ladder_is_associative() {
    let mut rng = SplitMix(0x00F0_1D00_2026_1004);
    let first = day_of(Day::new(2019, 12, 1).unwrap());
    let span = u64::try_from(day_of(Day::new(2026, 9, 4).unwrap()) - first).unwrap();
    let all = widths();
    let pairs = ladder_pairs();
    assert_eq!(
        pairs.len(),
        26,
        "the ladder's divisible, midnight-aligned pairs"
    );
    let mut bars_folded = 0_usize;
    for case in 0..100_000_u32 {
        let input = random_input(&mut rng, first, span);
        bars_folded += input.len();
        let exhaustive = case < 2_000;
        let rungs: Vec<Bucket> = if exhaustive {
            all.clone()
        } else {
            vec![all[usize::try_from(rng.below(9)).unwrap()]]
        };
        for bucket in rungs {
            let want = reference(&input, bucket.secs()).expect("volumes fit");
            assert_eq!(
                fold(&input, bucket).unwrap(),
                want,
                "case {case}: {}s",
                bucket.secs()
            );
        }
        let chosen: Vec<(u32, u32)> = if exhaustive {
            pairs.clone()
        } else {
            vec![pairs[usize::try_from(rng.below(26)).unwrap()]]
        };
        for (s, t) in chosen {
            let (s, t) = (Bucket::of_secs(s).unwrap(), Bucket::of_secs(t).unwrap());
            let step = fold_from_bars(&fold(&input, s).unwrap(), t, s).unwrap();
            assert_eq!(
                step,
                fold(&input, t).unwrap(),
                "case {case}: 1m -> {}s -> {}s must equal 1m -> {}s",
                s.secs(),
                t.secs(),
                t.secs()
            );
        }
    }
    assert!(bars_folded > 1_000_000, "{bars_folded} bars exercised");
}

/// THE REFERENCE FOR COMPLETENESS. A derived bucket is certified iff its day
/// is a calendar-full session with verified continuous index hours from
/// 09:15, and every scheduled minute in it is present exactly once, on the
/// minute grid, with nothing outside the schedule. 20,000 random sessions
/// with holes, duplicates, off-grid stamps and pre-open minutes.
#[test]
fn random_sessions_complete_minutes_certifies_exactly_the_whole_buckets() {
    let mut rng = SplitMix(0xC0_4417_E7E5);
    let first = day_of(Day::new(2019, 12, 1).unwrap());
    let span = u64::try_from(day_of(Day::new(2026, 9, 4).unwrap()) - first).unwrap();
    let all = widths();
    let mut certified = 0_usize;
    let mut withheld = 0_usize;
    for case in 0..20_000_u32 {
        let mut input = random_input(&mut rng, first, span);
        // Corrupt a few: a duplicate, an off-grid stamp.
        if rng.chance(100) && !input.is_empty() {
            let i = usize::try_from(rng.below(u64::try_from(input.len()).unwrap())).unwrap();
            let copy = input[i];
            input.insert(i, copy);
        }
        if rng.chance(100) && !input.is_empty() {
            let i = usize::try_from(rng.below(u64::try_from(input.len()).unwrap())).unwrap();
            input[i].ts_micros += 17_000_000;
            let ordered = input.windows(2).all(|w| w[0].ts_micros <= w[1].ts_micros);
            if !ordered {
                input[i].ts_micros -= 17_000_000;
            }
        }
        let bucket = all[usize::try_from(rng.below(9)).unwrap()];
        let width = i64::from(bucket.secs()) * 1_000_000;
        let (complete, _diagnostics) = complete_minutes(&input, bucket).unwrap();
        let folded = fold(&input, bucket).unwrap();
        let mut want = Vec::new();
        for b in &folded {
            let end = b.ts_micros + width;
            let day = (b.ts_micros + IST_US).div_euclid(DAY_US);
            let schedule = scheduled_index_minutes(day);
            let Some((open, close)) = schedule else {
                continue;
            };
            let mut seen: Vec<i64> = Vec::new();
            let mut ok = true;
            for src in &input {
                if src.ts_micros < b.ts_micros || src.ts_micros >= end {
                    continue;
                }
                let into = src.ts_micros - midnight(day);
                let m = into.div_euclid(MINUTE_US);
                ok &= into.rem_euclid(MINUTE_US) == 0
                    && (open..close).contains(&m)
                    && !seen.contains(&src.ts_micros);
                seen.push(src.ts_micros);
            }
            let from = (b.ts_micros - midnight(day))
                .div_euclid(MINUTE_US)
                .max(open);
            let to = ((end - midnight(day)) / MINUTE_US).min(close);
            let scheduled = usize::try_from((to - from).max(0)).unwrap();
            if ok && scheduled > 0 && seen.len() == scheduled {
                want.push(*b);
            }
        }
        certified += want.len();
        withheld += folded.len() - want.len();
        assert_eq!(complete, want, "case {case}: {}s", bucket.secs());
    }
    assert!(
        certified > 1_000 && withheld > 1_000,
        "{certified} / {withheld}"
    );
}

/// (open, close) minutes past IST midnight, close exclusive, for a day the
/// calendar measures as a full regular session with verified NSE index hours;
/// `None` where nothing may be certified.
fn scheduled_index_minutes(day: i64) -> Option<(i64, i64)> {
    if pull::calendar::kind_of(day) != DayKind::Open(CalendarSession::full()) {
        return None;
    }
    let civil = Day::from_days(u32::try_from(day).ok()?).ok()?;
    let hours = Venue::NseIndex.hours_on(civil).ok()?;
    if hours.kind() != SessionKind::Continuous || hours.open_minute() != 555 {
        return None;
    }
    Some((555, i64::from(hours.close_minute())))
}

/// A WIDTH THAT DOES NOT DIVIDE A DAY DRIFTS OFF 09:15. `fold` anchors its
/// intraday grid at 09:15 IST of 1970-01-01 and lets it run, which lands on
/// every later day's open only when the width divides 86,400. Every width
/// `Bucket::of_secs` accepts must open every day exactly at 09:15 (and the
/// day bucket at IST midnight). Exhaustive over every whole-minute width from
/// one minute to one day, on four days that include a month, a year and a
/// leap-day boundary.
#[test]
fn every_accepted_width_opens_every_session_at_the_open() {
    let days: Vec<i64> = [
        Day::new(2025, 7, 1).unwrap(),
        Day::new(2025, 12, 31).unwrap(),
        Day::new(2026, 1, 1).unwrap(),
        Day::new(2024, 2, 29).unwrap(),
    ]
    .iter()
    .map(|d| day_of(*d))
    .collect();
    let mut accepted = 0_u32;
    for secs in (60..=86_400_u32).step_by(60) {
        let Some(bucket) = Bucket::of_secs(secs) else {
            continue;
        };
        accepted += 1;
        for day in &days {
            let session: Vec<Bar> = (555..930)
                .map(|m| plain(midnight(*day) + m * MINUTE_US))
                .collect();
            let out = fold(&session, bucket).unwrap();
            let want = if secs >= 86_400 {
                midnight(*day)
            } else {
                midnight(*day) + 555 * MINUTE_US
            };
            assert_eq!(
                out[0].ts_micros, want,
                "{secs}s on day {day}: the first bar must start at the open, \
                 not before it holding part of the session"
            );
            assert_eq!(out.iter().map(|b| b.volume).sum::<i64>(), 375);
        }
    }
    // The divisors of 1,440 minutes: still every store rung and then some.
    assert_eq!(accepted, 36, "every whole-minute divisor of a day");
    for secs in [7_u32, 420, 3_607, 86_399] {
        assert_eq!(Bucket::of_secs(secs), None, "{secs}s does not divide a day");
    }
}

/// `fold_from_bars` states its input is BARS at `source` width. Two bars
/// sharing a stamp at that width are a repeated bar, not two snapshots, and
/// merging them doubles the volume. A bar off the source grid is not a bar of
/// that width. Both must be refused by name.
#[test]
fn fold_from_bars_refuses_a_repeated_or_off_grid_source_bar() {
    let open = midnight(day_of(Day::new(2025, 7, 1).unwrap())) + 555 * MINUTE_US;
    let five = Bucket::of_secs(300).unwrap();
    let dup = [plain(open), plain(open)];
    assert_eq!(
        fold_from_bars(&dup, five, Bucket::MINUTE),
        Err(FoldError::RepeatedBar {
            at: 1,
            ts_micros: open
        })
    );
    let off = [plain(open + 30_000_000)];
    assert_eq!(
        fold_from_bars(&off, five, Bucket::MINUTE),
        Err(FoldError::OffSourceGrid {
            at: 0,
            ts_micros: open + 30_000_000,
            source_secs: 60
        })
    );
    // A five-minute bar stamped 09:16 is not on the five-minute grid.
    let fifteen = Bucket::of_secs(900).unwrap();
    assert!(matches!(
        fold_from_bars(&[plain(open + MINUTE_US)], fifteen, five),
        Err(FoldError::OffSourceGrid { .. })
    ));
    // The refusal names the row and the remedy.
    let text = FoldError::RepeatedBar {
        at: 1,
        ts_micros: open,
    }
    .to_string();
    assert!(
        text.contains("repeated") && text.contains("summed"),
        "{text}"
    );
    // And complete_minutes keeps its per-bucket behaviour: it withholds the
    // bucket with a duplicate rather than refusing the whole month.
    let mut session: Vec<Bar> = (0..375).map(|m| plain(open + m * MINUTE_US)).collect();
    session.insert(3, session[2]);
    let (complete, diagnostics) = complete_minutes(&session, five).unwrap();
    assert_eq!(complete.len(), 74);
    assert_eq!(diagnostics.len(), 1);
}

/// A DAY BAR FROM A SOURCE WHOSE GRID DOES NOT LAND ON IST MIDNIGHT. The
/// intraday grid is counted from 09:15 and the day's from midnight; they share
/// an edge only where the source width divides 555 minutes. A 60-minute bar
/// stamped 23:15 covers 00:00-00:15 of the NEXT day and would be filed whole
/// under this one: exactly the edge-inside-a-source-bar case the guard exists
/// to refuse.
#[test]
fn a_day_from_a_source_grid_that_straddles_midnight_is_refused() {
    let day = Bucket::DAY;
    for secs in [120_u32, 600, 1_800, 3_600] {
        let source = Bucket::of_secs(secs).unwrap();
        assert_eq!(
            fold_from_bars(&[], day, source),
            Err(FoldError::GridMisaligned {
                want_secs: 86_400,
                source_secs: secs
            }),
            "{secs}s"
        );
    }
    for secs in [60_u32, 180, 300, 900] {
        let source = Bucket::of_secs(secs).unwrap();
        assert!(
            fold_from_bars(&[], day, source).is_ok(),
            "{secs}s divides 555 minutes"
        );
    }
    let text = FoldError::GridMisaligned {
        want_secs: 86_400,
        source_secs: 3_600,
    }
    .to_string();
    assert!(text.contains("midnight"), "{text}");
}

/// THE CLOSING STUBS of the 375-minute index session, exactly.
#[test]
fn the_last_bucket_of_each_rung_is_its_scheduled_stub() {
    let open = midnight(day_of(Day::new(2025, 7, 1).unwrap())) + 555 * MINUTE_US;
    let session: Vec<Bar> = (0..375).map(|m| plain(open + m * MINUTE_US)).collect();
    // (rung minutes, bar count, last stamp minutes after open, last volume)
    for (rung, count, last_at, last_volume) in [
        (2_i64, 188_usize, 374_i64, 1_i64),
        (3, 125, 372, 3),
        (5, 75, 370, 5),
        (10, 38, 370, 5),
        (15, 25, 360, 15),
        (30, 13, 360, 15),
        (60, 7, 360, 15),
    ] {
        let bucket = Bucket::of_secs(u32::try_from(rung * 60).unwrap()).unwrap();
        let out = fold(&session, bucket).unwrap();
        assert_eq!(out.len(), count, "{rung}m");
        let last = out.last().unwrap();
        assert_eq!(last.ts_micros, open + last_at * MINUTE_US, "{rung}m");
        assert_eq!(last.volume, last_volume, "{rung}m");
        let (complete, diagnostics) = complete_minutes(&session, bucket).unwrap();
        assert_eq!(
            complete, out,
            "{rung}m: the stub is scheduled, so certified"
        );
        assert!(diagnostics.is_empty(), "{rung}m: {diagnostics:?}");
    }
    // The hour rung's stamps are 09:15, 10:15 .. 15:15.
    let hours = fold(&session, Bucket::of_secs(3_600).unwrap()).unwrap();
    for (i, b) in hours.iter().enumerate() {
        let at = IstMoment::from_epoch_secs(b.ts_micros / 1_000_000).unwrap();
        assert_eq!(at.minute_of_day(), 555 + 60 * u32::try_from(i).unwrap());
    }
}

/// A bucket whose ONLY minute is its last one is stamped at the bucket start
/// and carries exactly that minute; completeness withholds it by count.
#[test]
fn a_bucket_holding_only_its_last_minute() {
    let open = midnight(day_of(Day::new(2025, 7, 1).unwrap())) + 555 * MINUTE_US;
    let mut rng = SplitMix(7);
    let only = bar(open + 59 * MINUTE_US, &mut rng);
    let out = fold(&[only], Bucket::of_secs(3_600).unwrap()).unwrap();
    assert_eq!(
        out,
        vec![Bar {
            ts_micros: open,
            ..only
        }]
    );
    let (complete, diagnostics) =
        complete_minutes(&[only], Bucket::of_secs(3_600).unwrap()).unwrap();
    assert!(complete.is_empty());
    assert!(
        diagnostics[0].contains("observed 1, scheduled 60"),
        "{diagnostics:?}"
    );
}

/// THE OPEN-INTEREST NULL is never treated as a number: never summed, never
/// maxed, never overwrites a real value; a bucket of nulls stays null, and a
/// real value survives later nulls.
#[test]
fn open_interest_is_the_last_that_carried_one_and_the_null_is_never_arithmetic() {
    let open = midnight(day_of(Day::new(2025, 7, 1).unwrap())) + 555 * MINUTE_US;
    let with = |m: i64, oi: i64| Bar {
        open_interest: oi,
        ..plain(open + m * MINUTE_US)
    };
    let five = Bucket::of_secs(300).unwrap();
    for (ois, want) in [
        (vec![i64::MIN, i64::MIN, i64::MIN], i64::MIN),
        (vec![i64::MIN, 7, i64::MIN], 7),
        (vec![5, i64::MIN], 5),
        (vec![5, 0], 0),
        (vec![i64::MIN, 0, i64::MIN, 9, i64::MIN], 9),
        (vec![i64::MAX, i64::MIN], i64::MAX),
    ] {
        let bars: Vec<Bar> = ois
            .iter()
            .enumerate()
            .map(|(i, oi)| with(i64::try_from(i).unwrap(), *oi))
            .collect();
        let out = fold(&bars, five).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].open_interest, want, "{ois:?}");
        // And through the ladder: 1m -> 5m -> 15m keeps the same answer.
        let fifteen = Bucket::of_secs(900).unwrap();
        assert_eq!(
            fold_from_bars(&out, fifteen, five).unwrap()[0].open_interest,
            want
        );
    }
}

/// VOLUME OVERFLOW is refused at every rung of the ladder, including when it
/// only overflows at the coarser rung, and a sum that fits exactly is kept.
#[test]
fn volume_overflow_is_refused_at_whichever_rung_it_happens() {
    let open = midnight(day_of(Day::new(2025, 7, 1).unwrap())) + 555 * MINUTE_US;
    let v = |m: i64, volume: i64| Bar {
        volume,
        ..plain(open + m * MINUTE_US)
    };
    let half = i64::MAX / 2;
    // Two minutes in different 5m buckets, same 15m bucket.
    let bars = [v(0, half + 1), v(5, half + 1)];
    let five = Bucket::of_secs(300).unwrap();
    let fifteen = Bucket::of_secs(900).unwrap();
    let fives = fold(&bars, five).unwrap();
    assert_eq!(fives.len(), 2);
    assert!(matches!(
        fold_from_bars(&fives, fifteen, five),
        Err(FoldError::VolumeOverflow { at: 1, .. })
    ));
    assert!(matches!(
        fold(&bars, fifteen),
        Err(FoldError::VolumeOverflow { at: 1, .. })
    ));
    // Exactly i64::MAX fits.
    let fits = [v(0, half), v(1, half + 1)];
    assert_eq!(fold(&fits, fifteen).unwrap()[0].volume, i64::MAX);
    assert!(matches!(
        complete_minutes(&bars, fifteen),
        Err(FoldError::VolumeOverflow { .. })
    ));
}

/// UNSORTED INPUT is refused with the offending row, at every rung and by
/// completeness; it is never sorted.
#[test]
fn unsorted_input_is_refused_not_sorted() {
    let open = midnight(day_of(Day::new(2025, 7, 1).unwrap())) + 555 * MINUTE_US;
    let bars = [plain(open + MINUTE_US), plain(open)];
    for bucket in widths() {
        assert_eq!(
            fold(&bars, bucket),
            Err(FoldError::OutOfOrder {
                at: 1,
                previous: open + MINUTE_US,
                found: open
            })
        );
        assert!(matches!(
            complete_minutes(&bars, bucket),
            Err(FoldError::OutOfOrder { at: 1, .. })
        ));
    }
}

/// MONTH, YEAR AND LEAP-DAY BOUNDARIES: one day bar per IST day, stamped at
/// IST midnight of that civil date, and no intraday bucket straddles two days.
#[test]
fn day_bars_across_month_year_and_leap_boundaries_land_on_their_own_dates() {
    for (a, b) in [
        (
            Day::new(2025, 12, 31).unwrap(),
            Day::new(2026, 1, 1).unwrap(),
        ),
        (
            Day::new(2024, 2, 29).unwrap(),
            Day::new(2024, 3, 1).unwrap(),
        ),
        (
            Day::new(2020, 2, 28).unwrap(),
            Day::new(2020, 2, 29).unwrap(),
        ),
        (
            Day::new(2025, 6, 30).unwrap(),
            Day::new(2025, 7, 1).unwrap(),
        ),
    ] {
        let mut bars: Vec<Bar> = Vec::new();
        for d in [a, b] {
            bars.extend((555..930).map(|m| plain(midnight(day_of(d)) + m * MINUTE_US)));
        }
        let days = fold(&bars, Bucket::DAY).unwrap();
        assert_eq!(days.len(), 2);
        for (got, d) in days.iter().zip([a, b]) {
            let at = IstMoment::from_epoch_secs(got.ts_micros / 1_000_000).unwrap();
            assert_eq!(at.day(), d);
            assert_eq!(at.minute_of_day(), 0);
            assert_eq!(got.volume, 375);
        }
        for bucket in widths() {
            for out in fold(&bars, bucket).unwrap() {
                let first = IstMoment::from_epoch_secs(out.ts_micros / 1_000_000).unwrap();
                assert!(first.day() == a || first.day() == b);
            }
        }
    }
}

/// EXCEPTIONAL SESSIONS: the 2025 Muhurat (13:45-14:44 IST, 2025-10-21) folds
/// at every rung but is certified at none, and the refusal is a diagnostic
/// that names it.
#[test]
fn the_2025_muhurat_is_folded_but_never_certified() {
    let day = day_of(Day::new(2025, 10, 21).unwrap());
    let DayKind::Open(session) = pull::calendar::kind_of(day) else {
        panic!("the 2025 Muhurat is a measured session");
    };
    assert_ne!(session, CalendarSession::full());
    let bars: Vec<Bar> = (0..1_440_u16)
        .filter(|m| session.expects(*m))
        .map(|m| plain(midnight(day) + i64::from(m) * MINUTE_US))
        .collect();
    assert_eq!(bars.len(), 60);
    for bucket in widths() {
        assert_eq!(
            fold(&bars, bucket)
                .unwrap()
                .iter()
                .map(|b| b.volume)
                .sum::<i64>(),
            60
        );
        let (complete, diagnostics) = complete_minutes(&bars, bucket).unwrap();
        assert!(complete.is_empty(), "{}s", bucket.secs());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.contains("exceptional session")),
            "{}s: {diagnostics:?}",
            bucket.secs()
        );
    }
}

/// Minutes either side of IST midnight share a 60-minute bucket (23:15-00:15)
/// in the raw fold; completeness never certifies it.
#[test]
fn an_overnight_bucket_is_never_certified() {
    let day = day_of(Day::new(2025, 7, 1).unwrap());
    let bars = [
        plain(midnight(day) + 1_430 * MINUTE_US),
        plain(midnight(day + 1) + 5 * MINUTE_US),
    ];
    let hour = Bucket::of_secs(3_600).unwrap();
    assert_eq!(fold(&bars, hour).unwrap().len(), 1);
    let (complete, diagnostics) = complete_minutes(&bars, hour).unwrap();
    assert!(complete.is_empty());
    assert!(!diagnostics.is_empty());
}

/// THE DAILY BAR IS SERVED, NEVER DERIVED, AND NEVER COMPARED. `ingest`
/// derives seven rungs from minutes and excludes `DAY_1` (D-0077): nothing in
/// the pipeline folds a day from minutes and checks it against the vendor's
/// day bar, so a disagreement is neither refused nor reported.
#[test]
fn the_day_rung_is_not_derived_from_minutes() {
    assert_eq!(
        pull::ingest::derived_count(store::path::Timeframe::MINUTE_1),
        7,
        "2, 3, 5, 10, 15, 30, 60 — not the day"
    );
}

/// Day <-> epoch-day round trip over EVERY day the type can name, checked
/// against an independent `succ` walk.
#[test]
fn every_civil_day_round_trips_and_succ_agrees() {
    let mut day = Day::new(1970, 1, 1).unwrap();
    let mut n = 0_u32;
    loop {
        assert_eq!(day.days_from_epoch(), n);
        assert_eq!(Day::from_days(n).unwrap(), day);
        match day.succ() {
            Ok(next) => {
                assert!(next > day);
                day = next;
                n += 1;
            }
            Err(_) => break,
        }
    }
    assert_eq!(n, pull::session::MAX_DAY_NUMBER);
    assert!(Day::from_days(n + 1).is_err());
    assert!(Day::from_days(u32::MAX).is_err());
}

/// IST instants at the session edges and at i64 extremes: in-session is
/// [09:15, 15:30), and the extremes refuse rather than wrap.
#[test]
fn ist_session_edges_and_extremes() {
    let day = day_of(Day::new(2025, 7, 1).unwrap());
    let at = |m: i64, s: i64| IstMoment::from_epoch_secs(midnight(day) / 1_000_000 + m * 60 + s);
    assert!(!at(554, 59).unwrap().in_regular_session());
    assert!(at(555, 0).unwrap().in_regular_session());
    assert!(at(929, 59).unwrap().in_regular_session());
    assert!(!at(930, 0).unwrap().in_regular_session());
    assert_eq!(at(929, 59).unwrap().second_of_minute(), 59);
    for secs in [i64::MIN, i64::MIN + 1, -IST_OFFSET_SECS - 1, i64::MAX] {
        assert!(IstMoment::from_epoch_secs(secs).is_err(), "{secs}");
    }
    assert_eq!(
        IstMoment::from_epoch_secs(-IST_OFFSET_SECS).unwrap().day(),
        Day::new(1970, 1, 1).unwrap()
    );
    for d in [i64::MIN, -1, i64::MAX] {
        assert_eq!(pull::calendar::kind_of(d), DayKind::Unmeasured, "{d}");
    }
    assert_eq!(pull::calendar::sessions_between(day + 1, day), None);
}

fn percentile(sorted: &[f64], p: usize) -> f64 {
    sorted[(sorted.len() - 1) * p / 100]
}

/// PER-INPUT-BAR FOLD COST at 10^3..10^6 bars. Each sample is one whole fold
/// divided by its input length; samples are repeated so every size has at
/// least 30. Printed for the report, and asserted flat within a generous
/// factor so a scan introduced into the loop fails here.
#[test]
fn fold_cost_per_input_bar_is_flat() {
    let mut rng = SplitMix(0x0001_0000_0000);
    let mut p50s = Vec::new();
    for exp in 3..=6_u32 {
        let n = 10_i64.pow(exp);
        let day0 = day_of(Day::new(2020, 1, 1).unwrap());
        let input: Vec<Bar> = (0..n)
            .map(|i| {
                let day = day0 + i / 375;
                bar(midnight(day) + (555 + i % 375) * MINUTE_US, &mut rng)
            })
            .collect();
        let reps = (30_000_000 / n).clamp(30, 3_000);
        let mut samples = Vec::new();
        let fifteen = Bucket::of_secs(900).unwrap();
        for _ in 0..reps {
            let started = std::time::Instant::now();
            let out = fold(&input, fifteen).unwrap();
            let elapsed = started.elapsed();
            assert!(!out.is_empty());
            samples.push(elapsed.as_secs_f64() * 1e9 / f64::from(u32::try_from(n).unwrap()));
        }
        samples.sort_by(f64::total_cmp);
        let (p50, p99, max) = (
            percentile(&samples, 50),
            percentile(&samples, 99),
            samples[samples.len() - 1],
        );
        eprintln!(
            "fold 15m n=10^{exp}: ns/bar p50 {p50:.2} p99 {p99:.2} max {max:.2} ({reps} reps)"
        );
        p50s.push(p50);
    }
    let lo = p50s.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = p50s.iter().copied().fold(0.0, f64::max);
    assert!(hi / lo < 8.0, "per-bar p50 across 10^3..10^6: {p50s:?}");
}

/// Refusing widths that do not divide a day must not lose a rung: every
/// `Timeframe::KNOWN` width still builds a bucket, and every one divides a day.
#[test]
fn every_known_timeframe_still_builds_a_bucket() {
    for tf in store::path::Timeframe::KNOWN {
        let bucket = Bucket::of_secs(tf.secs());
        assert_eq!(bucket.map(Bucket::secs), Some(tf.secs()), "{}", tf.as_str());
        assert_eq!(86_400 % tf.secs(), 0, "{}", tf.as_str());
    }
    assert_eq!(store::path::Timeframe::KNOWN.len(), 10);
}

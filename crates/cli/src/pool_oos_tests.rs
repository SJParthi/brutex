#![cfg(test)]
//! `pool-oos` on synthetic bars: the split, the correction and the catalog
//! handoff. D-1576, D-1577.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixtures: a missing value must fail the test"
)]

use super::{Instrument, judge, write_catalog};
use crate::frontier::Direction;
use crate::pool::{Candidate, PreparedSpan};
use indicators::Candle;
use std::sync::atomic::{AtomicU64, Ordering};

/// `is_monday` and `is_tuesday` (docs/03-vocabulary.md). They depend on the
/// calendar alone, so prices can be planted around them without moving them.
const MONDAY: u32 = 365;
const TUESDAY: u32 = 366;

/// 2025-01-06, a Monday, as days since the epoch.
const FIRST_MONDAY: i64 = 20_094;

fn mask(bit: u32) -> [u64; 6] {
    let mut words = [0_u64; 6];
    words[usize::try_from(bit / 64).expect("word")] |= 1 << (bit % 64);
    words
}

/// One weekday session of 375 one-minute bars from 09:15 IST. `drift` is the
/// paisa per minute the session moves; zero is a small zigzag.
fn session(day: i64, base: i64, drift: i64) -> Vec<Candle> {
    let open_utc_micros = (day * 86_400 + 3 * 3_600 + 45 * 60) * 1_000_000;
    let mut price = base;
    (0..375_i64)
        .map(|minute| {
            let open = price;
            price = if drift == 0 {
                base + (minute % 2)
            } else {
                price + drift
            };
            Candle {
                ts_micros: open_utc_micros + minute * 60_000_000,
                open,
                high: open.max(price),
                low: open.min(price),
                close: price,
                volume: 1_000,
                open_interest: indicators::OI_NULL,
            }
        })
        .collect()
}

/// `weeks` weeks of weekday sessions from `first_monday`. Tuesdays always rise;
/// Mondays rise when `mondays_rise`, else fall; every other day zigzags. The
/// size of each move varies by week, so no series has zero variance.
fn span(first_monday: i64, weeks: i64, base: i64, step: i64, mondays_rise: bool) -> Vec<Candle> {
    let mut bars = Vec::new();
    for week in 0..weeks {
        let size = step * (2 + week % 3);
        for weekday in 0..5 {
            let drift = match weekday {
                0 if mondays_rise => size,
                0 => -size,
                1 => size,
                _ => 0,
            };
            bars.extend(session(first_monday + week * 7 + weekday, base, drift));
        }
    }
    bars
}

fn prepared(bars: Vec<Candle>) -> PreparedSpan {
    let mut evaluator = crate::evaluator_stored(indicators::vwap::Availability::Absent)
        .expect("the stored evaluator");
    let column = indicators::column::Column::build(&bars, &mut evaluator);
    PreparedSpan {
        bars,
        column,
        horizon: runner::outcome::Horizon::bars(15).expect("a positive horizon"),
    }
}

/// Whether the column row built from bar `bar` carries `bit`.
fn bit_at(column: &indicators::column::Column, bar: usize, bit: u32) -> bool {
    column
        .sources()
        .iter()
        .zip(column.bits())
        .filter(|(source, _)| **source == bar)
        .any(|(_, row)| row.get(bit))
}

fn union() -> Vec<Candidate> {
    vec![
        Candidate {
            words: mask(MONDAY),
            direction: Direction::Long,
        },
        Candidate {
            words: mask(TUESDAY),
            direction: Direction::Long,
        },
    ]
}

/// **The planted in-sample-only winner fails out of sample, and the
/// persistent one holds.** audit-20261003 gaps-5.
///
/// Two instruments, a ₹20,000 index-sized one and a ₹100 share-sized one.
/// On the twelve training weeks every Monday and every Tuesday rises all
/// session, so "long on Monday" and "long on Tuesday" are both in-sample
/// winners. On the twelve later weeks Mondays fall and Tuesdays still rise.
///
/// The judge must call Tuesday HELD and Monday FAILED. The control is the
/// same judge with no split -- the training weeks offered as the "later"
/// span -- which holds Monday too: that is what an in-sample pool reports,
/// and it is the reading this verb exists to refuse.
#[test]
fn a_planted_in_sample_only_winner_fails_out_of_sample_and_a_persistent_one_holds() {
    let later_monday = FIRST_MONDAY + 12 * 7;
    let index_training = prepared(span(FIRST_MONDAY, 12, 2_000_000, 200, true));
    let index_later = prepared(span(later_monday, 12, 2_000_000, 200, false));
    let share_training = prepared(span(FIRST_MONDAY, 12, 10_000, 1, true));
    let share_later = prepared(span(later_monday, 12, 10_000, 1, false));
    // The plant reached the column: Monday's bit is set on a Monday bar and
    // clear on a Tuesday one, so the candidates below fire where planted. The
    // evaluator's warm-up takes the first week, so the second week is read.
    let monday = bit_at(&index_training.column, 5 * 375 + 30, MONDAY);
    let tuesday = bit_at(&index_training.column, 6 * 375 + 30, MONDAY);
    assert_eq!(
        (monday, tuesday),
        (true, false),
        "the weekday bit is planted"
    );

    let union = union();
    let split = [
        Instrument {
            training: &index_training,
            later: &index_later,
        },
        Instrument {
            training: &share_training,
            later: &share_later,
        },
    ];
    let judged = judge(&split, &union).expect("the family is judged");
    let row = |candidate: usize| {
        judged
            .rows
            .iter()
            .find(|row| row.candidate == candidate)
            .copied()
            .expect("every candidate has a row")
    };
    let (monday, tuesday) = (row(0), row(1));
    assert_eq!(judged.training_sessions, 60);
    assert_eq!(judged.later_sessions, 60);
    assert!(monday.training.trades > 0 && monday.later.trades > 0);
    assert!(
        monday.training.sum_ppm > 0,
        "Monday wins in sample: {monday:?}"
    );
    assert!(
        monday.later.sum_ppm < 0,
        "Monday loses out of sample: {monday:?}"
    );
    assert!(
        !monday.held,
        "the in-sample-only winner FAILS out of sample"
    );
    assert!(tuesday.training.sum_ppm > 0 && tuesday.later.sum_ppm > 0);
    assert!(tuesday.held, "the persistent winner HOLDS: {tuesday:?}");
    assert_eq!(
        judged.rows.first().map(|row| row.candidate),
        Some(1),
        "held rows lead"
    );
    // Pooled by return, not by price: the share contributes as much per
    // Tuesday as the index does, so its trades are not drowned.
    let share_only = judge(
        &[Instrument {
            training: &share_training,
            later: &share_later,
        }],
        &union,
    )
    .expect("one instrument judged");
    let share_tuesday = share_only
        .rows
        .iter()
        .find(|row| row.candidate == 1)
        .copied()
        .expect("row");
    assert!(share_tuesday.later.sum_ppm * 3 > tuesday.later.sum_ppm);

    // THE CONTROL: no split. The same training weeks judged as if they were
    // later ones hold the planted Monday winner.
    let unsplit = [
        Instrument {
            training: &index_training,
            later: &index_training,
        },
        Instrument {
            training: &share_training,
            later: &share_training,
        },
    ];
    let in_sample = judge(&unsplit, &union).expect("the in-sample family is judged");
    assert!(
        in_sample
            .rows
            .iter()
            .filter(|row| row.candidate == 0)
            .all(|row| row.held),
        "with no split the planted Monday winner looks real"
    );
}

/// An empty family has no verdict; none is invented.
#[test]
fn an_empty_family_is_refused_not_judged() {
    let span = prepared(span(FIRST_MONDAY, 1, 10_000, 1, true));
    let one = [Instrument {
        training: &span,
        later: &span,
    }];
    assert!(judge(&one, &[]).is_err());
    assert!(judge(&[], &union()).is_err());
}

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-pool-oos-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("scratch");
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// **The held candidates are a catalog the qualification verbs read as it
/// stands.** audit-20261003 gaps-11.
///
/// Written by [`write_catalog`] and read by
/// `crate::boolean_catalog_command::catalog` -- the reader
/// `boolean-qualified-campaign-stored`, `boolean-oos-stored` and
/// `boolean-campaign-stored` call on their `CATALOG_FILE`. Each program must
/// be the AND of exactly its mask's bits: true on a bar carrying all of them,
/// false on one missing any. One mask held on both sides is one program. An
/// existing path is refused and left as it was.
#[test]
fn held_candidates_become_a_catalog_the_qualification_readers_read_back() {
    use vocab::ConditionMask;
    use vocab::expression::Truth;
    let scratch = Scratch::new();
    let path = scratch.0.join("held.catalog");
    let mut both = mask(MONDAY);
    both[5] |= mask(TUESDAY)[5];
    let held = [
        Candidate {
            words: both,
            direction: Direction::Long,
        },
        Candidate {
            words: both,
            direction: Direction::Short,
        },
        Candidate {
            words: mask(TUESDAY),
            direction: Direction::Short,
        },
    ];
    assert_eq!(
        write_catalog(&path, &held, "pool-oos test\nsecond line").expect("written"),
        2
    );
    let programs = crate::boolean_catalog_command::catalog(
        &path,
        crate::boolean_catalog_command::CATALOG_BYTES,
    )
    .expect("the qualification reader reads it");
    assert_eq!(programs.len(), 2);
    let known = ConditionMask::from_words([u64::MAX; 6]);
    let all = ConditionMask::from_words(both);
    let monday_only = ConditionMask::from_words(mask(MONDAY));
    let tuesday_only = ConditionMask::from_words(mask(TUESDAY));
    assert_eq!(programs[0].evaluate(all, known), Truth::True);
    assert_eq!(programs[0].evaluate(monday_only, known), Truth::False);
    assert_eq!(programs[0].evaluate(tuesday_only, known), Truth::False);
    assert_eq!(programs[1].evaluate(tuesday_only, known), Truth::True);
    assert_eq!(programs[1].evaluate(monday_only, known), Truth::False);
    let text = std::fs::read_to_string(&path).expect("text");
    assert_eq!(
        text, "# pool-oos test\n# second line\n365 & 366\n366\n",
        "comments, then one AND of decimal bit ids per line"
    );

    // NEVER OVERWRITTEN.
    assert!(write_catalog(&path, &held[2..], "other").is_err());
    assert_eq!(std::fs::read_to_string(&path).expect("text"), text);
    // NOTHING TO WRITE IS REFUSED, AND NO FILE APPEARS.
    let empty = scratch.0.join("empty.catalog");
    assert!(write_catalog(&empty, &[], "none").is_err());
    assert!(!empty.exists());
}

/// The arm refuses a later span that overlaps or precedes the training span,
/// and a malformed argument, before anything is read.
#[test]
fn the_pool_oos_arm_refuses_an_overlapping_split_and_bad_words_before_reading() {
    let run = |words: &[&str]| {
        let args: Vec<String> = words.iter().map(ToString::to_string).collect();
        let mut page = String::new();
        let status = crate::dispatch(&args, &mut page);
        (status, page)
    };
    let base = [
        "pool-oos",
        "zerodha",
        "5min",
        "2025",
        "1",
        "2025",
        "3",
        "auto",
        "2025",
        "4",
        "2025",
        "6",
        "/nonexistent/held.catalog",
    ];
    for (changes, why) in [
        (
            &[(9, "3")][..],
            "the later span starts in the training span's last month",
        ),
        (
            &[(8, "2024")][..],
            "the later span precedes the training span",
        ),
        (&[(11, "3")][..], "the later span runs backwards"),
        (&[(2, "7min")][..], "an unknown rung"),
        (&[(4, "13")][..], "a month off the calendar"),
        (&[(7, "nope")][..], "an unreadable support"),
    ] {
        let mut words = base;
        for (at, word) in changes {
            words[*at] = word;
        }
        let (status, page) = run(&words);
        assert_eq!(status, crate::MISUSED, "{why}: {page}");
        assert!(page.starts_with("refused: "), "{why}: {page}");
        assert!(
            !page.contains(crate::STORED_PROVENANCE),
            "{why}: nothing was read"
        );
    }
    let (status, page) = run(&base[..12]);
    assert_eq!(status, crate::MISUSED, "a missing CATALOG_OUT: {page}");
}

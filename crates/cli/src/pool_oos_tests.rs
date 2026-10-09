#![cfg(test)]
//! `pool-oos` on synthetic bars: the split, the correction and the catalog
//! handoff. D-1576, D-1577.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixtures: a missing value must fail the test"
)]

use super::{Instrument, judge, judge_walked, pooled_later, walk_span, write_catalog};
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
    let horizon = runner::outcome::Horizon::bars(15).expect("a positive horizon");
    PreparedSpan {
        rules: crate::Rules::derived(&bars, horizon),
        bars,
        column,
        horizon,
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

/// **The streamed judge holds no span and no training series, and loses no
/// trade.** Rust and O(1) sweep OS-1, OS-2, D-2300, AFG-01. Proved by this
/// test, `cli::pool_oos::the_streamed_judge_books_every_trade_and_keeps_no_training_series`.
///
/// A training walk keeps tallies only. A later walk books every trade it
/// tallies, so each pooled row sums to exactly its tally. The judge over
/// walked spans is the judge over prepared ones, row for row. A family too
/// large to hold is refused by name, not aborted.
#[test]
fn the_streamed_judge_books_every_trade_and_keeps_no_training_series() {
    let union = union();
    let training = prepared(span(FIRST_MONDAY, 12, 2_000_000, 200, true));
    let later = prepared(span(FIRST_MONDAY + 12 * 7, 12, 2_000_000, 200, false));
    let horizon = training.horizon;
    let walked_training = walk_span(&training, horizon, &union, false).expect("training walked");
    assert!(
        walked_training.bookings.is_empty(),
        "no training series is kept"
    );
    assert!(walked_training.tallies.iter().all(|t| t.trades > 0));
    let walked_later = walk_span(&later, horizon, &union, true).expect("later walked");
    let booked: u64 = walked_later.bookings.len().try_into().expect("count");
    let tallied: u64 = walked_later.tallies.iter().map(|t| t.trades).sum();
    assert_eq!(booked, tallied, "every tallied trade is booked once");
    let series = pooled_later(&[&walked_later], union.len(), &walked_later.days).expect("series");
    for (row, tally) in series.iter().zip(&walked_later.tallies) {
        let total: i128 = row.iter().map(|v| i128::from(*v)).sum();
        assert_eq!(total, tally.sum_ppm, "the pooled row sums to its tally");
    }
    let streamed = judge_walked(&[(walked_training, walked_later)], &union).expect("streamed");
    let held = judge(
        &[Instrument {
            training: &training,
            later: &later,
        }],
        &union,
    )
    .expect("held");
    assert_eq!(streamed.rows, held.rows);
    assert_eq!(
        (streamed.training_sessions, streamed.later_sessions),
        (held.training_sessions, held.later_sessions)
    );
    let why = pooled_later(&[], usize::MAX / 2, &[1, 2]).expect_err("cannot be held");
    assert!(why.contains("cannot be held"), "{why}");
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
    // G18-cli-a-11, D-2002: each refusal is the ARM's own, named. The work
    // refuses most of these too, so `MISUSED` alone could not show that the
    // arm's guard or rung match had let a bad split through.
    let order = "the training months must be ordered";
    let rung_refusal = "is not a rung this engine sweeps";
    for (changes, why, named) in [
        (
            &[(9, "3")][..],
            "the later span starts in the training span's last month",
            order,
        ),
        (
            &[(8, "2024")][..],
            "the later span precedes the training span",
            order,
        ),
        (&[(11, "3")][..], "the later span runs backwards", order),
        (
            &[(3, "2026")][..],
            "the training span runs backwards",
            order,
        ),
        (&[(2, "7min")][..], "an unknown rung", rung_refusal),
        (
            &[(4, "13")][..],
            "a month off the calendar",
            "MONTH must be 1..=12",
        ),
        (&[(7, "nope")][..], "an unreadable support", "refused: "),
    ] {
        let mut words = base;
        for (at, word) in changes {
            words[*at] = word;
        }
        let (status, page) = run(&words);
        assert_eq!(status, crate::MISUSED, "{why}: {page}");
        assert!(page.starts_with("refused: "), "{why}: {page}");
        assert!(page.contains(named), "{why}: {page}");
        assert!(
            !page.contains(crate::STORED_PROVENANCE),
            "{why}: nothing was read"
        );
    }
    let (status, page) = run(&base[..12]);
    assert_eq!(status, crate::MISUSED, "a missing CATALOG_OUT: {page}");
    // An ordered split -- including a one-month training span and a one-month
    // later span starting the month after it -- passes the arm, whatever the
    // work then answers.
    for (changes, why) in [
        (&[][..], "the base split"),
        (
            &[(6, "1"), (9, "2"), (11, "2")][..],
            "one training month, then one later month",
        ),
    ] {
        let mut words = base;
        for (at, word) in changes {
            words[*at] = word;
        }
        let (_, page) = run(&words);
        assert!(!page.contains(order), "{why}: {page}");
        assert!(!page.contains(rung_refusal), "{why}: {page}");
    }
}

/// **The verb's own split check: strict after, single months admitted.**
/// G18-cli-b-05, D-2020.
///
/// `dispatch` parses the words, so the split is checked again by `run` for
/// every caller. Each boundary is held on both sides: a later span starting IN
/// the training span's last month is refused and one starting the month after
/// is not; a one-month span (first == last) is admitted on either side and a
/// backwards one refused. An admitted split goes on to the next refusal (an
/// unknown feed here), so nothing is read either way.
#[test]
fn the_verb_refuses_exactly_the_misordered_splits() {
    let catalog = std::path::Path::new("/nonexistent-brutex-pool-oos/held.catalog");
    let ordered = "the later months must be ordered";
    for (training, later, refused, why) in [
        (
            ((2025, 1), (2025, 3)),
            ((2025, 4), (2025, 6)),
            false,
            "after",
        ),
        (
            ((2025, 1), (2025, 3)),
            ((2025, 3), (2025, 6)),
            true,
            "overlaps",
        ),
        (
            ((2025, 1), (2025, 3)),
            ((2024, 4), (2025, 6)),
            true,
            "precedes",
        ),
        (
            ((2025, 1), (2025, 3)),
            ((2025, 4), (2025, 4)),
            false,
            "one later month",
        ),
        (
            ((2025, 1), (2025, 3)),
            ((2025, 6), (2025, 4)),
            true,
            "later backwards",
        ),
        (
            ((2025, 3), (2025, 3)),
            ((2025, 4), (2025, 6)),
            false,
            "one training month",
        ),
        (
            ((2025, 3), (2025, 1)),
            ((2025, 4), (2025, 6)),
            true,
            "training backwards",
        ),
    ] {
        let why_not = super::run("no-such-feed", "5min", training, later, None, catalog)
            .expect_err("every case is refused somewhere");
        assert_eq!(why_not.contains(ordered), refused, "{why}: {why_not}");
        let page = super::pool_oos("no-such-feed", "5min", training, later, None, catalog);
        assert_eq!(page, format!("refused: {why_not}\n"), "{why}");
    }
}

/// **An empty store is reported as a page, not refused, and an existing
/// `CATALOG_OUT` is refused before it.** G18-cli-b-06, D-2020.
#[test]
fn an_empty_store_is_a_page_and_an_existing_catalog_out_is_refused_first() {
    let scratch = Scratch::new();
    let vendor = crate::parse_vendor("zerodha").expect("a feed");
    let catalog = scratch.0.join("held.catalog");
    let page = super::run_under(
        &scratch.0,
        vendor,
        "zerodha",
        "5min",
        (((2025, 1), (2025, 3)), ((2025, 4), (2025, 6))),
        None,
        &catalog,
    )
    .expect("an empty surface is a page");
    assert!(
        page.contains(
            "OUT OF SAMPLE. Discovery reads 2025-01..2025-03 only. The union is then judged on \
             2025-04..2025-06"
        ),
        "{page}"
    );
    assert!(page.contains("0 instruments on the surface"), "{page}");
    assert!(!catalog.exists(), "no catalog for nothing judged");
    std::fs::write(&catalog, b"kept").expect("an existing catalog");
    let why = super::run_under(
        &scratch.0,
        vendor,
        "zerodha",
        "5min",
        (((2025, 1), (2025, 3)), ((2025, 4), (2025, 6))),
        None,
        &catalog,
    )
    .expect_err("an existing catalog is refused");
    assert!(why.contains("already exists"), "{why}");
    assert_eq!(std::fs::read(&catalog).expect("kept"), b"kept");
}

/// **Every surface instrument gets one entry, and a span that cannot be read
/// is that instrument's named refusal.** G18-cli-b-07, D-2020.
#[test]
fn walk_all_answers_once_per_instrument_and_names_an_unreadable_span() {
    let scratch = Scratch::new();
    let vendor = crate::parse_vendor("zerodha").expect("a feed");
    let surface = ["NSE-NIFTY".to_owned(), "NSE-BANKNIFTY".to_owned()];
    let walked = super::walk_all(
        &scratch.0,
        vendor,
        "5min",
        &surface,
        (((2025, 1), (2025, 3)), ((2025, 4), (2025, 6))),
        &union(),
    );
    assert_eq!(walked.len(), surface.len());
    for entry in walked {
        let why = entry.expect_err("nothing is stored");
        assert!(why.starts_with("training span: "), "{why}");
    }
}

fn judgement(candidate: usize, held: bool) -> super::Judgement {
    super::Judgement {
        candidate,
        training: super::Tally {
            trades: 3,
            sum_ppm: 30,
        },
        later: super::Tally {
            trades: 2,
            sum_ppm: 20,
        },
        held,
    }
}

fn judged(rows: Vec<super::Judgement>) -> super::Judged {
    super::Judged {
        rows,
        training_sessions: 10,
        later_sessions: 5,
        draws: 7,
        reality: None,
    }
}

/// **A held row becomes a written catalog and the page says where; no held
/// row writes nothing and says so.** G18-cli-b-08, D-2020.
#[test]
fn hand_off_writes_the_held_rows_and_says_where() {
    let scratch = Scratch::new();
    let union = union();
    let path = scratch.0.join("held.catalog");
    let mut out = String::new();
    super::hand_off(
        &mut out,
        &union,
        &judged(vec![judgement(1, true), judgement(0, false)]),
        &path,
        "heading",
        (2025, 6),
    );
    assert!(
        out.contains(&format!(
            "CATALOG WRITTEN: 1 program(s) at {}. It is the CATALOG_FILE",
            path.display()
        )),
        "{out}"
    );
    assert!(out.contains("qualify it on months after 2025-06"), "{out}");
    assert_eq!(
        std::fs::read_to_string(&path).expect("written"),
        "# heading\n366\n"
    );
    let mut refused = String::new();
    super::hand_off(
        &mut refused,
        &union,
        &judged(vec![judgement(1, true)]),
        &path,
        "heading",
        (2025, 6),
    );
    assert!(
        refused.contains("refused: the held catalog was not written"),
        "{refused}"
    );
    let none = scratch.0.join("none.catalog");
    let mut nothing = String::new();
    super::hand_off(
        &mut nothing,
        &union,
        &judged(vec![judgement(0, false)]),
        &none,
        "heading",
        (2025, 6),
    );
    assert!(nothing.contains("NO CATALOG WRITTEN"), "{nothing}");
    assert!(!none.exists());
}

/// **The table shows every held row and the operator's TOP, and counts the
/// rest exactly when there is a rest.** G18-cli-b-09, D-2020.
#[test]
fn render_counts_the_hidden_failed_rows_only_when_some_are_hidden() {
    let union = union();
    let top = crate::Rules::operator().top.max(1);
    let mut out = String::new();
    let shown_all: Vec<_> = (0..top).map(|_| judgement(0, false)).collect();
    super::render(&mut out, &union, &judged(shown_all), 2);
    assert!(
        out.contains(
            "OUT OF SAMPLE -- 2 candidate(s) pooled across 2 instrument(s); 10 training and 5 \
             later IST session(s); 7 bootstrap draws."
        ),
        "{out}"
    );
    assert!(out.contains("0 HELD under Romano-Wolf"), "{out}");
    assert!(out.contains("White's Reality Check: NOT COMPUTED"), "{out}");
    assert!(!out.contains("more FAILED"), "nothing hidden: {out}");
    assert_eq!(out.matches("FAILED").count(), top, "{out}");
    let mut hidden = String::new();
    let with_rest: Vec<_> = (0..top + 3).map(|_| judgement(0, false)).collect();
    super::render(&mut hidden, &union, &judged(with_rest), 2);
    assert!(
        hidden.contains("  ... and 3 more FAILED candidate(s), in discovery order, not shown."),
        "{hidden}"
    );
    let mut one_hidden = String::new();
    let one_more: Vec<_> = (0..=top).map(|_| judgement(0, false)).collect();
    super::render(&mut one_hidden, &union, &judged(one_more), 2);
    assert!(one_hidden.contains("... and 1 more FAILED"), "{one_hidden}");
}

/// **A catalog of exactly the readers' byte bound is written; one byte over
/// is refused by name before any file appears.** G18-cli-b-10, D-2020.
///
/// The text is `# <heading>\n366\n`, seven bytes beside the heading.
#[test]
fn a_catalog_at_the_byte_bound_is_written_and_one_byte_over_is_refused() {
    let scratch = Scratch::new();
    let bound = usize::try_from(crate::boolean_catalog_command::CATALOG_BYTES).expect("fits");
    let held = [Candidate {
        words: mask(TUESDAY),
        direction: Direction::Long,
    }];
    let at = scratch.0.join("at.catalog");
    assert_eq!(
        write_catalog(&at, &held, &"h".repeat(bound - 7)).expect("at the bound"),
        1
    );
    assert_eq!(
        std::fs::metadata(&at).expect("written").len(),
        crate::boolean_catalog_command::CATALOG_BYTES
    );
    let over = scratch.0.join("over.catalog");
    let why = write_catalog(&over, &held, &"h".repeat(bound - 6)).expect_err("one byte over");
    assert!(
        why.contains(&format!(
            "the held catalog is {} bytes, over the",
            bound + 1
        )),
        "{why}"
    );
    assert!(!over.exists(), "nothing was written");
}

/// `walk_span` as it stood before D-4705: [`runner::trade::walk`] once per
/// candidate, so the slice facts were rebuilt for every one. The reference
/// the hoisted walk must equal.
fn walk_span_per_candidate_facts(
    span: &PreparedSpan,
    horizon: runner::outcome::Horizon,
    union: &[Candidate],
    book: bool,
) -> super::Walked {
    let bars = span.bars.as_slice();
    let mut out = super::Walked {
        days: crate::session_index(bars),
        tallies: Vec::new(),
        bookings: Vec::new(),
    };
    for (at, candidate) in union.iter().enumerate() {
        let mask = vocab::ConditionMask::from_words(candidate.words);
        let walked = runner::trade::walk(bars, &span.column, &mask, horizon, candidate.direction);
        let mut tally = super::Tally::default();
        for trade in &walked.trades {
            let (entry, exit) = (&bars[trade.entry_bar], &bars[trade.exit_bar]);
            let ppm = i64::try_from(i128::from(trade.worst) * 1_000_000 / i128::from(entry.open))
                .expect("fits");
            tally.trades += 1;
            tally.sum_ppm += i128::from(ppm);
            if book {
                out.bookings
                    .push((at, indicators::ist_day(exit.ts_micros), ppm));
            }
        }
        out.tallies.push(tally);
    }
    out
}

/// **`walk_span` over a span equals the per-candidate walk it replaced, byte
/// for byte: days, every tally and every booking, on both sides, booked and
/// not, at two horizons.** G2-4, D-4705.
#[test]
fn the_hoisted_walk_equals_the_per_candidate_walk() {
    let spans = [
        prepared(span(FIRST_MONDAY, 6, 2_000_000, 200, true)),
        prepared(span(FIRST_MONDAY, 6, 10_000, 1, false)),
    ];
    let mut family = Vec::new();
    for words in [mask(MONDAY), mask(TUESDAY), [0; 6], {
        let mut both = mask(MONDAY);
        both[usize::try_from(TUESDAY / 64).expect("word")] |= 1 << (TUESDAY % 64);
        both
    }] {
        for direction in [Direction::Long, Direction::Short] {
            family.push(Candidate { words, direction });
        }
    }
    let mut fired = 0;
    for span in &spans {
        for horizon in [
            span.horizon,
            runner::outcome::Horizon::bars(40).expect("40"),
        ] {
            for book in [true, false] {
                let hoisted = walk_span(span, horizon, &family, book).expect("walked");
                let reference = walk_span_per_candidate_facts(span, horizon, &family, book);
                assert_eq!(hoisted.days, reference.days);
                assert_eq!(hoisted.tallies, reference.tallies, "{horizon:?} {book}");
                assert_eq!(hoisted.bookings, reference.bookings, "{horizon:?} {book}");
                fired += hoisted.tallies.iter().filter(|t| t.trades > 0).count();
            }
        }
    }
    assert!(
        fired > 0,
        "premise: candidates fire, so the comparison reads trades"
    );
}

/// **`walk_span` builds the span's slice facts ONCE, not once per union
/// candidate.** G2-4, the sixth site of AC-whp-o1-2's defect, D-4705.
#[test]
fn walk_span_builds_its_slice_facts_once() {
    let source = include_str!("pool_oos.rs");
    let from = source
        .find("\npub(crate) fn walk_span(")
        .expect("walk_span");
    let body = source
        .get(from..)
        .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to)))
        .expect("its body");
    assert!(!body.contains(concat!("trade::walk", "(")), "{body}");
    assert_eq!(
        body.matches(concat!("SliceFacts", "::of(")).count(),
        1,
        "{body}"
    );
    assert!(body.contains(concat!("trade::walk_over", "(")), "{body}");
    let facts = body.find(concat!("SliceFacts", "::of(")).expect("facts");
    let lanes = body.find(".par_iter()").expect("the candidate lanes");
    assert!(
        facts < lanes,
        "the facts are built before the candidate loop"
    );
}

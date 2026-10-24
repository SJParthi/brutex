//! IA-4: the options reader's per-row refusal reaches the import (D-4816),
//! the trailing-empty header is counted (D-4829), and bar definition 5's
//! per-kind currency (D-4877), every cell pinned. Every row and price here
//! is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;

use super::*;

/// Design §8 F44 (round 4, BL-E): an options day whose files refuse rows one
/// by one under definition 5 is filed, closes `done`, and its D-3335 line
/// counts the refused rows (`rows_refused=1`, never a silent 0) beside its
/// `definition=5`; a file read under a header with empty trailing fields is
/// `trailing_empty_files=1`. Before IA-4 the bad row refused its whole file,
/// the contract-day was lost, and the line said 0.
#[test]
fn an_options_days_refused_rows_reach_the_day_ledger() {
    let on = day(2019, 5, 30);
    let root = scratch("ia4-f44");
    let zero = ("0", "0.00");
    let bent = String::from_utf8(nfo_csv(
        "NIFTY30MAY1911900CE",
        on,
        zero,
        &[
            (hms_at(9, 15, 30), "50.00", 1, 9_000),
            (hms_at(9, 16, 0), "51.00", 75, 9_075),
            (hms_at(9, 16, 5), "52.00", 75, 9_150),
        ],
    ))
    .unwrap()
    // 2018-11-06's shape (design §8 F38): a letter in one LTP.
    .replacen(",51.00,", ",51.0x,", 1);
    let padded = String::from_utf8(nfo_csv(
        "SBIN30MAY19340CE",
        on,
        zero,
        &[
            (hms_at(9, 16, 0), "3.00", 1, 3_000),
            (hms_at(9, 30, 0), "3.10", 3_000, 6_000),
        ],
    ))
    .unwrap()
    // 2025-03-11's shape (design §8 F36, D-4829): k = 4 empty fields.
    .replace("\r\n", ",,,,\r\n");
    let files = vec![
        (
            entry_of(Tree::Options, on, "NIFTY30MAY1911900CE"),
            bent.into_bytes(),
        ),
        (
            entry_of(Tree::Options, on, "SBIN30MAY19340CE"),
            padded.into_bytes(),
        ),
    ];
    world_of(&root, &[(on, Tree::Options, files)]);
    let store = root.join("S");
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Options),
        (on, on),
        (&store, &[]),
        false,
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.files, 2, "both files filed, neither refused whole");
    let lines = flags_lines(&store);
    assert_eq!(lines.len(), 1, "{lines:?}");
    for field in [
        format!(" definition={BAR_DEFINITION} "),
        " rows_refused=1 ".to_owned(),
        " files=2 ".to_owned(),
        " trailing_empty_files=1 ".to_owned(),
    ] {
        assert!(lines[0].contains(&field), "{field}: {}", lines[0]);
    }
    assert_eq!(BAR_DEFINITION, 5);
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        journal
            .lines()
            .any(|line| line.starts_with("done options 2019-05-30 * ")
                && line.contains(" definition=5 ")),
        "{journal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// IA-GR-16 for IA-4's variant: an options file past the bad-row bound is
/// the capital-market reader's cause, code for code: the vendor's own bytes,
/// judged the same way every time, so it closes.
#[test]
fn too_many_bad_option_rows_have_the_capital_markets_cause() {
    let options = nfo_cause(&crate::gdfl_nfo::NfoRefusal::TooManyBadRows {
        refused: 65,
        rows: 100,
    });
    let cash = cm_cause(&crate::gdfl_cm::CmRefusal::TooManyBadRows {
        refused: 65,
        rows: 100,
    });
    assert_eq!(options, cash);
    assert_eq!(
        (options.stage, options.code, options.closes),
        (Stage::Vendor, "too_many_bad_rows", true)
    );
    // A row fault named alone (no row kept) keeps its pinned cause.
    assert_eq!(
        nfo_cause(&crate::gdfl_nfo::NfoRefusal::PriceRefused { line: 2 }).code,
        "price_refused"
    );
}

/// D-4877, bar definition 5: every (kind, closing-auction era, definition)
/// cell, asked through `definition_is_current` and through the journal's own
/// reader (`done` under the definition: current, else restated). 5 is
/// current for every kind; an OPTIONS day only under 5 (its days under 2, 3
/// or 4 restate: 4 left option bars as 3 built them); a share's day from
/// 2026-08-03 under 4 or 5; an index day, and a share's before 2026-08-03,
/// under 2 to 5. 0, 1 and any number past 5 are never current. Before IA-4,
/// an options day under 3 or 4 read as current.
#[test]
fn every_kind_era_and_definition_has_its_currency() {
    use ImportKind::{Indices, Options, Stocks};
    let (before, era) = (day(2026, 7, 31), day(2026, 8, 3));
    // (kind, day, the definitions that are current); every other in 0..=7
    // and u32::MAX is not.
    let cells: [(ImportKind, Day, &[u32]); 6] = [
        (Options, before, &[5]),
        (Options, era, &[5]),
        (Stocks, before, &[2, 3, 4, 5]),
        (Stocks, era, &[4, 5]),
        (Indices, before, &[2, 3, 4, 5]),
        (Indices, era, &[2, 3, 4, 5]),
    ];
    let root = scratch("ia4-currency");
    let mut text = String::new();
    let mut want = Vec::new();
    for (kind, on, current) in cells {
        for definition in (0..=7).chain([u32::MAX]) {
            let is = current.contains(&definition);
            assert_eq!(
                definition_is_current(kind, on, u64::from(definition)),
                is,
                "{} {on} under {definition}",
                kind.as_str()
            );
            let key = format!("{} {on} only{definition}", kind.as_str());
            let _ = writeln!(text, "begin {key}");
            let _ = writeln!(
                text,
                "done {key} definition={definition} files=1 seconds=1 failures=0"
            );
            want.push((key, is));
        }
    }
    assert!(!definition_is_current(
        Options,
        before,
        u64::from(u32::MAX) + 1
    ));
    std::fs::create_dir_all(root.join("imports")).unwrap();
    std::fs::write(journal_path(&root), text).unwrap();
    let journal = Journal::load(journal_path(&root)).unwrap();
    for (key, is) in want {
        assert_eq!(
            (journal.done.contains(&key), journal.restated.contains(&key)),
            (is, !is),
            "{key}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// The day ledger's options line counts a file read under a header with
/// empty trailing fields once, whatever its k, and the refused rows of every
/// file; a share's line carries neither field it does not count.
#[test]
fn the_options_line_counts_trailing_empty_files_and_refused_rows() {
    let session = crate::gdfl_cm::session_of(d1()).unwrap();
    let mut ledger = quality::DayLedger::new(ImportKind::Options, d1(), session);
    for (trailing, refused) in [(4, 1), (0, 2), (8, 0), (0, 0)] {
        let file = quality::FileQuality {
            reader: quality::ReaderCounts {
                trailing_empty: trailing,
                ..option_reader_counts(1, refused)
            },
            ..quality::FileQuality::default()
        };
        ledger.take("*", &file);
    }
    let lines = ledger.lines(true);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let fields = &lines[0].1;
    assert!(fields.contains(" rows_refused=3 "), "{fields}");
    assert!(
        fields.contains(" trailing_empty_files=2 max_defer_s="),
        "{fields}"
    );
    let mut shares = quality::DayLedger::new(ImportKind::Stocks, d1(), session);
    shares.take("SBIN", &quality::FileQuality::default());
    assert!(!shares.lines(true)[0].1.contains("trailing_empty_files"));
}

/// RA-308 / IA-GR-06d, the half that holds: an options day a run of
/// definition 4 began and never closed (a crash) is read again WHOLE under
/// 5, every file through the per-row reader, and closes `done` under 5,
/// named among the resumed days. Nothing of the old rule is kept as a mix:
/// the store answers each file's bars. (Naming "resumed under a new
/// definition" in the journal and on a page is NOT DONE: L-IA4-04.)
#[test]
fn a_day_begun_under_definition_4_is_read_whole_under_5() {
    let on = day(2019, 5, 30);
    let root = scratch("ia4-resume");
    let zero = ("0", "0.00");
    let files = vec![
        (
            entry_of(Tree::Options, on, "NIFTY30MAY1911900CE"),
            String::from_utf8(nfo_csv(
                "NIFTY30MAY1911900CE",
                on,
                zero,
                &[
                    (hms_at(9, 15, 30), "50.00", 1, 9_000),
                    (hms_at(9, 16, 0), "51.00", 75, 9_075),
                ],
            ))
            .unwrap()
            .replacen(",51.00,", ",51.0x,", 1)
            .into_bytes(),
        ),
        (
            entry_of(Tree::Options, on, "SBIN30MAY19340CE"),
            nfo_csv(
                "SBIN30MAY19340CE",
                on,
                zero,
                &[(hms_at(9, 16, 0), "3.00", 1, 3_000)],
            ),
        ),
    ];
    world_of(&root, &[(on, Tree::Options, files)]);
    let store = root.join("S");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    std::fs::write(
        journal_path(&store),
        "run 1790000000 source=unrecorded commit=unrecorded started=1790000000 definition=4\n\
         begin options 2019-05-30 *\n",
    )
    .unwrap();
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Options),
        (on, on),
        (&store, &[]),
        false,
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.files, 2, "every file of the day read again");
    assert_eq!(got.resumed, [on]);
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    let last = journal
        .lines()
        .rfind(|line| line.contains(" options 2019-05-30 * "))
        .unwrap();
    assert!(
        last.starts_with("done ") && last.contains(" definition=5 "),
        "{journal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

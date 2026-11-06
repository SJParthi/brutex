#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

use super::*;
use crate::catalogue::{all, hand};

/// A new, empty folder of this test process (never removed: nothing here deletes).
fn fresh(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!("psrb-ref-{tag}-{}-{nanos}", std::process::id()))
}

#[test]
fn the_repository_golden_files_are_the_catalogue() {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/psrb-preview-20261106/golden");
    let scenarios = all();
    let statuses = write_all(&dir, &scenarios).unwrap();
    assert_eq!(statuses.len(), 200);
    let differs: Vec<&String> = statuses
        .iter()
        .filter(|(_, s)| *s == Status::Differs)
        .map(|(n, _)| n)
        .collect();
    assert!(
        differs.is_empty(),
        "golden files differ from the catalogue: {differs:?}"
    );
}

#[test]
fn a_golden_file_is_written_once_compared_after_and_never_overwritten() {
    let dir = fresh("write");
    let some: Vec<Scenario> = hand().into_iter().take(2).collect();
    let first = write_all(&dir, &some).unwrap();
    assert!(first.iter().all(|(_, s)| *s == Status::Written));
    let again = write_all(&dir, &some).unwrap();
    assert!(again.iter().all(|(_, s)| *s == Status::Same));
    let path = dir.join(format!("{}.txt.md", some[0].name));
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut f, b"tampered\n").unwrap();
    let third = write_all(&dir, &some).unwrap();
    assert_eq!(third[0].1, Status::Differs);
    assert_eq!(third[1].1, Status::Same);
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .ends_with("tampered\n")
    );
}

#[test]
fn a_file_that_cannot_be_created_is_an_error() {
    let dir = fresh("error");
    let mut s = hand().into_iter().next().unwrap();
    s.name = "missing-folder/inside".to_owned();
    assert!(write_all(&dir, &[s]).is_err());
    let file = fresh("file");
    std::io::Write::write_all(
        &mut OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
            .unwrap(),
        b"x",
    )
    .unwrap();
    assert!(write_all(&file, &[]).is_err());
    // oob: the golden path exists as a folder: it cannot be read back for comparing
    let dir = fresh("folder");
    let s = hand().into_iter().next().unwrap();
    std::fs::create_dir_all(dir.join(format!("{}.txt.md", s.name))).unwrap();
    assert!(write_all(&dir, &[s]).is_err());
}

#[test]
fn every_word_of_the_grammar_renders() {
    assert_eq!(kind_word(DayKind::Full), "full");
    assert_eq!(kind_word(DayKind::Short), "short");
    assert_eq!(kind_word(DayKind::Muhurat), "muhurat");
    assert_eq!(kind_word(DayKind::Closed), "closed");
    assert_eq!(clock_word(Clock::Dated), "dated");
    assert_eq!(clock_word(Clock::Today), "today");
    assert_eq!(per_text(Per::Permille(-250)), "-250");
    assert_eq!(per_text(Per::Base(-7)), "base:-7");
    // a run without rows has no comparison and no floor
    let mut head = String::new();
    head_lines(&mut head, &[]);
    assert_eq!(head, "nearest-met-floor none\n");
    // a candidate with no trade has no fee line
    let unfilled = Record {
        candidate: crate::book::Candidate {
            day: crate::generator::day(2018, 9, 4),
            t: crate::time::OPEN + 300,
            symbol: "S01".to_owned(),
            side: crate::cond::Side::Long,
            outcome: Outcome::NotFilled,
        },
        trade: None,
    };
    assert_eq!(fees_line(0, &unfilled), None);
    assert_eq!(
        candidate_line(0, &unfilled),
        "c 0 2018-09-04 09:20:00 S01 LONG not-filled"
    );
}

#[test]
fn a_rendered_file_carries_the_format_the_token_and_both_sections() {
    let s = hand().into_iter().next().unwrap();
    let text = render(&s);
    assert!(text.starts_with("# Golden scenario `s001-"));
    assert!(text.contains(FORMAT) && text.contains(TOKEN));
    assert!(text.contains("## Inputs") && text.contains("## Expected outputs"));
    assert_eq!(text.matches("```").count(), 4);
    assert!(text.contains("\nm 09:15 100000 100000 100000 100000 1000 x375\n"));
    assert!(text.contains("\nrank 1 "));
    // the head: 6 timeframes x 3 accounts x 3 pairs, then the nearest met floor
    assert_eq!(text.matches("\nhead ").count(), 54);
    assert!(text.contains("\nhead 1 LONG AV2/VO trades="));
    assert!(text.contains("\nhead 60 BOTH VS/AV2 trades="));
    assert_eq!(text.matches("\nnearest-met-floor ").count(), 1);
}

#[test]
fn runs_compress_identical_neighbours_and_keep_the_rest() {
    let b = Bar::flat(5, 1);
    let c = Bar::flat(6, 1);
    let mut out = String::new();
    runs(
        &mut out,
        "m",
        &[(33_300, b), (33_360, b), (33_420, c), (33_540, c)],
        Some(60),
        hm_or_hms,
    );
    assert_eq!(
        out,
        "m 09:15 5 5 5 5 1 x2\nm 09:17 6 6 6 6 1\nm 09:19 6 6 6 6 1\n"
    );
    let mut s = String::new();
    runs(
        &mut s,
        "s",
        &[(33_301, b), (33_361, b), (33_421, b), (33_422, b)],
        None,
        hms,
    );
    assert_eq!(
        s,
        "s 09:15:01 5 5 5 5 1 x3 every 60\ns 09:17:02 5 5 5 5 1\n"
    );
    assert_eq!(hm_or_hms(33_330), "09:15:30");
}

#[test]
fn stopped_runs_render_by_name() {
    let charge = RunError::Charge {
        symbol: "S01".to_owned(),
        day: crate::generator::day(2018, 8, 7),
        error: crate::costs::ChargeError::OpenIntradayPosition,
    };
    assert_eq!(
        render_error(&charge),
        "run-stopped charge-refusal S01 2018-08-07 OpenIntradayPosition\n"
    );
    let dup = RunError::Duplicate(crate::book::DuplicateKey {
        day: crate::generator::day(2018, 9, 4),
        t: 35_160,
        symbol: "S01".to_owned(),
    });
    assert_eq!(
        render_error(&dup),
        "run-stopped duplicate-candidate S01 2018-09-04 09:46:00\n"
    );
}

#[test]
fn inputs_name_every_override_answer_and_flag() {
    let s = hand()
        .into_iter()
        .find(|s| s.name.starts_with("s038"))
        .unwrap();
    let text = render_inputs(&s.market);
    assert!(text.contains("calendar-override 2018-09-13 closed"));
    assert!(text.contains("calendar-override 2018-09-14 muhurat"));
    assert!(text.contains("calendar-override 2018-09-17 short"));
    let mut m = s.market.clone();
    m.membership.answers.insert(
        ("S01".to_owned(), crate::generator::day(2018, 9, 4)),
        Answer::Unknown,
    );
    m.membership.answers.insert(
        ("S01".to_owned(), crate::generator::day(2018, 9, 5)),
        Answer::No,
    );
    m.membership.answers.insert(
        ("S01".to_owned(), crate::generator::day(2018, 9, 6)),
        Answer::Yes,
    );
    m.membership.carry_from = Some(crate::generator::day(2018, 9, 6));
    m.membership.probe.opened = false;
    m.stocks[0]
        .withheld
        .insert(crate::generator::day(2018, 9, 4));
    m.stocks[0].gdfl_refused.insert(Month::of(2018, 9));
    let text = render_inputs(&m);
    assert!(text.contains("membership-answer S01 2018-09-04 unknown"));
    assert!(text.contains("membership-answer S01 2018-09-05 no"));
    assert!(text.contains("membership-answer S01 2018-09-06 yes"));
    assert!(text.contains("membership-carry-from 2018-09-06"));
    assert!(text.contains("membership-probe opened=no"));
    assert!(text.contains("withheld 2018-09-04"));
    assert!(text.contains("gdfl-refused 2018-09"));
}

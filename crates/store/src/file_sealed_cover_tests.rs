#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the audited door's comparison of a month's two forms,
//! one header fact at a time. Each case puts the sealed form of a raw month
//! written another way beside the raw month and asks the audited door; every
//! bar is invented (the sealed door tests' own one-second month).

use std::fs;

use super::super::BarFile;
use super::super::sealed_door_tests::{Root, month_path, sealed_bytes, seconds, write_raw};
use crate::format::Bar;
use crate::path::FileKind;

/// The symbol the sealed door tests write every month under.
const SYMBOL: u32 = 0x5C3;
const MICROS: i64 = 1_000_000;

/// The audited door's refusal for the raw month of `root`, with the sealed
/// form of `other` (written in two commits split at `split`, when given)
/// beside it.
fn disagreement(root: &Root, other: &[Bar], split: Option<usize>) -> String {
    let path = month_path(None);
    let elsewhere = Root::new("ls1-agree-other");
    match split {
        Some(at) => {
            write_raw(&elsewhere.0, path, &other[..at]);
            BarFile::open_or_create(&elsewhere.0, path, SYMBOL)
                .unwrap()
                .append(&other[at..])
                .unwrap();
        }
        None => write_raw(&elsewhere.0, path, other),
    }
    let foreign = sealed_bytes(&elsewhere.0, path);
    let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&root.0);
    fs::write(&sbar, &foreign).unwrap();
    BarFile::open_existing_audited(&root.0, path, SYMBOL, u64::MAX)
        .err()
        .expect("two forms that differ are never audited")
}

/// **EACH HEADER FACT THE TWO FORMS DISAGREE ON REFUSES THE AUDIT, NAMED.**
/// Both forms pass their own audits; the comparison is what refuses. One
/// row fewer, the same rows in two commits (generation 2 against 1), every
/// stamp a second later, and only the last stamp later each refuse, and the
/// sentence carries the rows, the generations and whether the records'
/// BLAKE3 agreed. The same bars in the same one commit are audited (the
/// door's own control).
#[test]
fn each_header_fact_the_two_forms_disagree_on_refuses_the_audit_by_name() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("ls1-agree");
    let path = month_path(None);
    let bars = seconds(900);
    write_raw(&root.0, path, &bars);

    // One row fewer: the first fact compared differs.
    let fewer = disagreement(&root, &bars[..899], None);
    assert!(
        fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
        "{fewer}"
    );
    // The same rows in two commits: the rows agree, the generation does not,
    // and the records' digest still agrees.
    let twice = disagreement(&root, &bars, Some(450));
    assert!(
        twice.contains("rows 900 against 900")
            && twice.contains("generation 2 against 1")
            && twice.contains("records BLAKE3 equal: true"),
        "{twice}"
    );
    // Every stamp one second later: rows and generation agree, the first
    // stamp does not.
    let later: Vec<Bar> = bars
        .iter()
        .map(|bar| Bar {
            ts_micros: bar.ts_micros + MICROS,
            ..*bar
        })
        .collect();
    let shifted = disagreement(&root, &later, None);
    assert!(
        shifted.contains("rows 900 against 900")
            && shifted.contains("generation 1 against 1")
            && shifted.contains("records BLAKE3 equal: false"),
        "{shifted}"
    );
    // Only the last stamp later: rows, generation and first stamp agree.
    let mut last_late = bars.clone();
    last_late[899].ts_micros += 7 * MICROS;
    let late = disagreement(&root, &last_late, None);
    assert!(
        late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
        "{late}"
    );
    // The control: the sealed form of the same bars in one commit is audited.
    let same = Root::new("ls1-agree-same");
    write_raw(&same.0, path, &bars);
    let good = sealed_bytes(&same.0, path);
    fs::write(
        path.with_file(FileKind::SealedBars).to_path_buf(&root.0),
        &good,
    )
    .unwrap();
    let audited = BarFile::open_existing_audited(&root.0, path, SYMBOL, u64::MAX).unwrap();
    assert_eq!(audited.evidence().header().n_valid, 900);
}

/// **A SEALED AUDIT OF ANOTHER SYMBOL OR ANOTHER TIMEFRAME DISAGREES WITH
/// THE RAW MONTH, NAMED** (the comparison asked directly: a sealed form the
/// door opens was already refused for either by its own head check, so only
/// a direct call hands the comparison such a pair). An audit equal in every
/// fact agrees, and none at all has nothing to compare.
#[test]
fn a_sealed_audit_of_another_symbol_or_timeframe_disagrees_with_the_raw_month() {
    let root = Root::new("ls1-agree-direct");
    let path = month_path(None);
    write_raw(&root.0, path, &seconds(80));
    let raw = BarFile::open_existing(&root.0, path, SYMBOL)
        .unwrap()
        .audit_checksums(u64::MAX)
        .unwrap();
    let evidence = raw.evidence();
    let same = super::SealedAudit {
        path: root.0.join("same.sbar"),
        header: evidence.header(),
        records_blake3: evidence.data_digest(),
        audited: crate::sealed::Audited {
            rows: 80,
            frames: 1,
            bytes: 4_096,
        },
    };
    assert_eq!(super::agree(None, &raw), Ok(()));
    assert_eq!(super::agree(Some(&same), &raw), Ok(()));
    for (case, header) in [
        (
            "symbol",
            crate::header::Header {
                symbol_id: SYMBOL + 1,
                ..evidence.header()
            },
        ),
        (
            "timeframe",
            crate::header::Header {
                timeframe_secs: 60,
                ..evidence.header()
            },
        ),
    ] {
        let other = super::SealedAudit {
            header,
            path: root.0.join("other.sbar"),
            ..same
        };
        let refused = super::agree(Some(&other), &raw).unwrap_err();
        assert!(
            refused.contains("other.sbar") && refused.contains("disagree"),
            "{case}: {refused}"
        );
    }
}

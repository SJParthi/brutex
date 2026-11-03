//! The read side's word on what the summary door did (D-7430).
//!
//! `checked` counts a month clean only when a reader, asked after the door,
//! finds a summary describing the month. A writer or a repair can land
//! between the door and that read-back, which no fixture can time, so these
//! hand `checked` each answer the read-back can give.

use std::path::PathBuf;

use store::file::{StoreError, Summarized};
use store::month_summary::Refused;

use super::{Line, ReaderView, checked};

/// What a reader's open answers while a writer holds the month.
fn held() -> StoreError {
    StoreError::Locked {
        path: PathBuf::from("bars/gdfl/NSE/INDEX/NIFTY/1s/2031-05.lock"),
    }
}

/// **A READ-BACK REFUSED AFTER THE DOOR IS SAID, NEVER COUNTED.** A writer
/// takes the month between the door and the read-back, so the reader's open
/// is refused: a small month is not called small, a kept or built summary is
/// not called kept or built, and each line names the refusal and is not
/// clean.
#[test]
fn a_read_back_refused_after_the_door_is_said_and_never_clean() {
    let refused = || -> ReaderView { Err(held()) };
    let said = format!(
        "the step finished and the read-back was refused: {}",
        held()
    );
    for door in [
        Summarized::BelowThreshold,
        Summarized::Kept,
        Summarized::Built("absent".to_owned()),
    ] {
        let line = checked(Ok(door.clone()), &refused);
        assert_eq!(line, Line::Refused { why: said.clone() }, "{door:?}");
        assert!(!line.clean(), "{door:?}");
    }
}

/// **A SUMMARY THE READER REFUSES AFTER THE DOOR IS NOT SUMMARISED.** A writer
/// commits between the door and the read-back, so the summary the door kept
/// or wrote no longer describes the header a reader opens: the line says the
/// reader's own refusal, with the rows it read, and is not clean. A small
/// month's read-back is asked only for its rows.
#[test]
fn a_summary_the_reader_refuses_after_the_door_is_not_summarised() {
    let stale = Refused::Stale {
        summary: (3, 1_500),
        header: (4, 1_540),
    };
    let read = || -> ReaderView { Ok((1_540, PathBuf::from("2031-05.sum"), Err(stale.clone()))) };
    for door in [Summarized::Kept, Summarized::Built("absent".to_owned())] {
        let line = checked(Ok(door.clone()), &read);
        assert_eq!(
            line,
            Line::NotSummarized {
                n_valid: 1_540,
                reader: stale.to_string(),
            },
            "{door:?}"
        );
        assert!(!line.clean(), "{door:?}");
    }
    assert_eq!(
        checked(Ok(Summarized::BelowThreshold), &read),
        Line::Small { n_valid: 1_540 }
    );
}

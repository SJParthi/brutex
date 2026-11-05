//! Rows of `recovery_journal.rs` no test reached before BASE-COV-API repair
//! 3 (shard AS1, D-8661): a recovery journal path that is not a regular file,
//! refused by name by each of the three readers that open one.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

/// **A RECOVERY JOURNAL THAT IS NOT A REGULAR FILE IS REFUSED BY NAME, BY
/// EVERY READER** (BASE-COV-API repair 3, D-8661). Extremes: a folder where
/// the journal goes, asked for its tail; the same folder asked for a
/// snapshot; a FIFO opened as the journal (`mkfifo`, the external program,
/// run from a test only, as `census.rs`' tests do); a tail of more records
/// than its bound; and, at once, a folder asked for a tail past its bound
/// (the bound is refused first); out of the box, an absent journal asked for
/// a tail (the open's own refusal, not "not a regular file").
#[test]
fn a_recovery_journal_that_is_not_a_regular_file_is_refused_by_every_reader() {
    let root = crate::scratch::path("recovery-journal-cover");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let folder = root.join("journal-folder.bin");
    std::fs::create_dir_all(&folder).expect("a folder where the journal goes");
    let not_regular = |error: io::Error| {
        assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{error}");
        assert!(
            error
                .to_string()
                .contains("recovery journal is not a regular file"),
            "{error}"
        );
    };
    not_regular(tail(&folder, 1).expect_err("a folder has no tail"));
    not_regular(snapshot(&folder).expect_err("a folder is no snapshot"));
    let past = tail(&folder, MAX_TAIL_RECORDS + 1).expect_err("past the bound");
    assert_eq!(past.kind(), io::ErrorKind::InvalidInput, "{past}");
    assert_eq!(
        tail(&root.join("absent.bin"), 1)
            .expect_err("absent")
            .kind(),
        io::ErrorKind::NotFound
    );

    let fifo = root.join("journal-fifo.bin");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "the FIFO is made");
    let Err(error) = Journal::open(&fifo) else {
        panic!("a FIFO is no journal");
    };
    assert!(
        error
            .to_string()
            .contains("recovery journal is not a regular file"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

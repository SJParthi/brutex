#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests state their expectations"
)]
//! BASE-COV-LIBS: `read_bounded`'s two refusals no lake file on disk reaches
//! in the other tests: a path that cannot be opened, and a source whose length
//! the metadata does not tell (a character device declares zero bytes) and
//! which then yields more than the cap. Extreme cases: a cap of zero, a cap of
//! one, `u64::MAX` (the `take` bound saturates instead of overflowing), a
//! directory, a missing path, an empty file and a file of exactly the cap.

use std::path::Path;

use super::read_bounded;
use crate::error::LakeError;

#[test]
fn a_path_that_cannot_be_opened_is_an_io_refusal_with_its_reason() {
    let missing = Path::new("/brutex-lake-cover-missing/none.parquet");
    match read_bounded(missing, 1_024) {
        Err(LakeError::Io { reason }) => assert!(!reason.is_empty(), "{reason}"),
        other => panic!("a missing file must be an Io refusal: {other:?}"),
    }
}

#[test]
fn a_source_that_grows_past_its_declared_length_is_refused_at_the_read() {
    let zero = Path::new("/dev/zero");
    for cap in [0, 1, 4_096] {
        match read_bounded(zero, cap) {
            Err(LakeError::TooLarge { bytes, cap: said }) => {
                assert_eq!(said, cap);
                assert_eq!(bytes, cap + 1, "the read stops one byte past the cap");
            }
            other => panic!("/dev/zero under cap {cap}: {other:?}"),
        }
    }
}

#[test]
fn a_file_of_exactly_its_cap_is_read_whole_and_one_byte_more_is_refused() {
    let dir = std::env::temp_dir().join(format!("brutex-lake-cover-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("exact.bin");
    std::fs::write(&path, [7_u8; 5]).unwrap();
    assert_eq!(read_bounded(&path, 5).unwrap(), vec![7; 5]);
    assert_eq!(read_bounded(&path, u64::MAX).unwrap(), vec![7; 5]);
    assert!(matches!(
        read_bounded(&path, 4),
        Err(LakeError::TooLarge { bytes: 5, cap: 4 })
    ));
    let empty = dir.join("empty.bin");
    std::fs::write(&empty, []).unwrap();
    assert_eq!(read_bounded(&empty, 0).unwrap(), Vec::<u8>::new());
    // A directory opens on this target, declares a few hundred bytes at most,
    // and then refuses the read itself (EISDIR): an Io refusal, never bytes.
    assert!(
        matches!(read_bounded(&dir, 1 << 20), Err(LakeError::Io { .. })),
        "a directory is never a lake file"
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&empty);
    let _ = std::fs::remove_dir(&dir);
}

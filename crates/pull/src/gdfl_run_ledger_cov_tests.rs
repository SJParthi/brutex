//! BASE-COV-PULL: the runs ledger's records of any other shape, which no test
//! had reached (Route B, recipe v4): a record is trusted exactly when its CRC
//! and each of its words read. Every id is invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

fn entry() -> Entry {
    Entry {
        id: 7,
        mode: "import".to_owned(),
        kind: "stocks".to_owned(),
        by: "cli".to_owned(),
        pid: 42,
        started: 1_700_000_000,
        resumes: 0,
    }
}

/// `body` padded into a record whose CRC is true, so only its words refuse.
fn sealed(body: &str) -> [u8; 128] {
    let mut out = [b' '; 128];
    for (slot, byte) in out.iter_mut().zip(body.bytes()) {
        *slot = byte;
    }
    let crc = format!(
        "crc={:08x}",
        store::crc::crc32c(out.get(..BODY).unwrap_or_default())
    );
    for (slot, byte) in out.iter_mut().skip(BODY).zip(crc.bytes()) {
        *slot = byte;
    }
    out[127] = b'\n';
    out
}

#[test]
fn a_record_refuses_each_word_it_does_not_hold() {
    for (mode, kind, by) in [
        ("no-mode", "stocks", "cli"),
        ("import", "no-kind", "cli"),
        ("import", "stocks", "nobody"),
    ] {
        let bad = Entry {
            mode: mode.to_owned(),
            kind: kind.to_owned(),
            by: by.to_owned(),
            ..entry()
        };
        assert!(
            bad.record()
                .unwrap_err()
                .contains("is not a word the ledger holds")
        );
    }
    let fine = entry().record().unwrap();
    assert_eq!(Entry::of(&fine), Some(entry()));
}

#[test]
fn a_record_of_any_other_shape_is_no_record() {
    let good = entry().record().unwrap();
    assert_eq!(Entry::of(&good[..127]), None, "a byte short");
    let mut no_newline = good;
    no_newline[127] = b' ';
    assert_eq!(Entry::of(&no_newline), None);
    let mut no_space = good;
    no_space[126] = b'x';
    assert_eq!(Entry::of(&no_space), None);
    for body in [
        "7 import stocks cli 42",
        "x import stocks cli 42 1700000000",
        "7 no-mode stocks cli 42 1700000000",
        "7 import no-kind cli 42 1700000000",
        "7 import stocks nobody 42 1700000000",
        "7 import stocks cli 42 1700000000 resumes=x",
        "7 import stocks cli 42 1700000000 extra=1",
        "7 import stocks cli 42 1700000000 resumes=1 extra=1",
    ] {
        assert_eq!(Entry::of(&sealed(body)), None, "{body}");
    }
    let resumed = Entry::of(&sealed("7 import stocks cli 42 1700000000 resumes=3")).unwrap();
    assert_eq!(resumed.resumes, 3);
}

/// A ledger of nothing is no records; of a directory, named; a whole record
/// that fails its CRC reads as its own offset, and a torn tail is not read.
#[test]
fn the_ledger_reader_names_a_bad_record_by_its_offset() {
    let root = scratch("cov-ledger-read");
    assert_eq!(read_all(&root).unwrap(), Vec::new());
    let path = root.join(LEDGER);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = entry().record().unwrap().to_vec();
    bytes.extend_from_slice(&[b'x'; 128]);
    bytes.extend_from_slice(b"a torn tail");
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(read_all(&root).unwrap(), vec![Ok(entry()), Err(128)]);
    let other = scratch("cov-ledger-dir");
    std::fs::create_dir_all(other.join(LEDGER)).unwrap();
    assert!(read_all(&other).is_err());
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(other).unwrap();
}

/// A claims line is trusted exactly as a record is: anything else, however
/// near, is passed over and its id never read.
#[test]
fn a_claims_line_is_trusted_only_as_a_record_would_be() {
    assert_eq!(claimed(b""), None);
    assert_eq!(claimed(b"run "), None);
    assert_eq!(claimed(b"not a claims line"), None);
    let long = format!("run {} crc=00000000 ", "x".repeat(BODY + 1));
    assert_eq!(claimed(long.as_bytes()), None, "a body past its width");
}

#[test]
fn a_store_with_no_claims_file_has_an_empty_tail() {
    let root = scratch("cov-claims-none");
    assert_eq!(largest_claimed_run(&root).unwrap(), 0);
    std::fs::create_dir_all(root.join(CLAIMS).parent().unwrap()).unwrap();
    std::fs::create_dir(root.join(CLAIMS)).unwrap();
    assert!(
        largest_claimed_run(&root)
            .unwrap_err()
            .contains("not a regular file")
    );
    std::fs::remove_dir_all(root).unwrap();
}

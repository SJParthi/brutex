//! BASE-COV-PULL: a spool's bytes of any other shape (Route B, recipe v4). Each
//! header below carries a true CRC, so each refusal is reached on its own.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

fn id() -> SpoolId {
    SpoolId {
        run: 7,
        segment: Segment::Cash,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 3).unwrap(),
    }
}

/// `header` with `at..` overwritten and its CRC made true again.
fn resealed(header: [u8; 64], at: usize, bytes: &[u8]) -> [u8; 64] {
    let mut out = header;
    put(&mut out, at, bytes);
    let crc = crc32c(out.get(..60).unwrap_or_default());
    put(&mut out, 60, &crc.to_le_bytes());
    out
}

fn refused(bytes: &[u8]) -> String {
    let path = Path::new("/invented/spool");
    decode_header(bytes, path).unwrap_err().to_string()
}

#[test]
fn a_header_is_refused_for_each_field_on_its_own() {
    let good = encode_header(&id(), Mode::ZerodhaMerge, 1_700_000_000);
    let read = decode_header(&good, Path::new("/x")).unwrap().unwrap();
    assert_eq!((read.id, read.mode), (id(), Mode::ZerodhaMerge));
    assert_eq!(decode_header(&[0; 64], Path::new("/x")).unwrap(), None);
    assert_eq!(decode_header(&good[..63], Path::new("/x")).unwrap(), None);
    let mut rotted = good;
    rotted[20] ^= 1;
    assert!(refused(&rotted).contains("its CRC-32C does not match"));
    for (at, bytes, word) in [
        (0, &b"NOTASPOO"[..], "does not begin BRUTEXVS"),
        (8, &[2, 0][..], "its version is not 1"),
        (10, &[1, 0][..], "its stride is not 160"),
        (12, &[250][..], "names a feed other than zerodha"),
        (13, &[250][..], "names no segment"),
        (14, &[250][..], "timeframe code and seconds disagree"),
        (15, &[250][..], "its mode is not a Zerodha pass"),
        (18, &[13][..], "names no month"),
    ] {
        let said = refused(&resealed(good, at, bytes));
        assert!(said.contains(word), "{word}: {said}");
    }
}

#[test]
fn a_log_record_is_trusted_only_whole_and_with_a_true_crc() {
    let record = LogRecord {
        day: 19_000,
        generation: 3,
        state: LogState::Committed,
        exchange: 1,
    };
    let bytes = record.encode();
    assert_eq!(LogRecord::decode(&bytes), Some(record));
    assert_eq!(LogRecord::decode(&bytes[..15]), None, "a byte short");
    let mut rotted = bytes;
    rotted[0] ^= 1;
    assert_eq!(LogRecord::decode(&rotted), None);
    let mut reserved = bytes;
    reserved[10] = 1;
    let crc = crc32c(&reserved[..12]);
    put(&mut reserved, 12, &crc.to_le_bytes());
    assert_eq!(
        LogRecord::decode(&reserved),
        None,
        "a true CRC, reserved set"
    );
    let mut state = bytes;
    state[8] = 250;
    let crc = crc32c(&state[..12]);
    put(&mut state, 12, &crc.to_le_bytes());
    assert_eq!(LogRecord::decode(&state), None, "a true CRC, no such state");
}

/// A spool past its cap is refused before a byte of it is read.
#[test]
fn a_spool_past_its_byte_cap_is_refused_unread() {
    let root = scratch("cov-spool-cap");
    let path = root.join("2024-03.vsp");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_SPOOL_BYTES + 1).unwrap();
    let said = read_spool(&path, &id()).unwrap_err().to_string();
    assert!(said.contains("is past the"), "{said}");
    std::fs::remove_dir_all(root).unwrap();
}

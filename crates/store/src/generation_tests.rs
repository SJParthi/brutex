//! The pointer record's bytes. D-3350, WP-13c.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test-only assertions"
)]

use super::{MAGIC, Pointer, RECORD_LEN};
use crate::repair::Revision;

fn pointer(record: u64) -> Pointer {
    Pointer {
        revision: Revision::new(3).unwrap(),
        previous: 2,
        n_valid: 7_875,
        first_ts_micros: 1_606_794_300_000_000,
        last_ts_micros: 1_609_407_540_000_000,
        header_generation: 1,
        promoted_at_micros: 1_791_500_000_000_000,
        record,
    }
}

/// Every field survives the round trip, at its documented offset.
#[test]
fn a_record_round_trips_at_its_documented_offsets() {
    let bytes = pointer(4).encode();
    assert_eq!(bytes.len() as u64, RECORD_LEN);
    assert_eq!(&bytes[0..8], &MAGIC);
    assert_eq!(bytes[8..12], 3u32.to_le_bytes());
    assert_eq!(bytes[12..16], 2u32.to_le_bytes());
    assert_eq!(bytes[16..24], 7_875u64.to_le_bytes());
    assert_eq!(bytes[56..60], [0; 4]);
    assert_eq!(Pointer::decode(&bytes, 4), Some(pointer(4)));
}

/// The seal binds the record's index: a record copied to another slot, a
/// flipped bit, a foreign magic or a nonzero reserved word does not decode.
#[test]
fn a_moved_or_damaged_record_does_not_decode() {
    let bytes = pointer(4).encode();
    assert_eq!(Pointer::decode(&bytes, 5), None, "moved to another slot");
    for at in [0usize, 9, 17, 33, 49, 57, 61] {
        let mut damaged = bytes;
        damaged[at] ^= 0x10;
        assert_eq!(Pointer::decode(&damaged, 4), None, "byte {at}");
    }
}

/// Ordinal zero is not a revision: such a record does not decode.
#[test]
fn an_ordinal_outside_the_namespace_does_not_decode() {
    let mut zero = pointer(0).encode();
    zero[8..12].copy_from_slice(&0u32.to_le_bytes());
    let crc = super::seal(&zero, 0);
    zero[60..64].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(Pointer::decode(&zero, 0), None);
}

//! BASE-COV-PULL-PS1 (D-7335): the store-tree comparer's one side no test had
//! taken (Route B, recipe v4): a provenance sidecar record that does not
//! decode is compared as `undecodable`, never dropped. Every value is
//! invented.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use super::*;
use store::provenance::{Record, SourceKind};

/// One invented record at `ordinal`: a tick-store day of 2024-04-01.
fn record(ordinal: u64) -> Record {
    let name = "INVENTED.bts";
    Record {
        ordinal,
        vendor: brutex_core::vendor::Vendor::Gdfl,
        source_kind: SourceKind::TickstoreDay,
        source_bytes: 4_096,
        source_crc32: 0,
        source_name: name.to_owned(),
        source_name_truncated: false,
        source_name_blake3: brutex_core::blake3::hash(name.as_bytes()),
        day: store::provenance::Day::new(2024, 4, 1).unwrap(),
        first_index: 750,
        n_bars: 375,
        bar_definition: 5,
        journal_ordinal: 7,
        bar_file_generation: 3,
        commit: None,
        landed_at_micros: 1_700_000_000_000_000,
    }
}

/// **A SIDECAR RECORD THAT DOES NOT DECODE IS COMPARED AS `undecodable`,
/// NEVER DROPPED.** A sidecar of two records, the first whole and the second
/// the first's own bytes moved to position 1 (its checksum covers its
/// ordinal, so it decodes nowhere else), compares as the first record's day,
/// bars, definition, journal line and generation, then `undecodable`: two
/// trees that differ only in a damaged record compare different. (Every
/// sidecar the import tests compared decoded whole.)
#[test]
fn a_sidecar_record_that_does_not_decode_compares_as_undecodable() {
    let whole = record(0);
    let image = whole.image();
    assert_eq!(
        Record::decode(&image, 0),
        Ok(whole),
        "a record a writer leaves"
    );
    let mut sidecar = vec![0_u8; 64];
    sidecar.extend_from_slice(&image);
    sidecar.extend_from_slice(&image);
    let compared = as_compared(Path::new("INVENTED.prv"), sidecar);
    assert_eq!(
        String::from_utf8(compared).unwrap(),
        "2024-04-01 750 375 5 7 3\nundecodable\n"
    );
}

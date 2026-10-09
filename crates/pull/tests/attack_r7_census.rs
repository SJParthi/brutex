//! Gap-audit finding 17/#16 (D-3680): the census entry decoder read version-3
//! contract meaning into version-2 entries, and turned a contract field it
//! could not read into the SPOT key instead of refusing the entry.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Closes, Entry, EntryFault, EntryKey, Held, Layout};
use store::crc::crc32c;
use store::path::{Timeframe, YearMonth};

/// The closes half: bytes 64..128 of an entry. Its contract text is at 16, the
/// length at 40, reserved 41..60, its CRC at 60.
const HALF: usize = 64;

fn held(contract: Option<&str>) -> Held {
    Held::new(
        Entry {
            key: EntryKey {
                contract: contract.map(|text| Contract::parse(text).expect("a contract")),
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                symbol: Symbol::new("NIFTY").expect("a symbol"),
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2025, 7).expect("a month"),
            },
            rows: 375,
            first_ts_micros: 1_000,
            last_ts_micros: 2_000,
        },
        Closes::known(2_500_000, 2_510_000).expect("two prices"),
    )
}

/// Recomputes the closes half's checksum after an edit, as a forged or
/// resealed slot would carry it.
fn reseal(image: &mut [u8]) {
    let crc = crc32c(&image[HALF..HALF + 60]);
    image[HALF + 60..HALF + 64].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn a_version_3_contract_field_it_cannot_read_is_refused_never_the_spot_key() {
    let good = held(Some("2025-07-31-2500000-CE")).image();
    assert_eq!(
        Layout::V3
            .decode_entry(&good)
            .map(|h| h.entry.key.contract.is_some()),
        Ok(true),
        "the premise: a v3 contract row reads as that contract"
    );

    // A LENGTH PAST THE FIELD.
    let mut long = good;
    long[HALF + 40] = 30;
    reseal(&mut long);
    assert!(matches!(
        Layout::V3.decode_entry(&long),
        Err(EntryFault::ContractUnreadable { .. })
    ));

    // TEXT THAT IS NOT A CONTRACT.
    let mut junk = good;
    junk[HALF + 16] = b'x';
    reseal(&mut junk);
    assert!(matches!(
        Layout::V3.decode_entry(&junk),
        Err(EntryFault::ContractUnreadable { .. })
    ));

    // A NONZERO RESERVED TAIL, 41..60.
    let mut tail = held(None).image();
    tail[HALF + 50] = 1;
    reseal(&mut tail);
    assert!(matches!(
        Layout::V3.decode_entry(&tail),
        Err(EntryFault::ReservedNotZero { .. })
    ));
}

#[test]
fn a_version_2_entry_reads_no_contract_from_bytes_version_2_reserved() {
    // A v2 entry's bytes 16..60 of the closes half were reserved. The v2-era
    // decoder ignored them; reading them as a contract reinterprets version 2.
    let image = held(Some("2025-07-31-2500000-CE")).image();
    let read = Layout::V2
        .decode_entry(&image)
        .expect("a checksum-valid v2 entry");
    assert_eq!(
        read.entry.key.contract, None,
        "version 2 has no contract field"
    );
}

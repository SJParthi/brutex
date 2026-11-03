//! BASE-COV-PULL-PS1 (D-7335): the census format's refusals no test had
//! reached (Route B, recipe v4): an epoch capacity past the header's field, an
//! epoch name with a sign or a letter among its digits, a magic that leaves the
//! family at any one byte, a partition header whose public fields declare a
//! capacity no file carries, a reserved byte at each offset of its run, a
//! partition file holding a row of another segment, a census at the last
//! generation, a single file asked to compact, an epoch below generation zero
//! and a census made by hand holding a row no door admits. Every key is
//! invented (`ZQIDX`), and every count and stamp with it.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// The one invented index month every row here is of.
fn key() -> EntryKey {
    EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol: Symbol::new("ZQIDX").unwrap(),
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2026, 1).unwrap(),
    }
}

/// A row of [`key`] holding `rows` rows, its closes unread.
fn row(rows: u64) -> Held {
    Held::unknown(Entry {
        key: key(),
        rows,
        first_ts_micros: 1_000,
        last_ts_micros: 2_000,
    })
}

/// An index partition at its floor holding one row of ten.
fn partition() -> Manifest {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Index, SPOT_FLOOR).unwrap();
    census.record_held(row(10)).unwrap();
    census
}

/// `census`'s whole file with `header` as its one published slot, sealed, at
/// the position `header`'s generation selects, and the other slot zero: the
/// bytes a writer that published `header` over these entries leaves.
fn with_header(census: &Manifest, header: ManifestHeader) -> Vec<u8> {
    let mut file = census.image();
    file[..HEADER_REGION_LEN].fill(0);
    let at = SLOT_STRIDE_LEN * usize::try_from(header.generation % SLOT_COUNT).unwrap();
    file[at..at + IMAGE_LEN].copy_from_slice(&header.image());
    file
}

/// **AN EPOCH CAPACITY PAST THE HEADER'S FIELD IS REFUSED, NEVER WRAPPED**:
/// twice the keys overflowing a `u64` is refused naming `u64::MAX`, and twice
/// one key more than half the field is refused naming itself; half the field
/// in keys is the most that is given a capacity.
#[test]
fn an_epoch_capacity_past_the_field_is_refused_never_wrapped() {
    assert_eq!(
        epoch_capacity(Segment::Fno, u64::MAX),
        Err(ManifestError::CapacityOutOfRange { capacity: u64::MAX })
    );
    let widest = MAX_CAPACITY / 2;
    assert_eq!(epoch_capacity(Segment::Index, widest), Ok(widest * 2));
    assert_eq!(
        epoch_capacity(Segment::Index, widest + 1),
        Err(ManifestError::CapacityOutOfRange {
            capacity: (widest + 1) * 2,
        })
    );
}

/// **AN EPOCH NAME WITH A SIGN OR A LETTER AMONG ITS SIX DIGITS IS NO EPOCH**,
/// even where the number would parse (`+00001` does): only six ASCII digits
/// name one.
#[test]
fn an_epoch_name_with_a_sign_or_a_letter_among_its_digits_is_no_epoch() {
    for name in ["+00001.man", "00000a.man", "0000-1.man", "00 001.man"] {
        assert_eq!(parse_epoch_name(name), None, "{name}");
    }
    assert_eq!(parse_epoch_name("000001.man"), Some(1));
}

/// **A MAGIC THAT LEAVES THE FAMILY AT ANY ONE OF ITS SEVEN BYTES IS REFUSED**,
/// named as the magic, whichever byte it is; the eighth byte is the version's
/// and any value there is in the family.
#[test]
fn a_magic_that_leaves_the_family_at_any_one_byte_is_refused() {
    for magic in [
        *b"xRUTEXM2",
        *b"BxUTEXM2",
        *b"BRxTEXM2",
        *b"BRUxEXM2",
        *b"BRUTxXM2",
        *b"BRUTExM2",
        *b"BRUTEXx2",
    ] {
        assert_eq!(
            Layout::declare(4, magic, ENTRY_STRIDE, ENTRY_LEN, true),
            Err(ManifestError::DegenerateLayout { field: "magic" }),
            "{}",
            String::from_utf8_lossy(&magic)
        );
    }
    assert!(Layout::declare(4, *b"BRUTEXM9", ENTRY_STRIDE, ENTRY_LEN, true).is_ok());
}

/// **A PARTITION HEADER WHOSE PUBLIC FIELDS DECLARE A CAPACITY NO FILE CARRIES
/// IS REFUSED BY EVERY CHECK**: zero, and one past the 32-bit field, refused by
/// name by `validate` and by `advance` (a decoded slot can hold neither, so
/// only a header built in memory reaches this). A single file is bound by
/// [`MAX_ENTRIES`] and not by the field, so the same zero passes there.
#[test]
fn a_partition_header_with_a_capacity_no_file_carries_is_refused() {
    let genesis =
        ManifestHeader::genesis_partition(Vendor::Gdfl, Segment::Index, SPOT_FLOOR).unwrap();
    for capacity in [0, MAX_CAPACITY + 1] {
        let made = ManifestHeader {
            capacity,
            ..genesis
        };
        let refused = ManifestError::CapacityOutOfRange { capacity };
        assert_eq!(made.validate(0), Err(refused), "{capacity}");
        assert_eq!(made.advance(1, 1, 1), Err(refused), "{capacity}");
    }
    let single = ManifestHeader {
        capacity: 0,
        ..ManifestHeader::genesis(Vendor::Gdfl)
    };
    assert_eq!(single.validate(0), Ok(()));
}

/// **A RESERVED BYTE SET ANYWHERE IN ITS RUN IS NAMED BY ITS OWN OFFSET**: each
/// of the three, set alone in a slot that otherwise checks, is the offset the
/// refusal names; the slot with all three zero decodes.
#[test]
fn a_reserved_byte_set_anywhere_in_its_run_is_named_by_its_own_offset() {
    let slot = ManifestHeader::genesis_partition(Vendor::Gdfl, Segment::Fno, FNO_FLOOR)
        .unwrap()
        .image();
    for at in H_V4_RESERVED..H_V4_RESERVED + 3 {
        let mut dirty = slot;
        dirty[at] = 1;
        seal(&mut dirty);
        assert_eq!(
            ManifestHeader::decode(&dirty),
            Err(ManifestError::ReservedNotZero { offset: at })
        );
    }
    assert_eq!(
        ManifestHeader::decode(&slot).map(|header| header.segment),
        Ok(Some(Segment::Fno))
    );
}

/// **A PARTITION FILE HOLDING A ROW OF ANOTHER SEGMENT IS REFUSED BY ITS
/// ORDINAL**: an index row under a header that says FNO is a month no reader
/// of FNO finds, so the load names the row, the partition and what it found.
/// The same bytes under their own header load.
#[test]
fn a_partition_file_holding_a_row_of_another_segment_is_refused_by_its_ordinal() {
    let census = partition();
    let header = census.header();
    let foreign = with_header(
        &census,
        ManifestHeader {
            segment: Some(Segment::Fno),
            ..header
        },
    );
    assert_eq!(
        Manifest::open_partition(Vendor::Gdfl, Segment::Fno, &foreign),
        Err(ManifestError::ForeignSegment {
            ordinal: 0,
            partition: Segment::Fno,
            found: Segment::Index,
        })
    );
    let own = with_header(&census, header);
    assert_eq!(
        Manifest::open_partition(Vendor::Gdfl, Segment::Index, &own).map(|loaded| loaded.keys()),
        Ok(1)
    );
}

/// **A CENSUS AT THE LAST GENERATION REFUSES EVERY NEXT EPOCH**: a partition
/// whose file publishes `u64::MAX` cannot compact, with keys withdrawn or
/// without, and a single file there cannot be split into partitions, each
/// refused by name and never wrapped; one generation below the last leaves
/// room for exactly one key's epoch. A single file is never compacted.
#[test]
fn a_census_at_the_last_generation_refuses_every_next_epoch() {
    let census = partition();
    let at = |generation: u64| {
        let file = with_header(
            &census,
            ManifestHeader {
                generation,
                ..census.header()
            },
        );
        Manifest::open_partition(Vendor::Gdfl, Segment::Index, &file).unwrap()
    };
    let last = at(u64::MAX);
    assert_eq!(last.compacted(), Err(ManifestError::GenerationExhausted));
    assert_eq!(
        last.compacted_without(|_| false),
        Err(ManifestError::GenerationExhausted)
    );
    assert_eq!(
        at(u64::MAX - 1)
            .compacted()
            .map(|epoch| epoch.header().generation),
        Ok(u64::MAX)
    );
    let mut single = Manifest::open(Vendor::Gdfl, &[], &[]).unwrap();
    single.record_held(row(10)).unwrap();
    assert_eq!(
        single.compacted_without(|_| false),
        Err(ManifestError::SingleFileNotCompacted)
    );
    let file = with_header(
        &single,
        ManifestHeader {
            generation: u64::MAX,
            ..single.header()
        },
    );
    let worn = Manifest::open_image(Vendor::Gdfl, &file).unwrap();
    assert_eq!(worn.partitions(), Err(ManifestError::GenerationExhausted));
}

/// **AN EPOCH CANNOT START BELOW GENERATION ZERO**: an epoch of one key asked
/// to publish generation zero would start one below it, and is refused by
/// name, never wrapped; asked to publish one, it starts at zero.
#[test]
fn an_epoch_cannot_start_below_generation_zero() {
    let rows = [row(10)];
    assert_eq!(
        Manifest::epoch_of(Vendor::Gdfl, Segment::Index, &rows, 0),
        Err(ManifestError::GenerationExhausted)
    );
    assert_eq!(
        Manifest::epoch_of(Vendor::Gdfl, Segment::Index, &rows, 1)
            .map(|epoch| (epoch.header().generation, epoch.keys())),
        Ok((1, 1))
    );
}

/// **A CENSUS HOLDING A ROW NO DOOR ADMITS IS NEVER SPLIT**: made by hand (no
/// load or record makes one: both run `Entry::check`), a census whose index
/// holds an empty row refuses its migration by the row's ordinal in the epoch
/// it would have entered, and returns no partition at all.
#[test]
fn a_census_holding_a_row_no_door_admits_is_never_split() {
    let mut census = Manifest::open(Vendor::Gdfl, &[], &[]).unwrap();
    let empty = Held::unknown(Entry {
        rows: 0,
        ..row(10).entry
    });
    census.index.insert(empty.entry.key, empty);
    assert_eq!(
        census.partitions(),
        Err(ManifestError::Entry {
            ordinal: 0,
            fault: EntryFault::EmptyEntry,
        })
    );
}

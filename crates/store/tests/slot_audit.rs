//! `store::header::slot_audit` — every header slot named, and a month that
//! silently reverted to its older generation named as reverted (R4-09a,
//! D-6240, D-6241).
//!
//! Every region here is built from invented headers through
//! [`Header::commit`]; no vendor row and no real month. The regression half of
//! each test pins what [`Header::read_region`] returns, byte for byte: the
//! audit names damage, it never changes which generation a reader gets.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use store::format::{FLAG_CHECKSUMS, FormatError, HEADER_LEN, MAX_SLOTS, RECORD_STRIDE, SLOT_LEN};
use store::header::{
    Commit, Header, MonthVerdict, NamedSlot, SlotAudit, SlotDamage, SlotState, slot_audit,
};

/// Bytes of header region.
const REGION_LEN: usize = 32_768;
/// Bytes from one header slot to the next, as a `usize`.
const SLOT_STRIDE_LEN: usize = 16_384;
/// Byte offset of the slot checksum within a slot.
const OFF_CRC: usize = 56;
/// 2024-06-03 09:15 IST, in microseconds; the bars are never read.
const T0: i64 = 1_717_386_300_000_000;
/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// A file length holding the header region and `records` records.
const fn file_len(records: u64) -> u64 {
    HEADER_LEN + records * RECORD_STRIDE
}

/// Writes the first `prefix` bytes of a commit — a write torn at that point
/// when `prefix < SLOT_LEN`.
fn apply(region: &mut [u8], commit: &Commit, prefix: usize) {
    let at = usize::try_from(commit.offset).expect("offset is in range");
    region
        .iter_mut()
        .skip(at)
        .zip(commit.bytes.iter().take(prefix))
        .for_each(|(dst, src)| *dst = *src);
}

/// The genesis header at `version` (2 or 3; 3 is the sealed, checksum-required
/// version every new month is born at).
fn genesis(version: u16) -> Header {
    Header {
        format_version: version,
        ..Header::genesis(7, 60, FLAG_CHECKSUMS)
    }
}

/// `count` more records after `header`, timestamps strictly after its last.
fn advance(header: &Header, count: u64) -> Header {
    let from = T0 + i64::try_from(header.n_valid).expect("small") * MINUTE;
    let to = from + (i64::try_from(count).expect("small") - 1) * MINUTE;
    header.advance(count, from, to).expect("fits")
}

/// A region holding `commits` whole commits after genesis (100 records each),
/// and every header published, oldest first.
fn committed(version: u16, commits: usize) -> (Vec<u8>, Vec<Header>) {
    let mut region = vec![0u8; REGION_LEN];
    let mut headers = vec![genesis(version)];
    apply(
        &mut region,
        &headers[0].commit().expect("genesis commits"),
        SLOT_LEN,
    );
    for _ in 0..commits {
        let next = advance(headers.last().expect("one"), 100);
        apply(&mut region, &next.commit().expect("commits"), SLOT_LEN);
        headers.push(next);
    }
    (region, headers)
}

/// Byte offset of the slot holding `generation`.
const fn slot_at(generation: u64) -> usize {
    (generation % 2) as usize * SLOT_STRIDE_LEN
}

/// A NEWEST SLOT WITH ONE FLIPPED CHECKSUM BYTE IS NAMED `crc` AND THE MONTH
/// `reverted` — and the read still returns exactly the older generation.
///
/// On `feat/db-fix` at 371b1a506 there was no audit at all: `read_region`
/// returned generation 1 with `Ok` and nothing anywhere said generation 2 had
/// existed. Both versions, because version 3 (sealed: checksums required) and
/// version 2 must follow the same rules.
#[test]
fn a_flipped_checksum_byte_in_the_newest_slot_is_named_crc_and_reverted() {
    for version in [2, 3] {
        let (mut region, headers) = committed(version, 2);
        let (older, newest) = (headers[1], headers[2]);
        let len = file_len(200);
        assert_eq!(
            Header::read_region(&region, len),
            Ok(newest),
            "v{version}: whole, the newest wins"
        );

        region[slot_at(newest.generation) + OFF_CRC] ^= 0x01;

        assert_eq!(
            Header::read_region(&region, len),
            Ok(older),
            "v{version}: the selection is unchanged — the older generation, every field"
        );
        let audit = slot_audit(&region, len);
        assert!(
            matches!(
                audit.slots[0],
                SlotState::Damaged {
                    reason: SlotDamage::Refused(FormatError::SlotChecksum { .. }),
                    generation: None,
                }
            ),
            "v{version}: {audit:?}"
        );
        assert_eq!(
            audit.slots[1],
            SlotState::Valid { generation: 1 },
            "v{version}"
        );
        let MonthVerdict::Reverted {
            chosen_generation,
            damaged_slot,
            reason,
            damaged_generation,
        } = audit.verdict
        else {
            panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
        };
        assert_eq!(
            (chosen_generation, damaged_slot, damaged_generation),
            (1, 0, None)
        );
        assert_eq!(reason.name(), "crc");
        assert!(!audit.is_clean());
        assert_eq!(audit.verdict.name(), "reverted");
        let said = audit.describe();
        assert!(
            said.starts_with("REVERTED: read generation 1; slot 0 (crc: "),
            "{said}"
        );
        assert!(
            said.ends_with("its generation is unreadable, so a newer commit may be lost"),
            "{said}"
        );
    }
}

/// A CLEAN FILE IS CLEAN: both slots valid, the newer one read.
#[test]
fn a_clean_file_has_every_slot_valid_and_a_clean_verdict() {
    for version in [2, 3] {
        let (region, headers) = committed(version, 3);
        let audit = slot_audit(&region, file_len(300));
        assert_eq!(
            audit,
            SlotAudit {
                slots: [
                    SlotState::Valid { generation: 2 },
                    SlotState::Valid { generation: 3 },
                ],
                verdict: MonthVerdict::Clean,
            },
            "v{version}"
        );
        assert!(audit.is_clean());
        assert_eq!(audit.describe(), "clean");
        assert_eq!(audit.verdict.name(), "clean");
        assert_eq!(Header::read_region(&region, file_len(300)), Ok(headers[3]));
    }
}

/// A FIRST-COMMIT FILE'S SECOND SLOT IS EMPTY, AND THAT IS CLEAN: no commit has
/// reached position 1, so its zeros are not damage.
#[test]
fn a_first_commit_file_with_an_empty_second_slot_is_clean() {
    for version in [2, 3] {
        let (region, headers) = committed(version, 0);
        let audit = slot_audit(&region, HEADER_LEN);
        assert_eq!(
            audit.slots,
            [SlotState::Valid { generation: 0 }, SlotState::Empty],
            "v{version}"
        );
        assert_eq!(audit.verdict, MonthVerdict::Clean);
        assert_eq!(Header::read_region(&region, HEADER_LEN), Ok(headers[0]));
    }
}

/// A ZEROED SLOT AT A POSITION THE READ GENERATION PROVES WAS WRITTEN IS
/// `erased`, not empty: generation 1 in slot 1 means generation 0 once sat in
/// slot 0, and a zero-filled newest slot looks exactly like this.
#[test]
fn a_zeroed_slot_the_generation_proves_was_written_is_erased() {
    let (mut region, headers) = committed(3, 3);
    region[slot_at(2)..slot_at(2) + SLOT_LEN].fill(0);
    assert_eq!(Header::read_region(&region, file_len(300)), Ok(headers[3]));
    let audit = slot_audit(&region, file_len(300));
    assert_eq!(
        audit.slots[0],
        SlotState::Damaged {
            reason: SlotDamage::Erased,
            generation: None,
        }
    );
    assert!(matches!(
        audit.verdict,
        MonthVerdict::Reverted {
            chosen_generation: 3,
            damaged_slot: 0,
            reason: SlotDamage::Erased,
            damaged_generation: None,
        }
    ));
    assert_eq!(SlotDamage::Erased.name(), "erased");
    assert!(
        SlotDamage::Erased.to_string().starts_with("erased: "),
        "{}",
        SlotDamage::Erased
    );
}

/// Tears `commit` at every prefix over the slot image `base` holds at its
/// position, and checks each region three ways, by what the slot then holds.
///
/// A prefix writes `new[..p]` over `old[p..]`. Until the first byte where the
/// two images differ has landed, the slot IS the old image, whole; once the
/// last differing byte has landed it IS the new one, whole (bytes 60 to 63
/// are zero in both). Neither is damage, and both must audit clean. Every
/// prefix strictly between is a mix of two images: the read must return
/// exactly `older`, and the audit must name the slot reverted with its
/// generation unknown. Returns how many prefixes were a mix.
fn tear_at_every_prefix(
    base: &[u8],
    commit: &Commit,
    older: Header,
    whole: Header,
    len: u64,
    version: u16,
) -> usize {
    let at = usize::try_from(commit.offset).expect("offset is in range");
    let before = &base[at..at + SLOT_LEN];
    let mut mixed = 0;
    for prefix in 1..SLOT_LEN {
        let mut region = base.to_vec();
        apply(&mut region, commit, prefix);
        let slot = &region[at..at + SLOT_LEN];
        let read = Header::read_region(&region, len);
        let audit = slot_audit(&region, len);
        if slot == before {
            assert_eq!(
                read,
                Ok(older),
                "v{version} prefix {prefix}: nothing landed"
            );
            assert!(audit.is_clean(), "v{version} prefix {prefix}: {audit:?}");
        } else if slot == commit.bytes.as_slice() {
            assert_eq!(read, Ok(whole), "v{version} prefix {prefix}: all landed");
            assert!(audit.is_clean(), "v{version} prefix {prefix}: {audit:?}");
        } else {
            mixed += 1;
            assert_eq!(
                read,
                Ok(older),
                "v{version} prefix {prefix}: the read is unchanged"
            );
            assert!(
                matches!(
                    audit.verdict,
                    MonthVerdict::Reverted {
                        damaged_generation: None,
                        ..
                    }
                ),
                "v{version} prefix {prefix}: {audit:?}"
            );
            let named = audit.verdict.named_slot().expect("a revert names its slot");
            assert_eq!(
                (named.slot, named.chosen_generation),
                (commit.slot, older.generation),
                "v{version} prefix {prefix}"
            );
        }
    }
    let differs: Vec<usize> = (0..SLOT_LEN)
        .filter(|&byte| before[byte] != commit.bytes[byte])
        .collect();
    let (first, last) = (differs[0], differs[differs.len() - 1]);
    assert_eq!(
        mixed,
        last - first,
        "v{version}: exactly the prefixes between the first and the last differing byte are a mix"
    );
    mixed
}

/// THE NEWEST SLOT TORN MID-WRITE, AT EVERY PREFIX THAT LEAVES A MIX OF TWO
/// IMAGES, is named reverted — and the read returns exactly the older
/// generation's header, as it did before this audit existed. A prefix that
/// has not yet changed a byte, or has already written every byte that
/// differs, is no damage and audits clean. Two shapes: a commit torn over the
/// slot of generation `g − 2`, and the very first write to an all-zero second
/// slot (a prefix of one image, then zeros).
#[test]
fn a_newest_slot_torn_at_every_prefix_is_named_and_the_older_header_is_returned() {
    for version in [2, 3] {
        // Over an older image: generation 3 torn over generation 1, in slot 1.
        let (base, headers) = committed(version, 2);
        let torn = advance(&headers[2], 100);
        let commit = torn.commit().expect("commits");
        let mixed = tear_at_every_prefix(&base, &commit, headers[2], torn, file_len(300), version);
        assert!(mixed > 0, "v{version}: the generation field alone differs");

        // The first write to slot 1: generation 1 torn into zeros.
        let (base, headers) = committed(version, 0);
        let first = advance(&headers[0], 100);
        let commit = first.commit().expect("commits");
        let mixed = tear_at_every_prefix(&base, &commit, headers[0], first, file_len(100), version);
        assert!(mixed > 0, "v{version}: a prefix of an image over zeros");
    }
}

/// A SINGLE BIT FLIPPED ANYWHERE IN THE NEWEST SLOT — all 512 — is named, and
/// never moves the read.
#[test]
fn every_single_bit_flip_in_the_newest_slot_is_named_and_never_moves_the_read() {
    let (base, headers) = committed(3, 2);
    for bit in 0..SLOT_LEN * 8 {
        let mut region = base.clone();
        region[slot_at(2) + bit / 8] ^= 1 << (bit % 8);
        assert_eq!(
            Header::read_region(&region, file_len(200)),
            Ok(headers[1]),
            "bit {bit}"
        );
        let audit = slot_audit(&region, file_len(200));
        assert!(
            matches!(
                audit.verdict,
                MonthVerdict::Reverted {
                    chosen_generation: 1,
                    damaged_slot: 0,
                    damaged_generation: None,
                    ..
                }
            ),
            "bit {bit}: {audit:?}"
        );
    }
}

/// A NEWEST SLOT COUNTING RECORDS PAST THE FILE'S END is `beyond_file_len`
/// with its verified generation — the crash table's fourth row: the header
/// became durable before its records. The read falls back exactly as before.
#[test]
fn a_newest_slot_beyond_the_file_length_is_named_beyond_file_len() {
    for version in [2, 3] {
        let (region, headers) = committed(version, 2);
        // 150 records on disk; generation 2 claims 200.
        let len = file_len(150);
        assert_eq!(Header::read_region(&region, len), Ok(headers[1]));
        let audit = slot_audit(&region, len);
        assert_eq!(
            audit.slots[0],
            SlotState::Damaged {
                reason: SlotDamage::Refused(FormatError::CounterExceedsFile),
                generation: Some(2),
            },
            "v{version}"
        );
        assert_eq!(
            audit.verdict,
            MonthVerdict::Reverted {
                chosen_generation: 1,
                damaged_slot: 0,
                reason: SlotDamage::Refused(FormatError::CounterExceedsFile),
                damaged_generation: Some(2),
            }
        );
        assert_eq!(
            audit.describe(),
            "REVERTED: read generation 1; slot 0 (beyond_file_len: n_valid claims \
             more records than the file holds): it holds generation 2, newer than the one read"
        );
    }
}

/// A FILE SHORTER THAN ITS HEADER REGION has every slot `beyond_file_len` and
/// the month is unreadable, with the refusal `read_region` returns.
#[test]
fn a_file_shorter_than_its_header_has_every_slot_beyond_and_is_unreadable() {
    let (region, _) = committed(3, 1);
    let len = HEADER_LEN - 1;
    let refused = Header::read_region(&region, len).expect_err("too short");
    assert_eq!(refused, FormatError::TooShortForHeader);
    let audit = slot_audit(&region, len);
    assert_eq!(audit.verdict, MonthVerdict::Unreadable(refused));
    assert_eq!(audit.verdict.name(), "unreadable");
    for state in audit.slots {
        assert!(
            matches!(
                state,
                SlotState::Damaged {
                    reason: SlotDamage::Refused(FormatError::TooShortForHeader),
                    generation: Some(_),
                }
            ),
            "{state:?}"
        );
    }
    assert_eq!(
        audit.describe(),
        format!("unreadable: {}", FormatError::TooShortForHeader)
    );
}

/// AN OLDER SLOT DAMAGED WITH ITS GENERATION VERIFIED lost nothing: the verdict
/// is `damaged`, not `reverted`, and the newer generation is the one read.
#[test]
fn an_older_slot_damaged_with_a_verified_generation_is_damaged_not_reverted() {
    let mut region = vec![0u8; REGION_LEN];
    let zero = genesis(3);
    apply(&mut region, &zero.commit().expect("commits"), SLOT_LEN);
    let big = advance(&zero, 1_000);
    apply(&mut region, &big.commit().expect("commits"), SLOT_LEN);
    // Generation 2 publishes the SAME counter again, re-stamped: a header whose
    // counter the file supports, while generation 1's 1,000 it does not.
    let small = Header {
        n_valid: 0,
        generation: 2,
        ..zero
    };
    apply(&mut region, &small.commit().expect("commits"), SLOT_LEN);
    let len = HEADER_LEN;
    assert_eq!(Header::read_region(&region, len), Ok(small));
    let audit = slot_audit(&region, len);
    assert_eq!(
        audit.verdict,
        MonthVerdict::Damaged {
            slot: 1,
            reason: SlotDamage::Refused(FormatError::CounterExceedsFile),
            chosen_generation: 2,
            damaged_generation: 1,
        }
    );
    assert_eq!(audit.verdict.name(), "damaged");
    assert_eq!(
        audit.describe(),
        "damaged: slot 1 (beyond_file_len: n_valid claims more records than the file \
         holds) holds generation 1, older than generation 2 that was read; no commit was lost"
    );
}

/// A COMMIT IN THE WRONG SLOT is `wrong_position` with its generation, and the
/// month is unreadable exactly as `read_region` refuses it.
#[test]
fn a_commit_in_the_wrong_slot_is_named_wrong_position() {
    let (mut region, headers) = committed(3, 1);
    // Generation 2 belongs in slot 0; write it into slot 1.
    let moved = advance(&headers[1], 100).commit().expect("commits");
    region[SLOT_STRIDE_LEN..SLOT_STRIDE_LEN + SLOT_LEN].copy_from_slice(&moved.bytes);
    let refused = Header::read_region(&region, file_len(200)).expect_err("misplaced");
    let audit = slot_audit(&region, file_len(200));
    assert_eq!(audit.verdict, MonthVerdict::Unreadable(refused));
    assert_eq!(
        audit.slots[1],
        SlotState::Damaged {
            reason: SlotDamage::Refused(FormatError::SlotPositionMismatch {
                expected: 0,
                found: 1
            }),
            generation: Some(2),
        }
    );
    let SlotState::Damaged { reason, .. } = audit.slots[1] else {
        panic!("damaged");
    };
    assert_eq!(reason.name(), "wrong_position");
}

/// A REGION SHORTER THAN ITS SLOTS names the missing position `truncated`.
#[test]
fn a_region_that_ends_inside_a_slot_names_it_truncated() {
    let (region, _) = committed(3, 1);
    for cut in [SLOT_STRIDE_LEN, SLOT_STRIDE_LEN + SLOT_LEN - 1] {
        let audit = slot_audit(&region[..cut], file_len(100));
        assert_eq!(
            audit.slots[1],
            SlotState::Damaged {
                reason: SlotDamage::Truncated,
                generation: None,
            },
            "cut at {cut}"
        );
        assert_eq!(
            audit.verdict,
            MonthVerdict::Unreadable(FormatError::HeaderRegionTooShort { slots: 1, need: 2 })
        );
    }
    assert_eq!(SlotDamage::Truncated.name(), "truncated");
    assert!(SlotDamage::Truncated.to_string().starts_with("truncated: "));
}

/// IT READS EXACTLY `MAX_SLOTS` POSITIONS. A region the length of a whole file
/// whose third and later strides hold intact, NEWER commits, and garbage, gives
/// the same audit as the header region alone: nothing past position
/// `MAX_SLOTS - 1` auditions, which is the bound `read_region` keeps (C-01) and
/// the one this audit's cost rests on (C-HA-01).
#[test]
fn it_reads_exactly_max_slots_positions_whatever_the_region_holds_beyond() {
    assert_eq!(MAX_SLOTS, 2, "the audit's array is one entry per position");
    let (region, headers) = committed(3, 2);
    let alone = slot_audit(&region, file_len(200));
    let mut long = region.clone();
    let newer = advance(&headers[2], 100).commit().expect("commits");
    for stride in 2..64 {
        long.extend(std::iter::repeat_n(0u8, SLOT_STRIDE_LEN));
        let at = stride * SLOT_STRIDE_LEN;
        if stride % 2 == 0 {
            long[at..at + SLOT_LEN].copy_from_slice(&newer.bytes);
        } else {
            long[at..at + SLOT_LEN].fill(0xA5);
        }
    }
    assert_eq!(slot_audit(&long, file_len(200)), alone);
    assert_eq!(alone.slots.len(), MAX_SLOTS);
    assert_eq!(Header::read_region(&long, file_len(200)), Ok(headers[2]));
}

/// EVERY REASON HAS ITS ONE WORD, the catch-all included.
#[test]
fn every_reason_has_one_stable_word() {
    for (refusal, word) in [
        (
            FormatError::SlotChecksum {
                stored: 1,
                computed: 2,
            },
            "crc",
        ),
        (FormatError::NotABarFile, "magic"),
        (FormatError::UnknownVersion(9), "version"),
        (FormatError::RetiredVersion(1), "version"),
        (FormatError::MagicVersionMismatch(3), "version"),
        (FormatError::StrideMismatch(24), "stride"),
        (
            FormatError::SlotPositionMismatch {
                expected: 0,
                found: 1,
            },
            "wrong_position",
        ),
        (FormatError::CounterExceedsFile, "beyond_file_len"),
        (FormatError::TooShortForHeader, "beyond_file_len"),
        (
            FormatError::TimestampsOutOfOrder {
                previous: 2,
                next: 1,
            },
            "timestamps_out_of_order",
        ),
        (FormatError::ReservedNotZero(1), "reserved"),
        (FormatError::UnknownFlags(4), "flags"),
        (FormatError::ChecksumsRequired(3), "checksums_required"),
        (FormatError::OffsetOverflow, "refused"),
    ] {
        let damage = SlotDamage::Refused(refusal);
        assert_eq!(damage.name(), word, "{refusal:?}");
        assert_eq!(damage.to_string(), format!("{word}: {refusal}"));
    }
}

/// A NON-ZERO SLOT WITHOUT THE FAMILY MAGIC is `magic`, never `empty`: a slot
/// torn in its first byte is damage, and only all 64 zero bytes are absence.
#[test]
fn a_nonzero_slot_without_the_magic_is_damaged_not_empty() {
    let (mut region, headers) = committed(3, 0);
    region[SLOT_STRIDE_LEN + SLOT_LEN - 1] = 0x01;
    assert_eq!(Header::read_region(&region, HEADER_LEN), Ok(headers[0]));
    let audit = slot_audit(&region, HEADER_LEN);
    assert_eq!(
        audit.slots[1],
        SlotState::Damaged {
            reason: SlotDamage::Refused(FormatError::NotABarFile),
            generation: None,
        }
    );
    assert!(matches!(audit.verdict, MonthVerdict::Reverted { .. }));
}

/// THE NAMED SLOT is the one shape both naming verdicts share, and a clean or
/// unreadable region names none.
#[test]
fn a_verdict_names_its_one_damaged_slot_or_none() {
    let crc = SlotDamage::Refused(FormatError::SlotChecksum {
        stored: 1,
        computed: 2,
    });
    assert_eq!(MonthVerdict::Clean.named_slot(), None);
    assert_eq!(
        MonthVerdict::Unreadable(FormatError::NoValidHeader).named_slot(),
        None
    );
    assert_eq!(
        MonthVerdict::Damaged {
            slot: 1,
            reason: crc,
            chosen_generation: 6,
            damaged_generation: 5,
        }
        .named_slot(),
        Some(NamedSlot {
            slot: 1,
            reason: crc,
            chosen_generation: 6,
            damaged_generation: Some(5),
        })
    );
    assert_eq!(
        MonthVerdict::Reverted {
            chosen_generation: 6,
            damaged_slot: 1,
            reason: crc,
            damaged_generation: None,
        }
        .named_slot(),
        Some(NamedSlot {
            slot: 1,
            reason: crc,
            chosen_generation: 6,
            damaged_generation: None,
        })
    );
}

/// A REGION CUT JUST AFTER SLOT 1's 64 BYTES still holds slot 1 whole: it is
/// audited as valid, while the read refuses the short region as before.
#[test]
fn a_region_cut_just_after_a_slot_holds_that_slot_whole() {
    let (region, _) = committed(3, 1);
    let cut = &region[..SLOT_STRIDE_LEN + SLOT_LEN];
    let audit = slot_audit(cut, file_len(100));
    assert_eq!(audit.slots[1], SlotState::Valid { generation: 1 });
    assert_eq!(
        Header::read_region(cut, file_len(100)),
        Err(FormatError::HeaderRegionTooShort { slots: 1, need: 2 })
    );
}

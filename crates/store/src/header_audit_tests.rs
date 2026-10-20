//! The two boundaries of the slot audit that no region built through
//! [`Header::commit`] can reach, pinned on the private functions directly
//! (R4-09a, D-6240): a damaged slot whose verified generation EQUALS the one
//! read, and an all-zero slot at the very position of the generation read.
//! Each is the chosen slot itself in any real region, so only a direct call
//! can say which side of the boundary it falls on — and a comparison nobody
//! can tell from its neighbour is a mutant nobody kills.

#![cfg(test)]
#![allow(clippy::indexing_slicing)]

use super::*;

const CRC: SlotDamage = SlotDamage::Refused(FormatError::SlotChecksum {
    stored: 1,
    computed: 2,
});

/// A damaged slot at the generation read is NOT "older": it is named
/// reverted, the conservative side, never damage that lost nothing.
#[test]
fn a_damaged_slot_at_the_generation_read_is_a_revert_not_older_damage() {
    let slots = [
        SlotState::Valid { generation: 4 },
        SlotState::Damaged {
            reason: CRC,
            generation: Some(4),
        },
    ];
    assert_eq!(
        verdict_of(&slots, 4),
        MonthVerdict::Reverted {
            chosen_generation: 4,
            damaged_slot: 1,
            reason: CRC,
            damaged_generation: Some(4),
        }
    );
}

/// A revert anywhere wins over older damage, whichever position comes first.
#[test]
fn a_revert_wins_over_older_damage_in_either_order() {
    let older = SlotState::Damaged {
        reason: CRC,
        generation: Some(1),
    };
    let unknown = SlotState::Damaged {
        reason: CRC,
        generation: None,
    };
    for (slots, at) in [([older, unknown], 1), ([unknown, older], 0)] {
        assert_eq!(
            verdict_of(&slots, 2),
            MonthVerdict::Reverted {
                chosen_generation: 2,
                damaged_slot: at,
                reason: CRC,
                damaged_generation: None,
            }
        );
    }
    assert_eq!(
        verdict_of(&[older, SlotState::Valid { generation: 2 }], 2),
        MonthVerdict::Damaged {
            slot: 0,
            reason: CRC,
            chosen_generation: 2,
            damaged_generation: 1,
        }
    );
}

/// An all-zero slot at the position of the generation read is erased (that
/// generation was written there), and one position past it is empty.
#[test]
fn a_zero_slot_is_erased_from_its_own_generation_on_and_empty_before() {
    let zeros = [0u8; SLOT_LEN];
    let erased = SlotState::Damaged {
        reason: SlotDamage::Erased,
        generation: None,
    };
    assert_eq!(slot_state(Some(&zeros), 1, 0, Some(1)), erased);
    assert_eq!(slot_state(Some(&zeros), 1, 0, Some(0)), SlotState::Empty);
    assert_eq!(slot_state(Some(&zeros), 0, 0, None), SlotState::Empty);
    assert_eq!(slot_state(Some(&zeros), 0, 0, Some(0)), erased);
}

/// Exactly 64 bytes is a whole slot; 63 is truncated.
#[test]
fn sixty_four_bytes_are_a_slot_and_sixty_three_are_not() {
    let zeros = [0u8; SLOT_LEN];
    assert_eq!(slot_state(Some(&zeros), 1, 0, None), SlotState::Empty);
    assert_eq!(
        slot_state(zeros.get(..SLOT_LEN - 1), 1, 0, None),
        SlotState::Damaged {
            reason: SlotDamage::Truncated,
            generation: None,
        }
    );
}

//! Deterministic enumeration of the entire fixed V1 expression grammar.
//!
//! Programs are visited by instruction count, then lexicographically. The
//! representation's existing capacity is the only size bound: there is no
//! sweep-depth parameter. AND, OR and NOT can occur at any valid position,
//! including repeated conditions and nested operators. Only structurally
//! canonical programs are emitted. Algebraically equivalent programs can still
//! differ; this is exhaustive syntax enumeration, not a Boolean-equivalence
//! classifier. No support-based pruning is applied to this mixed grammar.
//!
//! Each call has an explicit node-work budget and can checkpoint between any
//! two grammar choices. Total enumeration can be combinatorially enormous.

use crate::expression::{Expression, Instruction, MAX_INSTRUCTIONS};
use crate::{ConditionMask, table};

const BITS: usize = 384;
/// Fixed V1 cursor width. A caller binds and seals these bytes with its run ID.
pub const CURSOR_BYTES: usize = 16 + BITS * 2 + MAX_INSTRUCTIONS * 2;
const MAGIC: &[u8; 8] = b"BTXEGN01";

/// Invalid search alphabet or malformed checkpoint state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The alphabet must be nonempty, sorted, unique, live vocabulary positions.
    Alphabet,
    /// The cursor is inconsistent with the versioned traversal.
    Cursor,
    /// The caller's cumulative grammar-work counter cannot represent another step.
    WorkOverflow,
}

/// One bounded unit of traversal work.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "the fixed inline program deliberately avoids a heap allocation for every enumerated candidate"
)]
pub enum Step {
    /// A complete canonical expression. The cursor already points past it.
    Candidate(Expression),
    /// The requested node budget ended; checkpoint and continue unchanged.
    Paused,
    /// Every program in this fixed wire language has been visited.
    Exhausted,
}

/// A fixed-memory DFS cursor. No candidate set or historical bar is retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    offered: [u16; BITS],
    count: u16,
    length: u16,
    at: u16,
    finished: bool,
    // Before `at`, a slot is one past its selected alphabet rank. At `at`, it
    // is the next rank to try. All later slots are zero.
    digits: [u16; MAX_INSTRUCTIONS],
    code: [Instruction; MAX_INSTRUCTIONS],
}

impl Cursor {
    /// Start the fixed V1 grammar over an explicitly identified live alphabet.
    ///
    /// # Errors
    /// Empty, oversized, unsorted, duplicate, void or retired positions.
    pub fn new(live: &[u32]) -> Result<Self, Refusal> {
        if live.is_empty() || live.len() > BITS || live.windows(2).any(|p| p.first() >= p.get(1)) {
            return Err(Refusal::Alphabet);
        }
        let mut offered = [0; BITS];
        for (slot, &bit) in offered.iter_mut().zip(live) {
            let bit = u16::try_from(bit).map_err(|_| Refusal::Alphabet)?;
            if !table::definition(bit).is_some_and(|row| row.status == table::BitStatus::Live) {
                return Err(Refusal::Alphabet);
            }
            *slot = bit;
        }
        Ok(Self {
            offered,
            count: u16::try_from(live.len()).map_err(|_| Refusal::Alphabet)?,
            length: 1,
            at: 0,
            finished: false,
            digits: [0; MAX_INSTRUCTIONS],
            code: [Instruction::Pad; MAX_INSTRUCTIONS],
        })
    }

    /// The selected alphabet as an unchanged six-word identity component.
    #[must_use]
    pub fn alphabet(&self) -> ConditionMask {
        self.offered
            .iter()
            .take(usize::from(self.count))
            .fold(ConditionMask::ZERO, |mask, &bit| {
                mask.with_bit(u32::from(bit))
            })
    }

    /// Try at most `nodes` grammar choices. Zero pauses without advancing.
    /// `work` is incremented once per choice, including pruned invalid prefixes.
    ///
    /// # Errors
    /// An impossible cursor invariant or exhausted cumulative counter.
    pub fn advance(&mut self, nodes: u64, work: &mut u64) -> Result<Step, Refusal> {
        if self.finished {
            return Ok(Step::Exhausted);
        }
        for _ in 0..nodes {
            *work = work.checked_add(1).ok_or(Refusal::WorkOverflow)?;
            let at = usize::from(self.at);
            let next = self.digits.get_mut(at).ok_or(Refusal::Cursor)?;
            if *next == self.count + 3 {
                *next = 0;
                if self.at == 0 {
                    if usize::from(self.length) == MAX_INSTRUCTIONS {
                        self.finished = true;
                        return Ok(Step::Exhausted);
                    }
                    self.length += 1;
                    self.code.fill(Instruction::Pad);
                } else {
                    self.at -= 1;
                }
                // Backtracking is also bounded work; do not hide it behind a
                // candidate-only counter that could run forever on exclusions.
                continue;
            }
            let rank = *next;
            *next += 1;
            let instruction = self.instruction(rank).ok_or(Refusal::Cursor)?;
            let slot = self.code.get_mut(at).ok_or(Refusal::Cursor)?;
            *slot = instruction;
            let prefix = self.code.get(..=at).ok_or(Refusal::Cursor)?;
            if !valid_prefix(prefix, usize::from(self.length)) {
                continue;
            }
            if at + 1 == usize::from(self.length) {
                let mut code = self.code;
                for slot in code.iter_mut().skip(usize::from(self.length)) {
                    *slot = Instruction::Pad;
                }
                return Ok(Step::Candidate(Expression {
                    code,
                    len: usize::from(self.length),
                }));
            }
            self.at += 1;
        }
        Ok(Step::Paused)
    }

    fn instruction(&self, rank: u16) -> Option<Instruction> {
        if rank < self.count {
            self.offered
                .get(usize::from(rank))
                .copied()
                .map(Instruction::Bit)
        } else {
            match rank - self.count {
                0 => Some(Instruction::Not),
                1 => Some(Instruction::And),
                2 => Some(Instruction::Or),
                _ => None,
            }
        }
    }

    /// Exact deterministic cursor bytes; sealing and run binding are caller duties.
    #[must_use]
    pub fn encode(&self) -> [u8; CURSOR_BYTES] {
        let mut bytes = [0; CURSOR_BYTES];
        for (slot, byte) in bytes.iter_mut().zip(
            MAGIC
                .iter()
                .copied()
                .chain(self.count.to_le_bytes())
                .chain(self.length.to_le_bytes())
                .chain(self.at.to_le_bytes())
                .chain([u8::from(self.finished), 0])
                .chain(self.offered.iter().flat_map(|n| n.to_le_bytes()))
                .chain(self.digits.iter().flat_map(|n| n.to_le_bytes())),
        ) {
            *slot = byte;
        }
        bytes
    }

    /// Initial search descriptor for this alphabet, independent of progress.
    /// A run identity binds the entire traversal; resumable progress belongs in
    /// the separately sealed checkpoint and cannot create a different search.
    #[must_use]
    pub fn initial_descriptor(&self) -> [u8; CURSOR_BYTES] {
        let mut initial = self.clone();
        initial.length = 1;
        initial.at = 0;
        initial.finished = false;
        initial.digits.fill(0);
        initial.encode()
    }

    /// Reconstruct and validate a fixed cursor; never accept another alphabet by guess.
    ///
    /// # Errors
    /// Version, padding, bounds, alphabet or impossible partial grammar state.
    pub fn decode(bytes: &[u8; CURSOR_BYTES]) -> Result<Self, Refusal> {
        if bytes.get(..8) != Some(MAGIC.as_slice()) || bytes.get(15) != Some(&0) {
            return Err(Refusal::Cursor);
        }
        let field = |start| {
            bytes
                .get(start..start + 2)
                .and_then(|b| <[u8; 2]>::try_from(b).ok())
                .map(u16::from_le_bytes)
                .ok_or(Refusal::Cursor)
        };
        let count = field(8)?;
        let length = field(10)?;
        let at = field(12)?;
        let finished = match bytes.get(14) {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(Refusal::Cursor),
        };
        if count == 0 || length == 0 || usize::from(length) > MAX_INSTRUCTIONS || at >= length {
            return Err(Refusal::Cursor);
        }
        let mut live = [0_u32; BITS];
        for (index, slot) in live.iter_mut().enumerate() {
            *slot = u32::from(field(16 + index * 2)?);
        }
        if live.iter().skip(usize::from(count)).any(|&bit| bit != 0) {
            return Err(Refusal::Cursor);
        }
        let mut cursor = Self::new(live.get(..usize::from(count)).ok_or(Refusal::Cursor)?)?;
        cursor.length = length;
        cursor.at = at;
        cursor.finished = finished;
        for (index, slot) in cursor.digits.iter_mut().enumerate() {
            *slot = field(16 + BITS * 2 + index * 2)?;
            if *slot > count + 3
                || (index < usize::from(at) && *slot == 0)
                || (index > usize::from(at) && *slot != 0)
            {
                return Err(Refusal::Cursor);
            }
        }
        if finished
            && (usize::from(length) != MAX_INSTRUCTIONS
                || at != 0
                || cursor.digits.iter().any(|&d| d != 0))
        {
            return Err(Refusal::Cursor);
        }
        for index in 0..usize::from(at) {
            let rank = cursor
                .digits
                .get(index)
                .copied()
                .and_then(|d| d.checked_sub(1))
                .ok_or(Refusal::Cursor)?;
            let instruction = cursor.instruction(rank).ok_or(Refusal::Cursor)?;
            *cursor.code.get_mut(index).ok_or(Refusal::Cursor)? = instruction;
            if !valid_prefix(
                cursor.code.get(..=index).ok_or(Refusal::Cursor)?,
                usize::from(length),
            ) {
                return Err(Refusal::Cursor);
            }
        }
        Ok(cursor)
    }
}

fn valid_prefix(code: &[Instruction], length: usize) -> bool {
    let Some(remaining) = length.checked_sub(code.len()) else {
        return false;
    };
    let mut starts = [0_usize; MAX_INSTRUCTIONS];
    let mut depth = 0_usize;
    for (index, op) in code.iter().enumerate() {
        match op {
            Instruction::Bit(_) => {
                let Some(slot) = starts.get_mut(depth) else {
                    return false;
                };
                *slot = index;
                depth += 1;
            }
            Instruction::Not if depth > 0 => {}
            Instruction::And | Instruction::Or if depth >= 2 => {
                let Some(&left) = starts.get(depth - 2) else {
                    return false;
                };
                let Some(&right) = starts.get(depth - 1) else {
                    return false;
                };
                if code.get(left..right) > code.get(right..index) {
                    return false;
                }
                depth -= 1;
            }
            _ => return false,
        }
    }
    // Both callers admit nonempty prefixes in order. Keep the empty/internal
    // malformed case a refusal too: subtracting an absent stack item must not
    // panic or accidentally admit a prefix.
    depth
        .checked_sub(1)
        .is_some_and(|reductions| reductions <= remaining)
}

#[cfg(test)]
mod invariant_tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;

    #[test]
    fn prefix_validation_refuses_empty_overfull_and_unreducible_stacks() {
        assert!(!valid_prefix(&[], 0));
        assert!(!valid_prefix(&[], MAX_INSTRUCTIONS));
        assert!(!valid_prefix(
            &[Instruction::Bit(0); MAX_INSTRUCTIONS + 1],
            MAX_INSTRUCTIONS + 1
        ));
        assert!(!valid_prefix(&[Instruction::Bit(0), Instruction::Not], 1));
        // A later operand cannot repair a unary operator's earlier underflow.
        // This matters for the prefix validator independently of its callers,
        // which normally reject the one-token prefix before reaching this one.
        assert!(!valid_prefix(&[Instruction::Not, Instruction::Bit(0)], 2));
        assert!(!valid_prefix(
            &[Instruction::Not, Instruction::Bit(0), Instruction::Not],
            3
        ));
        assert!(!valid_prefix(
            &[Instruction::Bit(0), Instruction::Bit(63)],
            2
        ));
        assert!(valid_prefix(
            &[Instruction::Bit(0), Instruction::Bit(63)],
            3
        ));
        assert!(valid_prefix(
            &[Instruction::Bit(0), Instruction::Bit(63), Instruction::And],
            3
        ));
        assert!(!valid_prefix(
            &[Instruction::Bit(63), Instruction::Bit(0), Instruction::And],
            3
        ));
    }

    #[test]
    fn corrupted_internal_cursor_indices_refuse_with_bounded_work() {
        let mut cursor = Cursor::new(&[0]).expect("live alphabet");
        cursor.at = u16::try_from(MAX_INSTRUCTIONS).expect("fixed capacity");
        let before = cursor.encode();
        let mut work = 0;
        assert_eq!(cursor.advance(1, &mut work), Err(Refusal::Cursor));
        assert_eq!(cursor.encode(), before);
        assert_eq!(work, 1);

        let mut cursor = Cursor::new(&[0]).expect("live alphabet");
        cursor.digits[0] = cursor.count + 4;
        assert_eq!(cursor.advance(1, &mut work), Err(Refusal::Cursor));
        assert_eq!(work, 2);
        assert_eq!(cursor.instruction(cursor.count + 3), None);
        cursor.count = u16::try_from(BITS + 1).expect("small corrupted count");
        assert_eq!(
            cursor.instruction(u16::try_from(BITS).expect("fixed width")),
            None
        );
    }
}

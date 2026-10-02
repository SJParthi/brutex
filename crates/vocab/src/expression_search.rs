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
    // Derived, never encoded. Before `at`, `starts[i]` is where the subtree
    // ending at `i` begins and `depths[i]` is the stack depth after `i`. Every
    // later slot is zero. Each is written only once the search moves past `i`
    // and read only for positions before the one being placed, so a new
    // choice is checked in O(1) plus one sibling comparison instead of
    // re-walking the whole prefix over a cleared full-capacity array.
    starts: [u16; MAX_INSTRUCTIONS],
    depths: [u16; MAX_INSTRUCTIONS],
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
            starts: [0; MAX_INSTRUCTIONS],
            depths: [0; MAX_INSTRUCTIONS],
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
                    // The position returned to is chosen again, so what was
                    // recorded when the search moved past it no longer holds.
                    let back = usize::from(self.at);
                    *self.starts.get_mut(back).ok_or(Refusal::Cursor)? = 0;
                    *self.depths.get_mut(back).ok_or(Refusal::Cursor)? = 0;
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
            let Some((start, depth)) = self.place(at) else {
                continue;
            };
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
            *self.starts.get_mut(at).ok_or(Refusal::Cursor)? = start;
            *self.depths.get_mut(at).ok_or(Refusal::Cursor)? = depth;
            self.at += 1;
        }
        Ok(Step::Paused)
    }

    /// What `self.code[at]` makes of the prefix `..=at`, read from what is
    /// recorded for `..at`: the start of the subtree ending at `at` and the
    /// stack depth after it.
    ///
    /// `None` when the instruction underflows the stack, joins two siblings out
    /// of canonical order, or leaves more values than the instructions left in
    /// `length` can reduce to one. Every check is O(1) except the sibling
    /// comparison, which reads the two subtrees being joined and so is bounded
    /// by `length`.
    fn place(&self, at: usize) -> Option<(u16, u16)> {
        let remaining = usize::from(self.length).checked_sub(at.checked_add(1)?)?;
        let (below_start, below_depth) = match at.checked_sub(1) {
            None => (0, 0),
            Some(previous) => (*self.starts.get(previous)?, *self.depths.get(previous)?),
        };
        let (start, depth) = match self.code.get(at)? {
            Instruction::Bit(_) => (u16::try_from(at).ok()?, below_depth.checked_add(1)?),
            Instruction::Not if below_depth >= 1 => (below_start, below_depth),
            Instruction::And | Instruction::Or if below_depth >= 2 => {
                let right = usize::from(below_start);
                let left = usize::from(*self.starts.get(right.checked_sub(1)?)?);
                if self.code.get(left..right)? > self.code.get(right..at)? {
                    return None;
                }
                (u16::try_from(left).ok()?, below_depth - 1)
            }
            _ => return None,
        };
        (usize::from(depth) - 1 <= remaining).then_some((start, depth))
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
            let (start, depth) = cursor.place(index).ok_or(Refusal::Cursor)?;
            *cursor.starts.get_mut(index).ok_or(Refusal::Cursor)? = start;
            *cursor.depths.get_mut(index).ok_or(Refusal::Cursor)? = depth;
        }
        Ok(cursor)
    }
}

#[cfg(test)]
mod invariant_tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;

    /// The full re-walk the search used before audit o1engine-23, kept as the
    /// independent reference [`Cursor::place`] is checked against.
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

    /// Whether the search's incremental check admits `code` as a prefix of a
    /// program of `length` instructions, placing it one instruction at a time
    /// exactly as `advance` and `decode` do.
    fn admits(code: &[Instruction], length: usize) -> bool {
        let Ok(length) = u16::try_from(length) else {
            return false;
        };
        let mut cursor = Cursor::new(&[0]).expect("live alphabet");
        cursor.length = length;
        for (at, &instruction) in code.iter().enumerate() {
            let Some(slot) = cursor.code.get_mut(at) else {
                return false;
            };
            *slot = instruction;
            let Some((start, depth)) = cursor.place(at) else {
                return false;
            };
            cursor.starts[at] = start;
            cursor.depths[at] = depth;
        }
        !code.is_empty()
    }

    /// THE INCREMENTAL CHECK ADMITS EXACTLY THE PREFIXES THE FULL RE-WALK DID.
    /// Audit o1engine-23.
    ///
    /// Every prefix of every program of up to seven instructions over two
    /// leaves and the three operators, at every declared length from its own
    /// up to nine, gets the same answer from `admits` (what `advance` and
    /// `decode` now run) as from `valid_prefix`. The worst cases ride along:
    /// the empty prefix, a prefix past the wire capacity, a stack too deep to
    /// reduce in time, underflow at the first position, and siblings out of
    /// canonical order deep inside a nested join.
    #[test]
    fn the_incremental_prefix_check_admits_exactly_what_the_full_rewalk_did() {
        let alphabet = [
            Instruction::Bit(0),
            Instruction::Bit(63),
            Instruction::Not,
            Instruction::And,
            Instruction::Or,
        ];
        let mut code = Vec::with_capacity(7);
        let mut compared = 0_u32;
        let mut admitted = 0_u32;
        for len in 1..=7_u32 {
            for mut n in 0..5_usize.pow(len) {
                code.clear();
                for _ in 0..len {
                    code.push(alphabet[n % 5]);
                    n /= 5;
                }
                for length in code.len()..=9 {
                    let want = valid_prefix(&code, length);
                    assert_eq!(admits(&code, length), want, "{code:?} in {length}");
                    compared += 1;
                    admitted += u32::from(want);
                }
            }
        }
        assert!(
            compared > 100_000 && admitted > 1_000,
            "{compared} {admitted}"
        );
        for (code, length) in [
            (&[][..], 0),
            (&[][..], MAX_INSTRUCTIONS),
            (&[Instruction::Bit(0); 3][..], 3),
            (&[Instruction::And][..], 3),
        ] {
            assert!(!admits(code, length) && !valid_prefix(code, length));
        }
        let overfull = [Instruction::Bit(0); MAX_INSTRUCTIONS + 1];
        assert!(!admits(&overfull, MAX_INSTRUCTIONS + 1));
        let full = [Instruction::Bit(0); MAX_INSTRUCTIONS.div_ceil(2)];
        assert!(admits(&full, MAX_INSTRUCTIONS));
        assert!(!admits(&full, MAX_INSTRUCTIONS - 1));
    }

    /// A search that backtracks leaves nothing recorded at or past `at`, so a
    /// cursor rebuilt from its saved bytes holds the same derived state.
    #[test]
    fn a_resumed_search_records_exactly_what_the_live_one_does() {
        let mut live = Cursor::new(&[0, 369]).expect("live alphabet");
        let mut work = 0;
        let mut backtracked = false;
        for _ in 0..5_000 {
            let before = live.at;
            live.advance(1, &mut work).expect("bounded step");
            backtracked |= live.at < before;
            let at = usize::from(live.at);
            assert!(live.starts[at..].iter().all(|&s| s == 0));
            assert!(live.depths[at..].iter().all(|&d| d == 0));
            let resumed = Cursor::decode(&live.encode()).expect("saved cursor");
            assert_eq!(resumed.starts, live.starts);
            assert_eq!(resumed.depths, live.depths);
        }
        assert!(backtracked);
    }

    #[test]
    fn prefix_validation_refuses_empty_overfull_and_unreducible_stacks() {
        assert!(!admits(&[], 0));
        assert!(!admits(&[], MAX_INSTRUCTIONS));
        assert!(!admits(
            &[Instruction::Bit(0); MAX_INSTRUCTIONS + 1],
            MAX_INSTRUCTIONS + 1
        ));
        assert!(!admits(&[Instruction::Bit(0), Instruction::Not], 1));
        // A later operand cannot repair a unary operator's earlier underflow.
        // This matters for the prefix validator independently of its callers,
        // which normally reject the one-token prefix before reaching this one.
        assert!(!admits(&[Instruction::Not, Instruction::Bit(0)], 2));
        assert!(!admits(
            &[Instruction::Not, Instruction::Bit(0), Instruction::Not],
            3
        ));
        assert!(!admits(&[Instruction::Bit(0), Instruction::Bit(63)], 2));
        assert!(admits(&[Instruction::Bit(0), Instruction::Bit(63)], 3));
        assert!(admits(
            &[Instruction::Bit(0), Instruction::Bit(63), Instruction::And],
            3
        ));
        assert!(!admits(
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

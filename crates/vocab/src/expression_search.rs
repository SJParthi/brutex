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
///
/// Equality is the encoded state (D-0750): two cursors are equal exactly when
/// [`Self::encode`] writes the same bytes. The instruction scratch is not
/// state: every slot is rewritten before it is read, and [`Self::decode`]
/// rebuilds only the slots below `at`. A derived equality compared that scratch
/// and made a reopened checkpoint unequal to the cursor it was saved from.
/// `Debug` prints the same encoded fields and omits the scratch, so a reopened
/// checkpoint prints as its source does (D-0754).
#[derive(Clone)]
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

impl PartialEq for Cursor {
    fn eq(&self, other: &Self) -> bool {
        self.encode() == other.encode()
    }
}

impl Eq for Cursor {}

impl std::fmt::Debug for Cursor {
    /// The encoded fields only: the alphabet up to `count` and the digits up
    /// to `length`. `decode` refuses a nonzero slot past either.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cursor")
            .field(
                "offered",
                &self
                    .offered
                    .get(..usize::from(self.count))
                    .unwrap_or_default(),
            )
            .field("length", &self.length)
            .field("at", &self.at)
            .field("finished", &self.finished)
            .field(
                "digits",
                &self
                    .digits
                    .get(..usize::from(self.length))
                    .unwrap_or_default(),
            )
            .finish_non_exhaustive()
    }
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
    /// A node budget bounds choices, not progress (D-0752): every rank is
    /// tried at every position, so nodes per emitted candidate grow with the
    /// alphabet, and more than 4,096 nodes can pass between two candidates.
    /// One choice is not O(1) either (D-0753): it revalidates the whole prefix
    /// from its first instruction over a fresh `MAX_INSTRUCTIONS`-slot stack,
    /// bounded by the fixed 1,151-instruction capacity. The node counts are
    /// counted by `grammar_nodes_per_candidate_grow_with_the_alphabet_and_gaps_exceed_one_replay`
    /// in `crates/vocab/tests/expression_search.rs`; this loop's one whole-prefix
    /// call per choice, the inline stack and both edges of its width are
    /// pinned by
    /// `invariant_tests::prefix_validation_rescans_from_the_first_instruction_over_a_fixed_stack`
    /// in this module. The time per node is unmeasured, UNVERIFIED
    /// (`docs/06-limits.md`, D-0753).
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
        }
        // ONE check of the whole selected prefix, not one per index. The walk
        // refuses an operator the stack cannot feed at the index it occurs, so
        // every intermediate stack is checked by this one pass; and the slack
        // `remaining - (depth - 1)` never rises from one instruction to the
        // next (it falls by 2, 1 or 0), so a final prefix that can still
        // reduce implies every shorter one could. Checking each prefix
        // separately was Θ(at²) plus `at` clears of the walk's stack. D-1341.
        if at > 0
            && !valid_prefix(
                cursor.code.get(..usize::from(at)).ok_or(Refusal::Cursor)?,
                usize::from(length),
            )
        {
            return Err(Refusal::Cursor);
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

    /// ET-o1-proof-coverage-5 (D-0753): every grammar choice in `advance`
    /// revalidates its whole prefix from index 0 over a fresh stack of exactly
    /// `MAX_INSTRUCTIONS` `usize` slots. Behaviour pins both stack edges and
    /// the scan's start; the source shape pins that the stack is that inline
    /// array and that `advance` calls this whole-prefix validator once per
    /// choice rather than keeping an incremental one.
    #[test]
    fn prefix_validation_rescans_from_the_first_instruction_over_a_fixed_stack() {
        // A full stack of MAX_INSTRUCTIONS operands fits; one more does not,
        // although the length leaves room to reduce it. A narrower stack
        // refuses the first, a wider (or length-sized) one admits the second.
        assert!(valid_prefix(
            &[Instruction::Bit(0); MAX_INSTRUCTIONS],
            2 * MAX_INSTRUCTIONS - 1
        ));
        assert!(!valid_prefix(
            &[Instruction::Bit(0); MAX_INSTRUCTIONS + 1],
            2 * MAX_INSTRUCTIONS + 1
        ));
        // An invalid first instruction refuses a prefix whose tail alone would
        // fit, so the scan cannot have started later.
        let mut code = [Instruction::Bit(0); 5];
        assert!(valid_prefix(&code[..1], 9));
        code[0] = Instruction::Not;
        assert!(valid_prefix(&code[4..], 9));
        assert!(!valid_prefix(&code, 9));

        let source = include_str!("expression_search.rs");
        let body = |head: &str, tail: &str| {
            let start = source.find(head).expect("function present");
            let len = source[start..].find(tail).expect("function ends");
            &source[start..start + len]
        };
        let validator = body("fn valid_prefix(", "\n}\n");
        let stack = "let mut starts = [0_usize; MAX_INSTRUCTIONS];";
        let scan = "for (index, op) in code.iter().enumerate() {";
        assert_eq!(validator.matches(stack).count(), 1, "one inline stack");
        assert!(validator.find(stack) < validator.find(scan));
        assert_eq!(validator.matches("starts =").count(), 1);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(std::mem::size_of::<[usize; MAX_INSTRUCTIONS]>(), 9_208);

        let step = body("pub fn advance(", "\n    fn instruction(");
        let choice = step.find("for _ in 0..nodes {").expect("one choice loop");
        let prefix = step
            .find("let prefix = self.code.get(..=at).ok_or(Refusal::Cursor)?;")
            .expect("the whole prefix up to at");
        let call = step
            .find("if !valid_prefix(prefix, usize::from(self.length)) {")
            .expect("the whole-prefix validator");
        let paused = step.find("Ok(Step::Paused)").expect("loop end");
        assert!(choice < prefix && prefix < call && call < paused);
        assert_eq!(
            step.matches("valid_prefix").count(),
            1,
            "no second validator"
        );
    }

    /// ET-expressions-3 (D-0750): after a candidate the scratch slot at `at`
    /// still holds that candidate's last instruction, which `decode` leaves
    /// `Pad`. Equality ignores it; one more node step is a different state.
    #[test]
    fn a_reopened_candidate_checkpoint_equals_its_source_and_a_step_does_not() {
        let mut cursor = Cursor::new(&[0]).expect("live alphabet");
        let mut work = 0;
        assert_eq!(
            cursor.advance(1, &mut work),
            Ok(Step::Candidate(Expression::parse("0").expect("one leaf")))
        );
        let reopened = Cursor::decode(&cursor.encode()).expect("own checkpoint");
        assert_ne!(reopened.code, cursor.code, "the scratch really differs");
        assert!(
            reopened.eq(&cursor),
            "a reopened checkpoint equals its source"
        );
        assert!(cursor.eq(&reopened), "equality is symmetric");
        // D-0754: `Debug` is the encoded state too, so the differing scratch
        // does not print and a real step does.
        assert_eq!(format!("{reopened:?}"), format!("{cursor:?}"));
        assert_eq!(
            format!("{cursor:?}"),
            "Cursor { offered: [0], length: 1, at: 0, finished: false, digits: [1], .. }"
        );
        let before = cursor.clone();
        assert_eq!(cursor.advance(1, &mut work), Ok(Step::Paused));
        assert!(!before.eq(&cursor), "one node step is a different state");
        assert!(!cursor.eq(&before), "inequality is symmetric");
        assert_eq!(
            format!("{cursor:?}"),
            "Cursor { offered: [0], length: 1, at: 0, finished: false, digits: [2], .. }"
        );
        // A corrupted count or length past its array prints nothing, no panic.
        let mut corrupt = cursor.clone();
        corrupt.count = u16::MAX;
        corrupt.length = u16::MAX;
        let printed = format!("{corrupt:?}");
        assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
    }

    /// `W3-vocab1-0`: `docs/06-limits.md` says a sibling comparison can read a
    /// whole operand. Equal siblings are admitted, and siblings that differ only
    /// in their last instruction are told apart only there.
    #[test]
    fn a_sibling_comparison_can_read_the_whole_operand() {
        let (a, b) = (Instruction::Bit(0), Instruction::Bit(1));
        let (and, or) = (Instruction::And, Instruction::Or);
        assert!(valid_prefix(&[a, b, and, a, b, and, and], 7));
        assert!(valid_prefix(&[a, b, and, a, b, or, and], 7));
        assert!(!valid_prefix(&[a, b, or, a, b, and, and], 7));
    }

    /// `W3-vocab1-3`, held by the lib tests too: equality ignores the scratch
    /// program and sees every encoded field.
    #[test]
    fn equality_is_the_encoding_and_ignores_the_scratch_program() {
        let start = Cursor::new(&[0, 369]).expect("live alphabet");
        let mut cursor = start.clone();
        let mut work = 0;
        assert!(matches!(
            cursor.advance(1, &mut work),
            Ok(Step::Candidate(_))
        ));
        let decoded = Cursor::decode(&cursor.encode()).expect("own bytes decode");
        assert!(
            decoded.code != cursor.code,
            "the candidate left its instruction in the scratch program"
        );
        assert!(decoded == cursor, "equal bytes compare equal");
        assert!(cursor != start, "different progress compares unequal");
    }

    /// `decode` checks the selected prefix once instead of at every index.
    /// That is sound only if the final check implies every intermediate one.
    /// Exhaustive over five opcodes, every prefix up to six instructions and
    /// every program length up to eight: the two answers never differ.
    #[test]
    fn one_check_of_the_whole_prefix_equals_a_check_at_every_index() {
        let ops = [
            Instruction::Bit(0),
            Instruction::Bit(63),
            Instruction::Not,
            Instruction::And,
            Instruction::Or,
        ];
        let (mut compared, mut accepted, mut refused) = (0_u32, 0_u32, 0_u32);
        for size in 1..=6_usize {
            for mut n in 0..5_usize.pow(u32::try_from(size).expect("small")) {
                let mut code = [Instruction::Pad; 6];
                for slot in code.iter_mut().take(size) {
                    *slot = ops[n % 5];
                    n /= 5;
                }
                let code = &code[..size];
                for length in size..=8 {
                    let every = (0..size).all(|i| valid_prefix(&code[..=i], length));
                    let once = valid_prefix(code, length);
                    assert_eq!(every, once, "{code:?} length {length}");
                    compared += 1;
                    if once {
                        accepted += 1;
                    } else {
                        refused += 1;
                    }
                }
            }
        }
        assert_eq!(compared, 63_465, "sum of 5^s * (9 - s) for s in 1..=6");
        assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
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

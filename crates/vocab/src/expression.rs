//! Boolean expressions over stable vocabulary identities.
//!
//! Unknown is distinct from false: negating an unavailable indicator never
//! creates a signal. Parsing is bounded; evaluation allocates nothing. This is
//! an explicit-expression surface, not a claim to enumerate all Boolean
//! functions or to apply AND's Apriori pruning to OR.

use crate::{ConditionMask, table};

/// On-disk semantics, independent of the append-only vocabulary version.
pub const VERSION: u16 = 1;
/// Fixed V1 program capacity, enough for 384 signed leaves and binary joins.
/// This literal is a wire-format bound: widening it requires a new version.
/// Exceeding it refuses rather than truncates.
pub const MAX_INSTRUCTIONS: usize = 1151;
/// Maximum accepted source length in bytes.
pub const MAX_SOURCE_BYTES: usize = 4096;
/// Fixed serialized width, including version, count and zeroed padding.
pub const ENCODED_LEN: usize = 4 + MAX_INSTRUCTIONS * 3;
const MAX_NESTING: u16 = 32;

/// Strong Kleene truth: only `True` admits a signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    /// Definitely false.
    False,
    /// Definitely true.
    True,
    /// Required evidence is unavailable.
    Unknown,
}

impl Truth {
    /// Logical negation, preserving unknown.
    #[must_use]
    pub const fn negate(self) -> Self {
        match self {
            Self::False => Self::True,
            Self::True => Self::False,
            Self::Unknown => Self::Unknown,
        }
    }

    /// Conjunction; a definite false settles it.
    #[must_use]
    pub const fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    /// Disjunction; a definite true settles it.
    #[must_use]
    pub const fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Instruction {
    #[default]
    Pad,
    Bit(u16),
    Not,
    And,
    Or,
}

/// A malformed, unsupported or over-budget expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Input exceeds the fixed byte bound.
    SourceCapacity,
    /// The program exceeds its instruction bound.
    InstructionCapacity,
    /// Parentheses or negation exceed the parser's stack bound.
    NestingCapacity,
    /// Expected an operand, operator or closing parenthesis.
    Syntax,
    /// A name/index is absent, retired or void in the live vocabulary.
    UnavailableBit,
    /// Serialized version, opcode, stack shape, order or padding is invalid.
    Encoding,
}

/// A validated fixed-width postfix program. Bit order is never renumbered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expression {
    pub(crate) code: [Instruction; MAX_INSTRUCTIONS],
    pub(crate) len: usize,
}

impl Expression {
    /// Reopen the exact fixed-width descriptor without an external parser.
    /// Only canonical programs of this version are accepted. The fixed wire
    /// capacity bounds evaluation; the source nesting limit governs parsing.
    ///
    /// # Errors
    /// Wrong version, count, opcode, operand, nonlive bit, stack shape,
    /// noncanonical sibling order, or nonzero trailing padding.
    pub fn decode(encoded: &[u8; ENCODED_LEN]) -> Result<Self, Refusal> {
        let field = |start| {
            encoded
                .get(start..start + 2)
                .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
                .map(u16::from_le_bytes)
                .ok_or(Refusal::Encoding)
        };
        if field(0)? != VERSION {
            return Err(Refusal::Encoding);
        }
        let len = usize::from(field(2)?);
        if len == 0 || len > MAX_INSTRUCTIONS {
            return Err(Refusal::Encoding);
        }
        let mut result = Self {
            code: [Instruction::Pad; MAX_INSTRUCTIONS],
            len,
        };
        let mut starts = [0_usize; MAX_INSTRUCTIONS];
        let mut depth = 0_usize;
        for (index, bytes) in encoded
            .get(4..)
            .ok_or(Refusal::Encoding)?
            .chunks_exact(3)
            .enumerate()
        {
            let [op, low, high] = <[u8; 3]>::try_from(bytes).map_err(|_| Refusal::Encoding)?;
            if index >= len {
                if [op, low, high] != [0, 0, 0] {
                    return Err(Refusal::Encoding);
                }
                continue;
            }
            let operand = u16::from_le_bytes([low, high]);
            let instruction = match (op, operand) {
                (1, bit) => {
                    if !table::definition(bit)
                        .is_some_and(|row| row.status == table::BitStatus::Live)
                    {
                        return Err(Refusal::UnavailableBit);
                    }
                    *starts.get_mut(depth).ok_or(Refusal::Encoding)? = index;
                    depth += 1;
                    Instruction::Bit(bit)
                }
                (2, 0) if depth >= 1 => Instruction::Not,
                (3 | 4, 0) if depth >= 2 => {
                    let left = *starts.get(depth - 2).ok_or(Refusal::Encoding)?;
                    let right = *starts.get(depth - 1).ok_or(Refusal::Encoding)?;
                    if result.code.get(left..right).ok_or(Refusal::Encoding)?
                        > result.code.get(right..index).ok_or(Refusal::Encoding)?
                    {
                        return Err(Refusal::Encoding);
                    }
                    depth -= 1;
                    if op == 3 {
                        Instruction::And
                    } else {
                        Instruction::Or
                    }
                }
                _ => return Err(Refusal::Encoding),
            };
            *result.code.get_mut(index).ok_or(Refusal::Encoding)? = instruction;
        }
        if depth != 1 {
            return Err(Refusal::Encoding);
        }
        Ok(result)
    }

    /// Parse names or decimal bit IDs, `!`, `&`, `|`, and parentheses.
    /// Precedence is NOT, AND, OR. Whitespace and redundant parentheses do
    /// not change identity; commutative siblings have a canonical order.
    /// General Boolean equivalence is deliberately not asserted.
    ///
    /// # Errors
    /// Invalid syntax, nonlive bits, or any explicit capacity breach.
    pub fn parse(source: &str) -> Result<Self, Refusal> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(Refusal::SourceCapacity);
        }
        let mut parser = Parser {
            source: source.as_bytes(),
            at: 0,
            expression: Self {
                code: [Instruction::Pad; MAX_INSTRUCTIONS],
                len: 0,
            },
        };
        parser.disjunction(0)?;
        parser.space();
        if parser.at != parser.source.len() {
            return Err(Refusal::Syntax);
        }
        Ok(parser.expression)
    }

    /// Evaluate one bar in bounded space/time with no heap allocation.
    /// `known` must come from the evaluator's actual availability evidence.
    #[must_use]
    pub fn evaluate(&self, truth: ConditionMask, known: ConditionMask) -> Truth {
        let mut stack = [Truth::Unknown; MAX_INSTRUCTIONS];
        let mut used = 0_usize;
        for instruction in self.code.iter().take(self.len) {
            match instruction {
                Instruction::Bit(bit) => {
                    let value = if !known.get(u32::from(*bit)) {
                        Truth::Unknown
                    } else if truth.get(u32::from(*bit)) {
                        Truth::True
                    } else {
                        Truth::False
                    };
                    let Some(slot) = stack.get_mut(used) else {
                        return Truth::Unknown;
                    };
                    *slot = value;
                    used += 1;
                }
                Instruction::Not => {
                    let Some(top) = used.checked_sub(1).and_then(|i| stack.get_mut(i)) else {
                        return Truth::Unknown;
                    };
                    *top = top.negate();
                }
                Instruction::And | Instruction::Or => {
                    let Some(left_index) = used.checked_sub(2) else {
                        return Truth::Unknown;
                    };
                    let right = stack.get(used - 1).copied().unwrap_or(Truth::Unknown);
                    used -= 1;
                    let Some(left) = stack.get_mut(left_index) else {
                        return Truth::Unknown;
                    };
                    *left = if *instruction == Instruction::And {
                        left.and(right)
                    } else {
                        left.or(right)
                    };
                }
                Instruction::Pad => return Truth::Unknown,
            }
        }
        if used == 1 {
            stack.first().copied().unwrap_or(Truth::Unknown)
        } else {
            Truth::Unknown
        }
    }

    /// All referenced live bits, suitable for the existing mask identity term.
    /// Operators and ordering MUST also be bound through [`Self::encode`].
    #[must_use]
    pub fn referenced(&self) -> ConditionMask {
        self.code
            .iter()
            .take(self.len)
            .fold(ConditionMask::ZERO, |mask, instruction| {
                if let Instruction::Bit(bit) = instruction {
                    mask.with_bit(u32::from(*bit))
                } else {
                    mask
                }
            })
    }

    /// Stable bytes for the parameter identity term and saved descriptors.
    #[must_use]
    pub fn encode(&self) -> [u8; ENCODED_LEN] {
        let mut result = [0_u8; ENCODED_LEN];
        let count = u16::try_from(self.len).unwrap_or(u16::MAX).to_le_bytes();
        for (target, byte) in result
            .iter_mut()
            .zip(VERSION.to_le_bytes().into_iter().chain(count))
        {
            *target = byte;
        }
        let Some(body) = result.get_mut(4..) else {
            return result;
        };
        for (chunk, instruction) in body.chunks_exact_mut(3).zip(self.code.iter()) {
            let bytes = match instruction {
                Instruction::Pad => [0, 0, 0],
                Instruction::Bit(bit) => {
                    let b = bit.to_le_bytes();
                    [1, b[0], b[1]]
                }
                Instruction::Not => [2, 0, 0],
                Instruction::And => [3, 0, 0],
                Instruction::Or => [4, 0, 0],
            };
            for (target, byte) in chunk.iter_mut().zip(bytes) {
                *target = byte;
            }
        }
        result
    }
}

impl std::fmt::Display for Expression {
    /// Canonical fully parenthesized numeric infix, rendered iteratively.
    /// The fixed wire grammar can exceed the source parser's nesting/byte
    /// limits; display is lossless human text, not a promise to bypass them.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        render_program(self, &render_children(self)?, f)
    }
}

/// With L leaves, U unary and B binary nodes, L = B + 1 and the renderer visits
/// L + 3U + 4B = 3N - B - 2 tokens. Keep the 3N bound even if an internal child
/// link is corrupt; no malformed traversal can keep writing indefinitely.
fn render_program(
    expression: &Expression,
    children: &[(u16, u16); MAX_INSTRUCTIONS],
    f: &mut impl std::fmt::Write,
) -> std::fmt::Result {
    let mut work = [RenderToken::Text(Punctuation::Close); MAX_INSTRUCTIONS * 3];
    let mut used = 0_usize;
    push_render(
        &mut work,
        &mut used,
        &[RenderToken::Node(
            u16::try_from(expression.len.checked_sub(1).ok_or(std::fmt::Error)?)
                .map_err(|_| std::fmt::Error)?,
        )],
    )?;
    for _ in 0..MAX_INSTRUCTIONS * 3 {
        if used == 0 {
            return Ok(());
        }
        used -= 1;
        match *work.get(used).ok_or(std::fmt::Error)? {
            RenderToken::Text(text) => f.write_str(match text {
                Punctuation::Close => ")",
                Punctuation::Negate => "!(",
                Punctuation::And => " & ",
                Punctuation::Or => " | ",
                Punctuation::Open => "(",
            })?,
            RenderToken::Node(index) => {
                let index = usize::from(index);
                let pair = children.get(index).ok_or(std::fmt::Error)?;
                match expression.code.get(index).ok_or(std::fmt::Error)? {
                    Instruction::Bit(bit) => write!(f, "{bit}")?,
                    Instruction::Not => push_render(
                        &mut work,
                        &mut used,
                        &[
                            RenderToken::Text(Punctuation::Close),
                            RenderToken::Node(pair.0),
                            RenderToken::Text(Punctuation::Negate),
                        ],
                    )?,
                    op @ (Instruction::And | Instruction::Or) => push_render(
                        &mut work,
                        &mut used,
                        &[
                            RenderToken::Text(Punctuation::Close),
                            RenderToken::Node(pair.1),
                            RenderToken::Text(if *op == Instruction::And {
                                Punctuation::And
                            } else {
                                Punctuation::Or
                            }),
                            RenderToken::Node(pair.0),
                            RenderToken::Text(Punctuation::Open),
                        ],
                    )?,
                    Instruction::Pad => return Err(std::fmt::Error),
                }
            }
        }
    }
    Err(std::fmt::Error)
}

#[derive(Clone, Copy)]
enum RenderToken {
    Node(u16),
    Text(Punctuation),
}

#[derive(Clone, Copy)]
enum Punctuation {
    Close,
    Negate,
    And,
    Or,
    Open,
}

fn push_render(
    work: &mut [RenderToken],
    used: &mut usize,
    tokens: &[RenderToken],
) -> std::fmt::Result {
    for &token in tokens {
        *work.get_mut(*used).ok_or(std::fmt::Error)? = token;
        *used += 1;
    }
    Ok(())
}

fn render_children(
    expression: &Expression,
) -> Result<[(u16, u16); MAX_INSTRUCTIONS], std::fmt::Error> {
    let mut children = [(0, 0); MAX_INSTRUCTIONS];
    let mut stack = [0_u16; MAX_INSTRUCTIONS];
    let mut used = 0_usize;
    for (index, instruction) in expression.code.iter().take(expression.len).enumerate() {
        let pair = children.get_mut(index).ok_or(std::fmt::Error)?;
        let index = u16::try_from(index).map_err(|_| std::fmt::Error)?;
        match instruction {
            Instruction::Bit(_) => {
                *stack.get_mut(used).ok_or(std::fmt::Error)? = index;
                used += 1;
            }
            Instruction::Not => {
                let top = used.checked_sub(1).ok_or(std::fmt::Error)?;
                let slot = stack.get_mut(top).ok_or(std::fmt::Error)?;
                pair.0 = *slot;
                *slot = index;
            }
            Instruction::And | Instruction::Or => {
                used = used.checked_sub(1).ok_or(std::fmt::Error)?;
                pair.1 = *stack.get(used).ok_or(std::fmt::Error)?;
                let top = used.checked_sub(1).ok_or(std::fmt::Error)?;
                let slot = stack.get_mut(top).ok_or(std::fmt::Error)?;
                pair.0 = *slot;
                *slot = index;
            }
            Instruction::Pad => return Err(std::fmt::Error),
        }
    }
    if used != 1 {
        return Err(std::fmt::Error);
    }
    Ok(children)
}

struct Parser<'a> {
    source: &'a [u8],
    at: usize,
    expression: Expression,
}

impl Parser<'_> {
    fn space(&mut self) {
        self.advance_while(u8::is_ascii_whitespace);
    }

    /// Measure a bounded byte prefix before changing the position. Advancing
    /// an iterator cannot revisit the same input byte if a cursor is damaged.
    fn advance_while(&mut self, accepts: impl Fn(&u8) -> bool) {
        self.at += self.source.get(self.at..).map_or(0, |tail| {
            tail.iter().take_while(|byte| accepts(byte)).count()
        });
    }

    fn eat(&mut self, byte: u8) -> bool {
        self.space();
        if self.source.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn emit(&mut self, instruction: Instruction) -> Result<(), Refusal> {
        let slot = self
            .expression
            .code
            .get_mut(self.expression.len)
            .ok_or(Refusal::InstructionCapacity)?;
        *slot = instruction;
        self.expression.len += 1;
        Ok(())
    }

    fn binary(
        &mut self,
        start: usize,
        middle: usize,
        instruction: Instruction,
    ) -> Result<(), Refusal> {
        let end = self.expression.len;
        let code = self
            .expression
            .code
            .get_mut(start..end)
            .ok_or(Refusal::Syntax)?;
        let boundary = middle.checked_sub(start).ok_or(Refusal::Syntax)?;
        let (left, right) = code.split_at_mut_checked(boundary).ok_or(Refusal::Syntax)?;
        if left > right {
            code.rotate_left(boundary);
        }
        self.emit(instruction)
    }

    fn disjunction(&mut self, depth: u16) -> Result<(), Refusal> {
        let start = self.expression.len;
        self.conjunction(depth)?;
        while self.eat(b'|') {
            let middle = self.expression.len;
            self.conjunction(depth)?;
            self.binary(start, middle, Instruction::Or)?;
        }
        Ok(())
    }

    fn conjunction(&mut self, depth: u16) -> Result<(), Refusal> {
        let start = self.expression.len;
        self.operand(depth)?;
        while self.eat(b'&') {
            let middle = self.expression.len;
            self.operand(depth)?;
            self.binary(start, middle, Instruction::And)?;
        }
        Ok(())
    }

    fn operand(&mut self, depth: u16) -> Result<(), Refusal> {
        if depth >= MAX_NESTING {
            return Err(Refusal::NestingCapacity);
        }
        if self.eat(b'!') {
            self.operand(depth + 1)?;
            return self.emit(Instruction::Not);
        }
        if self.eat(b'(') {
            self.disjunction(depth + 1)?;
            return if self.eat(b')') {
                Ok(())
            } else {
                Err(Refusal::Syntax)
            };
        }
        self.space();
        let start = self.at;
        self.advance_while(|b| b.is_ascii_alphanumeric() || *b == b'_');
        if start == self.at {
            return Err(Refusal::Syntax);
        }
        let token = self
            .source
            .get(start..self.at)
            .and_then(|b| core::str::from_utf8(b).ok())
            .ok_or(Refusal::Syntax)?;
        let bit = token
            .parse::<u16>()
            .ok()
            .or_else(|| {
                table::TABLE
                    .iter()
                    .find(|row| row.name == token)
                    .map(|row| row.index)
            })
            .ok_or(Refusal::UnavailableBit)?;
        let definition = table::definition(bit).ok_or(Refusal::UnavailableBit)?;
        if definition.status != table::BitStatus::Live {
            return Err(Refusal::UnavailableBit);
        }
        self.emit(Instruction::Bit(bit))
    }
}

#[cfg(test)]
mod invariant_tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;

    fn program(code: &[Instruction]) -> Expression {
        let mut expression = Expression {
            code: [Instruction::Pad; MAX_INSTRUCTIONS],
            len: code.len(),
        };
        expression.code[..code.len()].copy_from_slice(code);
        expression
    }

    #[test]
    fn malformed_internal_stack_never_turns_an_existing_true_bit_into_a_signal() {
        let known = ConditionMask::ZERO.with_bit(0);
        for code in [
            &[][..],
            &[Instruction::Not],
            &[Instruction::And],
            &[Instruction::Or],
            &[Instruction::Pad],
            &[Instruction::Bit(0), Instruction::And],
            &[Instruction::Bit(0), Instruction::Or],
            &[Instruction::Bit(0), Instruction::Pad],
            &[Instruction::Bit(0), Instruction::Bit(0)],
        ] {
            let expression = program(code);
            assert_eq!(
                expression.evaluate(known, known),
                Truth::Unknown,
                "{code:?}"
            );
            assert!(render_children(&expression).is_err(), "{code:?}");
            assert!(
                Expression::decode(&expression.encode()).is_err(),
                "{code:?}"
            );
        }
    }

    #[test]
    fn rendering_refuses_bad_metadata_and_cyclic_links_with_fixed_work() {
        let mut expression = program(&[Instruction::Bit(0), Instruction::Not]);
        let valid = render_children(&expression).expect("valid tree");
        for len in [0, MAX_INSTRUCTIONS + 1, usize::MAX] {
            expression.len = len;
            assert!(render_program(&expression, &valid, &mut String::new()).is_err());
        }
        expression = program(&[Instruction::Pad]);
        assert!(render_program(&expression, &valid, &mut String::new()).is_err());
        expression = program(&[Instruction::Bit(0), Instruction::Not]);
        let mut children = valid;
        children[1].0 = u16::MAX;
        assert!(render_program(&expression, &children, &mut String::new()).is_err());
        children[1].0 = 1;
        let mut output = String::new();
        assert!(render_program(&expression, &children, &mut output).is_err());
        assert!(!output.is_empty());
        assert!(output.len() <= MAX_INSTRUCTIONS * 3 * 2);
    }

    #[test]
    fn renderer_stack_capacity_refuses_at_the_first_unavailable_slot() {
        let mut work = [RenderToken::Node(0); 2];
        let mut used = 0;
        assert!(push_render(&mut work, &mut used, &[]).is_ok());
        assert_eq!(used, 0);
        assert!(push_render(&mut work, &mut used, &[RenderToken::Node(7)]).is_ok());
        assert_eq!(used, 1);
        assert!(matches!(work[0], RenderToken::Node(7)));
        assert!(
            push_render(
                &mut work,
                &mut used,
                &[RenderToken::Node(8), RenderToken::Node(9)]
            )
            .is_err()
        );
        assert_eq!(used, 2);
        assert!(matches!(work[1], RenderToken::Node(8)));
    }

    #[test]
    fn invalid_parser_boundaries_refuse_without_rotating_the_program() {
        for (start, middle) in [(3, 3), (1, 0), (0, 3)] {
            let before = program(&[Instruction::Bit(0), Instruction::Bit(63)]);
            let mut parser = Parser {
                source: b"",
                at: 0,
                expression: before.clone(),
            };
            assert_eq!(
                parser.binary(start, middle, Instruction::And),
                Err(Refusal::Syntax)
            );
            assert_eq!(parser.expression, before);
        }
        let mut parser = Parser {
            source: b"0",
            at: usize::MAX,
            expression: program(&[]),
        };
        parser.space();
        assert_eq!(parser.at, usize::MAX);
        assert_eq!(parser.operand(0), Err(Refusal::Syntax));
    }

    #[test]
    fn equal_sibling_rotation_is_byte_equivalent_and_does_not_change_identity() {
        for op in [Instruction::And, Instruction::Or] {
            let sibling = [Instruction::Bit(0), Instruction::Not];
            let mut parser = Parser {
                source: b"",
                at: 0,
                expression: program(&[sibling, sibling].concat()),
            };
            let before = parser.expression.clone();
            parser
                .binary(0, sibling.len(), op)
                .expect("valid equal siblings");
            let mut rotated = before;
            rotated.code[..rotated.len].rotate_left(sibling.len());
            rotated.code[rotated.len] = op;
            rotated.len += 1;
            assert_eq!(parser.expression, rotated);
            assert_eq!(parser.expression.encode(), rotated.encode());
            assert_eq!(Expression::decode(&rotated.encode()), Ok(rotated));
        }
    }
}

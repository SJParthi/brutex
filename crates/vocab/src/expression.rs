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
/// Bits in one scratch word: a program whose stack never stands taller than
/// this evaluates in one word of each plane.
const WORD_SLOTS: usize = 64;
/// Scratch words for every taller program: the deepest stack any program of
/// [`MAX_INSTRUCTIONS`] can reach is `MAX_INSTRUCTIONS.div_ceil(2)` = 576
/// values, because a height of `h` needs `h` leaves and `h - 1` joins to come
/// back to one value, so `2h - 1 <= MAX_INSTRUCTIONS`. Nine words hold 576.
const DEEP_WORDS: usize = MAX_INSTRUCTIONS.div_ceil(2).div_ceil(WORD_SLOTS);

/// The two scratch widths [`Expression::evaluate`] dispatches on, chosen from
/// the program's own stack HEIGHT, not its length (o1engine-22, D-4484).
///
/// AN ENUM, NOT A MATCH ON THE WIDTH (D-1455). An exhaustive match on this
/// enum has no wildcard arm for a deleted case to fall into, and each arm
/// takes its width from [`Tier::words`], so an arm and the width it runs are
/// one fact.
///
/// # Why height and not length since D-4484
///
/// The tiers were 8, 64 and 576 one-byte slots chosen from the LENGTH, so a
/// 599-instruction AND of 300 conditions -- whose stack never stands taller
/// than two -- cleared 576 bytes on every bar. The height is measured once,
/// when the program is built ([`Expression::from_parts`]), and a stack of up
/// to 64 values is one 64-bit word per plane: nothing is cleared but two
/// registers. Only a program that really stacks past 64 values -- 129
/// instructions at the least -- clears the nine words of the deep tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    /// One word per plane: heights up to [`WORD_SLOTS`].
    Word,
    /// [`DEEP_WORDS`] words per plane: every height a program can reach.
    Deep,
}

impl Tier {
    /// The smallest tier a stack of `height` values fits.
    const fn of(height: usize) -> Self {
        if height <= WORD_SLOTS {
            Self::Word
        } else {
            Self::Deep
        }
    }

    /// The scratch words per plane this tier clears before a walk.
    const fn words(self) -> usize {
        match self {
            Self::Word => 1,
            Self::Deep => DEEP_WORDS,
        }
    }
}

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
    /// The tallest the evaluation stack stands, measured once by
    /// [`Self::from_parts`]. Derived from `code` and `len`, never encoded: the
    /// wire format and [`VERSION`] are unchanged (D-4484).
    pub(crate) height: usize,
}

/// The tallest the stack of `code[..len]` stands, counting a leaf as one more
/// value, a join as one fewer and a `Not` as neither.
///
/// It stops at the first `Pad` and never goes below zero, so a malformed
/// program gets a number too; [`Expression::evaluate`] refuses such a program
/// as it always has, and the number only chooses the scratch width.
fn peak(code: &[Instruction], len: usize) -> usize {
    let mut used = 0_usize;
    let mut tallest = 0_usize;
    for instruction in code.iter().take(len) {
        match instruction {
            Instruction::Bit(_) => {
                used += 1;
                tallest = tallest.max(used);
            }
            Instruction::Not => {}
            Instruction::And | Instruction::Or => used = used.saturating_sub(1),
            Instruction::Pad => break,
        }
    }
    tallest
}

/// One value of a bit-plane stack: bit `at` of each plane, as `(true, known)`.
///
/// `true` is never set without `known`, so the three [`Truth`] values are
/// `(0, 1)` false, `(1, 1)` true and `(0, 0)` unknown. A slot past the planes
/// reads as unknown; [`Expression::evaluate`] never reads above what it pushed.
fn slot<const WORDS: usize>(planes: &[[u64; WORDS]; 2], at: usize) -> (u64, u64) {
    let word = at / WORD_SLOTS;
    let shift = at % WORD_SLOTS;
    let [truth, known] = planes;
    (
        truth.get(word).map_or(0, |w| (w >> shift) & 1),
        known.get(word).map_or(0, |w| (w >> shift) & 1),
    )
}

/// Writes one value into bit `at` of each plane. `false` when `at` is past
/// the planes: the caller refuses rather than drops it.
fn put<const WORDS: usize>(planes: &mut [[u64; WORDS]; 2], at: usize, value: (u64, u64)) -> bool {
    let word = at / WORD_SLOTS;
    let shift = at % WORD_SLOTS;
    let [truth, known] = planes;
    let (Some(t), Some(k)) = (truth.get_mut(word), known.get_mut(word)) else {
        return false;
    };
    *t = (*t & !(1 << shift)) | (value.0 << shift);
    *k = (*k & !(1 << shift)) | (value.1 << shift);
    true
}

/// Strong Kleene AND on `(true, known)` bit pairs, the bit-plane form of
/// [`Truth::and`]: a known false on either side settles it false, two known
/// trues settle it true, anything else is unknown.
const fn and_bits(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    (a.0 & b.0, (a.1 & b.1) | (a.1 & !a.0) | (b.1 & !b.0))
}

/// Strong Kleene OR on `(true, known)` bit pairs, the bit-plane form of
/// [`Truth::or`]: a true on either side settles it true, two known falses
/// settle it false, anything else is unknown.
const fn or_bits(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    (a.0 | b.0, (a.1 & b.1) | a.0 | b.0)
}

impl Expression {
    /// A program and its measured stack height: the one way any code in this
    /// crate builds an [`Expression`], so the height every evaluation sizes
    /// its scratch from is the height of the code it walks (D-4484).
    pub(crate) fn from_parts(code: &[Instruction; MAX_INSTRUCTIONS], len: usize) -> Self {
        let height = peak(code, len);
        Self {
            code: *code,
            len,
            height,
        }
    }

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
        let mut result = Self::from_parts(&[Instruction::Pad; MAX_INSTRUCTIONS], len);
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
        Ok(Self::from_parts(&result.code, result.len))
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
            expression: Self::from_parts(&[Instruction::Pad; MAX_INSTRUCTIONS], 0),
        };
        parser.disjunction(0)?;
        parser.space();
        if parser.at != parser.source.len() {
            return Err(Refusal::Syntax);
        }
        Ok(Self::from_parts(
            &parser.expression.code,
            parser.expression.len,
        ))
    }

    /// Evaluate one bar in bounded space/time with no heap allocation.
    /// `known` must come from the evaluator's actual availability evidence.
    ///
    /// # Cost per bar
    ///
    /// Θ(`len`): one step per instruction, at most [`MAX_INSTRUCTIONS`]. This is
    /// not O(1) in the program, and cannot be: every instruction can change the
    /// answer. Each step is O(1) -- a leaf reads two mask bits and writes one
    /// bit of each plane, a join reads two values and writes one -- and the
    /// scratch cleared before the walk is two words when the program's stack
    /// never stands taller than 64, which every program shorter than 129
    /// instructions and every AND or OR chain satisfies, and eighteen words
    /// otherwise (D-4484). The per-instruction cost is held flat from 1 to
    /// 1,151 instructions and from height 2 to height 576 by the `FXD-04` and
    /// `FXD-05` rows of `benches/ratio.rs`, and the tier and the bit-plane
    /// logic by
    /// `vocab::expression::invariant_tests::the_scratch_is_sized_to_the_program_height_and_the_deepest_still_evaluates`
    /// and
    /// `vocab::expression::invariant_tests::the_bit_planes_answer_exactly_what_the_kleene_table_answers`.
    #[must_use]
    pub fn evaluate(&self, truth: ConditionMask, known: ConditionMask) -> Truth {
        match Tier::of(self.height) {
            Tier::Word => self.run::<{ Tier::Word.words() }>(truth, known),
            Tier::Deep => self.run::<{ Tier::Deep.words() }>(truth, known),
        }
    }

    /// The postfix walk over two bit planes of `WORDS` words each. A stack
    /// that would overflow, underflow or meet a `Pad` answers
    /// [`Truth::Unknown`], so a malformed program can only refuse a signal,
    /// never create one.
    fn run<const WORDS: usize>(&self, truth: ConditionMask, known: ConditionMask) -> Truth {
        let mut planes = [[0_u64; WORDS]; 2];
        let mut used = 0_usize;
        for instruction in self.code.iter().take(self.len) {
            match instruction {
                Instruction::Bit(bit) => {
                    let is_known = u64::from(known.get(u32::from(*bit)));
                    let is_true = u64::from(truth.get(u32::from(*bit))) & is_known;
                    if !put(&mut planes, used, (is_true, is_known)) {
                        return Truth::Unknown;
                    }
                    used += 1;
                }
                Instruction::Not => {
                    let Some(top) = used.checked_sub(1) else {
                        return Truth::Unknown;
                    };
                    let (is_true, is_known) = slot(&planes, top);
                    put(&mut planes, top, (is_true ^ is_known, is_known));
                }
                Instruction::And | Instruction::Or => {
                    let Some(left) = used.checked_sub(2) else {
                        return Truth::Unknown;
                    };
                    let (a, b) = (slot(&planes, left), slot(&planes, left + 1));
                    let joined = if *instruction == Instruction::And {
                        and_bits(a, b)
                    } else {
                        or_bits(a, b)
                    };
                    put(&mut planes, left, joined);
                    used -= 1;
                }
                Instruction::Pad => return Truth::Unknown,
            }
        }
        match (used, slot(&planes, 0)) {
            (1, (1, _)) => Truth::True,
            (1, (0, 1)) => Truth::False,
            _ => Truth::Unknown,
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
    /// Display is lossless human text, not a parser round trip (D-0751): each
    /// NOT renders as `!(`, two parser nesting levels, and each binary node
    /// adds one level and five bytes, so even a program [`Self::parse`]
    /// accepted can render past its nesting or byte limit. The exact round
    /// trip is [`Self::encode`] and [`Self::decode`].
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
            .or_else(|| table::index_of(token))
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
        let mut padded = [Instruction::Pad; MAX_INSTRUCTIONS];
        padded[..code.len()].copy_from_slice(code);
        Expression::from_parts(&padded, code.len())
    }

    /// The deepest stack a program of `len` instructions can reach: every leaf
    /// first, then every join, then a `Not` if `len` is even.
    fn deepest(len: usize) -> Expression {
        let leaves = len.div_ceil(2);
        let mut code = vec![Instruction::Bit(0); leaves];
        code.extend(std::iter::repeat_n(Instruction::And, leaves - 1));
        if len.is_multiple_of(2) {
            code.push(Instruction::Not);
        }
        assert_eq!(code.len(), len);
        program(&code)
    }

    /// An AND of `leaves` copies of bit 0, left-associated: `2 * leaves - 1`
    /// instructions whose stack never stands taller than two.
    fn chain(leaves: usize) -> Expression {
        let mut code = vec![Instruction::Bit(0)];
        for _ in 1..leaves {
            code.extend([Instruction::Bit(0), Instruction::And]);
        }
        program(&code)
    }

    /// THE SCRATCH IS SIZED TO THE PROGRAM'S HEIGHT, AND THE DEEPEST PROGRAM OF
    /// EVERY TIER STILL EVALUATES. Audit o1engine-22, D-1455, D-4484.
    ///
    /// `evaluate` cleared a 1,151-slot stack on every bar; D-1455 sized it from
    /// the LENGTH, so a 599-instruction chain of height two still cleared 576
    /// bytes. It now sizes it from the HEIGHT measured when the program was
    /// built: up to 64 values is one word per plane, more is nine. The width
    /// must never be below the height (an undersized stack answers `Unknown`,
    /// refusing a real signal). At each tier's edge the deepest program is
    /// evaluated: all-true leaves give `True` (or `False` under a trailing
    /// `Not`), a false leaf gives the opposite, and an unknown one `Unknown`.
    #[test]
    fn the_scratch_is_sized_to_the_program_height_and_the_deepest_still_evaluates() {
        assert_eq!(Tier::of(0), Tier::Word);
        assert_eq!(Tier::of(1), Tier::Word);
        assert_eq!(Tier::of(64), Tier::Word);
        assert_eq!(Tier::of(65), Tier::Deep);
        assert_eq!(Tier::of(MAX_INSTRUCTIONS.div_ceil(2)), Tier::Deep);
        assert_eq!(Tier::Word.words(), 1);
        assert_eq!(Tier::Deep.words(), 9);
        assert!(
            Tier::Deep.words() * WORD_SLOTS >= MAX_INSTRUCTIONS.div_ceil(2),
            "the deep tier holds the tallest stack a program can build"
        );
        for len in 1..=MAX_INSTRUCTIONS {
            let tallest = deepest(len);
            assert_eq!(tallest.height, len.div_ceil(2), "len {len}");
            assert!(
                Tier::of(tallest.height).words() * WORD_SLOTS >= tallest.height,
                "len {len} can stack past its tier"
            );
        }
        // THE CASE THE LENGTH TIER GOT WRONG: 599 instructions, height two.
        let long_and_shallow = chain(300);
        assert_eq!(long_and_shallow.len, 599);
        assert_eq!(long_and_shallow.height, 2);
        assert_eq!(Tier::of(long_and_shallow.height), Tier::Word);

        let known = ConditionMask::ZERO.with_bit(0);
        for len in [
            1,
            2,
            127,
            128,
            129,
            130,
            131,
            MAX_INSTRUCTIONS - 1,
            MAX_INSTRUCTIONS,
        ] {
            let expression = deepest(len);
            let (yes, no) = if len.is_multiple_of(2) {
                (Truth::False, Truth::True)
            } else {
                (Truth::True, Truth::False)
            };
            assert_eq!(expression.evaluate(known, known), yes, "len {len}");
            assert_eq!(
                expression.evaluate(ConditionMask::ZERO, known),
                no,
                "len {len}"
            );
            assert_eq!(
                expression.evaluate(known, ConditionMask::ZERO),
                Truth::Unknown,
                "len {len}"
            );
        }
        assert_eq!(long_and_shallow.evaluate(known, known), Truth::True);
        assert_eq!(
            long_and_shallow.evaluate(ConditionMask::ZERO, known),
            Truth::False
        );
        // Past the deepest tier's reach is refused, never read as a signal:
        // 577 bare leaves stack one value past nine words.
        let overflow = program(&[Instruction::Bit(0); 577]);
        assert_eq!(overflow.height, 577);
        assert_eq!(overflow.evaluate(known, known), Truth::Unknown);
        // And a program built with too small a height for its code -- only a
        // test can build one -- refuses at the first push past its tier.
        let mut short = program(&[Instruction::Bit(0); 65]);
        short.height = 1;
        assert_eq!(short.evaluate(known, known), Truth::Unknown);
    }

    /// `peak` counts a leaf up, a join down and a `Not` neither, never below
    /// zero, and stops at the first `Pad`.
    #[test]
    fn the_height_is_the_tallest_the_stack_stands() {
        use Instruction::{And, Bit, Not, Or, Pad};
        for (code, height) in [
            (&[][..], 0),
            (&[Bit(0)][..], 1),
            (&[Bit(0), Not][..], 1),
            (&[Bit(0), Bit(1), And][..], 2),
            (&[Bit(0), Bit(1), Or, Bit(2), And][..], 2),
            (&[Bit(0), Bit(1), Bit(2), And, And][..], 3),
            (&[And, Or, Bit(0)][..], 1),
            (&[Bit(0), Bit(1), Pad, Bit(2), Bit(3)][..], 2),
        ] {
            assert_eq!(peak(code, code.len()), height, "{code:?}");
            assert_eq!(program(code).height, height, "{code:?}");
        }
        assert_eq!(peak(&[Bit(0), Bit(1), Bit(2)], 2), 2, "only `len` is read");
    }

    /// The reference the bit planes are checked against: the byte-per-value
    /// walk over `Truth` that shipped until D-4484, unbounded so it cannot
    /// overflow, with the same refusals.
    fn reference(expression: &Expression, truth: ConditionMask, known: ConditionMask) -> Truth {
        let mut stack: Vec<Truth> = Vec::new();
        for instruction in expression.code.iter().take(expression.len) {
            match instruction {
                Instruction::Bit(bit) => stack.push(if !known.get(u32::from(*bit)) {
                    Truth::Unknown
                } else if truth.get(u32::from(*bit)) {
                    Truth::True
                } else {
                    Truth::False
                }),
                Instruction::Not => match stack.last_mut() {
                    Some(top) => *top = top.negate(),
                    None => return Truth::Unknown,
                },
                Instruction::And | Instruction::Or => {
                    let (Some(right), Some(left)) = (stack.pop(), stack.pop()) else {
                        return Truth::Unknown;
                    };
                    stack.push(if *instruction == Instruction::And {
                        left.and(right)
                    } else {
                        left.or(right)
                    });
                }
                Instruction::Pad => return Truth::Unknown,
            }
        }
        if stack.len() == 1 {
            stack[0]
        } else {
            Truth::Unknown
        }
    }

    /// **The bit planes answer exactly what the Kleene table answers** (D-4484).
    ///
    /// First the three operations on every pair of truth values, then whole
    /// programs: every program of up to five instructions over three bits --
    /// malformed ones included -- and a seeded sample of longer ones, each
    /// against every assignment of false, true and unknown to the three bits,
    /// compared with [`reference`].
    #[test]
    fn the_bit_planes_answer_exactly_what_the_kleene_table_answers() {
        let bits = |t: Truth| match t {
            Truth::False => (0_u64, 1_u64),
            Truth::True => (1, 1),
            Truth::Unknown => (0, 0),
        };
        let all = [Truth::False, Truth::True, Truth::Unknown];
        for a in all {
            let (t, k) = bits(a);
            assert_eq!((t ^ k, k), bits(a.negate()), "not {a:?}");
            for b in all {
                assert_eq!(
                    and_bits(bits(a), bits(b)),
                    bits(a.and(b)),
                    "{a:?} and {b:?}"
                );
                assert_eq!(or_bits(bits(a), bits(b)), bits(a.or(b)), "{a:?} or {b:?}");
            }
        }

        let alphabet = [
            Instruction::Bit(0),
            Instruction::Bit(1),
            Instruction::Bit(2),
            Instruction::Not,
            Instruction::And,
            Instruction::Or,
            Instruction::Pad,
        ];
        let assignments: Vec<(ConditionMask, ConditionMask)> = (0..27_u32)
            .map(|n| {
                (0..3_u32).fold(
                    (ConditionMask::ZERO, ConditionMask::ZERO),
                    |(truth, known), bit| match n / 3_u32.pow(bit) % 3 {
                        0 => (truth, known.with_bit(bit)),
                        1 => (truth.with_bit(bit), known.with_bit(bit)),
                        _ => (truth, known),
                    },
                )
            })
            .collect();
        let check = |code: &[Instruction]| {
            let expression = program(code);
            for &(truth, known) in &assignments {
                assert_eq!(
                    expression.evaluate(truth, known),
                    reference(&expression, truth, known),
                    "{code:?}"
                );
            }
        };
        let mut compared = 0_usize;
        for len in 0..=5_u32 {
            for n in 0..7_usize.pow(len) {
                let code: Vec<Instruction> = (0..len)
                    .map(|at| alphabet[n / 7_usize.pow(at) % 7])
                    .collect();
                check(&code);
                compared += 1;
            }
        }
        let mut seed = 0x5eed_u64;
        for _ in 0..2_000 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let len = usize::try_from(seed >> 54).unwrap_or(0) % 200;
            let code: Vec<Instruction> = (0..len)
                .map(|at| {
                    let draw = seed.rotate_left(u32::try_from(at % 64).unwrap_or(0)) >> 60;
                    // Leaves twice as often as anything else, so long
                    // programs stack high rather than underflow at once.
                    alphabet[usize::try_from(draw).unwrap_or(0) % 9 % 7]
                })
                .collect();
            check(&code);
            compared += 1;
        }
        assert!(compared > 21_000, "the sweep must actually run");
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

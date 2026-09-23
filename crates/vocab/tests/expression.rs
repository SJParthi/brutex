//! Independent truth tables and malformed-input boundaries for expression V1.
use vocab::{
    ConditionMask,
    expression::{Expression, MAX_INSTRUCTIONS, Refusal, Truth},
};

struct LimitedText {
    output: String,
    remaining: usize,
}

impl std::fmt::Write for LimitedText {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let written = text.len().min(self.remaining);
        self.output
            .push_str(text.get(..written).ok_or(std::fmt::Error)?);
        self.remaining -= written;
        if written == text.len() {
            Ok(())
        } else {
            Err(std::fmt::Error)
        }
    }
}

#[test]
fn canonical_display_propagates_each_output_failure_without_partial_success() {
    use std::fmt::Write as _;

    let expression = parse("!0 & (63 | 369)");
    let expected = "(!(0) & (63 | 369))";
    assert_eq!(expression.to_string(), expected);
    for remaining in 0..=expected.len() {
        let mut writer = LimitedText {
            output: String::new(),
            remaining,
        };
        let result = write!(&mut writer, "{expression}");
        assert_eq!(result.is_ok(), remaining == expected.len());
        assert_eq!(
            writer.output,
            expected.get(..remaining).unwrap_or_else(|| unreachable!())
        );
    }
}

#[test]
fn maximum_left_nested_binary_program_renders_every_pending_sibling() {
    let operators = (MAX_INSTRUCTIONS - 1) / 2;
    let mut bytes = [0; vocab::expression::ENCODED_LEN];
    bytes
        .get_mut(..2)
        .unwrap_or_else(|| unreachable!())
        .copy_from_slice(&vocab::expression::VERSION.to_le_bytes());
    bytes
        .get_mut(2..4)
        .unwrap_or_else(|| unreachable!())
        .copy_from_slice(
            &u16::try_from(MAX_INSTRUCTIONS)
                .unwrap_or_else(|_| unreachable!())
                .to_le_bytes(),
        );
    bytes
        .get_mut(4..7)
        .unwrap_or_else(|| unreachable!())
        .copy_from_slice(&[1, 0, 0]);
    for pair in bytes
        .get_mut(7..)
        .unwrap_or_else(|| unreachable!())
        .chunks_exact_mut(6)
    {
        pair.copy_from_slice(&[1, 113, 1, 3, 0, 0]);
    }
    let decoded = Expression::decode(&bytes);
    assert!(decoded.is_ok(), "valid wire: {decoded:?}");
    let expression = decoded.unwrap_or_else(|_| unreachable!());
    assert_eq!(expression.encode(), bytes);
    assert_eq!(
        expression.to_string(),
        format!("{}0{}", "(".repeat(operators), " & 369)".repeat(operators))
    );
    let known = ConditionMask::ZERO.with_bit(0).with_bit(369);
    assert_eq!(expression.evaluate(known, known), Truth::True);
    assert_eq!(
        expression.evaluate(known.without_bit(369), known),
        Truth::False
    );
    assert_eq!(
        expression.evaluate(known, known.without_bit(369)),
        Truth::Unknown
    );
}

fn parse(text: &str) -> Expression {
    let result = Expression::parse(text);
    assert!(result.is_ok(), "{text}: {result:?}");
    let expression = result.unwrap_or_else(|_| unreachable!());
    assert_eq!(
        Expression::decode(&expression.encode()),
        Ok(expression.clone())
    );
    expression
}

fn masks(states: [Truth; 3]) -> (ConditionMask, ConditionMask) {
    states.into_iter().zip([0, 63, 369]).fold(
        (ConditionMask::ZERO, ConditionMask::ZERO),
        |(truth, known), (state, bit)| {
            (
                if state == Truth::True {
                    truth.with_bit(bit)
                } else {
                    truth
                },
                if state == Truth::Unknown {
                    known
                } else {
                    known.with_bit(bit)
                },
            )
        },
    )
}

#[test]
fn every_three_valued_input_matches_an_independent_reference() {
    let expressions = vec![
        parse("0 & 63 | !369"),
        parse("!(0 | 63) & 369"),
        parse("!(0 & !0)"),
        parse("0 | !0"),
    ]
    .into_boxed_slice();
    for a in [Truth::False, Truth::True, Truth::Unknown] {
        for b in [Truth::False, Truth::True, Truth::Unknown] {
            for c in [Truth::False, Truth::True, Truth::Unknown] {
                let (truth, known) = masks([a, b, c]);
                // Independent ordering model: F=0, U=1, T=2; AND=min,
                // OR=max, NOT=2-value. It shares no production truth operator.
                let number = |v| match v {
                    Truth::False => 0_u8,
                    Truth::Unknown => 1,
                    Truth::True => 2,
                };
                let value = |n| match n {
                    0 => Truth::False,
                    1 => Truth::Unknown,
                    _ => Truth::True,
                };
                let (x, y, z) = (number(a), number(b), number(c));
                let expected = [
                    value(x.min(y).max(2 - z)),
                    value((2 - x.max(y)).min(z)),
                    value(2 - x.min(2 - x)),
                    value(x.max(2 - x)),
                ];
                for (expr, want) in expressions.iter().zip(expected) {
                    assert_eq!(expr.evaluate(truth, known), want, "{a:?} {b:?} {c:?}");
                }
            }
        }
    }
}

#[test]
fn spelling_parentheses_and_commutative_siblings_share_identity() {
    for (left, right) in [
        ("0", "close_above_ema20"),
        ("  ( 0 & 63 ) ", "63 & 0"),
        ("!(0 | 63)", "! (63 | 0)"),
    ] {
        assert_eq!(parse(left).encode(), parse(right).encode());
    }
    assert_ne!(parse("0&63").encode(), parse("0|63").encode());
    assert_ne!(parse("!0").encode(), parse("0").encode());
}

#[test]
fn all_live_bits_in_every_word_evaluate_and_nonlive_bits_refuse() {
    for bit in 0..=384_u16 {
        let parsed = Expression::parse(&bit.to_string());
        if vocab::table::definition(bit).is_some_and(|row| row.status == vocab::BitStatus::Live) {
            assert!(parsed.is_ok());
            let expr = parse(&bit.to_string());
            let mask = ConditionMask::ZERO.with_bit(u32::from(bit));
            assert_eq!(expr.referenced(), mask);
            assert_eq!(expr.evaluate(mask, mask), Truth::True);
            assert_eq!(expr.evaluate(ConditionMask::ZERO, mask), Truth::False);
            assert_eq!(expr.evaluate(mask, ConditionMask::ZERO), Truth::Unknown);
        } else {
            assert_eq!(parsed, Err(Refusal::UnavailableBit));
        }
    }
}

#[test]
fn malformed_and_oversized_inputs_refuse_without_partial_programs() {
    for text in [
        "", " ", "0 63", "0 && 63", "0 || 63", "&0", "0|", "()", "(0", "0)", "0+63", "0🚫", "!",
        "0;1",
    ] {
        assert_eq!(Expression::parse(text), Err(Refusal::Syntax), "{text}");
    }
    for text in [
        "NSE_INDIAVIX",
        "unknown",
        "65535",
        "65536",
        "99999999999999999999",
    ] {
        assert_eq!(Expression::parse(text), Err(Refusal::UnavailableBit));
    }
    assert_eq!(
        Expression::parse(&"!".repeat(4097)),
        Err(Refusal::SourceCapacity)
    );
    assert_eq!(parse(&format!("0{}", " ".repeat(4095))), parse("0"));
    assert_eq!(
        parse(&format!("{}0{}", "(".repeat(31), ")".repeat(31))),
        parse("0")
    );
    assert_eq!(
        Expression::parse(&format!("{}0", "!".repeat(32))),
        Err(Refusal::NestingCapacity)
    );
    for source in [
        format!("{}0{}", "(".repeat(32), ")".repeat(32)),
        format!("{}0{}", "!(".repeat(16), ")".repeat(16)),
    ] {
        assert_eq!(Expression::parse(&source), Err(Refusal::NestingCapacity));
    }
    let boundary = std::iter::repeat_n("0", MAX_INSTRUCTIONS.div_ceil(2))
        .collect::<Vec<_>>()
        .join("|");
    let boundary_program = parse(&boundary);
    let mut oversized = boundary_program.encode();
    for (slot, byte) in oversized.iter_mut().skip(2).take(2).zip(
        u16::try_from(MAX_INSTRUCTIONS + 1)
            .unwrap_or_else(|_| unreachable!())
            .to_le_bytes(),
    ) {
        *slot = byte;
    }
    assert_eq!(Expression::decode(&oversized), Err(Refusal::Encoding));
    assert_eq!(
        Expression::parse(&format!("{boundary}|0")),
        Err(Refusal::InstructionCapacity)
    );
}

#[test]
fn every_live_condition_fits_together_with_individual_negation() {
    let live: Vec<_> = vocab::table::TABLE
        .iter()
        .filter(|row| row.status == vocab::BitStatus::Live)
        .map(|row| row.index)
        .collect();
    let known = live.iter().fold(ConditionMask::ZERO, |mask, bit| {
        mask.with_bit(u32::from(*bit))
    });
    for separator in ["&", "|"] {
        let positive = parse(
            &live
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(separator),
        );
        let negative = parse(
            &live
                .iter()
                .map(|bit| format!("!{bit}"))
                .collect::<Vec<_>>()
                .join(separator),
        );
        assert_eq!(positive.referenced(), known);
        assert_eq!(negative.referenced(), known);
        assert_eq!(positive.evaluate(known, known), Truth::True);
        assert_eq!(negative.evaluate(ConditionMask::ZERO, known), Truth::True);
        assert_eq!(positive.evaluate(ConditionMask::ZERO, known), Truth::False);
        assert_eq!(negative.evaluate(known, known), Truth::False);
        assert_eq!(
            negative.evaluate(ConditionMask::ZERO, ConditionMask::ZERO),
            Truth::Unknown
        );
    }
}

#[test]
fn persisted_programs_reject_bad_versions_stack_shapes_operands_and_padding() {
    let base = parse("0 | !369").encode();
    for (offset, byte) in [
        (0, 2),
        (1, 1),
        (2, 0),
        (3, 255),
        (4, 0),
        (4, 2),
        (4, 3),
        (4, 4),
        (4, 5),
        (12, 1),
        (15, 1),
    ] {
        let mut bytes = base;
        *bytes.get_mut(offset).unwrap_or_else(|| unreachable!()) = byte;
        assert!(
            Expression::decode(&bytes).is_err(),
            "offset {offset}, byte {byte}"
        );
    }
    let mut padding = base;
    *padding.last_mut().unwrap_or_else(|| unreachable!()) = 1;
    assert_eq!(Expression::decode(&padding), Err(Refusal::Encoding));
    let mut unused = parse("0").encode();
    *unused.get_mut(5).unwrap_or_else(|| unreachable!()) = 255;
    *unused.get_mut(6).unwrap_or_else(|| unreachable!()) = 255;
    assert_eq!(Expression::decode(&unused), Err(Refusal::UnavailableBit));
    let mut extra_stack = parse("0 | 63").encode();
    *extra_stack.get_mut(2).unwrap_or_else(|| unreachable!()) = 2;
    for byte in extra_stack.iter_mut().skip(10).take(3) {
        *byte = 0;
    }
    assert_eq!(Expression::decode(&extra_stack), Err(Refusal::Encoding));
    let mut reversed = parse("0 | 63").encode();
    *reversed.get_mut(5).unwrap_or_else(|| unreachable!()) = 63;
    *reversed.get_mut(8).unwrap_or_else(|| unreachable!()) = 0;
    assert_eq!(Expression::decode(&reversed), Err(Refusal::Encoding));
}

#[test]
fn every_short_encoded_program_agrees_with_independent_infix_reconstruction() {
    // Different algorithms meet at the descriptor: the wire decoder uses
    // segment offsets; this reference constructs parenthesized source trees.
    let alphabet = [
        [1, 0, 0],
        [1, 63, 0],
        [2, 0, 0],
        [3, 0, 0],
        [4, 0, 0],
        [0, 0, 0],
    ];
    let mut accepted = 0;
    let mut refused = 0;
    for len in 1..=4_u32 {
        for mut seed in 0..6_usize.pow(len) {
            let mut bytes = [0_u8; vocab::expression::ENCODED_LEN];
            for (slot, byte) in bytes.iter_mut().zip(
                1_u16.to_le_bytes().into_iter().chain(
                    u16::try_from(len)
                        .unwrap_or_else(|_| unreachable!())
                        .to_le_bytes(),
                ),
            ) {
                *slot = byte;
            }
            let mut stack = Vec::<String>::new();
            let mut valid = true;
            for slot in bytes
                .iter_mut()
                .skip(4)
                .collect::<Vec<_>>()
                .chunks_mut(3)
                .take(usize::try_from(len).unwrap_or_else(|_| unreachable!()))
            {
                let choice = seed % 6;
                seed /= 6;
                let instruction = alphabet.get(choice).unwrap_or_else(|| unreachable!());
                for (target, byte) in slot.iter_mut().zip(instruction) {
                    **target = *byte;
                }
                match choice {
                    0 | 1 => stack.push(if choice == 0 { "0" } else { "63" }.to_owned()),
                    2 => {
                        if let Some(value) = stack.pop() {
                            stack.push(format!("!({value})"));
                        } else {
                            valid = false;
                        }
                    }
                    3 | 4 => {
                        if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                            let op = if choice == 3 { '&' } else { '|' };
                            stack.push(format!("({left}){op}({right})"));
                        } else {
                            valid = false;
                        }
                    }
                    _ => valid = false,
                }
            }
            let expected = valid
                && stack.len() == 1
                && stack
                    .first()
                    .and_then(|source| Expression::parse(source).ok())
                    .is_some_and(|expression| expression.encode() == bytes);
            let actual = Expression::decode(&bytes);
            assert_eq!(
                actual.is_ok(),
                expected,
                "length {len}, bytes {:?}",
                bytes.get(..16)
            );
            if let Ok(expression) = actual {
                assert_eq!(expression.encode(), bytes);
                accepted += 1;
            } else {
                refused += 1;
            }
        }
    }
    assert_eq!(accepted + refused, 1554);
    assert!(accepted > 0 && refused > accepted);
}

#[test]
fn canonical_display_round_trips_normal_programs_and_renders_maximum_wire_without_recursion()
-> Result<(), String> {
    use std::fmt::Write as _;
    struct Refusing;
    impl std::fmt::Write for Refusing {
        fn write_str(&mut self, _: &str) -> std::fmt::Result {
            Err(std::fmt::Error)
        }
    }
    for source in [
        "369 | !0",
        "(0 | 63) & !(63 & 369)",
        "!(!0)",
        "0 & 63 & 369",
    ] {
        let expression = parse(source);
        let rendered = expression.to_string();
        assert_eq!(
            Expression::parse(&rendered).map_err(|e| format!("{e:?}"))?,
            expression
        );
        assert_eq!(
            rendered,
            Expression::decode(&expression.encode())
                .map_err(|e| format!("{e:?}"))?
                .to_string()
        );
    }
    let mut wire = [0; vocab::expression::ENCODED_LEN];
    wire.get_mut(..2)
        .ok_or("version")?
        .copy_from_slice(&vocab::expression::VERSION.to_le_bytes());
    wire.get_mut(2..4).ok_or("length")?.copy_from_slice(
        &u16::try_from(MAX_INSTRUCTIONS)
            .map_err(|e| e.to_string())?
            .to_le_bytes(),
    );
    wire.get_mut(4..7)
        .ok_or("bit")?
        .copy_from_slice(&[1, 113, 1]);
    for op in wire.get_mut(7..).ok_or("ops")?.chunks_exact_mut(3) {
        *op.first_mut().ok_or("opcode")? = 2;
    }
    let expression = Expression::decode(&wire).map_err(|e| format!("{e:?}"))?;
    let rendered = expression.to_string();
    assert_eq!(
        rendered,
        format!(
            "{}369{}",
            "!(".repeat(MAX_INSTRUCTIONS - 1),
            ")".repeat(MAX_INSTRUCTIONS - 1)
        )
    );
    assert_eq!(Expression::parse(&rendered), Err(Refusal::NestingCapacity));
    assert!(write!(&mut Refusing, "{expression}").is_err());
    Ok(())
}

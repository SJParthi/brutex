//! The forty lines of JSON this crate needs, in both directions.
//!
//! # What is here and what is deliberately not
//!
//! Writing: the escape table RFC 8259 §7 names, applied byte-wise to a `&str`.
//! Byte-wise is correct and is not a shortcut — every byte of a multi-byte
//! UTF-8 sequence is `>= 0x80`, so it falls through the table untouched and
//! the sequence survives intact.
//!
//! Reading: a scanner over exactly the shape [`crate::record::Record::decode`]
//! expects — an object whose values are scalars or one nested object of
//! scalars. It is **not** a general JSON parser and does not pretend to be:
//! there is no array support beyond skipping one, no streaming, no
//! `serde::Deserialize`. What it does do is refuse anything it does not
//! understand by name rather than guessing, because a log line that decodes
//! into plausible-but-wrong values is worse than one that refuses.
//!
//! # Why not `serde_json`
//!
//! It is already in the workspace and `crates/pull` already takes it, so this
//! is a real choice and not an oversight. Three reasons, in order of weight:
//!
//! 1. **The dependency is the product.** This crate is meant to be taken by
//!    everything, including `core`, whose dependency table is deliberately
//!    empty, and `web`, which compiles to wasm32. One line in `Cargo.toml`
//!    here is a line in every consumer's graph.
//! 2. **`serde_json` serialises a value; this serialises a line.** The
//!    idiomatic path builds a `Map<String, Value>` per event — one allocation
//!    for the map, one per string key — for a struct that is discarded
//!    microseconds later. [`crate::encode`] writes straight into a buffer the
//!    sink reuses, so steady-state emission allocates nothing.
//! 3. **The surface is small enough to pin.** The escape table is nine arms
//!    and the tests below assert every one of them byte-for-byte, including
//!    the `\u00XX` fallback for the control characters that have no short
//!    form.
//!
//! The cost of the choice is stated where it belongs: this scanner is the one
//! place in the crate where a subtle bug would be invisible, which is why the
//! surrogate arms below are refusals rather than best-effort repairs.

/// Why one line could not be believed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineFault {
    /// The bytes ended in the middle of something.
    Truncated,
    /// A byte that cannot start what was expected here.
    Unexpected {
        /// How far into the line, in bytes.
        at: usize,
        /// The byte found.
        found: u8,
    },
    /// A `\u` escape whose four characters are not hexadecimal.
    BadEscape {
        /// How far into the line, in bytes.
        at: usize,
    },
    /// A surrogate half with no partner, which encodes no character.
    LoneSurrogate {
        /// The code unit found.
        unit: u32,
    },
    /// A number this build cannot hold in any of its three widths.
    BadNumber {
        /// The text that would not parse.
        text: String,
    },
    /// A string whose bytes are not UTF-8 once unescaped.
    NotText,
    /// A key every line must carry, and this one does not.
    MissingKey {
        /// Which key.
        key: &'static str,
    },
    /// A level word outside the five [`crate::Level`] writes.
    UnknownLevel {
        /// The word found.
        word: String,
    },
    /// Nesting past what this scanner will follow.
    ///
    /// Never produced by this crate's own writer; a line hand-edited into a
    /// thousand-deep structure is refused rather than recursed into.
    TooDeep,
}

impl core::fmt::Display for LineFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Truncated => f.write_str("the line ends mid-value"),
            Self::Unexpected { at, found } => {
                write!(f, "byte {found:#04x} at offset {at} cannot start a value")
            }
            Self::BadEscape { at } => write!(f, "a \\u escape at offset {at} is not hexadecimal"),
            Self::LoneSurrogate { unit } => {
                write!(
                    f,
                    "surrogate {unit:#06x} has no partner and encodes nothing"
                )
            }
            Self::BadNumber { ref text } => write!(f, "{text} is not a number this build holds"),
            Self::NotText => f.write_str("a string is not UTF-8 once unescaped"),
            Self::MissingKey { key } => write!(f, "the line carries no {key}"),
            Self::UnknownLevel { ref word } => write!(f, "{word} is not one of the five levels"),
            Self::TooDeep => f.write_str("nested past what this scanner follows"),
        }
    }
}

impl std::error::Error for LineFault {}

/// How deep a skipped value may nest before it is refused.
const MAX_DEPTH: u32 = 16;

// ---------------------------------------------------------------------------
// writing
// ---------------------------------------------------------------------------

/// `s` as a JSON string, quotes included.
pub(crate) fn push_quoted(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    push_escaped(out, s);
    out.push(b'"');
}

/// The escaped *content* of a JSON string, without its quotes.
pub(crate) fn push_escaped(out: &mut Vec<u8>, s: &str) {
    for byte in s.bytes() {
        match byte {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x09 => out.extend_from_slice(b"\\t"),
            0x0a => out.extend_from_slice(b"\\n"),
            0x0c => out.extend_from_slice(b"\\f"),
            0x0d => out.extend_from_slice(b"\\r"),
            // Every other control character. RFC 8259 §7 requires an escape
            // and gives no short form, so the six-byte one is the only legal
            // spelling.
            0x00..=0x1f => {
                out.extend_from_slice(b"\\u00");
                out.push(hex_digit(byte / 16));
                out.push(hex_digit(byte % 16));
            }
            // Including every byte of every multi-byte UTF-8 sequence, which
            // is why this is byte-wise and still correct.
            _ => out.push(byte),
        }
    }
}

/// One lowercase hexadecimal digit.
const fn hex_digit(nibble: u8) -> u8 {
    match nibble {
        0 => b'0',
        1 => b'1',
        2 => b'2',
        3 => b'3',
        4 => b'4',
        5 => b'5',
        6 => b'6',
        7 => b'7',
        8 => b'8',
        9 => b'9',
        10 => b'a',
        11 => b'b',
        12 => b'c',
        13 => b'd',
        14 => b'e',
        _ => b'f',
    }
}

// ---------------------------------------------------------------------------
// reading
// ---------------------------------------------------------------------------

/// A cursor over one line's bytes.
pub(crate) struct Scan<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Scan<'a> {
    /// A cursor at the start of `bytes`.
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    /// How far in the cursor is, for a fault to name.
    pub(crate) const fn offset(&self) -> usize {
        self.at
    }

    /// The byte under the cursor, without moving it.
    pub(crate) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Moves past one byte.
    pub(crate) fn bump(&mut self) {
        self.at = self.at.saturating_add(1);
    }

    /// Moves past every space, tab, carriage return and newline.
    pub(crate) fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.bump();
        }
    }

    /// Requires the next byte to be `want`, and moves past it.
    pub(crate) fn expect(&mut self, want: u8) -> Result<(), LineFault> {
        match self.peek() {
            Some(byte) if byte == want => {
                self.bump();
                Ok(())
            }
            Some(found) => Err(LineFault::Unexpected { at: self.at, found }),
            None => Err(LineFault::Truncated),
        }
    }

    /// Whether every byte has been consumed once trailing space is skipped.
    pub(crate) fn at_end(&mut self) -> bool {
        self.skip_space();
        self.at >= self.bytes.len()
    }

    /// A JSON string, unescaped.
    pub(crate) fn string(&mut self) -> Result<String, LineFault> {
        self.expect(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let byte = self.peek().ok_or(LineFault::Truncated)?;
            self.bump();
            match byte {
                b'"' => break,
                b'\\' => self.escape(&mut out)?,
                _ => out.push(byte),
            }
        }
        String::from_utf8(out).map_err(|_ignored| LineFault::NotText)
    }

    /// One escape sequence, already past its backslash.
    fn escape(&mut self, out: &mut Vec<u8>) -> Result<(), LineFault> {
        let byte = self.peek().ok_or(LineFault::Truncated)?;
        self.bump();
        let plain = match byte {
            b'"' => Some(b'"'),
            b'\\' => Some(b'\\'),
            b'/' => Some(b'/'),
            b'b' => Some(0x08),
            b'f' => Some(0x0c),
            b'n' => Some(0x0a),
            b'r' => Some(0x0d),
            b't' => Some(0x09),
            b'u' => None,
            found => {
                return Err(LineFault::Unexpected {
                    at: self.at.saturating_sub(1),
                    found,
                });
            }
        };
        if let Some(byte) = plain {
            out.push(byte);
            return Ok(());
        }
        let unit = self.four_hex()?;
        let scalar = if (0xd800..=0xdbff).contains(&unit) {
            // A high surrogate. Only a `\uDC00..=\uDFFF` immediately after it
            // completes a character; anything else encodes nothing and is a
            // refusal rather than a replacement character, because a silently
            // substituted glyph is a line that reads as if it were intact.
            self.expect(b'\\')?;
            self.expect(b'u')?;
            let low = self.four_hex()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(LineFault::LoneSurrogate { unit: low });
            }
            0x1_0000 + ((unit - 0xd800) << 10) + (low - 0xdc00)
        } else if (0xdc00..=0xdfff).contains(&unit) {
            return Err(LineFault::LoneSurrogate { unit });
        } else {
            unit
        };
        // Every value reaching here is a Unicode scalar by construction: below
        // 0xD800, above 0xDFFF and under 0x110000, or a combined pair which is
        // 0x10000..=0x10FFFF. The `ok_or` arm is a backstop no input drives.
        let ch = char::from_u32(scalar).ok_or(LineFault::LoneSurrogate { unit: scalar })?;
        let mut buffer = [0u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
        Ok(())
    }

    /// Four hexadecimal characters as one code unit.
    fn four_hex(&mut self) -> Result<u32, LineFault> {
        let start = self.at;
        let mut unit: u32 = 0;
        for _ in 0..4 {
            let byte = self.peek().ok_or(LineFault::Truncated)?;
            self.bump();
            let nibble = match byte {
                b'0'..=b'9' => u32::from(byte - b'0'),
                b'a'..=b'f' => u32::from(byte - b'a') + 10,
                b'A'..=b'F' => u32::from(byte - b'A') + 10,
                _ => return Err(LineFault::BadEscape { at: start }),
            };
            unit = unit * 16 + nibble;
        }
        Ok(unit)
    }

    /// A literal word — `true`, `false` or `null`.
    pub(crate) fn word(&mut self, want: &[u8]) -> Result<(), LineFault> {
        for byte in want {
            self.expect(*byte)?;
        }
        Ok(())
    }

    /// The bytes of a JSON number, as text.
    pub(crate) fn number_text(&mut self) -> Result<&'a str, LineFault> {
        let start = self.at;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.bump();
        }
        let slice = self.bytes.get(start..self.at).unwrap_or(&[]);
        if slice.is_empty() {
            return Err(LineFault::Truncated);
        }
        // Every byte accepted above is ASCII, so this cannot fail. The arm is a
        // backstop and no input drives it.
        core::str::from_utf8(slice).map_err(|_ignored| LineFault::NotText)
    }

    /// Steps past one value of any shape, keeping nothing.
    ///
    /// This is what makes the format forward-compatible: a key added to the
    /// line by a later build is skipped by an older reader rather than
    /// refused, so one journal written by two builds still renders.
    pub(crate) fn skip_value(&mut self) -> Result<(), LineFault> {
        self.skip_space();
        match self.peek() {
            None => Err(LineFault::Truncated),
            Some(b'"') => self.string().map(|_ignored| ()),
            Some(b't') => self.word(b"true"),
            Some(b'f') => self.word(b"false"),
            Some(b'n') => self.word(b"null"),
            Some(b'{') => self.skip_nested(b'{', b'}', 0),
            Some(b'[') => self.skip_nested(b'[', b']', 0),
            Some(b'-' | b'0'..=b'9') => self.number_text().map(|_ignored| ()),
            Some(found) => Err(LineFault::Unexpected { at: self.at, found }),
        }
    }

    /// Steps past a bracketed run, counting depth and honouring strings.
    fn skip_nested(&mut self, open: u8, close: u8, depth: u32) -> Result<(), LineFault> {
        if depth >= MAX_DEPTH {
            return Err(LineFault::TooDeep);
        }
        self.expect(open)?;
        loop {
            self.skip_space();
            match self.peek() {
                None => return Err(LineFault::Truncated),
                Some(byte) if byte == close => {
                    self.bump();
                    return Ok(());
                }
                Some(b'"') => {
                    self.string()?;
                }
                Some(b'{') => self.skip_nested(b'{', b'}', depth.saturating_add(1))?,
                Some(b'[') => self.skip_nested(b'[', b']', depth.saturating_add(1))?,
                Some(_) => self.bump(),
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{LineFault, Scan, hex_digit, push_escaped, push_quoted};

    fn escaped(s: &str) -> String {
        let mut out = Vec::new();
        push_escaped(&mut out, s);
        String::from_utf8(out).expect("escaping keeps it UTF-8")
    }

    fn read(s: &str) -> Result<String, LineFault> {
        Scan::new(s.as_bytes()).string()
    }

    /// EVERY ESCAPE RFC 8259 NAMES, AND NOTHING ELSE.
    ///
    /// A missing arm here does not fail loudly — it writes a raw control byte
    /// into the middle of a JSON string, which every consumer then refuses,
    /// and the events either side of it are still perfectly readable so
    /// nothing draws attention to the gap.
    #[test]
    fn every_escape_this_crate_writes_is_the_one_rfc8259_names() {
        assert_eq!(escaped("plain"), "plain");
        assert_eq!(escaped("a\"b"), "a\\\"b");
        assert_eq!(escaped("a\\b"), "a\\\\b");
        assert_eq!(escaped("\u{8}"), "\\b");
        assert_eq!(escaped("\t"), "\\t");
        assert_eq!(escaped("\n"), "\\n");
        assert_eq!(escaped("\u{c}"), "\\f");
        assert_eq!(escaped("\r"), "\\r");
        // The control characters with no short form, including the two either
        // side of every short form so an off-by-one in the range is visible.
        assert_eq!(escaped("\u{0}"), "\\u0000");
        assert_eq!(escaped("\u{1}"), "\\u0001");
        assert_eq!(escaped("\u{7}"), "\\u0007");
        assert_eq!(escaped("\u{b}"), "\\u000b");
        assert_eq!(escaped("\u{e}"), "\\u000e");
        assert_eq!(escaped("\u{1f}"), "\\u001f");
        // 0x20 is the first byte that needs nothing.
        assert_eq!(escaped(" "), " ");
        // A forward slash is legal unescaped and is left alone, so a path in a
        // message stays greppable as itself.
        assert_eq!(escaped("/Users/you/.brutex"), "/Users/you/.brutex");
        // Multi-byte UTF-8 passes through whole. Byte-wise escaping is only
        // correct because every continuation byte is >= 0x80.
        assert_eq!(escaped("₹ अ 🙂"), "₹ अ 🙂");
        // And the quotes are added by the other function, not this one.
        let mut out = Vec::new();
        push_quoted(&mut out, "a\nb");
        assert_eq!(String::from_utf8(out).unwrap(), "\"a\\nb\"");
    }

    #[test]
    fn every_hex_digit_has_a_byte_and_the_table_is_lowercase() {
        let all: Vec<u8> = (0..16).map(hex_digit).collect();
        assert_eq!(String::from_utf8(all).unwrap(), "0123456789abcdef");
    }

    /// A `\u` escape, assembled rather than typed.
    ///
    /// The backslash and the `u` are joined here instead of written as one
    /// sequence, so nothing between this file and the compiler can quietly
    /// turn the TEXT of an escape into the CHARACTER it denotes -- which is
    /// precisely the confusion these tests exist to rule out.
    fn u(hex: &str) -> String {
        format!("{}u{hex}", '\\')
    }

    /// A JSON string literal wrapping already-escaped content.
    fn quoted(content: &str) -> String {
        format!("\"{content}\"")
    }

    #[test]
    fn a_string_reads_back_through_every_escape_it_can_carry() {
        assert_eq!(read(r#""plain""#).unwrap(), "plain");
        assert_eq!(read(r#""a\"b""#).unwrap(), "a\"b");
        assert_eq!(read(r#""a\\b""#).unwrap(), "a\\b");
        assert_eq!(read(r#""a\/b""#).unwrap(), "a/b");
        assert_eq!(read(r#""\b\f\n\r\t""#).unwrap(), "\u{8}\u{c}\n\r\t");
        assert_eq!(read(r#""""#).unwrap(), "", "an empty string is a string");

        // The six-byte form, in both hexadecimal cases.
        assert_eq!(read(&quoted(&u("0000"))).unwrap(), "\u{0}");
        assert_eq!(read(&quoted(&u("001f"))).unwrap(), "\u{1f}");
        assert_eq!(read(&quoted(&u("001F"))).unwrap(), "\u{1f}", "uppercase");
        // Past ASCII, which is where the UTF-8 re-encoding starts mattering.
        assert_eq!(read(&quoted(&u("20b9"))).unwrap(), "\u{20b9}", "the rupee");
        // A surrogate PAIR is one character, and the arithmetic that joins the
        // halves is the one place an off-by-one prints a different glyph.
        let pair = format!("{}{}", u("d83d"), u("de42"));
        assert_eq!(read(&quoted(&pair)).unwrap(), "\u{1f642}");
        // And a raw multi-byte character needs no escape at all.
        assert_eq!(read(r#""₹ अ 🙂""#).unwrap(), "₹ अ 🙂");
    }

    #[test]
    fn a_broken_string_is_refused_by_name_rather_than_repaired() {
        assert_eq!(read(r#""unterminated"#), Err(LineFault::Truncated));
        assert_eq!(read(r#""half\"#), Err(LineFault::Truncated));
        assert_eq!(
            read(r#""\q""#),
            Err(LineFault::Unexpected { at: 2, found: b'q' })
        );
        assert_eq!(
            read(&quoted(&u("00zz"))),
            Err(LineFault::BadEscape { at: 3 }),
            "four characters, and they must all be hexadecimal"
        );
        assert_eq!(read(&format!("\"{}", u("00"))), Err(LineFault::Truncated));

        // A LONE SURROGATE ENCODES NOTHING and is refused rather than turned
        // into a replacement character. A substituted glyph would leave a line
        // that reads as though it were intact.
        assert_eq!(
            read(&quoted(&u("dc00"))),
            Err(LineFault::LoneSurrogate { unit: 0xdc00 }),
            "a low half with no high half"
        );
        assert_eq!(
            read(&quoted(&format!("{}{}", u("d83d"), u("0041")))),
            Err(LineFault::LoneSurrogate { unit: 0x41 }),
            "a high half followed by an escape that is not its partner"
        );
        assert_eq!(
            read(&quoted(&format!("{}A", u("d83d")))),
            Err(LineFault::Unexpected { at: 7, found: b'A' }),
            "a high half followed by no escape at all"
        );

        // Bytes that are not UTF-8 at all.
        assert_eq!(
            Scan::new(b"\"\xff\"").string(),
            Err(LineFault::NotText),
            "a raw invalid byte inside a string is refused"
        );
        assert_eq!(
            Scan::new(b"nope").string(),
            Err(LineFault::Unexpected { at: 0, found: b'n' })
        );
        assert_eq!(Scan::new(b"").string(), Err(LineFault::Truncated));
    }

    #[test]
    fn the_cursor_walks_space_words_and_numbers_and_names_what_it_will_not_take() {
        let mut scan = Scan::new(b"  \t\r\n{ ");
        scan.skip_space();
        assert_eq!(scan.offset(), 5);
        assert_eq!(scan.peek(), Some(b'{'));
        assert!(scan.expect(b'{').is_ok());
        assert!(scan.at_end(), "trailing space is not content");

        let mut number = Scan::new(b"-1.25e3,");
        assert_eq!(number.number_text().unwrap(), "-1.25e3");
        assert_eq!(number.peek(), Some(b','));
        assert_eq!(
            Scan::new(b",").number_text(),
            Err(LineFault::Truncated),
            "no digits is not a number"
        );

        assert!(Scan::new(b"true").word(b"true").is_ok());
        assert_eq!(
            Scan::new(b"trve").word(b"true"),
            Err(LineFault::Unexpected { at: 2, found: b'v' })
        );
        assert_eq!(Scan::new(b"tr").word(b"true"), Err(LineFault::Truncated));
    }

    #[test]
    fn an_unknown_value_is_stepped_over_whole_whatever_shape_it_is() {
        for text in [
            r#""a string with a } and a ] in it""#,
            "true",
            "false",
            "null",
            "-12.5e-3",
            r#"{"a":1,"b":{"c":[1,2,"}"]}}"#,
            r#"[1,[2,[3]],{"k":"v"}]"#,
        ] {
            let mut scan = Scan::new(text.as_bytes());
            scan.skip_value()
                .unwrap_or_else(|e| panic!("{text} should skip: {e}"));
            assert!(scan.at_end(), "{text} did not consume the whole value");
        }

        assert_eq!(Scan::new(b"").skip_value(), Err(LineFault::Truncated));
        assert_eq!(Scan::new(b"{").skip_value(), Err(LineFault::Truncated));
        assert_eq!(
            Scan::new(b"@").skip_value(),
            Err(LineFault::Unexpected { at: 0, found: b'@' })
        );
    }

    /// NESTING IS BOUNDED, so a hand-edited line cannot recurse the stack away.
    #[test]
    fn nesting_past_the_bound_is_refused_instead_of_recursing() {
        let deep = format!("{}{}", "[".repeat(64), "]".repeat(64));
        assert_eq!(
            Scan::new(deep.as_bytes()).skip_value(),
            Err(LineFault::TooDeep)
        );
        let fine = format!("{}{}", "[".repeat(8), "]".repeat(8));
        assert!(Scan::new(fine.as_bytes()).skip_value().is_ok());
    }

    #[test]
    fn every_fault_says_something_an_operator_can_act_on() {
        let said = [
            LineFault::Truncated.to_string(),
            LineFault::Unexpected { at: 3, found: 64 }.to_string(),
            LineFault::BadEscape { at: 9 }.to_string(),
            LineFault::LoneSurrogate { unit: 0xd800 }.to_string(),
            LineFault::BadNumber {
                text: "1e999999".to_owned(),
            }
            .to_string(),
            LineFault::NotText.to_string(),
            LineFault::MissingKey { key: "seq" }.to_string(),
            LineFault::UnknownLevel {
                word: "fatal".to_owned(),
            }
            .to_string(),
            LineFault::TooDeep.to_string(),
        ];
        for line in &said {
            assert!(!line.is_empty(), "a fault with no words is not a diagnosis");
        }
        assert!(said[1].contains("offset 3"), "{}", said[1]);
        assert!(said[3].contains("0xd800"), "{}", said[3]);
        assert!(said[4].contains("1e999999"), "{}", said[4]);
        assert!(said[6].contains("seq"), "{}", said[6]);
        assert!(said[7].contains("fatal"), "{}", said[7]);
    }
}

//! One line, read back into the thing that wrote it.
//!
//! The write side ([`crate::Event`]) borrows and the read side owns, because
//! an unescaped string has to live somewhere. Everything else is the same set
//! of facts, and [`Record::matches`] is what lets a test assert the round trip
//! as one equality instead of a walk over the fields.
//!
//! # An unknown key is skipped, not refused
//!
//! A line written by a later build carrying a key this build has never heard
//! of decodes cleanly, minus that key. The alternative — refusing the line —
//! would mean an upgrade in the middle of a backfill blanks every event
//! written before it. `CLAUDE.md` §3 rule 8: a format is appended to, never
//! mutated in place, and a reader that refuses an appended field is a reader
//! that makes appending impossible.
//!
//! A **missing required** key is a different thing and is refused by name.
//! Five keys are required — `seq`, `ms`, `level`, `target`, `msg` — because
//! without any one of them the line cannot be placed, filtered or read.

use crate::event::Event;
use crate::json::{LineFault, Scan};
use crate::level::Level;
use crate::value::OwnedValue;

/// One event, as it comes back off the disk.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// Its position in the stream, counted from the first event this sink
    /// wrote to the file it is currently appending to.
    pub seq: u64,
    /// The run this event belongs to, or zero when it belongs to none.
    ///
    /// **The key a reader groups by.** A log file spanning three backfills is
    /// three interleaved stories, and without this it is one unreadable one.
    /// Zero for an event outside any run — a served request, a startup line —
    /// because the writer omits the key entirely rather than writing a zero a
    /// reader would have to interpret.
    pub run: u64,
    /// When it was written, in milliseconds since the Unix epoch.
    pub at_unix_millis: i64,
    /// The same instant as UTC text, as the line carried it.
    ///
    /// Empty when the line carried no `ts` — which this crate's writer never
    /// does, and a hand-written line might.
    pub at_utc: String,
    /// How loud it is.
    pub level: Level,
    /// Which subsystem it came from.
    pub target: String,
    /// What it says.
    pub message: String,
    /// The typed fields, in the order the line carried them.
    pub fields: Vec<(String, OwnedValue)>,
    /// Whether some text on this line hit its ceiling and was cut.
    pub cut: bool,
    /// How many fields were counted rather than kept.
    pub dropped_fields: u64,
}

impl Record {
    /// One value by key, or `None`.
    ///
    /// A linear walk over at most [`crate::MAX_FIELDS`] entries — twelve —
    /// which is a constant, not a scan. A map here would be an allocation per
    /// record to search twelve things.
    #[must_use]
    pub fn field(&self, key: &str) -> Option<&OwnedValue> {
        self.fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// Whether this record is the event that was written, field for field.
    ///
    /// The comparison the round-trip test is written as. It does **not**
    /// compare `seq`, `at_unix_millis` or `at_utc`: those are stamped by the
    /// sink and are not properties of the event the caller built.
    #[must_use]
    pub fn matches(&self, event: &Event<'_>) -> bool {
        self.level == event.level()
            && self.target == event.target()
            && self.message == event.message()
            && self.fields.len() == event.fields().len()
            && u64::from(event.dropped_fields()) == self.dropped_fields
            && self
                .fields
                .iter()
                .zip(event.fields())
                .all(|((name, got), &(key, want))| name == key && *got == want)
    }

    /// One line, decoded.
    ///
    /// The trailing newline is optional: the tail reader has already split on
    /// it and a hand-`cat`-ed line has one.
    ///
    /// # Errors
    ///
    /// [`LineFault`], naming what was wrong and where. A line that will not
    /// decode is never repaired and never guessed at — the tail reader counts
    /// it and carries on, so one bad line does not blank the page around it.
    pub fn decode(line: &[u8]) -> Result<Self, LineFault> {
        let mut scan = Scan::new(line);
        scan.skip_space();
        scan.expect(b'{')?;

        let mut seq: Option<u64> = None;
        let mut run: u64 = 0;
        let mut millis: Option<i64> = None;
        let mut at_utc = String::new();
        let mut level: Option<Level> = None;
        let mut target: Option<String> = None;
        let mut message: Option<String> = None;
        let mut fields: Vec<(String, OwnedValue)> = Vec::new();
        let mut cut = false;
        let mut dropped_fields = 0u64;

        scan.skip_space();
        if scan.peek() == Some(b'}') {
            scan.bump();
        } else {
            loop {
                scan.skip_space();
                let key = scan.string()?;
                scan.skip_space();
                scan.expect(b':')?;
                scan.skip_space();
                match key.as_str() {
                    "seq" => seq = Some(unsigned(&mut scan)?),
                    // ABSENT IS ZERO, NOT A REFUSAL. Every line this crate has
                    // ever written before today lacks the key, and a decoder
                    // that refused them would make the whole existing log
                    // unreadable the moment the writer changed.
                    "run" => run = unsigned(&mut scan)?,
                    "ms" => millis = Some(signed(&mut scan)?),
                    "ts" => at_utc = scan.string()?,
                    "level" => {
                        let word = scan.string()?;
                        level =
                            Some(Level::of_label(&word).ok_or(LineFault::UnknownLevel { word })?);
                    }
                    "target" => target = Some(scan.string()?),
                    "msg" => message = Some(scan.string()?),
                    "cut" => cut = flag(&mut scan)?,
                    "dropped" => dropped_fields = unsigned(&mut scan)?,
                    "fields" => fields = object(&mut scan)?,
                    // A key a later build added. Stepped over whole.
                    _ => scan.skip_value()?,
                }
                scan.skip_space();
                match scan.peek() {
                    Some(b',') => scan.bump(),
                    Some(b'}') => {
                        scan.bump();
                        break;
                    }
                    Some(found) => {
                        return Err(LineFault::Unexpected {
                            at: scan.offset(),
                            found,
                        });
                    }
                    None => return Err(LineFault::Truncated),
                }
            }
        }
        if !scan.at_end() {
            let found = scan.peek().unwrap_or(0);
            return Err(LineFault::Unexpected {
                at: scan.offset(),
                found,
            });
        }
        Ok(Self {
            seq: seq.ok_or(LineFault::MissingKey { key: "seq" })?,
            run,
            at_unix_millis: millis.ok_or(LineFault::MissingKey { key: "ms" })?,
            at_utc,
            level: level.ok_or(LineFault::MissingKey { key: "level" })?,
            target: target.ok_or(LineFault::MissingKey { key: "target" })?,
            message: message.ok_or(LineFault::MissingKey { key: "msg" })?,
            fields,
            cut,
            dropped_fields,
        })
    }
}

/// The `fields` object: keys to scalars, nothing nested.
fn object(scan: &mut Scan<'_>) -> Result<Vec<(String, OwnedValue)>, LineFault> {
    scan.expect(b'{')?;
    let mut out = Vec::new();
    scan.skip_space();
    if scan.peek() == Some(b'}') {
        scan.bump();
        return Ok(out);
    }
    loop {
        scan.skip_space();
        let key = scan.string()?;
        scan.skip_space();
        scan.expect(b':')?;
        scan.skip_space();
        out.push((key, scalar(scan)?));
        scan.skip_space();
        match scan.peek() {
            Some(b',') => scan.bump(),
            Some(b'}') => {
                scan.bump();
                return Ok(out);
            }
            Some(found) => {
                return Err(LineFault::Unexpected {
                    at: scan.offset(),
                    found,
                });
            }
            None => return Err(LineFault::Truncated),
        }
    }
}

/// One field value.
fn scalar(scan: &mut Scan<'_>) -> Result<OwnedValue, LineFault> {
    match scan.peek() {
        None => Err(LineFault::Truncated),
        Some(b'"') => Ok(OwnedValue::Str(scan.string()?)),
        Some(b't') => scan.word(b"true").map(|()| OwnedValue::Bool(true)),
        Some(b'f') => scan.word(b"false").map(|()| OwnedValue::Bool(false)),
        Some(b'n') => scan.word(b"null").map(|()| OwnedValue::Null),
        Some(b'-' | b'0'..=b'9') => number(scan),
        Some(found) => Err(LineFault::Unexpected {
            at: scan.offset(),
            found,
        }),
    }
}

/// A number, in the narrowest of the three widths that holds it exactly.
///
/// Integer first, then unsigned for the range past [`i64::MAX`], then float.
/// The order matters: taking every number as a float would round a `u64`
/// count past 2^53 into a different number, silently.
fn number(scan: &mut Scan<'_>) -> Result<OwnedValue, LineFault> {
    let text = scan.number_text()?;
    let looks_real = text.bytes().any(|b| matches!(b, b'.' | b'e' | b'E'));
    if !looks_real {
        if let Ok(v) = text.parse::<i64>() {
            return Ok(OwnedValue::Int(v));
        }
        if let Ok(v) = text.parse::<u64>() {
            return Ok(OwnedValue::Uint(v));
        }
    }
    // A DECIMAL PAST THE RANGE OF AN f64 PARSES AS `inf`, WITHOUT AN ERROR.
    // Taking that would put a value on the page that is not the value in the
    // file, and `Infinity` is precisely what this crate refuses to write as a
    // number. Refused by name instead.
    match text.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(OwnedValue::Float(v)),
        _ => Err(LineFault::BadNumber {
            text: text.to_owned(),
        }),
    }
}

/// A `true`/`false` literal.
fn flag(scan: &mut Scan<'_>) -> Result<bool, LineFault> {
    match scan.peek() {
        Some(b't') => scan.word(b"true").map(|()| true),
        Some(b'f') => scan.word(b"false").map(|()| false),
        Some(found) => Err(LineFault::Unexpected {
            at: scan.offset(),
            found,
        }),
        None => Err(LineFault::Truncated),
    }
}

/// A `u64`, refused rather than rounded.
fn unsigned(scan: &mut Scan<'_>) -> Result<u64, LineFault> {
    let text = scan.number_text()?;
    text.parse::<u64>()
        .map_err(|_ignored| LineFault::BadNumber {
            text: text.to_owned(),
        })
}

/// An `i64`, refused rather than rounded.
fn signed(scan: &mut Scan<'_>) -> Result<i64, LineFault> {
    let text = scan.number_text()?;
    text.parse::<i64>()
        .map_err(|_ignored| LineFault::BadNumber {
            text: text.to_owned(),
        })
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
    use super::Record;
    use crate::encode::line;
    use crate::event::Event;
    use crate::json::LineFault;
    use crate::level::Level;
    use crate::value::{OwnedValue, Value};

    fn round_trip(event: &Event<'_>) -> Record {
        let mut out = Vec::new();
        line(&mut out, 9, 1_786_197_791_427, 0, event);
        Record::decode(&out).unwrap_or_else(|e| panic!("its own bytes did not decode: {e}"))
    }

    /// AN EVENT SURVIVES THE FILE, FIELD FOR FIELD.
    ///
    /// Not "a line was produced and it parsed" — every typed value goes out
    /// and comes back as the same type and the same bits. A field written as a
    /// number and read back as a string would still parse, would still render,
    /// and would break every filter written against it.
    #[test]
    fn an_event_round_trips_with_every_field_intact() {
        let event = Event::warn("pull.http", "vendor responded oddly")
            .with("status", 503u32)
            .with("bytes", 81_922usize)
            .with("instrument", "NSE-BANKNIFTY")
            .with("retried", true)
            .with("stalled", false)
            .with("close", Value::paisa(2_512_075))
            .with("drawdown", -0.0625f64)
            .with("whole", 3.0f64)
            .with("expiry", Value::Null)
            .with("huge", u64::MAX)
            .with("floor", i64::MIN)
            .with("note", "he said \"stop\"\nand it did");
        let back = round_trip(&event);

        assert!(back.matches(&event), "{back:?} is not {event:?}");
        assert_eq!(back.seq, 9);
        assert_eq!(back.at_unix_millis, 1_786_197_791_427);
        assert_eq!(back.at_utc, "2026-08-08T14:03:11.427Z");
        assert_eq!(back.level, Level::Warn);
        assert_eq!(back.target, "pull.http");
        assert_eq!(back.message, "vendor responded oddly");
        assert!(!back.cut);
        assert_eq!(back.dropped_fields, 0);

        // And each one by name, so a failure says which.
        assert_eq!(back.field("status").and_then(OwnedValue::as_u64), Some(503));
        assert_eq!(
            back.field("instrument").and_then(OwnedValue::as_str),
            Some("NSE-BANKNIFTY")
        );
        assert_eq!(back.field("retried"), Some(&OwnedValue::Bool(true)));
        assert_eq!(back.field("stalled"), Some(&OwnedValue::Bool(false)));
        assert_eq!(back.field("close"), Some(&OwnedValue::Int(2_512_075)));
        assert_eq!(back.field("drawdown"), Some(&OwnedValue::Float(-0.0625)));
        assert_eq!(
            back.field("whole"),
            Some(&OwnedValue::Float(3.0)),
            "a whole-valued float must not come back as an integer"
        );
        assert_eq!(back.field("expiry"), Some(&OwnedValue::Null));
        assert_eq!(
            back.field("huge"),
            Some(&OwnedValue::Uint(u64::MAX)),
            "a count past i64::MAX keeps its exact value"
        );
        assert_eq!(back.field("floor"), Some(&OwnedValue::Int(i64::MIN)));
        assert_eq!(
            back.field("note").and_then(OwnedValue::as_str),
            Some("he said \"stop\"\nand it did"),
            "the escapes came back as the bytes that went in"
        );
        assert_eq!(back.field("absent"), None);
    }

    #[test]
    fn a_cut_line_and_a_dropped_field_come_back_saying_so() {
        let long = "x".repeat(4_000);
        let mut event = Event::info("t", &long);
        for _ in 0..20 {
            event = event.with("k", 1u32);
        }
        let back = round_trip(&event);
        assert!(back.cut, "the message was cut and the line must say so");
        assert_eq!(back.dropped_fields, 8);
        assert!(!back.matches(&event), "a cut record is NOT the event");
    }

    #[test]
    fn a_key_this_build_has_never_heard_of_is_stepped_over_rather_than_refused() {
        let line = br#"{"seq":1,"ts":"x","ms":2,"level":"info","target":"t","msg":"m",
            "fields":{"a":1},"future":{"nested":[1,{"deep":true}]},"other":"ignored"}"#;
        let back = Record::decode(line).expect("an unknown key is not a refusal");
        assert_eq!(back.seq, 1);
        assert_eq!(back.field("a"), Some(&OwnedValue::Int(1)));
        assert_eq!(back.fields.len(), 1, "the unknown keys added no fields");
    }

    #[test]
    fn a_line_missing_a_key_it_cannot_do_without_is_refused_by_that_key_s_name() {
        for (line, missing) in [
            (
                r#"{"ts":"x","ms":2,"level":"info","target":"t","msg":"m"}"#,
                "seq",
            ),
            (r#"{"seq":1,"level":"info","target":"t","msg":"m"}"#, "ms"),
            (r#"{"seq":1,"ms":2,"target":"t","msg":"m"}"#, "level"),
            (r#"{"seq":1,"ms":2,"level":"info","msg":"m"}"#, "target"),
            (r#"{"seq":1,"ms":2,"level":"info","target":"t"}"#, "msg"),
        ] {
            assert_eq!(
                Record::decode(line.as_bytes()),
                Err(LineFault::MissingKey { key: missing }),
                "{line}"
            );
        }
        assert_eq!(
            Record::decode(b"{}"),
            Err(LineFault::MissingKey { key: "seq" }),
            "an empty object is a line with nothing on it"
        );
    }

    #[test]
    fn a_line_that_is_not_this_format_is_refused_and_never_guessed_at() {
        assert_eq!(
            Record::decode(b"brutex api listening on http://127.0.0.1:8080"),
            Err(LineFault::Unexpected { at: 0, found: b'b' }),
            "the eprintln! this crate replaces is not silently absorbed"
        );
        assert_eq!(Record::decode(b""), Err(LineFault::Truncated));
        assert_eq!(Record::decode(b"{"), Err(LineFault::Truncated));
        assert_eq!(
            Record::decode(br#"{"seq":1}{"seq":2}"#),
            Err(LineFault::Unexpected { at: 9, found: b'{' }),
            "two objects on one line is not one event"
        );
        assert_eq!(
            Record::decode(br#"{"seq":1 "ms":2}"#),
            Err(LineFault::Unexpected { at: 9, found: b'"' }),
            "a missing comma is named, not skipped"
        );
        assert_eq!(
            Record::decode(br#"{"level":"fatal","seq":1,"ms":2,"target":"t","msg":"m"}"#),
            Err(LineFault::UnknownLevel {
                word: "fatal".to_owned()
            })
        );
        assert_eq!(
            Record::decode(br#"{"seq":-1,"ms":2,"level":"info","target":"t","msg":"m"}"#),
            Err(LineFault::BadNumber {
                text: "-1".to_owned()
            }),
            "a sequence number is unsigned and a negative one is refused"
        );
        assert_eq!(
            Record::decode(br#"{"seq":1,"ms":1.5,"level":"info","target":"t","msg":"m"}"#),
            Err(LineFault::BadNumber {
                text: "1.5".to_owned()
            }),
            "a millisecond is an integer"
        );
        assert_eq!(
            Record::decode(br#"{"seq":1,"ms":2,"level":"info","target":"t","msg":"m","cut":1}"#),
            Err(LineFault::Unexpected {
                at: 60,
                found: b'1'
            })
        );
        assert_eq!(
            Record::decode(br#"{"seq":1,"ms":2,"level":"info","target":"t","msg":"m","cut":"#),
            Err(LineFault::Truncated)
        );
    }

    /// **`Truncated` means the bytes RAN OUT. It does not mean "malformed".**
    ///
    /// The two are kept apart deliberately and `crate::tail` depends on it: the
    /// last line of a file being appended to is routinely half-written, and
    /// that is normal, whereas a line whose bytes are all present and wrong is
    /// a fault an operator has to see. A decoder that answered `Truncated` for
    /// both would make a corrupt line indistinguishable from a live writer —
    /// `CLAUDE.md` §4's fallback that hides a failure.
    ///
    /// `head` is 63 bytes, so the first byte of `tail` sits at index 63 and
    /// `at` is the 0-based index of the offending byte — the convention
    /// `a_line_that_is_not_this_format_is_refused_and_never_guessed_at` fixes
    /// with `{"seq":1}{"seq":2}` reporting `at: 9` for the second `{`.
    #[test]
    fn a_broken_fields_object_is_refused_by_name() {
        let head = r#"{"seq":1,"ms":2,"level":"info","target":"t","msg":"m","fields":"#;
        assert_eq!(head.len(), 63, "every index below is measured from this");
        for (tail, expect) in [
            // `…"fields":{}` — the fields object closes, the LINE does not.
            // The bytes ran out, so this one really is truncated.
            ("{", LineFault::Truncated),
            // `…"fields":{"a":}` — NOT truncated. A byte is present where the
            // value belongs and it is `}`, so `scalar` reports the byte it
            // choked on. Calling this `Truncated` would say the writer was
            // interrupted when what actually happened is that it emitted a
            // malformed object.
            (
                "{\"a\":",
                LineFault::Unexpected {
                    at: 68,
                    found: b'}',
                },
            ),
            (
                "{\"a\":@}",
                LineFault::Unexpected {
                    at: 68,
                    found: b'@',
                },
            ),
            (
                "{\"a\":1 \"b\":2}",
                LineFault::Unexpected {
                    at: 70,
                    found: b'"',
                },
            ),
            // `…"fields":{"a":1}` — the fields object closes and the line ends
            // without its own `}`. Truncated again, and for the same reason as
            // the first case.
            ("{\"a\":1", LineFault::Truncated),
        ] {
            let line = format!("{head}{tail}}}");
            assert_eq!(Record::decode(line.as_bytes()), Err(expect), "{line}");
        }
        // And an EMPTY fields object is the ordinary case, not a fault.
        let empty = format!("{head}{{}}}}");
        assert_eq!(
            Record::decode(empty.as_bytes())
                .expect("empty is fine")
                .fields,
            Vec::new()
        );
    }

    #[test]
    fn a_number_lands_in_the_narrowest_width_that_holds_it_exactly() {
        let head = r#"{"seq":1,"ms":2,"level":"info","target":"t","msg":"m","fields":"#;
        let decode = |fields: &str| {
            let line = format!("{head}{fields}}}");
            Record::decode(line.as_bytes())
                .unwrap_or_else(|e| panic!("{line}: {e}"))
                .fields
        };
        assert_eq!(
            decode(r#"{"a":9223372036854775807,"b":9223372036854775808,"c":1e2,"d":-0.5}"#),
            vec![
                ("a".to_owned(), OwnedValue::Int(i64::MAX)),
                // Past i64::MAX. Taking every number as a float would round
                // this to 9223372036854775808 and lose which count it was.
                ("b".to_owned(), OwnedValue::Uint(9_223_372_036_854_775_808)),
                ("c".to_owned(), OwnedValue::Float(100.0)),
                ("d".to_owned(), OwnedValue::Float(-0.5)),
            ]
        );
        // A number past every width is refused rather than becoming infinity.
        let line = format!("{head}{{\"a\":1e999999}}}}");
        assert_eq!(
            Record::decode(line.as_bytes()),
            Err(LineFault::BadNumber {
                text: "1e999999".to_owned()
            })
        );
        // As is one past u64.
        let line = format!("{head}{{\"a\":99999999999999999999}}}}");
        assert!(
            matches!(
                Record::decode(line.as_bytes()),
                Ok(record) if record.fields.first().map(|f| f.1.clone())
                    == Some(OwnedValue::Float(1e20))
            ),
            "a decimal integer past u64 falls back to a float, which is lossy and stated"
        );
    }

    #[test]
    fn matches_is_an_equality_and_not_a_shape_check() {
        let event = Event::info("t", "m").with("a", 1u32);
        let back = round_trip(&event);
        assert!(back.matches(&event));
        assert!(!back.matches(&Event::info("t", "m")), "field count differs");
        assert!(
            !back.matches(&Event::warn("t", "m").with("a", 1u32)),
            "level"
        );
        assert!(
            !back.matches(&Event::info("u", "m").with("a", 1u32)),
            "target"
        );
        assert!(
            !back.matches(&Event::info("t", "n").with("a", 1u32)),
            "message"
        );
        assert!(!back.matches(&Event::info("t", "m").with("b", 1u32)), "key");
        assert!(
            !back.matches(&Event::info("t", "m").with("a", 2u32)),
            "value"
        );
    }

    /// **THE DECODER'S REFUSAL ARMS, AND THE LITERAL ITS OWN WRITER NEVER EMITS.**
    ///
    /// Three lines of `Record::decode` had never run. Each is a refusal path,
    /// and a decoder's whole contract is what it refuses: `crates/telemetry`
    /// reads files it did not write — truncated by a full disk, cut by a
    /// copy, hand-edited, or produced by an older build — so the arms that say
    /// "this line is not whole" are the ones that matter when the file is
    /// evidence.
    ///
    /// The `false` literal is unreachable from THIS crate's writer by design:
    /// `encode::line` only emits `"cut":true`, and omits the key otherwise. A
    /// reader that could not decode `"cut":false` would still be wrong, because
    /// the reader's job is to accept what the format permits rather than only
    /// what today's writer happens to produce.
    #[test]
    fn a_truncated_line_is_refused_and_an_explicit_false_still_decodes() {
        // TRUNCATED WHERE A SEPARATOR MUST BE. The line ends immediately after a
        // complete field, so neither `,` nor `}` follows.
        let head =
            br#"{"seq":1,"ts":"x","ms":2,"level":"info","target":"t","msg":"m","fields":{"a":1"#;
        assert_eq!(
            Record::decode(head),
            Err(LineFault::Truncated),
            "a line that stops after a field is TRUNCATED, not merely unexpected \
             — the distinction is the reader's only way to tell a cut file from \
             a corrupt one"
        );

        // TRUNCATED WHERE A VALUE MUST BE. The key and colon are there and the
        // scalar is not.
        let no_value =
            br#"{"seq":1,"ts":"x","ms":2,"level":"info","target":"t","msg":"m","fields":{"a":"#;
        assert_eq!(
            Record::decode(no_value),
            Err(LineFault::Truncated),
            "and so is one that stops where a value must start"
        );

        // A BYTE THAT IS SIMPLY WRONG is a different fault, and must not be
        // reported as truncation — otherwise every corrupt line reads as a cut
        // one and an operator looks for the wrong cause.
        let junk =
            br#"{"seq":1,"ts":"x","ms":2,"level":"info","target":"t","msg":"m","fields":{"a":1@}}"#;
        assert!(
            matches!(Record::decode(junk), Err(LineFault::Unexpected { .. })),
            "a stray byte is Unexpected, not Truncated"
        );

        // `"cut":false` — legal in the format, never written by this crate.
        let explicit_false = br#"{"seq":9,"ts":"x","ms":2,"level":"info","target":"t","msg":"m","cut":false,"fields":{}}"#;
        let decoded = Record::decode(explicit_false).expect("an explicit false is legal");
        assert!(
            !decoded.cut,
            "and it decodes to false rather than being refused or read as true"
        );
        assert_eq!(decoded.seq, 9, "the rest of the line survives it");
    }
}

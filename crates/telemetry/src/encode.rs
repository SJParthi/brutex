//! One event, as the bytes of one line.
//!
//! # The shape, and why the key order is fixed
//!
//! ```text
//! {"seq":41,"ts":"2026-08-08T14:03:11.427Z","ms":1786197791427,"level":"info",
//!  "target":"pull.http","msg":"vendor responded","fields":{"status":200,"bytes":81922}}
//! ```
//!
//! (One line on disk; wrapped here to fit.)
//!
//! JSON says an object's keys are unordered. A *file* is not JSON, it is lines
//! of it, and a person reads a file in columns. Fixing the order means `seq`
//! and `ts` start every line at the same place, `grep '"level":"error"'` is a
//! fixed string rather than a pattern, and a diff between two runs lines up.
//! The reader does not depend on the order — it dispatches on the key — so the
//! order is a courtesy to the human and costs the machine nothing.
//!
//! `"fields":{}` is written even when there are none, for the same reason: a
//! shape that is always the same is a shape nobody has to test for.
//!
//! # What is written only when it is true
//!
//! `"cut":true` — some text hit its ceiling and the line does not carry all of
//! it. `"dropped":N` — N fields past [`crate::MAX_FIELDS`] were counted rather
//! than kept. Both are absent in the ordinary case, so they cost nothing on a
//! healthy line and are impossible to miss on an unhealthy one. `CLAUDE.md`
//! §4: degrade loudly and name the reason.

use crate::clock::{push_i64, push_padded, push_rfc3339};
use crate::event::{
    Event, MAX_KEY_BYTES, MAX_MESSAGE_BYTES, MAX_STR_VALUE_BYTES, MAX_TARGET_BYTES,
};
use crate::json::{push_escaped, push_quoted};
use crate::value::Value;
use core::fmt::Write as _;

/// Appends one whole line, terminating newline included.
///
/// `out` is appended to and never cleared here: the sink owns the buffer and
/// its lifetime, and a function that cleared somebody else's buffer would be a
/// surprise waiting for a second caller.
pub(crate) fn line(out: &mut Vec<u8>, seq: u64, at_unix_millis: i64, run: u64, event: &Event<'_>) {
    let mut cut = false;
    out.extend_from_slice(b"{\"seq\":");
    push_padded(out, seq, 1);
    // THE RUN THIS EVENT BELONGS TO, OMITTED WHEN THERE IS NONE.
    //
    // A log file spanning three backfills cannot be split into three without
    // this, and splitting it is the first thing a reader who did not run the
    // job has to do. Zero means "no run is in progress" — a serving process
    // logging a request belongs to no backfill — and the key is left off the
    // line entirely rather than written as `0`, so a reader never has to decide
    // whether zero is a run identifier or the absence of one.
    if run != 0 {
        out.extend_from_slice(b",\"run\":");
        push_padded(out, run, 1);
    }
    out.extend_from_slice(b",\"ts\":\"");
    push_rfc3339(out, at_unix_millis);
    out.extend_from_slice(b"\",\"ms\":");
    push_i64(out, at_unix_millis);
    out.extend_from_slice(b",\"level\":\"");
    // A level word is one of five ASCII literals this crate owns, so it needs
    // no escaping and no ceiling.
    out.extend_from_slice(event.level().label().as_bytes());
    out.extend_from_slice(b"\",\"target\":\"");
    cut |= push_capped(out, event.target(), MAX_TARGET_BYTES);
    out.extend_from_slice(b"\",\"msg\":\"");
    cut |= push_capped(out, event.message(), MAX_MESSAGE_BYTES);
    out.extend_from_slice(b"\",\"fields\":{");
    for (i, &(key, value)) in event.fields().iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        out.push(b'"');
        cut |= push_capped(out, key, MAX_KEY_BYTES);
        out.extend_from_slice(b"\":");
        cut |= push_value(out, value);
    }
    out.push(b'}');
    if cut {
        out.extend_from_slice(b",\"cut\":true");
    }
    if event.dropped_fields() > 0 {
        out.extend_from_slice(b",\"dropped\":");
        push_padded(out, u64::from(event.dropped_fields()), 1);
    }
    out.extend_from_slice(b"}\n");
}

/// One value, in its JSON type. Reports whether it was cut.
fn push_value(out: &mut Vec<u8>, value: Value<'_>) -> bool {
    match value {
        Value::Str(s) => {
            out.push(b'"');
            let cut = push_capped(out, s, MAX_STR_VALUE_BYTES);
            out.push(b'"');
            cut
        }
        Value::Int(v) => {
            push_i64(out, v);
            false
        }
        Value::Uint(v) => {
            push_padded(out, v, 1);
            false
        }
        Value::Float(v) => {
            push_float(out, v);
            false
        }
        Value::Bool(true) => {
            out.extend_from_slice(b"true");
            false
        }
        Value::Bool(false) => {
            out.extend_from_slice(b"false");
            false
        }
        Value::Null => {
            out.extend_from_slice(b"null");
            false
        }
    }
}

/// A float, in a spelling that reads back as the same float.
///
/// Rust's `Display` for `f64` is the shortest decimal that round-trips, which
/// is exactly the guarantee wanted here. Two adjustments:
///
/// * A whole-valued float prints as `1`, which the reader would take back as
///   an integer. `.0` is appended so the type survives the round trip.
/// * JSON has no literal for a non-finite number. Writing one anyway would
///   produce a line no consumer can parse — and it would be the line about the
///   calculation that went wrong, which is the line least affordable to lose.
///   They are written as strings instead, which is loud, greppable and legal.
fn push_float(out: &mut Vec<u8>, value: f64) {
    if !value.is_finite() {
        let word = if value.is_nan() {
            "NaN"
        } else if value.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        };
        push_quoted(out, word);
        return;
    }
    // FORMATTED STRAIGHT INTO THE CALLER'S BUFFER, NOT THROUGH A `String`.
    //
    // `value.to_string()` was one heap allocation and one free PER FLOAT FIELD,
    // and it ran inside `Sink::emit`'s critical section — so it was also the one
    // thing every other thread waiting on that mutex paid for. It was the ONLY
    // allocation left in the encoder: `clock::push_padded` hand-rolls its digits
    // into a stack array, and `push_escaped`/`push_quoted` only extend the
    // caller's `Vec`. The crate went out of its way to be allocation-free for
    // integers and text and then called `to_string()` on the float arm, which
    // made "zero allocations in steady state" false wherever a float appeared.
    //
    // `core::fmt`'s `Display` for `f64` is the same shortest-round-trip
    // formatter `to_string` used, so the bytes are unchanged — held by
    // `one_ordinary_event_renders_to_exactly_these_bytes`.
    let start = out.len();
    let wrote = write!(Utf8Sink(&mut *out), "{value}");
    // NOT DISCARDED SILENTLY. `Utf8Sink::write_str` returns `Ok` unconditionally,
    // so this cannot fail — but `CLAUDE.md` §4 objects to a swallowed error, and
    // an assertion says which it is. `debug_assert!` takes the value the `write!`
    // ALREADY produced, so the formatting happens in release builds too.
    debug_assert!(wrote.is_ok(), "Utf8Sink::write_str never returns Err");
    if !out
        .get(start..)
        .unwrap_or_default()
        .iter()
        .any(|b| matches!(b, b'.' | b'e' | b'E' | b'i' | b'N'))
    {
        out.extend_from_slice(b".0");
    }
}

/// A `core::fmt::Write` that appends UTF-8 to a byte buffer.
///
/// Exists so [`push_float`] can use the standard float formatter without the
/// `String` that `ToString` forces. Writing is infallible: a `Vec` push cannot
/// fail, so `write_str` has no error to report and says so.
struct Utf8Sink<'a>(&'a mut Vec<u8>);

impl core::fmt::Write for Utf8Sink<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// Escaped text, cut to at most `cap` **input** bytes on a character boundary.
///
/// The cap is on the input rather than on the escaped output, because the
/// input is the thing a caller can reason about. A pathological string of
/// control characters therefore expands sixfold, and the ceiling on a whole
/// line is `6 * (cap of each part)` — which is why the caps are as small as
/// they are.
///
/// Cut on a character boundary, never mid-sequence: half a multi-byte
/// character is not UTF-8, and a line whose own reader refuses it is worse
/// than a shorter line. This is the same rule and the same reason as
/// `api::audit::keep`.
fn push_capped(out: &mut Vec<u8>, text: &str, cap: usize) -> bool {
    if text.len() <= cap {
        push_escaped(out, text);
        return false;
    }
    let mut end = 0;
    for (at, ch) in text.char_indices() {
        let next = at.saturating_add(ch.len_utf8());
        if next > cap {
            break;
        }
        end = next;
    }
    push_escaped(out, text.get(..end).unwrap_or(""));
    true
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
    use super::{line, push_float};
    use crate::event::{
        Event, MAX_FIELDS, MAX_MESSAGE_BYTES, MAX_STR_VALUE_BYTES, MAX_TARGET_BYTES,
    };
    use crate::value::Value;

    fn rendered(event: &Event<'_>, seq: u64, millis: i64) -> String {
        let mut out = Vec::new();
        line(&mut out, seq, millis, 0, event);
        String::from_utf8(out).expect("a line is UTF-8")
    }

    /// THE SHAPE IS THE CONTRACT.
    ///
    /// Every consumer — the tail reader, a `grep`, a future page — reads this
    /// byte string. It is asserted whole rather than key by key, because a
    /// key-by-key assertion passes when the key ORDER changes and the order is
    /// half of what makes the file readable by a person.
    #[test]
    fn one_ordinary_event_renders_to_exactly_these_bytes() {
        let event = Event::info("pull.http", "vendor responded")
            .with("status", 200u32)
            .with("bytes", 81_922usize)
            .with("instrument", "NSE-NIFTY")
            .with("ok", true)
            .with("close", Value::paisa(2_512_075))
            .with("skew", -0.25f64)
            .with("expiry", Value::Null);
        assert_eq!(
            rendered(&event, 41, 1_786_197_791_427),
            "{\"seq\":41,\"ts\":\"2026-08-08T14:03:11.427Z\",\"ms\":1786197791427,\
             \"level\":\"info\",\"target\":\"pull.http\",\"msg\":\"vendor responded\",\
             \"fields\":{\"status\":200,\"bytes\":81922,\"instrument\":\"NSE-NIFTY\",\
             \"ok\":true,\"close\":2512075,\"skew\":-0.25,\"expiry\":null}}\n"
        );
    }

    #[test]
    fn an_event_with_no_fields_still_carries_the_object_and_ends_in_one_newline() {
        let out = rendered(&Event::error("api", "cannot bind"), 0, 0);
        assert!(out.ends_with("\"fields\":{}}\n"), "{out}");
        assert_eq!(out.matches('\n').count(), 1, "one line is one newline");
        assert!(out.contains("\"seq\":0"));
        assert!(out.contains("\"level\":\"error\""));
        assert!(!out.contains("\"cut\""), "nothing was cut");
        assert!(!out.contains("\"dropped\""), "nothing was dropped");
    }

    /// A LINE THAT LOST SOMETHING SAYS SO.
    ///
    /// Without the `cut` flag, an operator reading a truncated vendor error
    /// has no way to know the sentence ended early — the line is well-formed,
    /// the message is a sentence, and the half that named the actual fault is
    /// simply not there.
    #[test]
    fn text_past_its_ceiling_is_cut_on_a_character_boundary_and_the_line_says_cut() {
        // Every character is three bytes, so a 256-byte ceiling holds 85 of
        // them and cutting at 256 would land inside the 86th.
        let long = "अ".repeat(120);
        assert_eq!(long.len(), 360);
        let out = rendered(&Event::warn("t", &long), 1, 0);
        assert!(out.contains("\"cut\":true"), "{out}");
        let kept = out
            .split("\"msg\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("a message");
        assert_eq!(kept.chars().count(), MAX_MESSAGE_BYTES / 3);
        assert_eq!(kept.len(), 255, "85 whole characters, not 256 bytes");

        // A string FIELD has its own, smaller ceiling.
        let value = "x".repeat(MAX_STR_VALUE_BYTES + 10);
        let out = rendered(&Event::info("t", "m").with("why", value.as_str()), 1, 0);
        assert!(out.contains("\"cut\":true"), "{out}");
        assert!(out.contains(&format!("\"why\":\"{}\"", "x".repeat(MAX_STR_VALUE_BYTES))));

        // And exactly at the ceiling is not cut.
        let exact = "y".repeat(MAX_STR_VALUE_BYTES);
        let out = rendered(&Event::info("t", "m").with("why", exact.as_str()), 1, 0);
        assert!(!out.contains("\"cut\""), "{out}");
    }

    #[test]
    fn fields_past_the_ceiling_are_counted_on_the_line() {
        let mut event = Event::info("t", "m");
        for _ in 0..MAX_FIELDS + 3 {
            event = event.with("k", 1u32);
        }
        let out = rendered(&event, 1, 0);
        assert!(out.contains("\"dropped\":3"), "{out}");
    }

    #[test]
    fn a_message_that_is_a_json_document_cannot_break_out_of_its_own_string() {
        // The injection this format has to survive: a vendor error containing
        // quotes, braces, a newline and a backslash. The line must stay ONE
        // line and the payload must stay inside the message.
        let nasty = "he said \"{\\\"level\\\":\\\"error\\\"}\"\nand stopped";
        let out = rendered(&Event::warn("pull", nasty), 7, 0);
        assert_eq!(out.matches('\n').count(), 1, "still one line: {out}");
        assert!(out.contains("\\n"), "the newline is escaped: {out}");
        assert!(out.starts_with("{\"seq\":7,"));
        assert!(out.ends_with("\"fields\":{}}\n"), "{out}");
    }

    #[test]
    fn a_float_survives_the_round_trip_and_a_non_finite_one_stays_legal_json() {
        let rendered_float = |v: f64| {
            let mut out = Vec::new();
            push_float(&mut out, v);
            String::from_utf8(out).unwrap()
        };
        // A whole-valued float keeps its type. Without the appended `.0` this
        // renders as `1` and reads back as an integer.
        assert_eq!(rendered_float(1.0), "1.0");
        assert_eq!(rendered_float(-2.0), "-2.0");
        assert_eq!(rendered_float(0.0), "0.0");
        assert_eq!(rendered_float(0.25), "0.25");
        // Rust's `Display` never uses exponent notation, so a very large or
        // very small float is a long decimal. That is legal JSON and it round
        // trips, which is the property that matters; the length is not.
        // COMPARED AS BITS, NOT WITH `==`. Round-tripping is an identity
        // claim, not a nearness one: the decimal text must name the very same
        // f64, so the assertion is on the 64 bits themselves. That is stricter
        // than `==` -- it also separates `0.0` from `-0.0`, which `==` calls
        // equal -- and it is the honest way to say "exact", rather than
        // silencing `float_cmp` over a comparison the lint is right to
        // distrust everywhere else.
        for extreme in [1e300, 1e-300, f64::MAX, f64::MIN_POSITIVE, -1.5e-8] {
            let text = rendered_float(extreme);
            assert_eq!(
                text.parse::<f64>().expect(&text).to_bits(),
                extreme.to_bits(),
                "{text} did not read back as the float that produced it"
            );
        }
        assert_eq!(rendered_float(1.5e-8), "0.000000015");
        // JSON has no literal for these, so they are strings and the line
        // stays parseable.
        assert_eq!(rendered_float(f64::NAN), "\"NaN\"");
        assert_eq!(rendered_float(f64::INFINITY), "\"Infinity\"");
        assert_eq!(rendered_float(f64::NEG_INFINITY), "\"-Infinity\"");
    }

    #[test]
    fn a_negative_time_and_a_maximal_sequence_both_render() {
        let out = rendered(&Event::trace("t", "m"), u64::MAX, -1);
        assert!(out.contains("\"seq\":18446744073709551615"), "{out}");
        assert!(out.contains("\"ms\":-1"), "{out}");
        assert!(out.contains("\"ts\":\"1969-12-31T23:59:59.999Z\""), "{out}");
    }

    /// **AN OVER-LONG TARGET SETS `cut`, LIKE EVERY OTHER TRUNCATION.**
    ///
    /// `line` accumulates `cut |= push_capped(..)` for the target, the message
    /// and each value. Mutating the TARGET one to `&=` survived: `cut` is
    /// `false` when the target is reached, so `&=` pins it false forever — the
    /// target is still truncated on the line, but the line no longer says so,
    /// and a reader shows a shortened subsystem name as though it were whole.
    ///
    /// It survived because `MAX_TARGET_BYTES` appeared nowhere but its own
    /// definition: no test had ever emitted a target longer than 48 bytes.
    #[test]
    fn a_target_past_its_ceiling_is_truncated_and_the_line_admits_it() {
        let long = "a".repeat(MAX_TARGET_BYTES * 2);
        let mut out = Vec::new();
        line(&mut out, 1, 0, 0, &Event::info(&long, "m"));
        let text = String::from_utf8(out).expect("ascii");

        assert!(
            text.contains(r#""cut":true"#),
            "the target was cut, so the line must say so: {text}"
        );
        let decoded = crate::record::Record::decode(text.trim_end().as_bytes()).expect("decodes");
        assert!(decoded.cut, "and a reader sees the flag");
        assert_eq!(
            decoded.target.len(),
            MAX_TARGET_BYTES,
            "the target really is truncated to its ceiling"
        );

        // A target INSIDE the ceiling sets nothing, which is what separates
        // `|=` from an unconditional `true`.
        let mut out = Vec::new();
        line(&mut out, 1, 0, 0, &Event::info("short.target", "m"));
        let text = String::from_utf8(out).expect("ascii");
        assert!(
            !text.contains(r#""cut":true"#),
            "a target that fits raises no flag: {text}"
        );
    }
}

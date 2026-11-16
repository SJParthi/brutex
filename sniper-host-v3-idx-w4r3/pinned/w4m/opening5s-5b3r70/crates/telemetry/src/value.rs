//! What a field carries: a typed scalar, never a sentence.
//!
//! # Why this is not `&str`
//!
//! The whole reason this crate exists rather than another `eprintln!` is that
//! a consumer must be able to answer "every event where `status` was not 200"
//! without parsing prose. That is only possible if `200` was written as the
//! JSON number `200` and not as part of a message. A field is therefore a
//! typed value at the call site, it is written as its JSON type, and it comes
//! back as the same type.
//!
//! # Two types, one on each side of the file
//!
//! [`Value`] is the *write* side: it borrows, it is `Copy`, and building an
//! event out of it allocates nothing at all. [`OwnedValue`] is the *read*
//! side, because an unescaped string has to be owned by somebody. They compare
//! across the boundary — `OwnedValue: PartialEq<Value>` — which is what makes
//! the round-trip test an equality rather than a field-by-field walk.
//!
//! # Money
//!
//! `CLAUDE.md` §7: prices are paisa integers, never a float. [`Value::paisa`]
//! is the constructor to reach for, and it produces an [`Value::Int`]; there
//! is no float path for a price and [`Value::Float`] is for the statistics
//! §7 says keep full precision.

/// One field's value, borrowed from the call site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    /// Text. Escaped on the way out, unescaped on the way back.
    Str(&'a str),
    /// A signed integer. This is what money is: paisa, per `CLAUDE.md` §7.
    Int(i64),
    /// An unsigned integer, for counts and byte lengths.
    Uint(u64),
    /// A float, for the statistics that keep full precision.
    ///
    /// Never a price. Non-finite values are written as the JSON *strings*
    /// `"NaN"`, `"Infinity"` and `"-Infinity"`, because JSON has no literal
    /// for them and a line that is not JSON is a line no consumer can read.
    Float(f64),
    /// A flag.
    Bool(bool),
    /// Known, and known to be nothing. Not the same as an absent key.
    Null,
}

impl<'a> Value<'a> {
    /// Money, in paisa.
    ///
    /// A named constructor rather than a variant, so a call site reads as the
    /// thing it is and a reviewer can grep for every place a price enters the
    /// log. `CLAUDE.md` §7 — never a float.
    #[must_use]
    pub const fn paisa(paisa: i64) -> Self {
        Self::Int(paisa)
    }

    /// A count whose type is `usize`.
    ///
    /// Saturating rather than wrapping: a count that wrapped would report a
    /// smaller number than the truth, which is the one direction a count must
    /// never be wrong in. On every host this builds for `usize` is 64 bits and
    /// the conversion cannot fail, so **the saturating arm is a backstop and
    /// no test on this machine can drive it** — `CLAUDE.md` §3 rule 6, said
    /// rather than left for a coverage report to find. It is the same shape
    /// and the same admission as `api::audit::as_u64`.
    #[must_use]
    pub fn count(n: usize) -> Self {
        Self::Uint(u64::try_from(n).unwrap_or(u64::MAX))
    }

    /// The text, when it is text.
    #[must_use]
    pub const fn as_str(self) -> Option<&'a str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(v: &'a str) -> Self {
        Self::Str(v)
    }
}

impl From<i64> for Value<'_> {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<i32> for Value<'_> {
    fn from(v: i32) -> Self {
        Self::Int(i64::from(v))
    }
}

impl From<u64> for Value<'_> {
    fn from(v: u64) -> Self {
        Self::Uint(v)
    }
}

impl From<u32> for Value<'_> {
    fn from(v: u32) -> Self {
        Self::Uint(u64::from(v))
    }
}

impl From<usize> for Value<'_> {
    fn from(v: usize) -> Self {
        Self::count(v)
    }
}

impl From<bool> for Value<'_> {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<f64> for Value<'_> {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

/// One field's value, as it comes back off the disk.
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedValue {
    /// Text, unescaped.
    Str(String),
    /// A signed integer.
    Int(i64),
    /// An unsigned integer past [`i64::MAX`].
    Uint(u64),
    /// A float.
    Float(f64),
    /// A flag.
    Bool(bool),
    /// JSON `null`.
    Null,
}

impl OwnedValue {
    /// The text, when it is text.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Self::Str(ref s) => Some(s),
            _ => None,
        }
    }

    /// The value as a signed integer, whichever integer variant it arrived in.
    ///
    /// A `Uint` past [`i64::MAX`] is `None` rather than a negative number:
    /// a count must never be reported as smaller than the truth, and wrapping
    /// is exactly that failure.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Int(v) => Some(v),
            Self::Uint(v) => i64::try_from(v).ok(),
            _ => None,
        }
    }

    /// The value as an unsigned integer, when it is one and is not negative.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Self::Uint(v) => Some(v),
            Self::Int(v) => u64::try_from(v).ok(),
            _ => None,
        }
    }

    /// The value as a float, when it is one.
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Float(v) => Some(v),
            _ => None,
        }
    }

    /// The value as a flag, when it is one.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Bool(v) => Some(v),
            _ => None,
        }
    }
}

/// The comparison the round-trip test is written as.
///
/// A written [`Value`] and the [`OwnedValue`] it decodes back to are equal
/// exactly when every byte survived, so this is the assertion rather than a
/// walk over the variants at each call site.
impl PartialEq<Value<'_>> for OwnedValue {
    fn eq(&self, other: &Value<'_>) -> bool {
        match (self, *other) {
            (Self::Str(a), Value::Str(b)) => a == b,
            (&Self::Int(a), Value::Int(b)) => a == b,
            (&Self::Uint(a), Value::Uint(b)) => a == b,
            // JSON HAS NO SIGNEDNESS. `200u32` and `200i64` are the same six
            // bytes on the line, so the reader cannot tell them apart and this
            // must not pretend it can: integers are compared by VALUE, across
            // the two widths, and a `Uint` past `i64::MAX` is simply not equal
            // to any `Int`. Pinned by
            // `the_two_integer_widths_compare_by_value_because_the_file_cannot_tell_them_apart`.
            (&Self::Uint(a), Value::Int(b)) => i64::try_from(a).is_ok_and(|a| a == b),
            (&Self::Int(a), Value::Uint(b)) => u64::try_from(a).is_ok_and(|a| a == b),
            (&Self::Bool(a), Value::Bool(b)) => a == b,
            // Bit equality, not numeric equality. `NaN != NaN` numerically, and
            // a round trip that preserved a NaN exactly must still be reported
            // as preserved. Two different NaN payloads compare unequal here,
            // which is correct: the file carries the word `"NaN"` and nothing
            // else, so a payload is not preserved and this must not claim it
            // was.
            (&Self::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (&Self::Null, Value::Null) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{OwnedValue, Value};

    #[test]
    fn a_price_enters_as_an_integer_and_there_is_no_float_door_for_one() {
        // CLAUDE.md section 7. `paisa` is the named constructor, and what it
        // produces is an integer variant — not a float, not a string.
        assert_eq!(Value::paisa(2_512_075), Value::Int(2_512_075));
        assert_eq!(Value::paisa(-1), Value::Int(-1));
        // And i64::MIN survives, because store::bar uses it as the
        // open-interest null sentinel.
        assert_eq!(Value::paisa(i64::MIN), Value::Int(i64::MIN));
    }

    #[test]
    fn every_from_lands_on_the_variant_its_type_means() {
        assert_eq!(Value::from("x"), Value::Str("x"));
        assert_eq!(Value::from(-5i64), Value::Int(-5));
        assert_eq!(Value::from(-5i32), Value::Int(-5));
        assert_eq!(Value::from(5u64), Value::Uint(5));
        assert_eq!(Value::from(5u32), Value::Uint(5));
        assert_eq!(Value::from(5usize), Value::Uint(5));
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(0.5f64), Value::Float(0.5));
        assert_eq!(Value::count(0), Value::Uint(0));
    }

    #[test]
    fn a_borrowed_value_reads_its_text_back_and_nothing_else_does() {
        assert_eq!(Value::Str("NSE-NIFTY").as_str(), Some("NSE-NIFTY"));
        assert_eq!(Value::Int(1).as_str(), None);
        assert_eq!(Value::Null.as_str(), None);
    }

    #[test]
    fn an_owned_value_reads_back_as_the_type_it_is_and_refuses_the_others() {
        let s = OwnedValue::Str("a".to_owned());
        assert_eq!(s.as_str(), Some("a"));
        assert_eq!(s.as_i64(), None);
        assert_eq!(s.as_u64(), None);
        assert_eq!(s.as_f64(), None);
        assert_eq!(s.as_bool(), None);

        assert_eq!(OwnedValue::Int(-3).as_i64(), Some(-3));
        assert_eq!(OwnedValue::Int(-3).as_u64(), None, "never a wrap");
        assert_eq!(OwnedValue::Uint(3).as_i64(), Some(3));
        assert_eq!(OwnedValue::Uint(3).as_u64(), Some(3));
        assert_eq!(OwnedValue::Uint(3).as_str(), None);
        assert_eq!(OwnedValue::Float(1.5).as_f64(), Some(1.5));
        assert_eq!(OwnedValue::Bool(false).as_bool(), Some(false));
        assert_eq!(OwnedValue::Null.as_bool(), None);
        assert_eq!(OwnedValue::Null.as_f64(), None);

        // A count past i64::MAX is refused rather than reported as negative.
        let huge = OwnedValue::Uint(u64::MAX);
        assert_eq!(huge.as_i64(), None);
        assert_eq!(huge.as_u64(), Some(u64::MAX));
        // And a negative signed value is not an unsigned one.
        assert_eq!(OwnedValue::Int(i64::MIN).as_u64(), None);
    }

    /// THE TWO INTEGER WIDTHS COMPARE BY VALUE, BECAUSE THE FILE CANNOT TELL
    /// THEM APART.
    ///
    /// `200u32` and `200i64` are the same six bytes on the line. A comparison
    /// that insisted on the variant would make the round-trip test assert
    /// something the format does not promise, and the honest thing is to say
    /// so here rather than to invent a signedness marker nobody asked for.
    #[test]
    fn the_two_integer_widths_compare_by_value_because_the_file_cannot_tell_them_apart() {
        assert_eq!(OwnedValue::Int(200), Value::Uint(200));
        assert_eq!(OwnedValue::Uint(200), Value::Int(200));
        assert_eq!(OwnedValue::Int(0), Value::Uint(0));
        assert_ne!(OwnedValue::Int(-1), Value::Uint(1));
        // A negative signed value is no unsigned value at all...
        assert_ne!(OwnedValue::Int(i64::MIN), Value::Uint(0));
        // ...and a count past i64::MAX is no signed value at all.
        assert_ne!(OwnedValue::Uint(u64::MAX), Value::Int(-1));
        assert_ne!(OwnedValue::Uint(u64::MAX), Value::Int(i64::MAX));
    }

    #[test]
    fn the_cross_side_comparison_matches_only_the_same_type_and_the_same_bytes() {
        assert_eq!(OwnedValue::Str("a".to_owned()), Value::Str("a"));
        assert_ne!(OwnedValue::Str("a".to_owned()), Value::Str("b"));
        assert_eq!(OwnedValue::Int(-1), Value::Int(-1));
        assert_ne!(OwnedValue::Int(-1), Value::Int(1));
        assert_eq!(OwnedValue::Uint(9), Value::Uint(9));
        assert_ne!(OwnedValue::Uint(9), Value::Uint(8));
        assert_eq!(OwnedValue::Bool(true), Value::Bool(true));
        assert_ne!(OwnedValue::Bool(true), Value::Bool(false));
        assert_eq!(OwnedValue::Float(0.25), Value::Float(0.25));
        assert_ne!(OwnedValue::Float(0.25), Value::Float(0.5));
        assert_eq!(OwnedValue::Null, Value::Null);

        // A type mismatch is not equality.
        assert_ne!(OwnedValue::Null, Value::Bool(false));
        assert_ne!(OwnedValue::Str("1".to_owned()), Value::Int(1));

        // NaN compares by bits, so a preserved NaN is reported as preserved.
        assert_eq!(OwnedValue::Float(f64::NAN), Value::Float(f64::NAN));
        assert_ne!(OwnedValue::Float(f64::NAN), Value::Float(0.0));
    }
}

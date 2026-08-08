//! One thing that happened, built without touching the allocator.
//!
//! # Why the fields are a fixed array and not a `Vec`
//!
//! An event is built on the hot path — inside a request handler, inside the
//! per-window loop of a twelve-hour backfill — and thrown away microseconds
//! later. A `Vec<(&str, Value)>` would be one heap allocation per event for
//! data that never outlives the statement that made it. The array is
//! [`MAX_FIELDS`] slots of a `Copy` type, so constructing an [`Event`] is a
//! few stores to the stack and nothing else. The whole emit path from here to
//! the `write` syscall allocates zero bytes in steady state.
//!
//! # Why every length has a ceiling
//!
//! `docs/07-o1-architecture.md` law 5 — bound every input at the boundary. A
//! log line is written from data this crate did not produce: a vendor's error
//! string, an operator's path, a URL. Without a ceiling, one pathological
//! value turns a 250-byte line into a megabyte, blows through the rotation
//! bound in a single event, and makes the per-event cost a function of
//! somebody else's input rather than a constant.
//!
//! The ceilings are applied at *encode* time, not here, so an `Event` always
//! carries exactly what the call site passed and the truncation is a property
//! of the file rather than of the value. Whenever anything was cut, the line
//! carries `"cut":true` — `CLAUDE.md` §4, degrade loudly and name it, never
//! silently.

use crate::level::Level;
use crate::value::Value;

/// The most fields one event carries.
///
/// A thirteenth is counted, not kept: [`Event::dropped_fields`] reports it and
/// the encoder writes it onto the line as `"dropped"`. A field that vanished
/// without a trace would be the fallback `CLAUDE.md` §4 forbids.
pub const MAX_FIELDS: usize = 12;

/// The longest target kept on a line, in bytes.
pub const MAX_TARGET_BYTES: usize = 48;

/// The longest message kept on a line, in bytes.
pub const MAX_MESSAGE_BYTES: usize = 256;

/// The longest field key kept on a line, in bytes.
pub const MAX_KEY_BYTES: usize = 32;

/// The longest field *string* value kept on a line, in bytes.
///
/// Numbers have no ceiling because they have no room to need one: the widest
/// `u64` is twenty digits.
pub const MAX_STR_VALUE_BYTES: usize = 128;

/// One thing that happened.
///
/// Borrowed for its whole lifetime and `Copy`-cheap to build. It carries no
/// timestamp and no sequence number: both are stamped by the sink at the
/// moment the bytes are placed, so the time on the line is the time it
/// reached the file and not the time somebody assembled a struct.
#[derive(Debug, Clone, Copy)]
pub struct Event<'a> {
    level: Level,
    target: &'a str,
    message: &'a str,
    items: [(&'a str, Value<'a>); MAX_FIELDS],
    len: usize,
    dropped: u32,
}

impl<'a> Event<'a> {
    /// An event at a level, from a subsystem, saying one short thing.
    ///
    /// `target` names the subsystem in the dotted form the tail filter
    /// understands as a prefix — `"pull"`, `"pull.http"`, `"api.server"` — so
    /// that asking for `"pull"` returns `"pull.http"` too.
    #[must_use]
    pub const fn new(level: Level, target: &'a str, message: &'a str) -> Self {
        Self {
            level,
            target,
            message,
            items: [("", Value::Null); MAX_FIELDS],
            len: 0,
            dropped: 0,
        }
    }

    /// An event at [`Level::Trace`].
    #[must_use]
    pub const fn trace(target: &'a str, message: &'a str) -> Self {
        Self::new(Level::Trace, target, message)
    }

    /// An event at [`Level::Debug`].
    #[must_use]
    pub const fn debug(target: &'a str, message: &'a str) -> Self {
        Self::new(Level::Debug, target, message)
    }

    /// An event at [`Level::Info`].
    #[must_use]
    pub const fn info(target: &'a str, message: &'a str) -> Self {
        Self::new(Level::Info, target, message)
    }

    /// An event at [`Level::Warn`].
    #[must_use]
    pub const fn warn(target: &'a str, message: &'a str) -> Self {
        Self::new(Level::Warn, target, message)
    }

    /// An event at [`Level::Error`].
    #[must_use]
    pub const fn error(target: &'a str, message: &'a str) -> Self {
        Self::new(Level::Error, target, message)
    }

    /// One more typed field.
    ///
    /// Past [`MAX_FIELDS`] the field is counted rather than kept; see
    /// [`Event::dropped_fields`].
    #[must_use]
    pub fn with(mut self, key: &'a str, value: impl Into<Value<'a>>) -> Self {
        match self.items.get_mut(self.len) {
            Some(slot) => {
                *slot = (key, value.into());
                self.len = self.len.saturating_add(1);
            }
            None => self.dropped = self.dropped.saturating_add(1),
        }
        self
    }

    /// How loud it is.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// Which subsystem it came from.
    #[must_use]
    pub const fn target(&self) -> &'a str {
        self.target
    }

    /// What it says.
    #[must_use]
    pub const fn message(&self) -> &'a str {
        self.message
    }

    /// The fields it kept, in the order they were added.
    #[must_use]
    pub fn fields(&self) -> &[(&'a str, Value<'a>)] {
        self.items.get(..self.len).unwrap_or(&[])
    }

    /// How many fields were offered past [`MAX_FIELDS`] and counted instead.
    #[must_use]
    pub const fn dropped_fields(&self) -> u32 {
        self.dropped
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
    use super::{Event, MAX_FIELDS};
    use crate::level::Level;
    use crate::value::Value;

    #[test]
    fn every_constructor_lands_on_the_level_it_is_named_after() {
        for (event, level) in [
            (Event::trace("t", "m"), Level::Trace),
            (Event::debug("t", "m"), Level::Debug),
            (Event::info("t", "m"), Level::Info),
            (Event::warn("t", "m"), Level::Warn),
            (Event::error("t", "m"), Level::Error),
        ] {
            assert_eq!(event.level(), level);
            assert_eq!(event.target(), "t");
            assert_eq!(event.message(), "m");
            assert!(event.fields().is_empty());
            assert_eq!(event.dropped_fields(), 0);
        }
    }

    #[test]
    fn fields_keep_the_order_they_were_added_in_and_their_types() {
        let event = Event::info("pull.http", "vendor responded")
            .with("status", 200u32)
            .with("instrument", "NSE-NIFTY")
            .with("bytes", 81_922usize)
            .with("ok", true)
            .with("close", Value::paisa(2_512_075));
        assert_eq!(
            event.fields(),
            [
                ("status", Value::Uint(200)),
                ("instrument", Value::Str("NSE-NIFTY")),
                ("bytes", Value::Uint(81_922)),
                ("ok", Value::Bool(true)),
                ("close", Value::Int(2_512_075)),
            ]
        );
    }

    /// A FIELD PAST THE CEILING IS COUNTED, NOT VANISHED.
    ///
    /// If the overflow arm merely returned `self`, a call site that added
    /// thirteen fields would produce a line indistinguishable from one that
    /// added twelve — and the thirteenth, which is as likely as any other to
    /// be the one that explains the failure, would be gone with nothing on the
    /// page saying so. `CLAUDE.md` §4: degrade loudly and name the reason.
    #[test]
    fn a_field_past_the_ceiling_is_counted_and_the_kept_ones_are_the_first_ones() {
        let mut event = Event::info("t", "m");
        for i in 0..MAX_FIELDS {
            event = event.with("k", u64::try_from(i).unwrap());
        }
        assert_eq!(event.fields().len(), MAX_FIELDS);
        assert_eq!(event.dropped_fields(), 0, "exactly at the ceiling is fine");

        let over = event.with("late", 1u32).with("later", 2u32);
        assert_eq!(over.fields().len(), MAX_FIELDS, "no thirteenth slot");
        assert_eq!(over.dropped_fields(), 2, "and both are counted");
        assert_eq!(
            over.fields().last(),
            Some(&("k", Value::Uint(u64::try_from(MAX_FIELDS - 1).unwrap()))),
            "the ones kept are the FIRST twelve, not the last twelve"
        );
    }
}

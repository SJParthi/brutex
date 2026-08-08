//! How loud one event is, as five ordered ranks.
//!
//! The rank is what a filter compares, and it is stored on disk as a *word*
//! rather than as the number: a number in the file would make the ladder's
//! order part of the format, and re-ordering it later would silently re-read
//! every line already written as a different level. `CLAUDE.md` §3 rule 8 —
//! a format is never mutated in place. The word costs four to five bytes a
//! line and can be `grep`-ed by a human, which is the whole point of the
//! format.

/// How loud one event is.
///
/// Ordered: `Trace < Debug < Info < Warn < Error`. The derived `Ord` is the
/// filter's comparison and the declaration order above is therefore
/// load-bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Every step, including the ones that are only interesting when
    /// something has already gone wrong.
    Trace,
    /// The detail an operator asks for when a number looks wrong.
    Debug,
    /// What happened, at the granularity a person would describe it.
    Info,
    /// It carried on, and somebody should know.
    Warn,
    /// It did not carry on, or it carried on having lost something.
    Error,
}

/// Every level, quietest first.
///
/// Written once, here. A surface that offers a level menu reads this rather
/// than spelling its own list, which is what stops a page and a filter
/// disagreeing about which levels exist.
pub const LEVELS: [Level; 5] = [
    Level::Trace,
    Level::Debug,
    Level::Info,
    Level::Warn,
    Level::Error,
];

impl Level {
    /// Where this sits on the ladder, quietest at zero.
    ///
    /// Not written to disk. It exists so a caller can hold a level in an
    /// atomic without a lock; see [`crate::Sink::set_min_level`].
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Trace => 0,
            Self::Debug => 1,
            Self::Info => 2,
            Self::Warn => 3,
            Self::Error => 4,
        }
    }

    /// The level a rank names, or `None` past the ladder.
    #[must_use]
    pub const fn of_rank(rank: u8) -> Option<Self> {
        match rank {
            0 => Some(Self::Trace),
            1 => Some(Self::Debug),
            2 => Some(Self::Info),
            3 => Some(Self::Warn),
            4 => Some(Self::Error),
            _ => None,
        }
    }

    /// The word written on the line, and the word a person greps for.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    /// The level a word names, ASCII-case-insensitively.
    ///
    /// Case-insensitive because one caller of this is a query string typed by
    /// a person. The *writer* only ever emits the lowercase form, so the file
    /// stays uniform however the query arrived.
    #[must_use]
    pub fn of_label(word: &str) -> Option<Self> {
        LEVELS
            .into_iter()
            .find(|level| word.eq_ignore_ascii_case(level.label()))
    }

    /// Whether this level passes a floor.
    #[must_use]
    pub const fn at_least(self, floor: Self) -> bool {
        self.rank() >= floor.rank()
    }
}

impl core::fmt::Display for Level {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.label())
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
    use super::{LEVELS, Level};

    /// THE LADDER'S ORDER IS THE FILTER.
    ///
    /// `min_level` is implemented as `event.level >= floor`, so the derived
    /// `Ord` — which follows declaration order — decides what a "warn and
    /// above" query returns. Swapping two variants in the declaration would
    /// compile, would keep every label correct, and would quietly change which
    /// events an operator sees. Asserted as the full chain rather than as one
    /// pair.
    #[test]
    fn the_ladder_runs_quietest_to_loudest_and_the_ranks_agree_with_it() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);

        for (i, level) in LEVELS.into_iter().enumerate() {
            assert_eq!(
                usize::from(level.rank()),
                i,
                "{level} sits at rank {} and index {i}",
                level.rank()
            );
            assert_eq!(Level::of_rank(level.rank()), Some(level));
        }
        assert_eq!(Level::of_rank(5), None, "the ladder has exactly five rungs");
        assert_eq!(Level::of_rank(u8::MAX), None);
    }

    #[test]
    fn a_floor_admits_itself_and_everything_louder_and_nothing_quieter() {
        assert!(Level::Warn.at_least(Level::Warn));
        assert!(Level::Error.at_least(Level::Warn));
        assert!(!Level::Info.at_least(Level::Warn));
        assert!(Level::Trace.at_least(Level::Trace));
    }

    #[test]
    fn a_label_round_trips_and_case_does_not_change_the_answer() {
        for level in LEVELS {
            assert_eq!(Level::of_label(level.label()), Some(level));
            assert_eq!(Level::of_label(&level.label().to_uppercase()), Some(level));
            assert_eq!(level.to_string(), level.label());
        }
        assert_eq!(Level::of_label("WaRn"), Some(Level::Warn));
        assert_eq!(Level::of_label("fatal"), None, "not a rung on this ladder");
        assert_eq!(Level::of_label(""), None);
        assert_eq!(Level::of_label("info "), None, "no trimming happens here");
    }

    /// The five words are the on-disk alphabet. A rename is a format change
    /// that would re-read every line already written as an unknown level.
    #[test]
    fn the_five_words_on_disk_are_these_five_and_they_are_lowercase() {
        assert_eq!(
            LEVELS.map(Level::label),
            ["trace", "debug", "info", "warn", "error"]
        );
    }
}

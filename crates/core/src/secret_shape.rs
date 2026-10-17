//! One detector for a value that has the SHAPE of a credential (SEC-07).
//!
//! # Why it is here
//!
//! A credential value is read from Parameter Store and nowhere else
//! (`CLAUDE.md` §8). The places a person can type text into this system —
//! the settings route today, a log line or an answer tomorrow — must never
//! keep one by accident, and the commonest accident is a paste: the access
//! token itself, or the parameter path `/<org>/<env>/<vendor>/<field>` whose
//! real segments must never be tracked either. Two callers want the same
//! answer — `api`'s settings route refuses such a value before anything is
//! written, and the leak scan looks for one in what was served — so the
//! detector lives in `core`, which depends on nothing and so can be named by
//! both. `core` holds no field name of `pull`'s: the caller passes them.
//!
//! # What the two shapes are
//!
//! * [`value_looks_like_a_token`]: a whitespace-delimited word holding no path
//!   separator (`/` or `\`) whose letters and digits run unbroken for at least
//!   [`TOKEN_RUN`] characters, in a value that does not itself open with a
//!   separator (that value is a path, whole, and may hold spaces). The alphabet is base64url's and hex's letters
//!   and digits; base64url's own `-` and `_` delimit the run, exactly as they
//!   do in `pull::config`'s "undelimited alphanumeric" backstop. They must:
//!   every store symbol this system opens on is upper-case words joined by
//!   `-` (`NSE-INDEX-NIFTYNXT50` is twenty characters), and counting the `-`
//!   would refuse the operator's own instrument. A word holding a path
//!   separator is a path and is judged by the second shape alone, so a long
//!   legitimate folder is allowed.
//! * [`value_looks_like_a_parameter_path`]: a word that opens with `/` and has
//!   exactly four non-empty segments, the last of which is one of the
//!   credential field names the caller passes.
//!
//! # What it is not
//!
//! A heuristic over a shape, not a proof about entropy (`CLAUDE.md` §3 rule
//! 6). It misses a token shorter than [`TOKEN_RUN`], one whose every run is
//! broken by `-`, `_`, `.`, `+` or `=`, and one pasted inside a path. What it
//! refuses wrongly is an unbroken run of twenty letters and digits that is
//! not a secret — a commit hash pasted into a reason is one — and that
//! refusal is loud, names the key, and keeps nothing.
//!
//! # Cost
//!
//! One pass over at most [`MAX_VALUE_BYTES`]: O(value length), capped. A
//! longer value is [`Shape::OverCap`] without being read further.

/// The fewest unbroken letters and digits that read as a token.
///
/// Twenty: exactly twenty is refused and nineteen is allowed (the package's
/// stated bounds). No setting this build holds carries a run that long: a
/// date is ten characters, a feed word seven, a store symbol's longest word
/// ten.
pub const TOKEN_RUN: usize = 20;

/// The longest value the detector reads. A longer one is refused whole.
pub const MAX_VALUE_BYTES: usize = 4096;

/// The segments a parameter path has: `/<org>/<env>/<vendor>/<field>`.
pub const PATH_SEGMENTS: usize = 4;

/// Which credential shape a value has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A run of [`TOKEN_RUN`] or more letters and digits in a word with no
    /// path separator.
    Token,
    /// `/<org>/<env>/<vendor>/<field>` with a field name the caller passed.
    ParameterPath,
    /// Longer than [`MAX_VALUE_BYTES`]: not read, refused whole.
    OverCap,
}

impl Shape {
    /// The class word a refusal names: `token`, `parameter_path` or
    /// `over_cap`. Never the value.
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::ParameterPath => "parameter_path",
            Self::OverCap => "over_cap",
        }
    }
}

/// Whether `c` separates path segments.
const fn is_separator(c: char) -> bool {
    c == '/' || c == '\\'
}

/// Whether `word` holds a run of [`TOKEN_RUN`] or more ASCII letters and
/// digits and no path separator.
fn word_is_a_token(word: &str) -> bool {
    if word.chars().any(is_separator) {
        return false;
    }
    let mut run = 0_usize;
    for c in word.chars() {
        if c.is_ascii_alphanumeric() {
            run += 1;
            if run >= TOKEN_RUN {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// Whether `word` is `/<a>/<b>/<c>/<field>` with `field` among `fields`.
fn word_is_a_parameter_path(word: &str, fields: &[&str]) -> bool {
    let Some(rest) = word.strip_prefix('/') else {
        return false;
    };
    let mut segments = rest.split('/');
    let mut last = "";
    for _ in 0..PATH_SEGMENTS {
        match segments.next() {
            Some(segment) if !segment.is_empty() => last = segment,
            _ => return false,
        }
    }
    segments.next().is_none() && fields.contains(&last)
}

/// Whether some word of `value` has the shape of a pasted token.
///
/// A value over [`MAX_VALUE_BYTES`] answers `true`: it is not read, and a
/// caller that asks this question refuses it.
#[must_use]
pub fn value_looks_like_a_token(value: &str) -> bool {
    if value.len() > MAX_VALUE_BYTES {
        return true;
    }
    // A VALUE THAT OPENS WITH A SEPARATOR IS A PATH, whole: a folder may hold
    // spaces, and a long word after one is a segment, not a paste.
    !value.trim_start().starts_with(is_separator) && value.split_whitespace().any(word_is_a_token)
}

/// Whether some word of `value` is a credential parameter path whose last
/// segment is one of `fields`.
///
/// A value over [`MAX_VALUE_BYTES`] answers `true`, as above.
#[must_use]
pub fn value_looks_like_a_parameter_path(value: &str, fields: &[&str]) -> bool {
    value.len() > MAX_VALUE_BYTES
        || value
            .split_whitespace()
            .any(|word| word_is_a_parameter_path(word, fields))
}

/// The shape `value` has, or `None`: the cap first, then the path (a more
/// specific answer than the token), then the token.
#[must_use]
pub fn shape_of(value: &str, fields: &[&str]) -> Option<Shape> {
    if value.len() > MAX_VALUE_BYTES {
        Some(Shape::OverCap)
    } else if value_looks_like_a_parameter_path(value, fields) {
        Some(Shape::ParameterPath)
    } else if value_looks_like_a_token(value) {
        Some(Shape::Token)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    const FIELDS: [&str; 2] = ["access-token", "api-key"];

    #[test]
    fn exactly_twenty_is_refused_and_nineteen_is_allowed() {
        let twenty = "a1b2c3d4e5f6g7h8i9j0";
        assert_eq!(twenty.len(), TOKEN_RUN);
        assert!(value_looks_like_a_token(twenty));
        assert_eq!(shape_of(twenty, &FIELDS), Some(Shape::Token));
        let nineteen = &twenty[..19];
        assert!(!value_looks_like_a_token(nineteen));
        assert_eq!(shape_of(nineteen, &FIELDS), None);
        // Inside a sentence, the word alone decides.
        assert!(value_looks_like_a_token(&format!("pasted {twenty} here")));
        assert!(!value_looks_like_a_token(&format!(
            "pasted {nineteen} here"
        )));
        // Upper case and digits count the same: an all-capital key is a key.
        assert!(value_looks_like_a_token("ABCDEFGHIJKLMNOPQRST"));
        assert!(value_looks_like_a_token("01234567890123456789"));
    }

    #[test]
    fn the_operators_own_values_are_allowed() {
        for value in [
            "2018-01-01",
            "2026-09-30",
            "zerodha",
            "gdfl",
            "NSE-INDEX-NIFTY",
            // Twenty characters with the hyphens, ten without: a store symbol.
            "NSE-INDEX-NIFTYNXT50",
            "NSE-INDEX-NIFTY,NSE-INDEX-BANKNIFTY,NSE-INDEX-MIDCPNIFTY",
            "finest",
            "newest",
            "INDEX",
            // A LONG LEGITIMATE PATH, with a long segment and spaces in it.
            "/Volumes/WD_BLACK/NSE (Stock+Indices)_01.09.2018 to 24.09.2026_Tick.zip",
            "/Volumes/WD_BLACK/brutex/fresh20260919workfolderwithaverylongname/masters",
            "/Volumes/WD_BLACK/an archive folder with a twentyfivecharactername",
            "refold coarse rungs for the special session under D-3319",
        ] {
            assert_eq!(shape_of(value, &FIELDS), None, "{value}");
        }
    }

    #[test]
    fn a_parameter_path_is_recognised_by_its_field_and_its_segments() {
        assert!(value_looks_like_a_parameter_path(
            "/org/env/vendor/access-token",
            &FIELDS
        ));
        assert_eq!(
            shape_of("/org/env/vendor/access-token", &FIELDS),
            Some(Shape::ParameterPath)
        );
        assert!(value_looks_like_a_parameter_path(
            "see /o/e/v/api-key now",
            &FIELDS
        ));
        for not in [
            "/org/env/vendor/masters",
            "/org/env/access-token",
            "/a/b/c/d/access-token",
            "org/env/vendor/access-token",
            "/org//vendor/access-token",
            "/org/env/vendor/",
        ] {
            assert!(!value_looks_like_a_parameter_path(not, &FIELDS), "{not}");
        }
        // No field passed, no path refused: core holds no field of its own.
        assert!(!value_looks_like_a_parameter_path(
            "/org/env/vendor/access-token",
            &[]
        ));
    }

    #[test]
    fn a_value_over_the_cap_is_refused_unread() {
        let over = "a ".repeat(MAX_VALUE_BYTES / 2 + 1);
        assert!(over.len() > MAX_VALUE_BYTES);
        assert_eq!(shape_of(&over, &FIELDS), Some(Shape::OverCap));
        assert!(value_looks_like_a_token(&over));
        assert!(value_looks_like_a_parameter_path(&over, &FIELDS));
        let at = "a ".repeat(MAX_VALUE_BYTES / 2);
        assert_eq!(at.len(), MAX_VALUE_BYTES);
        assert_eq!(shape_of(&at, &FIELDS), None);
        assert_eq!(Shape::OverCap.class(), "over_cap");
        assert_eq!(Shape::Token.class(), "token");
        assert_eq!(Shape::ParameterPath.class(), "parameter_path");
    }

    #[test]
    fn empty_and_delimited_values_are_allowed() {
        for value in ["", " ", "a-b_c.d+e=f", "abcdefghij-abcdefghij", "\\\\x"] {
            assert_eq!(shape_of(value, &FIELDS), None, "{value:?}");
        }
        // A token behind a backslash is a path word, judged as a path.
        assert!(!value_looks_like_a_token("C:\\abcdefghijabcdefghijk"));
    }
}

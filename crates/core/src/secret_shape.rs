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
//!   [`TOKEN_RUN`] characters. In a value that opens with a separator (a
//!   path, which may hold spaces) every such word is read too, and is a token
//!   when its run mixes letters and digits, so `/x <token>` is refused and a
//!   long all-letter folder word is not. A full-width form reads as its ASCII
//!   character and a zero-width character as nothing, so neither hides a
//!   token. The alphabet is base64url's and hex's letters
//!   and digits; base64url's own `-` and `_` delimit the run, exactly as they
//!   do in `pull::config`'s "undelimited alphanumeric" backstop. They must:
//!   every store symbol this system opens on is upper-case words joined by
//!   `-` (`NSE-INDEX-NIFTYNXT50` is twenty characters), and counting the `-`
//!   would refuse the operator's own instrument (D-5321). A word holding a path
//!   separator is a path and is judged by the second shape alone, so a long
//!   legitimate folder is allowed.
//! * [`value_looks_like_a_parameter_path`]: a word that, from its first `/`,
//!   has exactly four segments, the last of which is one of the credential
//!   field names the caller passes, in any letter case. Repeated, leading and
//!   trailing `/` and `.` segments are no segments, and a quote or a stop
//!   after the field is not part of it.
//!
//! # What it is not
//!
//! A heuristic over a shape, not a proof about entropy (`CLAUDE.md` §3 rule
//! 6). It misses a token shorter than [`TOKEN_RUN`], one whose every run is
//! broken by `-`, `_`, `.`, `+` or `=` (a base64url token can be), one pasted
//! as a segment of a path, and one of only letters after a path's first
//! word. What it refuses wrongly is an unbroken run of twenty letters and
//! digits that is not a secret — a commit hash pasted into a reason is one —
//! and that refusal is loud, names the key, and keeps nothing.
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

/// The character `c` reads as for the shape tests, or `None` when it reads
/// as nothing. A full-width ASCII form (U+FF01..=U+FF5E) reads as its ASCII
/// letter, digit or sign, so a token typed in full width is a token; a
/// zero-width or soft-hyphen format character (U+00AD, U+200B..=U+200F,
/// U+2060..=U+2064, U+FEFF) reads as nothing — it neither counts toward a
/// run nor breaks one, so a token split by an invisible character is still
/// one.
fn read_as(c: char) -> Option<char> {
    match c {
        '\u{00AD}' | '\u{200B}'..='\u{200F}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' => None,
        '\u{FF01}'..='\u{FF5E}' => char::from_u32(u32::from(c) - 0xFEE0),
        other => Some(other),
    }
}

/// Whether `word` holds a run of [`TOKEN_RUN`] or more ASCII letters and
/// digits (read through [`read_as`]) and no path separator. With `mixed`,
/// the run must hold a letter and a digit both: the test a word after a
/// path's first word gets, so a long all-letter folder word is a folder.
fn word_is_a_token(word: &str, mixed: bool) -> bool {
    if word.chars().filter_map(read_as).any(is_separator) {
        return false;
    }
    let (mut run, mut letter, mut digit) = (0_usize, false, false);
    for c in word.chars().filter_map(read_as) {
        if c.is_ascii_alphanumeric() {
            run += 1;
            letter |= c.is_ascii_alphabetic();
            digit |= c.is_ascii_digit();
            if run >= TOKEN_RUN && (!mixed || (letter && digit)) {
                return true;
            }
        } else {
            (run, letter, digit) = (0, false, false);
        }
    }
    false
}

/// Whether `word`, from its first `/`, is `/<a>/<b>/<c>/<field>` with
/// `field` among `fields`, read tolerantly: repeated, leading and trailing
/// `/` and `.` segments are no segments, the field's letter case does not
/// matter, a quote or a stop after it is not part of it, and a full-width or
/// zero-width character reads as [`read_as`] says. So `path='//org/./env/
/// vendor/ACCESS-TOKEN/'` is a parameter path.
fn word_is_a_parameter_path(word: &str, fields: &[&str]) -> bool {
    let read: String = word.chars().filter_map(read_as).collect();
    let Some(at) = read.find('/') else {
        return false;
    };
    let segments: Vec<&str> = read[at..]
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();
    segments.len() == PATH_SEGMENTS
        && segments.last().is_some_and(|last| {
            let last = last
                .trim_end_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'));
            fields.iter().any(|field| field.eq_ignore_ascii_case(last))
        })
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
    // A VALUE THAT OPENS WITH A SEPARATOR IS A PATH: a folder may hold
    // spaces, and its words are folder words. Every word is still read; a
    // word without a separator in such a value is a token only when its run
    // mixes letters and digits, as a pasted token's does and a folder's
    // word seldom does (`/x <token>` was let through whole before, R4-06
    // repair 1).
    let path = value
        .trim_start()
        .chars()
        .find_map(read_as)
        .is_some_and(is_separator);
    value
        .split_whitespace()
        .any(|word| word_is_a_token(word, path))
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

    /// An invented 32-character token, letters and digits mixed.
    const PASTE: &str = "q7Zt0invented0token0value0x9K2mW";

    /// **EACH EVASION THE SECOND AGENT TRIED** (`verify-R4-06-r1.json`
    /// attacks, 16 Oct) answered `None` before this change; each is refused
    /// now, by the shape it has.
    #[test]
    fn every_evasion_the_second_agent_tried_is_refused() {
        let full_width: String = PASTE
            .chars()
            .map(|c| char::from_u32(u32::from(c) + 0xFEE0).unwrap())
            .collect();
        let zero_width = format!("{}\u{200B}{}", &PASTE[..12], &PASTE[12..]);
        let soft = format!("{}\u{00AD}{}", &PASTE[..10], &PASTE[10..]);
        for (value, shape) in [
            (format!("/x {PASTE}"), Shape::Token),
            (format!("/Volumes/WD_BLACK/a folder {PASTE}"), Shape::Token),
            (full_width, Shape::Token),
            (zero_width, Shape::Token),
            (soft, Shape::Token),
            (
                "/org/env/vendor/access-token/".to_owned(),
                Shape::ParameterPath,
            ),
            (
                "//org/env/vendor/access-token".to_owned(),
                Shape::ParameterPath,
            ),
            (
                "/org//env/vendor/access-token".to_owned(),
                Shape::ParameterPath,
            ),
            (
                "/ORG/ENV/VENDOR/ACCESS-TOKEN".to_owned(),
                Shape::ParameterPath,
            ),
            (
                "/org/./env/vendor/access-token".to_owned(),
                Shape::ParameterPath,
            ),
            (
                "path='/org/env/vendor/api-key'".to_owned(),
                Shape::ParameterPath,
            ),
            ("/org/env/vendor/Api-Key.".to_owned(), Shape::ParameterPath),
            (
                "/org/env/vendor/access\u{200B}-token".to_owned(),
                Shape::ParameterPath,
            ),
        ] {
            assert_eq!(shape_of(&value, &FIELDS), Some(shape), "{value:?}");
        }
        // STILL ALLOWED: a folder whose later word is long and all letters,
        // a path of five segments, one of three, and a field-like word that
        // is not the field.
        for value in [
            "/Volumes/WD_BLACK/an archive folder with a twentyfivecharactername",
            "/a/b/c/d/access-token",
            "env/vendor/access-token",
            "/org/env/vendor/access-tokens",
        ] {
            assert_eq!(shape_of(value, &FIELDS), None, "{value}");
        }
        // THE LIMITS STATED IN THE MODULE DOCS, pinned so a change to them
        // is a change someone sees: a token as a path segment, a base64url
        // token whose runs `-` breaks, and one of letters only after a
        // path's first word.
        for value in [
            format!("/Volumes/{PASTE}"),
            "q7Zt0inven-ted0token0-value0x9K2mW".to_owned(),
            "/x abcdefghijabcdefghijabcdefghij".to_owned(),
        ] {
            assert_eq!(shape_of(&value, &FIELDS), None, "{value}");
        }
    }

    #[test]
    fn a_full_width_or_zero_width_character_reads_as_the_shape_tests_say() {
        assert_eq!(read_as('\u{FF21}'), Some('A'));
        assert_eq!(read_as('\u{FF0F}'), Some('/'));
        assert_eq!(read_as('\u{FF5E}'), Some('~'));
        assert_eq!(read_as('\u{FF5F}'), Some('\u{FF5F}'));
        for c in [
            '\u{00AD}', '\u{200B}', '\u{200F}', '\u{2060}', '\u{2064}', '\u{FEFF}',
        ] {
            assert_eq!(read_as(c), None, "{c:?}");
        }
        assert_eq!(read_as('\u{2065}'), Some('\u{2065}'));
        // A full-width solidus is a separator: such a word is a path word.
        assert!(!value_looks_like_a_token(&format!("x\u{FF0F}{PASTE}")));
    }
}

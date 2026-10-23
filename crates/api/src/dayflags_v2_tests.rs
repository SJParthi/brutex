//! IA-16's guard of the D-3335 flags-line reader (D-4841; D-6900, moved in
//! from IA-10b): a `v=2` line reads like a `v=1` line, a `v=2` field the
//! line lacks is `null` and never `0`, and a version this reader does not
//! know is refused by name. The lines are built from the reader's own field
//! tables, every value invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;

use super::*;

/// One `flags` line of `kind` at version `v`: every field of the kind's
/// table at an invented value (`7`, or `-` for a clock), then `more`.
fn line(kind: ImportKind, v: &str, more: &str) -> String {
    let scope = if kind == ImportKind::Indices {
        "NIFTY"
    } else {
        "*"
    };
    let mut text = format!(
        "flags {} 2024-04-02 {scope} v={v} src=import definition=4",
        kind.as_str()
    );
    for (name, shape) in EVERY_KIND.iter().chain(own_fields(kind)) {
        let value = match shape {
            Shape::Count => "7",
            Shape::Clock => "-",
        };
        let _ = write!(text, " {name}={value}");
    }
    if !more.is_empty() {
        text.push(' ');
        text.push_str(more);
    }
    text
}

/// The names of the `v=2` fields of `kind` (D-4841).
fn v2_of(kind: ImportKind) -> Vec<&'static str> {
    let own: &[(&str, Shape)] = match kind {
        ImportKind::Indices => &V2_INDICES,
        ImportKind::Stocks => &V2_STOCKS,
        ImportKind::Options => &V2_OPTIONS,
    };
    V2_EVERY.iter().chain(own).map(|(name, _)| *name).collect()
}

#[test]
fn a_v2_flags_line_reads_like_v1_and_a_missing_v2_field_is_never_zero() {
    for kind in [ImportKind::Indices, ImportKind::Stocks, ImportKind::Options] {
        let fields = v2_of(kind);
        assert!(!fields.is_empty(), "{kind:?}");
        // v=1: read, every v=2 field a key of the record and null.
        let one = parse_quality(1, &line(kind, "1", "")).unwrap();
        let json = one.json();
        for name in &fields {
            assert_eq!(one.count(name), None, "{kind:?} {name}");
            assert!(
                json.as_object().unwrap().contains_key(*name),
                "{kind:?} {name}"
            );
            assert_eq!(json[*name], Value::Null, "{kind:?} {name}: never 0");
        }
        // v=2 carrying only its first field: that one is read, every other
        // stays null; the v=1 fields read the same as on the v=1 line.
        let first = fields[0];
        let two = parse_quality(2, &line(kind, "2", &format!("{first}=0"))).unwrap();
        assert_eq!(two.count(first), Some(0), "{kind:?}: a real 0 is a 0");
        for name in &fields[1..] {
            assert_eq!(two.json()[*name], Value::Null, "{kind:?} {name}");
        }
        assert_eq!(two.fields, one.fields, "{kind:?}: v=1's fields alike");
        // v=2 carrying every field: each read.
        let all: Vec<String> = fields
            .iter()
            .enumerate()
            .map(|(at, name)| format!("{name}={}", at + 1))
            .collect();
        let full = parse_quality(3, &line(kind, "2", &all.join(" "))).unwrap();
        for (at, name) in fields.iter().enumerate() {
            assert_eq!(
                full.count(name),
                Some(u64::try_from(at + 1).unwrap()),
                "{kind:?} {name}"
            );
        }
        // A v=2 field twice, or not a count, is refused by name.
        let twice = format!("{first}=1 {first}=2");
        let refused = parse_quality(4, &line(kind, "2", &twice)).unwrap_err();
        assert!(refused.contains("appears twice"), "{kind:?}: {refused}");
        let refused = parse_quality(5, &line(kind, "2", &format!("{first}=x"))).unwrap_err();
        assert!(refused.contains("is not a Count"), "{kind:?}: {refused}");
        // A version this reader does not know, refused by name.
        for v in ["3", "0", "", "2.0"] {
            let refused = parse_quality(6, &line(kind, v, "")).unwrap_err();
            assert!(
                refused.contains(&format!("v={v} is not 1 or 2")),
                "{kind:?} v={v}: {refused}"
            );
        }
        let refused = parse_quality(7, &line(kind, "1", "").replace(" v=1", "")).unwrap_err();
        assert!(refused.contains("v=(none)"), "{kind:?}: {refused}");
    }
}

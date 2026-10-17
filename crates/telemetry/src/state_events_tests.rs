//! The state-change table's proofs (R3-24).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::{Class, StateEvent, TABLE, class_of, is_state_change, row_of};
use crate::level::Level;
use crate::sink::is_audited;

/// The ten state changes the audit of 12 Oct found reaching the short log
/// alone, in its own words (R3-24's goal, item 4).
const NAMED: [&str; 10] = [
    "autopilot halted",
    "autopilot revived",
    "stop requested",
    "masters refreshed",
    "census migrated",
    "census compacted",
    "census recounted",
    "server started",
    "server stopped",
    "census lag",
];

/// Every emit site a row names, as text: the files are read at compile time,
/// so a row whose site moved fails to build rather than to match.
const SITES: [(&str, &str); 9] = [
    (
        "crates/api/src/autopilot.rs",
        include_str!("../../api/src/autopilot.rs"),
    ),
    (
        "crates/api/src/mastersrun.rs",
        include_str!("../../api/src/mastersrun.rs"),
    ),
    (
        "crates/pull/src/census_store.rs",
        include_str!("../../pull/src/census_store.rs"),
    ),
    (
        "crates/api/src/server.rs",
        include_str!("../../api/src/server.rs"),
    ),
    (
        "crates/pull/src/ingest.rs",
        include_str!("../../pull/src/ingest.rs"),
    ),
    (
        "crates/store/src/sort_index_writer.rs",
        include_str!("../../store/src/sort_index_writer.rs"),
    ),
    (
        "crates/store/src/month_summary_writer.rs",
        include_str!("../../store/src/month_summary_writer.rs"),
    ),
    (
        "crates/cli/src/lib.rs",
        include_str!("../../cli/src/lib.rs"),
    ),
    (
        "crates/pull/src/gdfl_import_day.rs",
        include_str!("../../pull/src/gdfl_import_day.rs"),
    ),
];

/// The one site file `gdfl.import` is spelt in, apart from its emit site.
const GDFL_TARGET_AT: (&str, &str) = (
    "crates/pull/src/gdfl_import.rs",
    include_str!("../../pull/src/gdfl_import.rs"),
);

/// The text of a site file a row names.
fn site_text(path: &str) -> &'static str {
    SITES
        .iter()
        .chain(std::iter::once(&GDFL_TARGET_AT))
        .find(|(named, _)| *named == path)
        .map(|(_, text)| *text)
        .unwrap_or_else(|| panic!("{path} is named by a row and not read by this test"))
}

/// THE TABLE TEST: every row, at every level its site emits it at, reaches
/// the audit stream and is one of the state stream's lines. Before R3-24
/// "listening", "census compacted", "census migrated", "the instrument masters
/// were refreshed from the browser" and "hand runs asked to stop" reached the
/// rolling ring alone, so this failed on six of them.
#[test]
fn every_state_change_event_is_audited() {
    for row in &TABLE {
        assert!(!row.levels.is_empty(), "{row:?} names no level");
        for level in row.levels {
            assert!(
                is_audited(*level, row.target, row.message),
                "{} / {} at {} would not reach the audit stream ({})",
                row.target,
                row.message,
                level.label(),
                row.change
            );
            assert!(
                is_state_change(*level, row.target, row.message),
                "{} / {} at {} would not reach the state stream",
                row.target,
                row.message,
                level.label()
            );
        }
    }
    for change in NAMED {
        assert!(
            TABLE.iter().any(|row| row.change == change),
            "the state change {change:?} has no row"
        );
    }
    for class in Class::ALL {
        assert!(
            TABLE.iter().any(|row| row.class == class),
            "{class:?} has no row"
        );
    }
}

/// A row names an emit that exists: its message, quoted, in its site file and
/// its target, quoted, in the file that spells it.
#[test]
fn every_row_names_its_emit_site() {
    for row in &TABLE {
        let site = site_text(row.site);
        assert!(
            site.contains(&format!("\"{}\"", row.message)),
            "{} does not emit {:?}",
            row.site,
            row.message
        );
        let target = site_text(row.target_at);
        assert!(
            target.contains(&format!("\"{}\"", row.target)),
            "{} does not spell the target {:?}",
            row.target_at,
            row.target
        );
    }
    // EVERY FILE READ IS ONE A ROW NAMES: no stale site lingers here.
    for (path, _) in SITES.iter().chain(std::iter::once(&GDFL_TARGET_AT)) {
        assert!(
            TABLE
                .iter()
                .any(|row| row.site == *path || row.target_at == *path),
            "{path} is read and no row names it"
        );
    }
}

/// The table is a set, and each row fits on a line whole: a target or a
/// message past its ceiling is cut when encoded, and a cut row would never
/// match what was written.
#[test]
fn the_table_is_a_set_of_lines_that_fit() {
    for (n, row) in TABLE.iter().enumerate() {
        assert!(
            row.target.len() <= crate::event::MAX_TARGET_BYTES,
            "{row:?}"
        );
        assert!(
            row.message.len() <= crate::event::MAX_MESSAGE_BYTES,
            "{row:?}"
        );
        assert!(!row.change.is_empty() && !row.site.is_empty());
        for other in TABLE.iter().skip(n + 1) {
            assert!(
                (row.target, row.message) != (other.target, other.message),
                "{row:?} twice"
            );
        }
        assert_eq!(row_of(row.target, row.message), Some(row));
        assert_eq!(class_of(row.target, row.message), Some(row.class));
    }
}

/// What the table does NOT take, and what the state stream leaves to the
/// rolling audit stream: a refusal is audited and is no state change; an
/// access record is audited and is no state change; a `warn` that is no row
/// is neither; a row below `info` is neither; a lifecycle line is both.
#[test]
fn a_refusal_is_audited_and_is_no_state_change() {
    let refusal = ("pull.spot", "member did not land");
    assert!(is_audited(Level::Error, refusal.0, refusal.1));
    assert!(!is_state_change(Level::Error, refusal.0, refusal.1));
    assert_eq!(class_of(refusal.0, refusal.1), None);

    assert!(is_audited(Level::Info, "api.access", "read served"));
    assert!(!is_state_change(Level::Info, "api.access", "read served"));

    assert!(!is_audited(Level::Warn, "pull.derive", "rung not derived"));
    assert!(!is_state_change(
        Level::Warn,
        "pull.derive",
        "rung not derived"
    ));

    assert!(!is_audited(Level::Debug, "api.serve", "listening"));
    assert!(!is_state_change(Level::Debug, "api.serve", "listening"));
    assert!(!is_state_change(Level::Trace, "autopilot", "paused"));

    // A ROW'S NEIGHBOURS ARE NOT ROWS: one byte off in either half.
    assert_eq!(row_of("api.serve", "listening "), None);
    assert_eq!(row_of("api.serv", "listening"), None);
    assert!(!is_audited(Level::Info, "api.serve", "listening!"));

    for (target, message) in [
        ("cli.lifecycle", "command refused"),
        ("gdfl.import", "run started"),
        ("gdfl.import.lifecycle", "claimed"),
    ] {
        assert!(is_audited(Level::Info, target, message), "{target}");
        assert!(is_state_change(Level::Info, target, message), "{target}");
    }
    // A RUN STOPPED BY A REFUSAL is both: the refusal's line names the stop.
    assert!(is_state_change(
        Level::Error,
        "gdfl.import",
        "census refused; run stopped"
    ));
}

/// Every class has its own word, and the page lists them in one order.
#[test]
fn every_class_has_its_own_word() {
    let words: Vec<&str> = Class::ALL.iter().map(|class| class.as_str()).collect();
    assert_eq!(words, ["operator", "server", "maintenance", "lag", "run"]);
    let row: &StateEvent = row_of("autopilot", "paused").expect("a row");
    assert_eq!(row.class.as_str(), "operator");
}

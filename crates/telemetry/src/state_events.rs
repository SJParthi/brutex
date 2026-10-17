//! The state changes: one table naming every event that records a change of
//! the system's own state, with its class (R3-24).
//!
//! # Why a table, and why here
//!
//! The audit stream took an event by its SHAPE: every `error`, and at `info`
//! or above a target ending `lifecycle` or `access`, or a message holding one
//! of `AUDIT_LIFECYCLE_WORDS`. A state change whose words happened to miss
//! that shape reached the short rolling ring alone and was gone within
//! minutes under a pull's flood (DB-023: an 8 MiB file rolled about every
//! 35 s). The audit of 12 Oct found ten of them: the autopilot halted and
//! revived, a stop requested, the masters refreshed, the census migrated,
//! compacted and recounted, the server started and stopped, and a census
//! that lags its bars. "listening" says nothing a word rule can see, and
//! "census compacted" is not a lifecycle word.
//!
//! So the state changes are NAMED, not guessed: each row is one
//! `(target, message)` a writer emits, the levels it emits it at, its
//! [`Class`], and the file its emit sits in. `crate::sink::is_audited` takes
//! every row at `info` or above, and [`is_state_change`] is what the sink's
//! never-rolling state stream (`STATE_STREAM_DIR`) keeps. `telemetry`
//! depends on nothing and every writer names it, which is the reason
//! [`crate::run_dirs`] gives for its own table; the two tables live side by
//! side for that reason.
//!
//! # What binds a row to its writer
//!
//! `telemetry` may not depend on the crates that emit these events, so the
//! tie is a test that reads each row's emit site as text
//! (`state_events_tests::every_row_names_its_emit_site`): a message renamed
//! at its site fails the build here instead of leaving a row that matches
//! nothing. And `every_state_change_event_is_audited` fails the build the day
//! a row stops reaching the audit stream at any level it is emitted at.
//!
//! # Cost
//!
//! [`row_of`] compares against at most [`TABLE`]'s length of rows, a constant
//! of this build; it runs only for an event at `info` or above that the shape
//! rule did not already take.

use crate::level::Level;

/// What kind of state a row changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// An operator's decision: a pause, a stop, a refresh asked for.
    Operator,
    /// The server process itself: it started serving, it began to stop.
    Server,
    /// The store's own bookkeeping changed form: a census migrated,
    /// compacted, sealed or recounted, or the universe re-parsed.
    Maintenance,
    /// A derived record fell behind what it derives from: a census that does
    /// not count bars on disk, an index or summary behind its month.
    Lag,
    /// A run began or ended.
    Run,
}

impl Class {
    /// Every class, in the order the page lists them.
    pub const ALL: [Self; 5] = [
        Self::Operator,
        Self::Server,
        Self::Maintenance,
        Self::Lag,
        Self::Run,
    ];

    /// The word an answer names it by.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::Server => "server",
            Self::Maintenance => "maintenance",
            Self::Lag => "lag",
            Self::Run => "run",
        }
    }
}

/// One state-change event a writer in this workspace emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateEvent {
    /// The event's target, exactly as emitted.
    pub target: &'static str,
    /// The event's message, exactly as emitted.
    pub message: &'static str,
    /// What kind of state it changes.
    pub class: Class,
    /// The state change in the operator's words, as the audit named it.
    pub change: &'static str,
    /// Every level the site emits it at.
    pub levels: &'static [Level],
    /// The file, from the workspace root, whose emit writes it.
    pub site: &'static str,
    /// The file, from the workspace root, that spells the target: the site
    /// itself unless the site names a constant defined elsewhere.
    pub target_at: &'static str,
}

/// Every state-change event, by class. A new state change is a row here, at
/// the same commit as its emit.
pub const TABLE: [StateEvent; 21] = [
    StateEvent {
        target: "autopilot",
        message: "paused",
        class: Class::Operator,
        change: "autopilot halted",
        levels: &[Level::Warn],
        site: "crates/api/src/autopilot.rs",
        target_at: "crates/api/src/autopilot.rs",
    },
    StateEvent {
        target: "autopilot",
        message: "resumed",
        class: Class::Operator,
        change: "autopilot revived",
        levels: &[Level::Info],
        site: "crates/api/src/autopilot.rs",
        target_at: "crates/api/src/autopilot.rs",
    },
    StateEvent {
        target: "api.pull",
        message: "hand runs asked to stop",
        class: Class::Operator,
        change: "stop requested",
        levels: &[Level::Warn],
        site: "crates/api/src/autopilot.rs",
        target_at: "crates/api/src/autopilot.rs",
    },
    StateEvent {
        target: "api.masters",
        message: "the instrument masters were refreshed from the browser",
        class: Class::Operator,
        change: "masters refreshed",
        levels: &[Level::Warn, Level::Error],
        site: "crates/api/src/mastersrun.rs",
        target_at: "crates/api/src/mastersrun.rs",
    },
    StateEvent {
        target: "api.masters.reload",
        message: "the universe was re-parsed and every page now answers from it",
        class: Class::Maintenance,
        change: "masters refreshed",
        levels: &[Level::Info],
        site: "crates/api/src/mastersrun.rs",
        target_at: "crates/api/src/mastersrun.rs",
    },
    StateEvent {
        target: "api.masters.reload",
        message: "the universe could NOT be re-parsed, so the previous one still stands",
        class: Class::Maintenance,
        change: "masters refreshed",
        levels: &[Level::Error],
        site: "crates/api/src/mastersrun.rs",
        target_at: "crates/api/src/mastersrun.rs",
    },
    StateEvent {
        target: "pull.census",
        message: "census migrated",
        class: Class::Maintenance,
        change: "census migrated",
        levels: &[Level::Info],
        site: "crates/pull/src/census_store.rs",
        target_at: "crates/pull/src/census_store.rs",
    },
    StateEvent {
        target: "pull.census",
        message: "legacy census sealed",
        class: Class::Maintenance,
        change: "census migrated",
        levels: &[Level::Info],
        site: "crates/pull/src/census_store.rs",
        target_at: "crates/pull/src/census_store.rs",
    },
    StateEvent {
        target: "pull.census",
        message: "census compacted",
        class: Class::Maintenance,
        change: "census compacted",
        levels: &[Level::Info],
        site: "crates/pull/src/census_store.rs",
        target_at: "crates/pull/src/census_store.rs",
    },
    StateEvent {
        target: "pull.census",
        message: "census recounted",
        class: Class::Maintenance,
        change: "census recounted",
        levels: &[Level::Info, Level::Warn],
        site: "crates/pull/src/census_store.rs",
        target_at: "crates/pull/src/census_store.rs",
    },
    StateEvent {
        target: "api.serve",
        message: "listening",
        class: Class::Server,
        change: "server started",
        levels: &[Level::Info],
        site: "crates/api/src/server.rs",
        target_at: "crates/api/src/server.rs",
    },
    StateEvent {
        target: "api.serve",
        message: "stop started",
        class: Class::Server,
        change: "server stopped",
        levels: &[Level::Info],
        site: "crates/api/src/server.rs",
        target_at: "crates/api/src/server.rs",
    },
    StateEvent {
        target: "api.serve",
        message: "stop asked during the start-up load",
        class: Class::Server,
        change: "server stopped",
        levels: &[Level::Info],
        site: "crates/api/src/server.rs",
        target_at: "crates/api/src/server.rs",
    },
    StateEvent {
        target: "pull.census",
        message: "loaded degraded",
        class: Class::Lag,
        change: "census lag",
        levels: &[Level::Warn],
        site: "crates/pull/src/ingest.rs",
        target_at: "crates/pull/src/ingest.rs",
    },
    StateEvent {
        target: "pull.census",
        message: "not published",
        class: Class::Lag,
        change: "census lag",
        levels: &[Level::Error],
        site: "crates/pull/src/ingest.rs",
        target_at: "crates/pull/src/ingest.rs",
    },
    StateEvent {
        target: "pull.census",
        message: "bars not counted",
        class: Class::Lag,
        change: "census lag",
        levels: &[Level::Error],
        site: "crates/pull/src/ingest.rs",
        target_at: "crates/pull/src/ingest.rs",
    },
    StateEvent {
        target: "store.srt",
        message: "sort index lags the bars",
        class: Class::Lag,
        change: "index lag",
        levels: &[Level::Warn],
        site: "crates/store/src/sort_index_writer.rs",
        target_at: "crates/store/src/sort_index_writer.rs",
    },
    StateEvent {
        target: "store.sum",
        message: "month summary lags the bars",
        class: Class::Lag,
        change: "index lag",
        levels: &[Level::Warn],
        site: "crates/store/src/month_summary_writer.rs",
        target_at: "crates/store/src/month_summary_writer.rs",
    },
    StateEvent {
        target: "cli.lifecycle",
        message: "command started",
        class: Class::Run,
        change: "run started",
        levels: &[Level::Info],
        site: "crates/cli/src/lib.rs",
        target_at: "crates/cli/src/lib.rs",
    },
    StateEvent {
        target: "cli.lifecycle",
        message: "command finished",
        class: Class::Run,
        change: "run finished",
        levels: &[Level::Info],
        site: "crates/cli/src/lib.rs",
        target_at: "crates/cli/src/lib.rs",
    },
    StateEvent {
        target: "gdfl.import",
        message: "run finished",
        class: Class::Run,
        change: "run finished",
        levels: &[Level::Info],
        site: "crates/pull/src/gdfl_import_day.rs",
        target_at: "crates/pull/src/gdfl_import.rs",
    },
];

/// The row `target` and `message` are, or `None`. At most [`TABLE`]'s length
/// of comparisons, each failing at the first differing byte.
#[must_use]
pub fn row_of(target: &str, message: &str) -> Option<&'static StateEvent> {
    TABLE
        .iter()
        .find(|row| row.target == target && row.message == message)
}

/// The class of the state change `target` and `message` record, or `None`
/// for an event that is no row of [`TABLE`].
#[must_use]
pub fn class_of(target: &str, message: &str) -> Option<Class> {
    row_of(target, message).map(|row| row.class)
}

/// Whether an event is one of the who-did-what lines the never-rolling state
/// stream keeps: at `info` or above, and a row of [`TABLE`] or a run's
/// lifecycle as the audit stream reads one (a target ending `lifecycle`, or a
/// message holding one of `AUDIT_LIFECYCLE_WORDS`).
///
/// A refusal is NOT one: refusals are the flood (DB-G88) and stay in the
/// rolling audit stream, which is exactly what lets this one never roll. An
/// access record is not one either: its writer samples it per minute, which
/// is a stream that grows by the day, not by the change.
#[must_use]
pub fn is_state_change(level: Level, target: &str, message: &str) -> bool {
    level.at_least(Level::Info)
        && (row_of(target, message).is_some() || crate::sink::is_lifecycle(target, message))
}

#[cfg(test)]
#[path = "state_events_tests.rs"]
mod tests;

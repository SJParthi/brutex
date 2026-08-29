//! Every runtime knob, settable by the caller that starts a run.
//!
//! # Why this exists
//!
//! Twenty-four `BRUTEX_*` names steer this engine — the support floor, the
//! screen cap, how many combinations are recorded, whether walk-forward runs,
//! the grid resolution, the ceiling. Every one of them was read straight from
//! the PROCESS ENVIRONMENT, and that has a consequence nobody chose:
//!
//! **a knob set in the environment can only be changed by restarting the
//! process.**
//!
//! MEASURED, 2026-08-29. The operator's server was started from their IDE with
//! no `BRUTEX_` variables at all. Every knob therefore took its built-in
//! default, and two of those defaults compose into a run that cannot finish:
//! `screen_cap` defaults to 10,000 and `validates` is
//! `raw.is_none_or(|v| v.trim() != "0")` — **unset means TRUE**. Pressing the
//! page's own Run Sweep button would have priced twenty times more combinations
//! than the previous run AND put walk-forward, PBO and the bootstrap on every
//! one of them. The button was reachable, the configuration was unreachable,
//! and the only way to see the difference was to read the process's environment
//! from outside it.
//!
//! That is the same failure this session has now hit three times in three
//! different clothes: a value that decides the answer, living somewhere the
//! person asking the question cannot see or reach. A hardcoded 20% support. A
//! hardcoded `validate: true`. Now a default that is only a default because
//! nothing was set.
//!
//! # What changes
//!
//! Nothing, when nobody sets a knob. `var` falls back to `std::env::var_os`, so
//! a process started with `BRUTEX_TOP=500` behaves exactly as it always did, and
//! every existing test that sets an environment variable still passes.
//!
//! What is new is that a caller can `set` a knob for the run it is about to
//! start, and `clear_all` it afterwards. The HTTP layer does this from the sweep
//! request, so the browser decides the support floor and the screen cap without
//! anyone restarting anything.
//!
//! # Why a process-wide store is safe here
//!
//! Because a sweep is already one-at-a-time. `sweeprun` refuses a second run
//! while `in_flight` is true — three separate guards — so there is never a
//! moment when two runs want different values for the same knob. This store is
//! no more global than the environment it replaces; it is only *reachable*.
//!
//! # Cost
//!
//! `CLAUDE.md` §3 rule 4 asks that each operation be O(1), and both are:
//!
//! | operation | cost | why |
//! |---|---|---|
//! | `var` (set) | **O(1)** | one hash probe |
//! | `var` (unset) | **O(1)** | one hash probe, then one `getenv` |
//! | `set` | **O(1)** | one hash insert |
//! | `clear_all` | O(set knobs) | and that count is bounded by 24 |
//!
//! The lock is an `RwLock`, so the read path — which is the one on every
//! knob lookup — does not serialise the rungs against each other. It is never
//! taken inside a loop over bars or candidates: every knob in this workspace is
//! read once per rung or once per run, which is the granularity gate 17
//! prescribes for anything that is not free.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// The knobs a caller has set for this process, empty until one does.
///
/// `OnceLock` rather than `LazyLock` for no reason beyond matching what the rest
/// of this workspace reaches for; the two are equivalent here.
fn store() -> &'static RwLock<HashMap<String, String>> {
    static STORE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Read a knob: what the caller set, or failing that the environment.
///
/// # Why the caller wins over the environment and not the other way round
///
/// Because the caller is the more specific statement. An environment variable
/// is set once when the process starts and describes every run it will ever
/// perform; a knob set on a request describes THIS run. When the two disagree
/// the request is the one that knows which question is being asked.
///
/// A poisoned lock is treated as "nobody set this knob" rather than as a panic.
/// The only way to poison it is a panic inside `set` or `clear_all`, neither of
/// which does anything that can panic — but if it somehow happened, falling
/// through to the environment degrades to exactly the behaviour this module
/// replaced, which is `CLAUDE.md` §4's "degrade loudly and name the reason, or
/// refuse" satisfied by there being nothing to name: the fallback IS the old
/// behaviour, not a silent substitute for it.
#[must_use]
pub fn var(name: &str) -> Option<String> {
    resolve(
        set_here(name).as_deref(),
        std::env::var_os(name)
            .map(|raw| raw.to_string_lossy().into_owned())
            .as_deref(),
    )
}

/// What this process has been asked to use for `name`, if anything.
fn set_here(name: &str) -> Option<String> {
    store().read().ok()?.get(name).cloned()
}

/// The precedence rule itself, over raw values.
///
/// Split out for the reason `cli::validates` is split out, in that function's
/// own words: *"the rule itself, over the raw value, so it can be tested
/// without touching the environment."* Setting a real environment variable in a
/// test needs an `unsafe` block, which this workspace denies — and rightly, since
/// the process environment is shared by every test running in parallel and a
/// test that mutates it can fail a test it never names.
fn resolve(here: Option<&str>, environment: Option<&str>) -> Option<String> {
    here.or(environment).map(str::to_owned)
}

/// Set one knob for every run this process performs until it is cleared.
///
/// An empty value CLEARS the knob rather than setting it to the empty string,
/// because "the browser sent nothing for this field" and "the browser sent an
/// empty string" arrive here identically over HTTP, and of the two readings only
/// one is ever meant. A knob whose value is `""` would then be read as SET, and
/// every parse of it would fail into a default — which is the same class of
/// silent fallback §4 bans.
pub fn set(name: &str, value: &str) {
    if let Ok(mut held) = store().write() {
        if value.trim().is_empty() {
            held.remove(name);
        } else {
            held.insert(name.to_owned(), value.trim().to_owned());
        }
    }
}

/// Serialises every TEST that touches this process-wide store.
///
/// # Why it lives here and not in a test module
///
/// Because it was in one, and one was not enough. `knobs::tests` had a private
/// `serially()` and `horizon_tests` grew its own, so the two modules serialised
/// against themselves and not against EACH OTHER -- and `clear_all` is not
/// scoped to a name. `horizon_tests::an_explicit_count_is_taken_verbatim` passed
/// alone and failed in the full suite on the first run after the second module
/// landed, which is exactly the signal a flaky test is worst at giving.
///
/// One lock for the crate. Any test that calls `set` or `clear_all` takes it.
#[cfg(test)]
pub(crate) fn serially() -> std::sync::MutexGuard<'static, ()> {
    static ONE_AT_A_TIME: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    ONE_AT_A_TIME
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Forget every knob, so the next run starts from the environment again.
///
/// Called after a run finishes rather than before one starts, so that a run
/// which panics cannot leave its settings behind for the next one.
pub fn clear_all() {
    if let Ok(mut held) = store().write() {
        held.clear();
    }
}

/// Every knob currently set, sorted, for the audit event that records them.
///
/// # Why this is reported and not merely applied
///
/// A run's identity already covers the knobs that change the answer, so two runs
/// at different settings are two runs. What the identity does NOT do is tell a
/// person reading the log which settings produced the row in front of them —
/// they would have to recompute a blake3 hash to find out. The operator's
/// standing requirement is that anything which runs must be auditable, and a
/// knob that steered a seven-hour sweep is exactly the thing an audit is for.
///
/// Sorted so two runs at the same settings log the same string, per §3 rule 5.
#[must_use]
pub fn describe() -> String {
    let Ok(held) = store().read() else {
        return String::new();
    };
    render(
        held.iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect(),
    )
}

/// The rendering itself, over the pairs, so the ordering can be tested without
/// the global store.
///
/// # Why this is split out and not tested through `describe`
///
/// Because the store is process-wide and the test harness runs tests in
/// PARALLEL THREADS. A test that sets three knobs and asserts the exact string
/// `describe` returns will see a fourth knob the moment another test sets one —
/// and it did, on the first run of this module: the suite failed once and passed
/// on the retry, which is the worst possible signal.
///
/// A flaky test is worse than no test, so the ordering property lives here where
/// nothing shared can reach it, and the store's own tests assert only about
/// names unique to themselves. Same reason `resolve` is split out of `var`.
fn render(mut pairs: Vec<(&str, &str)>) -> String {
    pairs.sort_unstable();
    pairs
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::serially;
    use super::{clear_all, render, resolve, set, set_here, var};

    /// The whole point: a knob nobody set reads through to the environment, so
    /// every existing caller and every existing test is unaffected.
    ///
    /// Tested over `resolve` rather than by setting a real variable, because
    /// `std::env::set_var` needs an `unsafe` block this workspace denies — and
    /// rightly: the environment is shared by every test running in parallel, so
    /// a test that mutates it can fail a test it never names.
    #[test]
    fn an_unset_knob_falls_through_to_the_environment() {
        assert_eq!(
            resolve(None, Some("from-the-environment")).as_deref(),
            Some("from-the-environment")
        );
        assert_eq!(resolve(None, None), None, "and nothing set is no answer");
    }

    /// The caller is the more specific statement, so it wins.
    #[test]
    fn a_set_knob_beats_the_environment() {
        assert_eq!(
            resolve(Some("caller"), Some("environment")).as_deref(),
            Some("caller"),
            "the request knows which question is being asked"
        );
        assert_eq!(
            resolve(Some("caller"), None).as_deref(),
            Some("caller"),
            "and it does not need the environment to have an opinion"
        );
    }

    /// `set` and `var` agree, and `clear_all` undoes it — the store half of the
    /// same property, over a name no environment would ever carry.
    #[test]
    fn setting_a_knob_is_what_var_then_reads() {
        let _serial = serially();
        clear_all();
        let name = "BRUTEX_KNOBS_TEST_ROUNDTRIP";
        assert_eq!(set_here(name), None, "nothing is set to begin with");
        set(name, "500");
        assert_eq!(var(name).as_deref(), Some("500"));
        clear_all();
        assert_eq!(
            var(name),
            None,
            "and clearing leaves nothing, since no environment carries this name"
        );
    }

    /// An empty value is ABSENCE, not the empty string — the distinction the
    /// doc comment argues for, and the one HTTP cannot make on its own.
    #[test]
    fn an_empty_value_clears_rather_than_setting_the_empty_string() {
        let _serial = serially();
        clear_all();
        let name = "BRUTEX_KNOBS_TEST_EMPTY";
        set(name, "500");
        assert_eq!(var(name).as_deref(), Some("500"));
        set(name, "   ");
        assert_eq!(
            var(name),
            None,
            "whitespace is nothing, not a value that parses to a default"
        );
        clear_all();
    }

    /// Values are trimmed, because a browser field carries whatever was typed
    /// and `" 500 "` must not parse differently from `"500"`.
    #[test]
    fn a_value_is_trimmed_before_it_is_stored() {
        let _serial = serially();
        clear_all();
        set("BRUTEX_KNOBS_TEST_TRIM", "  500  ");
        assert_eq!(var("BRUTEX_KNOBS_TEST_TRIM").as_deref(), Some("500"));
        clear_all();
    }

    /// Sorted, so two runs at the same settings log the same string — §3 rule 5
    /// reaching the audit trail and not only the output.
    ///
    /// Over `render` and not `describe`, so it needs no guard at all: nothing
    /// shared can reach it.
    #[test]
    fn render_is_sorted_so_two_runs_log_the_same_string() {
        let one = render(vec![
            ("BRUTEX_TOP", "500"),
            ("BRUTEX_SCREEN_CAP", "500"),
            ("BRUTEX_VALIDATE", "0"),
        ]);
        // The same three pairs handed over in a different order, which is what a
        // `HashMap` iteration gives you between two runs of the same program.
        let other = render(vec![
            ("BRUTEX_VALIDATE", "0"),
            ("BRUTEX_TOP", "500"),
            ("BRUTEX_SCREEN_CAP", "500"),
        ]);
        assert_eq!(
            one,
            "BRUTEX_SCREEN_CAP=500 BRUTEX_TOP=500 BRUTEX_VALIDATE=0"
        );
        assert_eq!(one, other, "iteration order must not reach the log");
        assert_eq!(render(Vec::new()), "", "and nothing set renders empty");
    }

    /// `clear_all` on an empty store is not an error, because it runs after
    /// EVERY run including one that set nothing.
    #[test]
    fn clearing_an_empty_store_is_not_an_error() {
        let _serial = serially();
        clear_all();
        clear_all();
    }
}

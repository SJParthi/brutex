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
//! | `count` | **O(1)** | one `var`, one parse, and at most one tree insert |
//! | `clear_all` | O(set knobs) | and that count is bounded by 24 |
//! | `refused` | O(refused knobs) | bounded by the same 24, and it is called ONCE per run |
//!
//! The lock is an `RwLock`, so the read path — which is the one on every
//! knob lookup — does not serialise the rungs against each other. It is never
//! taken inside a loop over bars or candidates: every knob in this workspace is
//! read once per rung or once per run, which is the granularity gate 17
//! prescribes for anything that is not free.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::sync::{OnceLock, RwLock};

/// The knobs a caller has set for this process, empty until one does.
///
/// `OnceLock` rather than `LazyLock` for no reason beyond matching what the rest
/// of this workspace reaches for; the two are equivalent here.
fn store() -> &'static RwLock<HashMap<String, String>> {
    static STORE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Every knob whose raw value a reader could not use, and what it said.
///
/// # The `CLAUDE.md` §4 ban this closes, and it is named in the table
///
/// *"A fallback that hides a failure — degrade loudly and name the reason, or
/// refuse. Never both silently."* Every count knob in this crate read
/// `.and_then(parse).filter(|&n| n > 0).unwrap_or(DEFAULT)`, so `abc`, `0`,
/// `-1` and an integer past `usize` all produced the caller's fallback **with no
/// message anywhere**. An operator who typed `12` and meant it, and typo'd, got
/// a run at a setting they did not choose and a report that never mentioned it.
///
/// It is not hypothetical: `BRUTEX_SCREEN_CAP` and `BRUTEX_GRID_RUNGS` are both
/// reachable from an unauthenticated `POST` through the browser's knobs table,
/// where a field is a free-text box.
///
/// # Recorded rather than refused, and the choice is not laziness
///
/// `ceiling_asked` REFUSES, and it is right to: the ceiling decides whether the
/// ladder walks at all. These do not. `Rules::at`'s own doc states the reasoning
/// for its half — *"halting a sweep over a typo in an optional variable would be
/// a worse failure than using the documented figure"* — and a seven-hour sweep
/// thrown away over a stray character is a real cost, not a rhetorical one.
///
/// So the fallback stays and the SILENCE goes. `refused` renders what was
/// unusable, `cli::audit_bars` prints it beneath the provenance banner, and the
/// run says on its own face which of its settings did not take.
///
/// A `BTreeMap` so the block is ordered and two identical runs render the same
/// string, which is §3 rule 5 reaching the page. First value per name wins: a
/// knob read five times is one defect, not five lines.
fn refusals() -> &'static RwLock<BTreeMap<String, String>> {
    static REFUSED: OnceLock<RwLock<BTreeMap<String, String>>> = OnceLock::new();
    REFUSED.get_or_init(|| RwLock::new(BTreeMap::new()))
}

/// Record that `name`'s raw value could not be used.
///
/// A poisoned lock drops the record rather than panicking, for the reason `var`
/// gives about its own: the alternative is turning a diagnostic into a crash.
fn refuse(name: &str, raw: &str) {
    if let Ok(mut held) = refusals().write() {
        held.entry(name.to_owned())
            .or_insert_with(|| raw.to_owned());
    }
}

/// A knob read as a COUNT — one or more — with an unusable value recorded.
///
/// `None` means "no usable value", which every caller already turns into its own
/// documented default. What changes is that the reason is now retrievable
/// through [`refused`] instead of being discarded at the `filter`.
///
/// `u64` because this is also the reader for counts whose domain really is
/// `u64`, such as a millisecond budget. Machine-sized counts go through
/// [`count_usize`] so a value that parses as `u64` but cannot fit this target is
/// recorded rather than disappearing at a later conversion.
#[must_use]
pub(crate) fn count(name: &str) -> Option<u64> {
    let raw = var(name)?;
    match raw.trim().parse::<u64>() {
        Ok(n) if n > 0 => Some(n),
        // ZERO IS THE CASE WORTH NAMING. A rung count of zero prices only the
        // no-exit baseline and a screen cap of zero considers nothing at all, so
        // both are settings that would silently answer a different question.
        _ => {
            refuse(name, &raw);
            None
        }
    }
}

/// A positive, machine-sized count, with representation failures recorded.
///
/// Parsing in the target type matters on a 32-bit target: `u64::MAX` is a valid
/// [`count`] but cannot be a `Vec` bound or an iterator count there. Letting each
/// caller append `.and_then(usize::try_from(..).ok())` would reintroduce the
/// exact silent fallback this module exists to expose, one step after parsing.
#[must_use]
pub(crate) fn count_usize(name: &str) -> Option<usize> {
    let raw = var(name)?;
    match raw.trim().parse::<usize>() {
        Ok(n) if n > 0 => Some(n),
        _ => {
            refuse(name, &raw);
            None
        }
    }
}

/// Record an unusable value for a knob this module cannot parse for the caller.
///
/// `Rules::stated` admits ZERO — a floor of zero drops its rule, which is a
/// real choice an operator makes — so it cannot go through [`count`], and it
/// keeps its own `i64` parse. This is the half it was missing.
pub(crate) fn refuse_value(name: &str, raw: &str) {
    refuse(name, raw);
}

/// What this run could not use, as a block to print, or `None` when every knob
/// was fine.
///
/// Shouted, and named per knob, for the same reason [`crate::UNVALIDATED`] is: a
/// run at a fallback and a run at the operator's own figure are byte-identical
/// in shape, so the only thing that can separate them is a line saying which
/// happened.
#[must_use]
pub fn refused() -> Option<String> {
    let held = refusals().read().ok()?;
    if held.is_empty() {
        return None;
    }
    let mut out = String::from(
        "!! KNOB REFUSED -- a value below could not be used and the run took that \
         knob's documented fallback instead.\n",
    );
    for (name, raw) in held.iter() {
        // THE RULE IS NOT SPELLED OUT HERE, and that is deliberate: `count`
        // wants one or more while `Rules::stated` admits zero and refuses a
        // negative, so a single sentence claiming one rule would be false about
        // the other. What every reader needs and this can state truthfully is
        // WHICH knob and WHAT it said -- the two facts a person cannot recover
        // from a report that took a fallback.
        //
        // `writeln!` and not `push_str(&format!(..))`: the second allocates a
        // whole String per row only to copy and drop it, which is what
        // `clippy::format_push_string` refuses. A write into a `String` cannot
        // fail, so the `Result` is discarded the way every other `writeln!` in
        // this workspace discards it.
        let _ = writeln!(
            out,
            "   {name}={raw:?} -- this run could not use that value and took the \
             knob's documented fallback. Correct it and rerun, or unset it to \
             choose the fallback deliberately."
        );
    }
    Some(out)
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
///
/// THE REFUSALS GO WITH THEM, and they must: a refusal names a value THIS
/// request supplied, so carrying it into the next one would print a stranger's
/// typo on a clean run's report — which is the same class of leak
/// `sweeprun::Applied::drop` exists to stop for the settings themselves.
pub fn clear_all() {
    if let Ok(mut held) = store().write() {
        held.clear();
    }
    if let Ok(mut held) = refusals().write() {
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
    use super::{
        clear_all, count, count_usize, describe, refuse_value, refused, render, resolve, set,
        set_here, var,
    };

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

    /// `clear_all` FORGETS, and clearing twice forgets no more than once.
    ///
    /// # This test asserted nothing, and `CLAUDE.md` §4 bans that by name
    ///
    /// It was three statements and no `assert`: `serially()`, `clear_all()`,
    /// `clear_all()`. It passed with `clear_all` replaced by an empty body, so
    /// it could not kill a single mutant — the table's own words are *"a
    /// surviving mutant is a missing test and blocks the build"*, and this was
    /// the missing test wearing a present one's clothes.
    ///
    /// The property it was reaching for is real and is now stated: the first
    /// call must actually remove what was set, the second must be a no-op rather
    /// than an error, and the store must read empty after both. It runs after
    /// EVERY run including one that set nothing, so idempotence is the contract.
    #[test]
    fn clearing_forgets_what_was_set_and_clearing_twice_is_idempotent() {
        let _serial = serially();
        clear_all();
        let name = "BRUTEX_KNOBS_TEST_CLEAR_TWICE";
        set(name, "500");
        assert_eq!(var(name).as_deref(), Some("500"), "it was set");

        clear_all();
        assert_eq!(var(name), None, "and the first clear removed it");

        // THE SECOND CLEAR IS THE HALF THE OLD TEST WAS ABOUT, and it is
        // asserted rather than merely performed.
        clear_all();
        assert_eq!(var(name), None, "the second clear leaves it removed");
        assert_eq!(
            describe(),
            String::new(),
            "and nothing else survived either -- an empty store renders empty"
        );
    }

    /// AN UNUSABLE VALUE IS RECORDED, NOT SWALLOWED.
    ///
    /// `count` replaces `.and_then(parse).filter(|&n| n > 0).unwrap_or(DEFAULT)`,
    /// which produced the compiled default for `abc`, `0`, `-1` and an integer
    /// past `u64` with no message anywhere — a fallback that hides a failure,
    /// which `CLAUDE.md` §4 bans. Both halves are asserted: the answer is still
    /// `None` so every caller's documented default is unchanged, and the reason
    /// is now retrievable.
    #[test]
    fn an_unusable_count_is_named_rather_than_silently_defaulted() {
        let _serial = serially();
        let name = "BRUTEX_KNOBS_TEST_COUNT";

        for bad in ["abc", "0", "-1", "12.5", "99999999999999999999999999"] {
            clear_all();
            set(name, bad);
            assert_eq!(
                count(name),
                None,
                "`{bad}` is not a count, so the caller still takes its default"
            );
            let block = refused().expect("an unusable value must be recorded");
            assert!(
                block.contains(name) && block.contains(bad),
                "the block must name the knob AND what it said: {block}"
            );
            assert!(
                block.contains("KNOB REFUSED"),
                "and it must be shouted, like every other banner that separates \
                 two runs which render alike: {block}"
            );
        }

        // A USABLE VALUE RECORDS NOTHING. A block on a clean run would be the
        // opposite defect: an operator who learns to ignore the line.
        clear_all();
        set(name, " 12 ");
        assert_eq!(count(name), Some(12), "trimmed and taken");
        assert_eq!(refused(), None, "a good value is not a refusal");

        // AND AN UNSET KNOB IS NOT A REFUSAL EITHER. Nobody stated anything, so
        // there is nothing to name.
        clear_all();
        assert_eq!(count(name), None, "nothing set, nothing read");
        assert_eq!(refused(), None, "and nothing to report");
    }

    /// A machine-sized reader owns the narrowing as well as the parse.
    ///
    /// The exact largest accepted value is target-dependent; the externally
    /// visible contract is not. A value outside it returns no count and leaves
    /// the raw setting available for the report.
    #[test]
    fn a_count_too_large_for_the_machine_is_named_too() {
        let _serial = serially();
        clear_all();
        let name = "BRUTEX_KNOBS_TEST_USIZE_COUNT";
        let too_large = "999999999999999999999999999999999999999";
        set(name, too_large);

        assert_eq!(count_usize(name), None, "the count cannot be represented");
        let block = refused().expect("the failed count must be reportable");
        assert!(
            block.contains(name) && block.contains(too_large),
            "the report must preserve both facts: {block}"
        );

        clear_all();
        set(name, "42");
        assert_eq!(
            count_usize(name),
            Some(42),
            "a representable count is taken"
        );
        assert_eq!(refused(), None, "and does not leave a warning behind");
        clear_all();
    }

    /// `refuse_value` is the door for a reader whose own rule `count` cannot
    /// express, and `clear_all` forgets through it too.
    ///
    /// `Rules::stated` admits ZERO — a floor of zero drops its rule, which is a
    /// choice an operator makes — so it keeps its own `i64` parse and reports
    /// through here instead.
    #[test]
    fn a_readers_own_refusal_is_recorded_and_cleared_with_the_rest() {
        let _serial = serially();
        clear_all();
        refuse_value("BRUTEX_KNOBS_TEST_OWN", "50%");
        let block = refused().expect("a recorded refusal renders");
        assert!(
            block.contains("BRUTEX_KNOBS_TEST_OWN") && block.contains("50%"),
            "{block}"
        );

        // FIRST VALUE PER NAME WINS. A knob read five times in one run is one
        // defect, and five identical lines would bury the other knobs.
        refuse_value("BRUTEX_KNOBS_TEST_OWN", "later");
        let again = refused().expect("still recorded");
        assert!(
            !again.contains("later"),
            "a second reading of the same knob must not add a line: {again}"
        );

        clear_all();
        assert_eq!(
            refused(),
            None,
            "a refusal names a value THIS request supplied, so it must not \
             outlive the request"
        );
    }
}

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
//! `CLAUDE.md` §3 rule 4 asks that each operation be O(1). The table says which
//! are, and the environment read is the one that is not:
//!
//! | operation | cost | why |
//! |---|---|---|
//! | `var` (set) | one expected-O(1) hash probe, plus a copy of the value | the store answers, so the environment is not read |
//! | `var` (unset) | not claimed O(1) | one hash probe, then `std::env::var_os`: a lookup over the process environment, whose size this workspace neither bounds nor measures |
//! | `set` | **O(1)** | one hash insert |
//! | `count` | the `var` row it takes | one `var`, one parse, and at most one tree insert |
//! | `clear_all` | O(set knobs) | and that count is bounded by 24 |
//! | `refused` | O(refused knobs) | bounded by the same 24, and it is called ONCE per run |
//!
//! The lock is an `RwLock`, so the read path — which is the one on every
//! knob lookup — does not serialise the rungs against each other. It is never
//! taken inside a loop over bars: a knob is read per rung or per run, which is
//! the granularity gate 17 prescribes for anything that is not free, and in one
//! place more often than that: `Rules::operator` reads ten knobs per call and the
//! tier ladder asks for it per tier, 388,143 reads in one generated 12-session
//! audit at a 50,000 ceiling, every one on the run's own thread (counted by a
//! throwaway probe, BASE-REDS-1023 repair 6; this said "once per rung or once per
//! run" until then; `docs/06-limits.md` L-BRED23-21).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

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
/// [`count_usize_within`] so a value that parses as `u64` but cannot fit this target is
/// recorded rather than disappearing at a later conversion.
#[must_use]
pub(crate) fn count(name: &str) -> Option<u64> {
    let raw = var(name)?;
    if let Some(n) = positive_count(&raw) {
        Some(n)
    } else {
        // ZERO IS THE CASE WORTH NAMING. A rung count of zero prices only the
        // no-exit baseline and a screen cap of zero considers nothing at all, so
        // both are settings that would silently answer a different question.
        refuse(name, &raw);
        None
    }
}

/// A positive, machine-sized count, with representation failures recorded,
/// refusing anything above `ceiling`.
///
/// Parsing in the target type matters on a 32-bit target: `u64::MAX` is a valid
/// [`count`] but cannot be a `Vec` bound or an iterator count there. Letting each
/// caller append `.and_then(usize::try_from(..).ok())` would reintroduce the
/// exact silent fallback this module exists to expose, one step after parsing.
///
/// **An unbounded count is a way to kill the process from a text box.** Three
/// knobs reach a `with_capacity` before a single bar is scored -- the priced
/// cap, the candidate ceiling and the forward horizon -- and the browser wires
/// all three to free-text fields with no numeric validation at the HTTP layer.
/// A `Marked<Scored>` is ~136 bytes, so `1_000_000_000` asks the allocator for
/// 136 GB, and an allocation failure calls `abort()`: not a panic, not
/// catchable, no refusal, no log line. The server simply disappears.
///
/// `grid_rungs` was given a ceiling after a documented 935 GB abort. Its three
/// siblings were not. This is that guard, shared.
pub(crate) fn count_usize_within(name: &str, ceiling: usize) -> Option<usize> {
    let raw = var(name)?;
    if let Some(n) = machine_count(&raw, ceiling) {
        Some(n)
    } else {
        refuse(name, &raw);
        None
    }
}

/// Pure scalar readers shared by ordinary resolution and strict admission.
pub(crate) fn positive_count(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok().filter(|value| *value > 0)
}

pub(crate) fn machine_count(raw: &str, ceiling: usize) -> Option<usize> {
    raw.trim()
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0 && *value <= ceiling)
}

pub(crate) fn nonnegative_floor(raw: &str) -> Option<i64> {
    raw.trim().parse::<i64>().ok().filter(|value| *value >= 0)
}

/// One policy knob's value, by the ONE rule every reader of it applies.
///
/// # Three readers, two rules (P8-04, D-2723)
///
/// `Rules::stated` (screen, elite, ranges, pool, ledgers, the browser sweep)
/// and `strict_range_knobs` admitted any `i64 >= 0` through
/// [`nonnegative_floor`], while `expression_pricing::validate_overrides`
/// refused a win rate above 10,000 bp and any `BRUTEX_PROTECTED_EXITS` other
/// than `0` or `1`. So `BRUTEX_MIN_WIN_RATE_BP=20000` -- a 200% win rate --
/// ran silently on one path and was refused on the other, and
/// `BRUTEX_PROTECTED_EXITS=2` was "on" on one and refused on the other. Every
/// reader now asks this function; what each does with a `None` (a named
/// fallback, or a refusal) is still that reader's own documented choice.
///
/// A win rate is a fraction of trades and cannot exceed 10,000 bp; the
/// protective-exit switch is a boolean and has exactly two spellings.
pub(crate) fn policy_floor(name: &str, raw: &str) -> Option<i64> {
    let value = nonnegative_floor(raw)?;
    match name {
        "BRUTEX_MIN_WIN_RATE_BP" => (value <= 10_000).then_some(value),
        "BRUTEX_PROTECTED_EXITS" => matches!(raw.trim(), "0" | "1").then_some(value),
        _ => Some(value),
    }
}

pub(crate) fn horizon_count(raw: &str) -> Option<runner::outcome::Horizon> {
    raw.trim()
        .parse::<u32>()
        .ok()
        .and_then(runner::outcome::Horizon::bars)
}

pub(crate) fn sizing_rate(raw: &str) -> Option<i64> {
    raw.trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 5_000 && *value < 10_000)
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
    #[cfg(test)]
    if !scope::this_thread_sees_the_store() {
        return None;
    }
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
    resolve(set_here(name), || {
        std::env::var_os(name).map(|raw| raw.to_string_lossy().into_owned())
    })
}

/// What this process has been asked to use for `name`, if anything.
fn set_here(name: &str) -> Option<String> {
    #[cfg(test)]
    if !scope::this_thread_sees_the_store() {
        return None;
    }
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
///
/// The environment is a closure, called only when nothing is set: a set knob
/// never reads it, so the set path's cost does not include a lookup over the
/// process environment. W2-cli7-4, D-0931.
fn resolve(here: Option<String>, environment: impl FnOnce() -> Option<String>) -> Option<String> {
    here.or_else(environment)
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
    #[cfg(test)]
    scope::assert_a_tests_thread_holds_the_guard();
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
/// A test that only reads needs nothing: see [`scope`].
#[cfg(test)]
pub(crate) fn serially() -> scope::Serially {
    scope::Serially::take()
}

/// What a test's thread may see of this process-wide store.
///
/// # The race this closes, and why the readers needed nothing done to them
///
/// `serially` serialised the tests that MOVE a knob, and no test that only READS
/// one took it: a count of the crate's lib tests by a throwaway probe in this
/// module (BASE-REDS-1023 repair 6, 1 Nov 2026) found 68 tests reading knobs with
/// no guard, 696,136 reads in one run. So a knob another test set could move
/// between two reads of one run. `the_audit_renders_every_stage_of_the_institutional_stack` ended `RESULT
/// NOT RECORDED: the final admitted long candidate could not reproduce its
/// selected exit cell` in the full suite and passed alone, because
/// `BRUTEX_MAX_STOP_POINTS` moved between its screen and its exact rebuild: a
/// thread moving that knob every 3 ms ended 4 of 12 audits that way, no knob
/// moving 0 of 4 (check r6).
///
/// A guard in each of the 68 would have held only for the readers someone
/// remembered, and made the next one a race again. So the cause is closed where
/// it is: **a test's own thread that holds no guard sees no knob another test
/// set**. It reads the store as the process starts it, empty, and blocks on
/// nothing. Only test builds compile this.
///
/// The same count found no read on any other thread, in guarded and unguarded
/// tests alike, and no move by a thread that held no guard. Should a read move
/// to a worker: a thread that is not a test's own cannot be told apart by name,
/// so it sees the store only while some test holds the guard, which is what a
/// holder's own worker needs. A worker of a test with no guard reads nothing
/// then, and could read a holder's knob while another test holds the guard: that
/// one is `docs/06-limits.md` L-BRED23-20, and no such read exists today.
#[cfg(test)]
pub(crate) mod scope {
    use std::cell::Cell;
    use std::sync::{Mutex, MutexGuard, PoisonError, TryLockError};

    /// The one lock of [`Serially`].
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    thread_local! {
        /// Whether this thread holds [`ONE_AT_A_TIME`], through [`Serially`].
        static HOLDS_THE_GUARD: Cell<bool> = const { Cell::new(false) };
    }

    /// The guard of [`super::serially`]: its thread sees the store while it lives.
    #[must_use = "the guard must be held for the test, not dropped immediately"]
    pub(crate) struct Serially {
        /// Released after [`Drop::drop`] says this thread no longer holds it.
        _held: MutexGuard<'static, ()>,
    }

    impl Serially {
        /// Waits for the lock. A test that panicked holding it leaves it free.
        pub(super) fn take() -> Self {
            let held = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
            HOLDS_THE_GUARD.with(|holds| holds.set(true));
            Self { _held: held }
        }
    }

    impl Drop for Serially {
        fn drop(&mut self) {
            HOLDS_THE_GUARD.with(|holds| holds.set(false));
        }
    }

    /// The test harness names each test's thread by the test's path; the threads
    /// this crate names (`brutex-ordered-3`, `index-stop-boundaries`) and the
    /// unnamed workers carry no `::`, and `main` runs no test here.
    pub(super) fn is_a_tests_thread(name: Option<&str>) -> bool {
        name.is_some_and(|name| name.contains("::"))
    }

    /// The rule, over its three facts, so each side can be driven whatever else
    /// the suite is doing. The lock is asked about last: no test thread asks it.
    pub(super) fn sees_the_store(
        holds_the_guard: bool,
        a_tests_thread: bool,
        guard_is_held: impl FnOnce() -> bool,
    ) -> bool {
        holds_the_guard || (!a_tests_thread && guard_is_held())
    }

    /// Whether some thread holds the lock right now. A lock poisoned by a test
    /// that panicked holding it is free: nobody holds it.
    pub(super) fn guard_is_held() -> bool {
        matches!(ONE_AT_A_TIME.try_lock(), Err(TryLockError::WouldBlock))
    }

    /// A test's own thread moves a knob only under the guard: `clear_all` is not
    /// scoped to a name, so one with no guard would take a holder's knobs away in
    /// the middle of its run. Refused by name, every time, whatever else runs.
    pub(super) fn assert_a_tests_thread_holds_the_guard() {
        let thread = std::thread::current();
        assert!(
            HOLDS_THE_GUARD.with(Cell::get) || !is_a_tests_thread(thread.name()),
            "{} moved a knob with no knobs::serially()",
            thread.name().unwrap_or_default()
        );
    }

    /// [`sees_the_store`] for the calling thread.
    pub(super) fn this_thread_sees_the_store() -> bool {
        let thread = std::thread::current();
        sees_the_store(
            HOLDS_THE_GUARD.with(Cell::get),
            is_a_tests_thread(thread.name()),
            guard_is_held,
        )
    }
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
    #[cfg(test)]
    scope::assert_a_tests_thread_holds_the_guard();
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
    #[cfg(test)]
    if !scope::this_thread_sees_the_store() {
        return String::new();
    }
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
        clear_all, count, count_usize_within, describe, refuse_value, refused, render, resolve,
        scope, set, set_here, var,
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
            resolve(None, || Some("from-the-environment".to_owned())).as_deref(),
            Some("from-the-environment")
        );
        assert_eq!(resolve(None, || None), None, "and nothing set is no answer");
    }

    /// The caller is the more specific statement, so it wins.
    #[test]
    fn a_set_knob_beats_the_environment() {
        assert_eq!(
            resolve(Some("caller".to_owned()), || Some("environment".to_owned())).as_deref(),
            Some("caller"),
            "the request knows which question is being asked"
        );
        assert_eq!(
            resolve(Some("caller".to_owned()), || None).as_deref(),
            Some("caller"),
            "and it does not need the environment to have an opinion"
        );
    }

    /// A set knob does not read the environment at all, which is what lets
    /// the cost table's set row leave the environment lookup out.
    ///
    /// Before this row `var` evaluated `std::env::var_os(name)` as an argument,
    /// so the lookup ran on every read even when the store already answered.
    /// W2-cli7-4, D-0931.
    #[test]
    fn a_set_knob_never_reads_the_environment() {
        let reads = std::cell::Cell::new(0_u32);
        let environment = || {
            reads.set(reads.get() + 1);
            Some("environment".to_owned())
        };
        assert_eq!(
            resolve(Some("caller".to_owned()), environment).as_deref(),
            Some("caller")
        );
        assert_eq!(
            reads.get(),
            0,
            "the store answered, so nothing looked further"
        );
        assert_eq!(resolve(None, environment).as_deref(), Some("environment"));
        assert_eq!(reads.get(), 1, "an unset knob reads the environment once");
    }

    /// The module's cost table does not call an environment read O(1).
    ///
    /// `var` reads `std::env::var_os`, a lookup over the process environment
    /// whose size this workspace neither bounds nor measures. The table said
    /// `var` (unset) was "**O(1)** | one hash probe, then one `getenv`" and
    /// `var` (set) "**O(1)** | one hash probe" while the set path read the
    /// environment too. W2-cli7-4, D-0931. Proved here:
    /// `cli::knobs::the_cost_table_does_not_call_an_environment_read_constant`.
    #[test]
    fn the_cost_table_does_not_call_an_environment_read_constant() {
        let source = include_str!("knobs.rs");
        let row = |label: &str| {
            source
                .lines()
                .find(|line| line.starts_with(&format!("//! | {label} |")))
                .unwrap_or_else(|| panic!("the cost table has a {label} row"))
        };
        let unset = row("`var` (unset)");
        assert!(
            unset.contains("not claimed O(1)") && unset.contains("environment"),
            "an environment read is not claimed constant: {unset}"
        );
        let set_row = row("`var` (set)");
        assert!(
            set_row.contains("environment is not read"),
            "the set path must say it does not read the environment: {set_row}"
        );
        let count_row = row("`count`");
        assert!(
            !count_row.contains("**O(1)**"),
            "count reads through `var`, so it inherits the unset row: {count_row}"
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

        assert_eq!(
            count_usize_within(name, usize::MAX),
            None,
            "the count cannot be represented"
        );
        let block = refused().expect("the failed count must be reportable");
        assert!(
            block.contains(name) && block.contains(too_large),
            "the report must preserve both facts: {block}"
        );

        clear_all();
        set(name, "42");
        assert_eq!(
            count_usize_within(name, usize::MAX),
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

    /// CHECK R6'S RACE, BY ITS CAUSE, WITH NO CLOCK: while another test holds the
    /// guard with a knob and a refusal set, a test that holds no guard reads
    /// neither, and that other test reads both. The two threads hand over by
    /// channels, so the read happens with the knob set on every run.
    #[test]
    fn another_tests_knob_never_reaches_a_test_that_holds_no_guard() {
        let name = "BRUTEX_KNOBS_TEST_ANOTHER_TESTS";
        let (set_done, knob_is_set) = std::sync::mpsc::channel();
        let (read_done, reader_has_read) = std::sync::mpsc::channel::<()>();
        let mover = std::thread::Builder::new()
            .name("knobs::tests::a_test_that_moves_a_knob".to_owned())
            .spawn(move || {
                let _serial = serially();
                clear_all();
                set(name, "7");
                refuse_value(name, "50%");
                let saw = (var(name), describe(), refused());
                set_done.send(()).expect("the reader waits");
                reader_has_read.recv().expect("the reader answers");
                clear_all();
                saw
            })
            .expect("a thread");
        knob_is_set.recv().expect("the mover sets it");
        assert_eq!(var(name), None, "another test's knob");
        assert_eq!(
            describe(),
            "",
            "another test's knobs, as an audit event says them"
        );
        assert_eq!(refused(), None, "another test's refusal");
        assert_eq!(count(name), None, "and through a counting reader");
        read_done.send(()).expect("the mover waits");
        let (value, said, refusal) = mover.join().expect("the mover ends");
        assert_eq!(value.as_deref(), Some("7"), "the holder reads its own");
        assert!(said.contains("BRUTEX_KNOBS_TEST_ANOTHER_TESTS=7"), "{said}");
        assert!(refusal.is_some_and(|block| block.contains("50%")));
    }

    /// A holder's own worker reads what its test set: a thread that is no test's
    /// own sees the store while the guard is held.
    #[test]
    fn a_worker_of_the_test_that_holds_the_guard_reads_what_it_set() {
        let name = "BRUTEX_KNOBS_TEST_WORKER";
        let _serial = serially();
        clear_all();
        set(name, "8");
        let on_a_worker = std::thread::spawn(move || (var(name), describe()))
            .join()
            .expect("the worker ends");
        assert_eq!(on_a_worker.0.as_deref(), Some("8"));
        assert!(
            on_a_worker.1.contains("BRUTEX_KNOBS_TEST_WORKER=8"),
            "{}",
            on_a_worker.1
        );
        clear_all();
    }

    /// When the guard ends, what its thread sees ends with it, whatever is still
    /// in the store. The next holder starts from `clear_all`, and this one clears.
    #[test]
    fn the_end_of_the_guard_ends_what_its_thread_sees() {
        let name = "BRUTEX_KNOBS_TEST_GUARD_ENDS";
        {
            let _serial = serially();
            clear_all();
            set(name, "9");
            assert_eq!(var(name).as_deref(), Some("9"));
        }
        assert_eq!(var(name), None, "no guard, no knob");
        let _serial = serially();
        clear_all();
    }

    /// A test's own thread is told by the path in its name; the harness names it
    /// so, and this test's own thread is the proof. No thread this crate names
    /// reads as one.
    #[test]
    fn a_tests_thread_is_told_by_the_path_in_its_name() {
        assert!(scope::is_a_tests_thread(std::thread::current().name()));
        assert!(scope::is_a_tests_thread(Some("knobs::tests::a_test")));
        for other in [
            Some("brutex-ordered-3"),
            Some("index-stop-boundaries"),
            Some("main"),
            None,
        ] {
            assert!(!scope::is_a_tests_thread(other), "{other:?}");
        }
    }

    /// The rule on each of its sides, the lock asked only for a thread that is no
    /// test's own and holds no guard.
    #[test]
    fn each_side_of_what_a_thread_sees() {
        let asked = std::cell::Cell::new(0);
        let held = || {
            asked.set(asked.get() + 1);
            true
        };
        let free = || {
            asked.set(asked.get() + 1);
            false
        };
        assert!(scope::sees_the_store(true, true, &held), "the holder");
        assert!(
            scope::sees_the_store(true, false, &free),
            "a holder by any name"
        );
        assert!(
            !scope::sees_the_store(false, true, &held),
            "a test with no guard"
        );
        assert_eq!(asked.get(), 0, "none of them asks the lock");
        assert!(
            scope::sees_the_store(false, false, &held),
            "a worker, the guard held"
        );
        assert!(
            !scope::sees_the_store(false, false, &free),
            "a worker, nobody holds it"
        );
        assert_eq!(asked.get(), 2);
    }

    /// The lock says it is held while this test holds it.
    #[test]
    fn the_guard_is_held_while_a_test_holds_it() {
        let _serial = serially();
        assert!(scope::guard_is_held());
    }

    /// A test's own thread with no guard moves nothing: its `set` is refused by
    /// name, on every run, so a new test that forgets the guard fails at once.
    #[test]
    #[should_panic(expected = "moved a knob with no knobs::serially()")]
    fn a_tests_thread_with_no_guard_may_not_set_a_knob() {
        set("BRUTEX_KNOBS_TEST_NO_GUARD", "1");
    }

    /// And its `clear_all`: it is not scoped to a name, so it would take a holder's
    /// knobs away.
    #[test]
    #[should_panic(expected = "moved a knob with no knobs::serially()")]
    fn a_tests_thread_with_no_guard_may_not_clear_the_knobs() {
        clear_all();
    }
}

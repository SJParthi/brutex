//! **Every instrument-month in the store, swept in one command.**
//!
//! # The gap this closes
//!
//! `sweep-stored` takes one vendor, one underlying, one rung, one year and one
//! month, and sweeps exactly that. It had no alternative: until
//! `store::catalog` landed, **nothing in this workspace could list what the
//! store held** — `crates/store` contained zero `read_dir` calls, and the only
//! directory walks lived in `crates/pull` and `crates/api`, neither of which
//! `cli` may take.
//!
//! `docs/07-plan.md` §4 targets ~800 instruments over ~68 months. One
//! invocation each is 54,000 invocations. This is the loop.
//!
//! # Why the loop is here and not behind an HTTP route
//!
//! A sweep of the whole store is minutes to hours of CPU. That belongs in
//! something an operator starts and leaves running, with output they can
//! redirect — not in a browser tab that a reload abandons. The web surface
//! should call the same code when it exists; it should not be where the code
//! first appears.
//!
//! # Gate 17 and progress reporting
//!
//! CI gate 17 forbids **any** `telemetry::` reference in the crates that hold
//! the mask, the vocabulary and the sweep, and its reason is arithmetic: the
//! ladder evaluates `(bits & mask) == mask` billions of times, so *"each call is
//! cheap"* is not the bar — *"the innermost loop calls nothing at all"* is.
//!
//! This module honours that by reporting **between** instrument-months, never
//! inside one. A caller watching a 54,000-month run sees a line per month, which
//! is the granularity that is both useful and free. There is no progress bar
//! from inside the ladder and there must not be one.
//!
//! # Cost
//!
//! O(instrument-months) walks, plus the sweep of each. The enumeration itself is
//! O(entries) and `docs/06-limits.md` §86 declares it. Neither is a rule-4
//! operation: those five run per bar or per candidate.
//!
//! **The months run in PARALLEL, and the totals do not.** Each instrument-month
//! is wholly independent — its own file, its own evaluator, its own ladder, its
//! own identity — so the walk is `par_iter`, and wall-clock divides by the core
//! count rather than the work being done one core at a time as it was.
//!
//! The parallelism is over MONTHS and not over candidates, and that is forced
//! rather than chosen: gate 22 pins `vocab indicators engine` to `vocab` alone,
//! so `crates/engine`, which holds the candidate loop, cannot take a `rayon`
//! arrow at all. `docs/01-architecture.md` claimed "data parallel over
//! candidates via rayon" while **no crate in the workspace took that arrow** —
//! the sweep was entirely single-threaded. This is the axis `cli` owns.
//!
//! §3 rule 5 survives by shape, and it takes THREE properties, not two. Indexed
//! `collect` preserves order; [`Tally`] is folded sequentially over the collected
//! rows rather than mutated from the workers; and each month is given an
//! explicit [`BATCH_CEILING`] so its halt point is a stated constant rather than
//! the machine free memory. The third was missing at first and it was the one
//! that mattered: `engine::Ladder` halts on a real `try_reserve` probe, so N
//! concurrent ladders would each halt at a point decided by what the others held
//! — making depth, kept and completed scheduling-dependent. D-0234.

use crate::stored;
use core::fmt::Write as _;
use engine::Ladder;
use rayon::prelude::*;
use runner::Sweeper;
use store::catalog::{self, Held};

/// Candidates one instrument-month may hold before its ladder halts.
///
/// # Why the whole-store sweep needs its own ceiling
///
/// `engine::DEFAULT_CEILING` is `1 << 26` and its own doc prices that at
/// **8 GiB** — more than an ordinary machine has. So on a single sweep the
/// constant never binds and the real bound is `Ladder::cannot_grow`, a genuine
/// `try_reserve` probe. `engine::Breach::Memory`'s doc states that as the
/// design: the sweep *"uses what a 4 GB machine has and what a 48 GB machine
/// has, discovers which at runtime, and asks nobody"*.
///
/// That is a good answer for **one** sweep and the wrong one for N at once.
/// With [`sweep_under`] running months in parallel, an allocator-defined halt
/// makes each month's `depth`, `kept` and `completed` depend on what the other
/// workers held at that instant — and those three fields reach the report. Two
/// runs over one store would print different bytes, which `CLAUDE.md` §3 rule 5
/// forbids.
///
/// # 2^20, and it is a stated assumption
///
/// One million candidates. At the 128 bytes per retained candidate
/// `engine::DEFAULT_CEILING`'s own arithmetic uses, that is **128 MiB per
/// month** — so sixteen workers sit near 2 GiB, which an ordinary machine has
/// with room to spare, and the number is the same on every machine.
///
/// It is smaller than a single-month sweep would be allowed, and that is the
/// trade being made rather than hidden: a 54,000-month walk cannot give every
/// month 8 GiB, so *some* per-month budget is required by the batch path
/// whatever value it takes. Stating it beats discovering it. A month that
/// reaches it halts **loudly** with `Breach::Ceiling` and prints
/// `CEILING — depth not reached by extinction`, which is the §4 refusal shape;
/// it is never silently truncated.
///
/// No document names a right value, so under §3 rule 1 this is the operator's
/// choice with a default rather than a derivation. D-0234.
const BATCH_CEILING: usize = 1 << 20;

/// What one instrument-month produced, or why it produced nothing.
#[derive(Debug, Clone)]
struct Row {
    /// `groww NIFTY 1min 2026-08`, for the report.
    label: String,
    /// Bars actually swept, after warm-up and refusals.
    bars: u64,
    /// Levels the ladder reached. `0` when it went extinct at k=1.
    depth: usize,
    /// Combinations meeting the threshold across every level.
    kept: usize,
    /// `false` when a level breached the candidate ceiling.
    completed: bool,
    /// This month's run identity, as hex.
    ///
    /// # Why a per-ROW identity and not one for the batch
    ///
    /// `CLAUDE.md` §3 rule 3 identifies a run by a hash whose terms include the
    /// instrument and the `data_digest` of the bars swept. A whole-store sweep
    /// touches many instruments and many months, so there is no single value
    /// those terms can take — one identity for the batch would have to invent
    /// an instrument, which is the fabrication §3 rule 1 forbids. Each
    /// instrument-month IS a run, and each carries its own.
    ///
    /// This field did not exist, and its absence was the report's own banner
    /// telling a lie: `crate::STORED_PROVENANCE` states that "the run identity
    /// beneath names the exact column they came from", and beneath it were
    /// counts and no identity at all. The single-month path has always printed
    /// one, so a reader who had seen that report reasonably assumed this one
    /// carried it too.
    ///
    /// `None` only for a refused month, which performed no computation and so
    /// has nothing to identify.
    identity: Option<String>,
    /// Present when the month could not be swept at all.
    refused: Option<String>,
}

/// Every outcome, so a short report cannot hide a long tail of refusals.
#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    /// Instrument-months the catalog offered after filtering.
    offered: u64,
    /// Months that produced a sweep.
    swept: u64,
    /// Months that could not be loaded or swept, each named in the report.
    refused: u64,
    /// Months whose ladder stopped on the candidate ceiling rather than
    /// extinction. **Counted separately because it is not a failure and not a
    /// clean answer** — §6 says depth is decided by extinction, and a ceiling
    /// breach means this month did not get that far.
    incomplete: u64,
    /// Bars swept across every month.
    bars: u64,
    /// Combinations kept across every month.
    kept: u64,
}

impl Tally {
    /// Every offered month is either swept or refused.
    const fn reconciles(&self) -> bool {
        self.swept.saturating_add(self.refused) == self.offered
    }

    /// Folds one finished month into the running totals.
    ///
    /// # Why this exists, and why it is the thing that makes the walk parallel
    ///
    /// The counters used to be incremented from inside [`one`], which took
    /// `&mut Tally`. That is exactly the shared mutable state a parallel walk
    /// cannot have — and, worse for this repository, it would have made the
    /// totals depend on the order threads happened to finish in, which
    /// `CLAUDE.md` §3 rule 5 forbids reaching the output.
    ///
    /// Every field is derivable from the [`Row`] alone: a row with a refusal is
    /// a refusal, any other row was swept, and its bars, kept and completed
    /// fields carry the rest. So `one` became pure, the sweep became parallel,
    /// and the fold stayed **sequential over the collected rows in their
    /// original order** — which is what keeps a rerun byte-identical.
    fn fold(&mut self, row: &Row) {
        if row.refused.is_some() {
            self.refused = self.refused.saturating_add(1);
            return;
        }
        self.swept = self.swept.saturating_add(1);
        self.bars = self.bars.saturating_add(row.bars);
        self.kept = self
            .kept
            .saturating_add(u64::try_from(row.kept).unwrap_or(u64::MAX));
        if !row.completed {
            self.incomplete = self.incomplete.saturating_add(1);
        }
    }
}

/// Sweeps every stored instrument-month matching `vendor_word` and `rung`.
///
/// Returns the rendered report. A month that refuses is recorded and the walk
/// continues: one unreadable file must not abandon 53,999 others, which is the
/// same reasoning `store::catalog` applies to an unreadable subdirectory.
///
/// The feed and the rung are both validated before anything is read — see
/// [`swept_rung`] for why an unvalidated rung was worse than an unvalidated
/// feed, and for why `1day` and `1s` are refused here while remaining perfectly
/// legal to load.
#[must_use]
pub fn sweep_all(vendor_word: &str, rung: &str, min_hits: u64) -> String {
    match run(vendor_word, rung, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// The rung word, refused unless the store carries it **and** the engine sweeps it.
///
/// # The refusal this replaces was a clean success
///
/// [`sweep_under`] filters the catalog on `h.timeframe.as_str() == rung` and
/// nothing validated the word first. So `cli sweep-all dhan 5sec 5` walked the
/// whole catalog, matched nothing, printed
/// `0 swept · 0 refused · 0 bars · 0 combinations kept` and exited **0** — the
/// exact bytes an empty store prints. A typo and a fresh clone were
/// indistinguishable, which is the failure wearing a success's clothes
/// `CLAUDE.md` §4 bans. `sweep-stored` has always refused the same word by name;
/// this command was the one that did not.
///
/// # Two questions, asked in this order because they have two different answers
///
/// [`stored::rung`] answers *"is this a rung at all"* against the store's own
/// `Timeframe::KNOWN`, so `5sec` is told the words that exist. [`crate::EVERY_RUNG`]
/// then answers *"is it one this engine SWEEPS"*, and that is EIGHT of those ten.
///
/// The order is load-bearing rather than stylistic: `EVERY_RUNG` is a strict
/// subset of `Timeframe::KNOWN`, so asking it first would make the store's own
/// refusal unreachable — a branch no input could take, which is the untestable
/// dead region this workspace's coverage floor exists to refuse.
///
/// # `1day` and `1s` are STORED and are not SWEPT
///
/// [`crate::EVERY_RUNG`]'s own doc gives the reason and it is `CLAUDE.md` §3
/// rule 7: a daily signal bar acts on a condition that was not knowable until
/// the session it describes had already closed. `1s` is absent for the other
/// reason — nothing has measured what a second-resolution span costs, and §3
/// rule 6 says an unmeasured bound is not a bound. Both have real directories
/// and both were swept here.
///
/// The guard is at this ENTRY POINT and deliberately not inside
/// [`stored::rung`], which is shared with the readers that must keep `1day`:
/// `stored::load_daily_context` opens it on every stored sweep. Narrowing the
/// shared parser would break a correct caller to fix an incorrect one.
fn swept_rung(rung: &str) -> Result<(), String> {
    // Is it a rung the store carries? The refusal names the ten that exist.
    stored::rung(rung)?;
    if !crate::EVERY_RUNG.contains(&rung) {
        return Err(format!(
            "`{rung}` is not a rung this engine sweeps. The eight are: {}. \
             It is stored and readable and it is not swept. Nothing was read.",
            crate::EVERY_RUNG.join(", ")
        ));
    }
    Ok(())
}

/// The fallible half, so the caller above has exactly one refusal shape.
fn run(vendor_word: &str, rung: &str, min_hits: u64) -> Result<String, String> {
    // BEFORE A SINGLE BAR IS READ. `CLAUDE.md` §3 rule 3 forbids a computation
    // whose identity cannot be recorded, and a 54,000-month run that discovers
    // that at the end has burned hours to produce nothing citable.
    let commit = crate::commit_stamp().ok_or_else(|| {
        "this build carries no verified commit stamp, so §3 rule 3's run identity \
         cannot be recorded and no sweep will run. Restore every Rust/Cargo input \
         to HEAD (normally by committing the intended change), then rebuild. An \
         explicit BRUTEX_COMMIT is accepted only when it exactly equals clean HEAD"
            .to_owned()
    })?;
    // Parsed here as well as in sweep_under so a bad feed name refuses BEFORE
    // the store is touched, which is what an operator typing a typo expects.
    crate::parse_vendor(vendor_word)?;
    // AND SO IS THE RUNG, for exactly the same reason and for a worse failure:
    // an unvalidated feed at least refused somewhere, while an unvalidated rung
    // matched nothing and reported that as a completed run. Checked in both
    // places on the vendor's own precedent — here so a typo never reaches the
    // store, and in `sweep_under` so no caller of the body can skip it.
    swept_rung(rung)?;
    let root = crate::store_root()?;
    sweep_under(&root, vendor_word, rung, min_hits, commit)
}

/// The body, with the store root supplied rather than read from the environment.
///
/// Split out so a test can drive a fixture store without touching `BRUTEX_STORE`
/// — two tests setting one process-wide variable is a race, and a test that
/// races is a test that will one day be deleted for flapping rather than fixed.
fn sweep_under(
    root: &std::path::Path,
    vendor_word: &str,
    rung: &str,
    min_hits: u64,
    commit: &str,
) -> Result<String, String> {
    let vendor = crate::parse_vendor(vendor_word)?;
    // BEFORE THE WALK, so an unknown or unswept rung costs no `read_dir` at all
    // and the report it cannot produce is never started.
    swept_rung(rung)?;
    let holdings = catalog::walk(root).map_err(|why| why.to_string())?;
    let wanted: Vec<&Held> = holdings
        .held
        .iter()
        .filter(|h| h.vendor == vendor && h.timeframe.as_str() == rung)
        .collect();

    let mut tally = Tally {
        offered: u64::try_from(wanted.len()).unwrap_or(u64::MAX),
        ..Tally::default()
    };
    // THE 54,000 SWEEPS, IN PARALLEL, AND STILL BYTE-IDENTICAL.
    //
    // Each instrument-month is wholly independent: its own file, its own
    // evaluator, its own ladder, its own run identity. Nothing is shared and
    // nothing is written. This was a `for` loop on one core while `rayon` sat in
    // the workspace manifest with no crate taking the arrow at all.
    //
    // WHY THE PARALLELISM IS HERE AND NOT OVER CANDIDATES, which is where the
    // architecture document claimed it was: gate 22 pins `vocab indicators
    // engine` to `vocab` alone, so `crates/engine` — which holds the candidate
    // loop — cannot take a `rayon` dependency. Parallelism over candidates is
    // not expressible without a law change. This axis is `cli`'s own and needs
    // none.
    //
    // DETERMINISM (§3 rule 5) IS HELD BY SHAPE, NOT BY LUCK. Two properties do
    // it, and both are needed:
    //   * `map(..).collect()` on an INDEXED parallel iterator preserves order,
    //     so `rows` is the same sequence whatever order the threads finish in;
    //   * the `Tally` is folded SEQUENTIALLY over `rows` below rather than
    //     mutated from the workers, so no counter depends on scheduling.
    // A rerun therefore produces the same bytes, which is what makes reruns safe.
    // SHARE CPU AS WELL AS MEMORY. `par_iter` can keep at most the Rayon pool's
    // workers active at once; declaring the full catalog length would divide a
    // fourteen-core machine by 54,000 even though only fourteen months can be
    // in flight. The engine's inner support lanes read this same bounded share,
    // preventing N outer sweeps from each spawning one worker per machine core.
    let concurrent = wanted.len().min(rayon::current_num_threads()).max(1);
    let _sharing = crate::SharedBy::these(concurrent);
    let rows: Vec<Row> = wanted
        .par_iter()
        .map(|held| one(root, held, min_hits, commit))
        .collect();
    for row in &rows {
        tally.fold(row);
    }

    Ok(render(
        vendor_word,
        rung,
        min_hits,
        commit,
        &holdings.census,
        &tally,
        &rows,
    ))
}

/// Sweeps one instrument-month. Pure: it reads the store and returns a row.
///
/// Takes no `&mut Tally`. That parameter was what stopped the walk above being
/// parallel, and removing it is what `Tally::fold` exists for.
#[expect(
    clippy::too_many_lines,
    reason = "one stored batch row keeps its signal, daily, exact-minute, identity, and refusal receipts together"
)]
fn one(root: &std::path::Path, held: &Held, min_hits: u64, commit: &str) -> Row {
    let label = format!(
        "{} {} {} {}",
        held.vendor.as_str(),
        held.symbol,
        held.timeframe.as_str(),
        held.month
    );
    let loaded = match stored::load(
        root,
        held.vendor,
        &held.symbol,
        held.timeframe.as_str(),
        held.month.year(),
        held.month.month(),
    ) {
        Ok(loaded) => loaded,
        Err(why) => {
            return Row {
                label,
                bars: 0,
                depth: 0,
                kept: 0,
                completed: false,
                identity: None,
                refused: Some(why),
            };
        }
    };
    let daily = match stored::load_daily_context(
        root,
        held.vendor,
        &held.symbol,
        (
            (held.month.year(), held.month.month()),
            (held.month.year(), held.month.month()),
        ),
        &loaded.bars,
    ) {
        Ok(daily) => daily,
        Err(why) => {
            return Row {
                label,
                bars: 0,
                depth: 0,
                kept: 0,
                completed: false,
                identity: None,
                refused: Some(why),
            };
        }
    };
    let exact_minute = match stored::load_exact_minute_context(
        root,
        held.vendor,
        &held.symbol,
        (
            (held.month.year(), held.month.month()),
            (held.month.year(), held.month.month()),
        ),
        &loaded.bars,
    ) {
        Ok(context) => context,
        Err(why) => {
            return Row {
                label,
                bars: 0,
                depth: 0,
                kept: 0,
                completed: false,
                identity: None,
                refused: Some(why),
            };
        }
    };
    let signal_length = match stored::rung_length_micros(held.timeframe.as_str()) {
        Ok(length) => length,
        Err(why) => {
            return Row {
                label,
                bars: 0,
                depth: 0,
                kept: 0,
                completed: false,
                identity: None,
                refused: Some(why),
            };
        }
    };
    let availability = crate::stored::vwap_availability(&loaded.key);
    let column = match crate::stored_anchored_column(
        &loaded.bars,
        &daily,
        &exact_minute,
        signal_length,
        availability,
    ) {
        Ok(column) => column,
        Err(why) => {
            return Row {
                label,
                bars: 0,
                depth: 0,
                kept: 0,
                completed: false,
                identity: None,
                refused: Some(why),
            };
        }
    };
    let digest = match crate::stored_anchored_digest(&loaded.bars, &exact_minute, &daily) {
        Ok(digest) => digest,
        Err(why) => {
            return Row {
                label,
                bars: 0,
                depth: 0,
                kept: 0,
                completed: false,
                identity: None,
                refused: Some(why),
            };
        }
    };
    // A STATED CEILING, BECAUSE THE DEFAULT ONE IS THE MACHINE'S FREE MEMORY.
    //
    // This was `Ladder::with_min_hits(min_hits)` alone, which leaves
    // `engine::DEFAULT_CEILING` in force — and that constant is `1 << 26`,
    // priced by its own doc at 8 GiB, more than an ordinary machine has. So the
    // bound that actually binds is not the constant: it is `cannot_grow`, a real
    // `try_reserve` probe. `Breach::Memory`'s doc says that is deliberate — the
    // sweep "uses what a 4 GB machine has and what a 48 GB machine has,
    // discovers which at runtime, and asks nobody".
    //
    // THAT IS SOUND FOR ONE SWEEP AND WRONG FOR N AT ONCE. Since the walk above
    // became parallel, each month's halt point is a function of what the other
    // N-1 threads happened to be holding at that instant — so `depth`, `kept`
    // and `completed`, read off the sweep below and pushed into the `Row` and
    // the `Tally`, become scheduling-dependent. Two runs over an identical store
    // on an identical binary could print different bytes. `CLAUDE.md` §3 rule 5
    // forbids exactly that, and D-0232 asserted the opposite.
    //
    // Giving each month an explicit ceiling makes the halt a STATED CONSTANT
    // rather than an ambient discovery: identical on every thread, every machine
    // and every run. The allocator probe stays as a backstop, but it now fires
    // only in genuine exhaustion — a crash-level event, not a normal outcome.
    // Capping the thread count would not have fixed this; it only changes N,
    // and the halt would still be allocator-defined.
    let ladder = Ladder::with_min_hits(min_hits)
        .with_ceiling(BATCH_CEILING)
        .with_support_lanes(crate::shared_support_lanes());
    let outcome = Sweeper::new(ladder).run_prepared_streamed(&column);

    // THE IDENTITY THIS REPORT'S BANNER HAS ALWAYS PROMISED.
    //
    // Built from the ladder that ACTUALLY RAN rather than from `min_hits` as
    // typed — `Params::of` reads the ladder, so a zero the ladder raised to one
    // is recorded as the one that ran. Same construction as `sweep_stored`, so
    // sweeping a month here and sweeping it alone produce the same 64 hex
    // characters, which is the only thing that makes the two reports comparable.
    let id = runner::identity::identity(&runner::identity::Run {
        // `Default::default()` and not the named path, for the reason
        // `crate::sweep_stored` gives at its own call site: spelling
        // `ConditionMask` needs a `vocab` arrow that `CLAUDE.md` §5 does not
        // draw for `cli`, and adding one to satisfy a lint would be the silent
        // scope change §3 rule 2 forbids.
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        direction: runner::identity::Direction::Undirected,
        instrument: &loaded.key,
        timeframe: loaded.timeframe,
        params: runner::identity::Params::of(ladder),
        data_digest: digest,
        commit,
        // The feed this month's file was actually read from. A whole-store
        // sweep walks several vendors in one run, so this is the term that keeps
        // two feeds' rows for one instrument-month from carrying one identity.
        feed: loaded.vendor.as_str(),
    });

    let kept = usize::try_from(
        outcome
            .sweep
            .levels
            .iter()
            .map(|level| level.survivors)
            .sum::<u64>(),
    )
    .unwrap_or(usize::MAX);
    let depth = outcome.sweep.depth();
    let completed = outcome.sweep.completed();

    // ONE EVENT PER INSTRUMENT-MONTH, which is this module's own stated
    // granularity: "reporting BETWEEN instrument-months, never inside one".
    // A 54,000-month walk is hours of CPU, and a line per month is the only
    // progress signal that is both useful and free — there is no progress bar
    // from inside the ladder and there must not be one.
    crate::note(
        &telemetry::Event::info("cli.sweep", "stored month swept")
            .with("identity", id.hex().as_str())
            .with("feed", loaded.vendor.as_str())
            .with("label", label.as_str())
            .with("bars", outcome.census.swept)
            .with("depth", u64::try_from(depth).unwrap_or(u64::MAX))
            .with("kept", u64::try_from(kept).unwrap_or(u64::MAX))
            .with("completed", completed),
    );

    // AND THE LEDGER, WHICH IS THE HALF THE EVENT ABOVE COULD NOT BE.
    //
    // The comment on `id` says this identity is built "same construction as
    // `sweep_stored`, so sweeping a month here and sweeping it alone produce the
    // same 64 hex characters, which is the only thing that makes the two
    // reports comparable." They were comparable in the REPORT and nowhere else:
    // the row was never appended, so nothing could put the two side by side,
    // which is what comparable is for.
    //
    // THIS NEEDED THE STREAMED WALK FIRST, and that is why it is landing now
    // rather than with the other three verbs. `Sweep` carries no count of
    // combinations CONSIDERED -- the nearest figure it holds is
    // `all_frequent().count()`, the KEPT total, and the ledger's `combinations`
    // field means considered. Writing one into the other would be a mislabelled
    // row in an append-only file, which cannot be corrected later. `Streamed`
    // has the real count, is the same walk by `engine`'s own test, and retains
    // less -- which matters most here, where months run in parallel.
    let filed = crate::record_swept_run(
        crate::Recording {
            root,
            feed: loaded.vendor.as_str(),
            underlying: held.symbol.as_str(),
            timeframe: held.timeframe.as_str(),
            from: (held.month.year(), held.month.month()),
            to: (held.month.year(), held.month.month()),
            attempt: None,
            months_asked: 1,
            months_found: 1,
        },
        &id,
        &outcome.sweep,
        outcome.census.swept,
        min_hits,
    );
    // A MONTH THAT SWEPT AND COULD NOT BE FILED IS A FACT THIS REPORT SHOWS.
    // `refused` is the field that already exists for it, and the sweep's own
    // numbers stay in the row beside the reason -- the walk happened.
    let refused = filed.err().map(|why| format!("not recorded: {why}"));

    Row {
        label,
        bars: outcome.census.swept,
        depth,
        kept,
        completed,
        identity: Some(runner::identity::RunId::hex(&id)),
        refused,
    }
}

/// The report. One line per instrument-month, then the arithmetic.
fn render(
    vendor_word: &str,
    rung: &str,
    min_hits: u64,
    commit: &str,
    walk: &catalog::Census,
    tally: &Tally,
    rows: &[Row],
) -> String {
    let mut out = String::from(crate::STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "feed {vendor_word} · rung {rung} · min_hits {min_hits} · built at {commit}"
    );
    let _ = writeln!(
        out,
        "store holds {} spot instrument-month(s); {} match this feed and rung",
        walk.spot, tally.offered
    );
    out.push('\n');

    for row in rows {
        if let Some(ref why) = row.refused {
            let _ = writeln!(out, "  REFUSED  {}  — {why}", row.label);
        } else {
            let _ = writeln!(
                out,
                "  {}  {:>9} bars  k={}  {:>7} kept{}",
                row.label,
                row.bars,
                row.depth,
                row.kept,
                if row.completed {
                    ""
                } else {
                    "  CEILING — depth not reached by extinction"
                }
            );
            // THE IDENTITY, ON ITS OWN LINE UNDER THE MONTH IT IDENTIFIES.
            //
            // Indented past the label so a reader scanning months is not made to
            // read 64 hex characters per line, and printed rather than omitted
            // because `STORED_PROVENANCE` two screens above tells them it is
            // here. A run whose identity is not recorded is a run `CLAUDE.md` §3
            // rule 3 does not permit.
            if let Some(ref hex) = row.identity {
                let _ = writeln!(out, "      identity {hex}");
            }
        }
    }

    out.push('\n');
    let _ = writeln!(
        out,
        "{} swept · {} refused · {} bars · {} combinations kept",
        tally.swept, tally.refused, tally.bars, tally.kept
    );
    if tally.incomplete > 0 {
        let _ = writeln!(
            out,
            "{} month(s) stopped on the candidate ceiling. §6 says depth is decided \
             by extinction; those did not get that far and their depth is a floor, \
             not an answer.",
            tally.incomplete
        );
    }
    if !tally.reconciles() {
        let _ = writeln!(
            out,
            "CENSUS DOES NOT RECONCILE: {} offered, {} swept + {} refused. A month \
             has been lost, which is the silent shortfall §4 bans.",
            tally.offered, tally.swept, tally.refused
        );
    }
    out
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes — a test \
              that cannot panic cannot fail. `.expect` panics inside core, which \
              llvm-cov does not instrument, so no dead region is left behind. \
              `clippy::panic` is deliberately NOT listed: nothing here uses the \
              macro, and an unfulfilled expectation is itself an error"
)]
mod tests {
    use super::{Tally, render, sweep_under};

    /// A private store root, named for its owner so two tests cannot collide.
    fn scratch(name: &str) -> std::path::PathBuf {
        let mut root = std::env::temp_dir();
        root.push(format!("brutex-batch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch root is creatable");
        root
    }

    /// **Every offered month is swept or refused, and nothing else.**
    ///
    /// The arithmetic that stops a short report hiding a long tail. A run
    /// reporting "12 swept" out of 54,000 offered has lost 53,988 months, and
    /// without this the report would look like a success.
    #[test]
    fn the_tally_reconciles_only_when_every_month_is_accounted_for() {
        let good = Tally {
            offered: 10,
            swept: 7,
            refused: 3,
            ..Tally::default()
        };
        assert!(good.reconciles(), "7 + 3 == 10");

        let lost = Tally {
            offered: 10,
            swept: 7,
            refused: 1,
            ..Tally::default()
        };
        assert!(!lost.reconciles(), "two months vanished and must be caught");
    }

    /// An empty store is an empty report, not a refusal.
    ///
    /// A fresh clone has pulled nothing. Refusing would make the normal first
    /// state look like a fault.
    #[test]
    fn an_empty_store_sweeps_nothing_and_says_so() {
        let root = scratch("empty");
        let text = sweep_under(&root, "groww", "1min", 100, "deadbeef")
            .expect("an empty store is not an error");
        assert!(text.contains("0 spot instrument-month"), "{text}");
        assert!(text.contains("0 swept · 0 refused"), "{text}");
    }

    /// A file the store cannot read is REFUSED BY NAME and the walk continues.
    ///
    /// The property that matters at 54,000 months: one unreadable file must not
    /// abandon the other 53,999. Written as a bar path the catalog will offer
    /// and the loader will then reject, because that is the real failure —
    /// a path that looks right and holds nothing.
    #[test]
    fn a_month_that_cannot_be_loaded_is_named_and_the_run_continues() {
        let root = scratch("refused");
        let dir = root.join("bars/groww/NSE/INDEX/NIFTY/1min");
        std::fs::create_dir_all(&dir).expect("dirs are creatable");
        std::fs::write(dir.join("2026-08.bin"), b"not a bar file").expect("writable");

        let text = sweep_under(&root, "groww", "1min", 100, "deadbeef").expect("the run completes");
        assert!(
            text.contains("REFUSED"),
            "the month is named as refused: {text}"
        );
        assert!(text.contains("NIFTY"), "and named by instrument: {text}");
        assert!(text.contains("1 refused"), "and counted: {text}");
        assert!(
            !text.contains("DOES NOT RECONCILE"),
            "one refusal still reconciles: {text}"
        );
    }

    /// The feed and rung filter, so a run sweeps what was asked for and no more.
    #[test]
    fn only_the_named_feed_and_rung_are_swept() {
        let root = scratch("filter");
        for rel in [
            "groww/NSE/INDEX/NIFTY/1min/2026-08.bin",
            "groww/NSE/INDEX/NIFTY/5min/2026-08.bin",
            "dhan/NSE/INDEX/NIFTY/1min/2026-08.bin",
        ] {
            let full = root.join("bars").join(rel);
            std::fs::create_dir_all(full.parent().expect("has a parent")).expect("creatable");
            std::fs::write(&full, b"x").expect("writable");
        }
        let text = sweep_under(&root, "groww", "1min", 100, "deadbeef").expect("runs");
        assert!(
            text.contains("3 spot instrument-month(s); 1 match"),
            "three held, one matched: {text}"
        );
    }

    /// An unknown feed refuses before the store is touched.
    #[test]
    fn an_unknown_feed_is_refused_by_name() {
        let root = scratch("badfeed");
        let why = sweep_under(&root, "nosuchfeed", "1min", 100, "deadbeef")
            .expect_err("an unknown feed refuses");
        assert!(why.contains("nosuchfeed"), "the refusal names it: {why}");
    }

    /// **An unknown rung refuses by name, instead of reporting a clean run.**
    ///
    /// `sweep-all dhan 5sec 5` filtered the catalog on raw string equality with
    /// nothing validating the word, so it walked the whole store, matched
    /// nothing, printed `0 swept · 0 refused · 0 bars · 0 combinations kept` and
    /// exited 0. Those are the exact bytes an empty store prints, so a typo and
    /// a fresh clone were indistinguishable — and `sweep-stored` refused the
    /// same word by name the whole time.
    #[test]
    fn an_unknown_rung_is_refused_by_name_and_not_reported_as_an_empty_store() {
        let root = scratch("badrung");
        // A month IS present, so a report would have had something to reconcile
        // against and the old zeroes were never "there was nothing here".
        let full = root.join("bars/groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
        std::fs::create_dir_all(full.parent().expect("has a parent")).expect("creatable");
        std::fs::write(&full, b"x").expect("writable");

        let why = sweep_under(&root, "groww", "5sec", 100, "deadbeef")
            .expect_err("an unknown rung is not a sweep of nothing");
        assert!(
            why.contains("5sec"),
            "the refusal names what was typed: {why}"
        );
        assert!(
            why.contains("not a rung this store carries"),
            "and answers the first question, which is whether it exists: {why}"
        );
        assert!(!why.contains("0 swept"), "and is never a report: {why}");
    }

    /// The empty word is the same refusal, not a filter that matches nothing.
    #[test]
    fn an_empty_rung_word_is_refused_rather_than_matching_every_nothing() {
        let root = scratch("emptyrung");
        let why = sweep_under(&root, "groww", "", 100, "deadbeef")
            .expect_err("the empty word is not a rung");
        assert!(
            why.contains("not a rung this store carries"),
            "an absent argument is refused like any other bad one: {why}"
        );
    }

    /// **`1day` and `1s` are stored, are readable, and are NOT swept here.**
    ///
    /// `stored::rung` admits all ten of `Timeframe::KNOWN` because non-sweep
    /// readers need `1day` — the daily context of every stored sweep is opened
    /// through it. `crate::EVERY_RUNG` is the eight this engine sweeps, and this
    /// command was one of the doors that swept the other two anyway: a daily
    /// signal bar acts on a condition that was not knowable until the session it
    /// describes had closed, which is the look-ahead `CLAUDE.md` §3 rule 7
    /// forbids.
    #[test]
    fn the_two_stored_but_unswept_rungs_are_refused_by_name() {
        let root = scratch("unswept");
        for word in ["1day", "1s"] {
            let full = root
                .join("bars/groww/NSE/INDEX/NIFTY")
                .join(word)
                .join("2026-08.bin");
            std::fs::create_dir_all(full.parent().expect("has a parent")).expect("creatable");
            std::fs::write(&full, b"x").expect("writable");

            let why = sweep_under(&root, "groww", word, 100, "deadbeef")
                .expect_err("a stored rung this engine does not sweep");
            assert!(why.contains(word), "the refusal names it: {why}");
            assert!(
                why.contains("is not a rung this engine sweeps"),
                "and names the surface it is outside of: {why}"
            );
            assert!(
                why.contains("1min, 2min, 3min, 5min, 10min, 15min, 30min, 60min"),
                "and lists the eight that are inside it: {why}"
            );
            assert!(
                !why.contains("not a rung this store carries"),
                "{word} IS carried by the store, and saying otherwise would send \
                 the operator to pull a month they already have: {why}"
            );
        }
    }

    /// Every rung the engine does sweep still reaches the walk.
    ///
    /// The other half of the guard. A check that refused a legal rung would be
    /// a worse defect than the one it replaced, and `EVERY_RUNG` is iterated
    /// rather than retyped so appending a ninth cannot leave this behind.
    #[test]
    fn all_eight_swept_rungs_cross_the_guard_and_reach_the_report() {
        let root = scratch("eight");
        for rung in crate::EVERY_RUNG {
            let text = sweep_under(&root, "groww", rung, 100, "deadbeef")
                .expect("a rung the engine sweeps must reach the walk");
            assert!(
                text.contains("0 swept · 0 refused"),
                "{rung} reached an empty store and reported it: {text}"
            );
        }
    }

    /// A report whose tally does not reconcile says so in the report itself.
    ///
    /// The renderer is driven directly with a broken tally, because the only
    /// other way to reach this line is a bug in the loop that produces it — and
    /// a line that can only be reached by a bug is a line no test would cover.
    #[test]
    fn a_report_over_a_broken_tally_announces_the_shortfall() {
        let broken = Tally {
            offered: 9,
            swept: 1,
            refused: 1,
            ..Tally::default()
        };
        let text = render(
            "groww",
            "1min",
            100,
            "deadbeef",
            &store::catalog::Census::default(),
            &broken,
            &[],
        );
        assert!(
            text.contains("DOES NOT RECONCILE"),
            "the shortfall is announced, not swallowed: {text}"
        );
    }

    /// A ceiling breach is reported as a floor on depth, not as an answer.
    ///
    /// §6 says depth is decided by extinction. A month that stopped on the
    /// candidate ceiling did not get there, and a report that printed its `k`
    /// without saying so would be stating a depth the search never reached.
    #[test]
    fn a_ceiling_breach_is_reported_as_a_floor_and_not_as_a_depth() {
        let tally = Tally {
            offered: 1,
            swept: 1,
            incomplete: 1,
            ..Tally::default()
        };
        let text = render(
            "groww",
            "1min",
            100,
            "deadbeef",
            &store::catalog::Census::default(),
            &tally,
            &[],
        );
        assert!(text.contains("candidate ceiling"), "{text}");
        assert!(
            text.contains("floor"),
            "and calls the depth a floor: {text}"
        );
    }

    /// The report leads with the REAL-bars banner, never the generated one.
    ///
    /// A sweep over real data and one over invented data are byte-identical in
    /// shape. The banner is the only thing separating them, and this command
    /// only ever reads the store.
    #[test]
    fn the_report_carries_the_real_data_banner() {
        let root = scratch("banner");
        let text = sweep_under(&root, "groww", "1min", 100, "deadbeef").expect("runs");
        assert!(
            text.contains("REAL MARKET DATA"),
            "the stored banner leads: {text}"
        );
        assert!(
            !text.contains("GENERATED"),
            "and never the generated one: {text}"
        );
    }
}

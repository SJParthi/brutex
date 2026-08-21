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

use crate::stored;
use core::fmt::Write as _;
use engine::Ladder;
use runner::Sweeper;
use store::catalog::{self, Held};

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
}

/// Sweeps every stored instrument-month matching `vendor_word` and `rung`.
///
/// Returns the rendered report. A month that refuses is recorded and the walk
/// continues: one unreadable file must not abandon 53,999 others, which is the
/// same reasoning `store::catalog` applies to an unreadable subdirectory.
#[must_use]
pub fn sweep_all(vendor_word: &str, rung: &str, min_hits: u64) -> String {
    match run(vendor_word, rung, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// The fallible half, so the caller above has exactly one refusal shape.
fn run(vendor_word: &str, rung: &str, min_hits: u64) -> Result<String, String> {
    // BEFORE A SINGLE BAR IS READ. `CLAUDE.md` §3 rule 3 forbids a computation
    // whose identity cannot be recorded, and a 54,000-month run that discovers
    // that at the end has burned hours to produce nothing citable.
    let commit = crate::commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and no sweep will run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;
    // Parsed here as well as in sweep_under so a bad feed name refuses BEFORE
    // the store is touched, which is what an operator typing a typo expects.
    crate::parse_vendor(vendor_word)?;
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
    let mut rows: Vec<Row> = Vec::with_capacity(wanted.len());

    for held in wanted {
        rows.push(one(root, held, min_hits, commit, &mut tally));
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

/// Sweeps one instrument-month, folding the outcome into `tally`.
fn one(root: &std::path::Path, held: &Held, min_hits: u64, commit: &str, tally: &mut Tally) -> Row {
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
            tally.refused = tally.refused.saturating_add(1);
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
    let mut ev = match crate::evaluator() {
        Ok(ev) => ev,
        Err(why) => {
            tally.refused = tally.refused.saturating_add(1);
            return Row {
                label,
                bars: 0,
                depth: 0,
                kept: 0,
                completed: false,
                identity: None,
                refused: Some(why.to_owned()),
            };
        }
    };
    let ladder = Ladder::with_min_hits(min_hits);
    let outcome = Sweeper::new(ladder).run(&loaded.bars, &mut ev);

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
        data_digest: runner::identity::data_digest(&loaded.bars),
        commit,
        // The feed this month's file was actually read from. A whole-store
        // sweep walks several vendors in one run, so this is the term that keeps
        // two feeds' rows for one instrument-month from carrying one identity.
        feed: loaded.vendor.as_str(),
    });

    let kept = outcome.sweep.all_frequent().count();
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

    tally.swept = tally.swept.saturating_add(1);
    tally.bars = tally.bars.saturating_add(outcome.census.swept);
    tally.kept = tally
        .kept
        .saturating_add(u64::try_from(kept).unwrap_or(u64::MAX));
    if !completed {
        tally.incomplete = tally.incomplete.saturating_add(1);
    }

    Row {
        label,
        bars: outcome.census.swept,
        depth,
        kept,
        completed,
        identity: Some(runner::identity::RunId::hex(&id)),
        refused: None,
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

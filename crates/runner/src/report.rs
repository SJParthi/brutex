//! One sweep, rendered so a human can audit it without reading the code.
//!
//! # Why a report, and why here
//!
//! CI gate 17 forbids `crates/vocab`, `crates/indicators` and `crates/engine`
//! from emitting anything at all: the sweep evaluates a mask per (bar,
//! combination), and a billion O(1) calls is still a billion calls. What it
//! prescribes instead is **plain integer counters, incremented in the loop with
//! no atomic and no allocation, emitted ONCE at a structural boundary**.
//!
//! `crates/runner` is not on gate 17's list, and this is that structural
//! boundary. Every number below was already counted by the sweep; nothing here
//! measures anything, it only renders what the walk carried out. So the audit
//! costs one string per run rather than anything per bar.
//!
//! # What it is for
//!
//! A [`Sweep`] read without its census cannot tell "no combination was frequent"
//! from "almost every bar was refused as corrupt", and a halted walk read
//! without [`engine::Halt`] looks like a complete one. `CLAUDE.md` §4 bans a
//! result that hides a failure behind a success, so the report puts the refusals,
//! the warm-up loss and the halt reason beside the answer rather than in a
//! different place a reader has to think to look.
//!
//! # Plain text on purpose
//!
//! No JSON, no HTML, no dependency. A sweep is audited from a terminal or a log
//! file, and both of those are text. `web/` is where a browser view belongs and
//! this crate cannot reach it.
//!
//! # Cost
//!
//! One `String` per run, and it **touches no bar** -- which is the whole reason
//! it may exist outside gate 17's silence. It does touch every frequent
//! COMBINATION, and this paragraph claimed otherwise until an audit measured it.
//!
//! The SIGNIFICANCE section deflates the trial count by the redundancy
//! `crate::closed` finds, and finding it means walking the answer. Measured on
//! the twelve-level fixture, same column, only `min_hits` varied:
//!
//! | frequent sets | levels | render |
//! |---|---|---|
//! | 0 | 1 | 2,041 ns |
//! | 456 | 9 | 88,875 ns |
//! | 3,689 | 12 | 978,542 ns |
//!
//! Levels grew 1.33x and the render grew 7.2x: **the cost is O(frequent sets),
//! not O(levels)**, and effectively all of it is the deflation.
//!
//! `C-R-02` in `crates/runner/benches/ratio.rs` cannot see that. It neuters both
//! ladders to `min_hits = u64::MAX`, which yields ZERO frequent sets, so the
//! deflation is a no-op inside the timed region. The row is honest about what it
//! measures -- cost per level as the COLUMN grows -- and was cited here as proof
//! of a bound it structurally cannot cover. That citation is withdrawn.
//!
//! This is a once-per-run boundary, so O(answer) is affordable. It is stated
//! rather than claimed away.

use core::fmt::Write as _;

use crate::identity::RunId;
use crate::rank::Ranked;
use crate::{Auto, Outcome};
use engine::column::set_positions;
use engine::{Sweep, Why};
use indicators::column::Census;
use vocab::ConditionMask;

/// The condition names a mask requires, in ascending position order.
///
/// # Why this function is the point of the whole product
///
/// Until it existed, `vocab::table::name` had **zero production call sites**
/// workspace-wide: the 280-row name table shipped in the binary and never
/// reached an operator. A sweep would report `combinations found 3,689` and
/// there was no surface anywhere -- not in the CLI, not over HTTP -- that could
/// turn one of those 3,689 masks back into the conditions it is made of. The
/// mask reached the report as a sort key, a dedup key and a hash input, and in
/// none of those roles is it readable. An audit named this the headline hole and
/// it was right: a brute-force search whose answer cannot be read is a counter,
/// not a research tool.
///
/// # A position with no name is printed, not skipped
///
/// `table::name` returns `None` above the table, and a mask CAN carry such a bit
/// -- `ConditionMask` is 384 wide against a 280-row table, and `with_bit`
/// silently ignores nothing below 384. Rendering that as `?<position>` rather
/// than dropping it keeps the printed arity equal to `popcount`, so a reader
/// counting names against the `k` the ladder reports cannot be misled by a
/// silent omission. `CLAUDE.md` §4: degrade loudly and name the reason.
///
/// # Cost
///
/// O(popcount) lookups, each a direct index into a fixed array -- `set_positions`
/// walks set bits by `trailing_zeros`, never by scanning 384 positions. The
/// allocation is per RENDERED ROW, not per bar or per candidate, so it is off
/// every path `CLAUDE.md` §3 rule 4 bounds.
#[must_use]
pub fn condition_names(mask: &ConditionMask) -> Vec<String> {
    set_positions(mask)
        .map(|position| {
            u16::try_from(position)
                .ok()
                .and_then(vocab::table::name)
                .map_or_else(|| format!("?{position}"), ToOwned::to_owned)
        })
        .collect()
}

/// The positions that never entered the ladder, **by name and by reason**.
///
/// # D-0080 recorded these and nothing ever printed them
///
/// `engine::Sweep::excluded` has carried a `Vec<Excluded>` — position, measured
/// support, and a `Why` — since D-0080, whose whole requirement is that a
/// position excluded before k=1 be *"named in the run output rather than
/// silently dropped, so that a reader of a result can tell 'this condition was
/// never tried' apart from 'this condition was tried and lost'"*.
///
/// The report printed `excluded.len()` and nothing else. A count is exactly the
/// thing D-0080 says is not enough.
///
/// # What this makes visible, and it is not a small thing
///
/// On every run this binary can perform, **twenty of the 238 live positions are
/// the VWAP family and every one of them is permanently false** — `cli` and
/// `runner` both pass `vwap::Availability::Absent`, because deriving
/// availability reads the whole slice and that would be the look-ahead
/// `CLAUDE.md` §3 rule 7 forbids. The ladder is handed them as live, measures
/// support 0, and excludes them. Correct at every step, and until now the
/// operator was told only that "20 positions were excluded" — not that a
/// documented family of the vocabulary is switched off, nor why.
///
/// `CLAUDE.md` §4: degrade loudly and name the reason. This is the naming.
///
/// # Cost
///
/// One line per excluded position, at most `table::COUNT`. Off every path §3
/// rule 4 bounds — it runs once, at the same structural boundary as the rest of
/// the report.
fn excluded_by_name(out: &mut String, sweep: &Sweep) {
    if sweep.excluded.is_empty() {
        return;
    }
    let _ = writeln!(out, "  EXCLUDED BEFORE k=1, by name");
    for e in &sweep.excluded {
        // `Why::NotLive` is the only variant with no measurement, and it says so
        // rather than printing a zero — a support of 0 and a support that was
        // never taken are different facts, and §3 rule 6 forbids naming a
        // measurement that was not made.
        let support = e
            .support
            .map_or_else(|| "not measured".to_owned(), |s| s.to_string());
        let why = match e.reason {
            Why::AlwaysFalse => "never true on any loaded bar",
            Why::AlwaysTrue => "true on every loaded bar, so it partitions nothing",
            Why::NotLive => "retired, void, or outside the table",
        };
        let name = u16::try_from(e.position)
            .ok()
            .and_then(vocab::table::name)
            .unwrap_or("?");
        let _ = writeln!(
            out,
            "    {:<3} {name:<34} support {support:>10}  {why}",
            e.position
        );
    }
    let _ = writeln!(out);
}

/// The condition names of `mask`, joined for one line of a report.
///
/// `·` rather than `,` because several condition names contain no punctuation
/// and a comma reads as part of the name at a glance; the separator has to be a
/// character the vocabulary never uses. An empty mask renders as a named empty
/// set rather than as an empty string, because a blank where a combination
/// should be is indistinguishable from a rendering bug.
fn conditions_line(mask: &ConditionMask) -> String {
    let names = condition_names(mask);
    if names.is_empty() {
        return "(no conditions — an empty mask matches every bar)".to_owned();
    }
    names.join(" · ")
}

/// Observations below which a normal-quantile bar cannot rule on a t-statistic.
///
/// `Edge::t` is Student-t with `n - 1` degrees of freedom; every threshold in
/// `crate::significance` is a NORMAL quantile. They converge as `n` grows and
/// diverge sharply below about thirty: an audit measured the bar understated by
/// 4.89 at `n = 13`, which spends 729x the family-wise budget on one row.
///
/// Thirty is the conventional crossing point and it is a stated convention, not
/// a derivation. Below it the report refuses a verdict rather than issuing one
/// across two distributions -- `CLAUDE.md` §4 prefers a named refusal to a
/// confident wrong answer.
const MIN_OBSERVATIONS: u64 = 30;

/// Column width for the label side of every row.
const LABEL: usize = 34;

/// One line: a label, a value, and an optional note.
fn row(out: &mut String, label: &str, value: &str, note: &str) {
    let _ = writeln!(out, "  {label:<LABEL$}{value:>14}  {note}");
}

/// A percentage in tenths, computed in integers.
///
/// `CLAUDE.md` §7 forbids a float here as firmly as it does in the engine: a
/// report that disagreed with the counters it renders would be worse than no
/// report. Tenths are enough to read and exact to compute.
fn permille(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "-".to_owned();
    }
    let tenths = part.saturating_mul(1_000) / whole;
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// Where every offered bar went, and what the ladder did with what survived.
///
/// The identity is optional because a caller may want the audit before it has
/// assembled one — but `CLAUDE.md` §3 rule 3 says no computation runs without an
/// identity recorded, so a report without one says so in place of the digest
/// rather than leaving the line out.
#[must_use]
pub fn render(outcome: &Outcome, id: Option<&RunId>) -> String {
    let mut out = String::with_capacity(2_048);

    let _ = writeln!(out, "SWEEP");
    let _ = writeln!(
        out,
        "  {:<LABEL$}{:>14}",
        "identity",
        id.map_or_else(|| "NOT RECORDED".to_owned(), RunId::hex)
    );
    let _ = writeln!(out);

    bars(&mut out, outcome.census);
    ladder(&mut out, &outcome.sweep);
    significance(&mut out, &outcome.sweep);
    verdict(&mut out, outcome);
    out
}

/// How large a t-statistic must be before it beats the best of luck.
///
/// Printed beside the answer rather than left for a reader to look up, because
/// the whole point of the number is that it is easy to forget: a sweep that
/// tested sixty-one million hypotheses has a noise floor near t = 6, and every
/// habit a reader brings from hand-built strategies says 2 or 3 is fine.
///
/// Derived entirely from this run's own counters — nobody is asked for a
/// number, the same standard depth and memory are held to.
fn significance(out: &mut String, sweep: &Sweep) {
    let raw = crate::significance::trials(sweep);
    // THE BAR IS COMPUTED FROM `raw`, AND IT USED TO BE COMPUTED FROM THE
    // EFFECTIVE COUNT.
    //
    // That paragraph read: "The EFFECTIVE count is what the bar is computed
    // from. Two masks with identical support are one hypothesis counted twice,
    // and charging for a trial nobody ran raises the bar against real
    // findings." Every word of it is still true about the STATISTICS, and it is
    // no longer what this report can afford.
    //
    // `effective_trials` subtracts `closed::redundant_count`, which is
    // `O(sum |F_k| * k)` -- one hash lookup per set bit of every itemset --
    // against a set hashbrown rounds to 67,108,864 buckets at forty million
    // survivors. MEASURED 2026-08-29: a run sat at 93% CPU, one core of
    // fourteen, for five minutes inside that call. An `audit-range` run reached
    // it FOUR times; this is the third of those removed.
    //
    // `rank::walk` orders on `bonferroni_t(trials(..))` for the same reason, and
    // the two must agree or the report prints "clears" for a row the ordering
    // demoted -- which `one_sweep_prints_one_bar_and_the_rows_are_judged_against_it`
    // exists to catch, and did.
    //
    // NOT SUBTRACTING IS THE SAFE DIRECTION. Counting a duplicate twice makes
    // `n` larger and the bar STRICTER, so the report can refuse a finding the
    // exact count would have allowed. It cannot admit one the exact count would
    // have refused. The deflation is a sharpening this run cannot pay for on the
    // ranking path, not a correction it needs.
    let n = raw;
    // THE DEFLATED COUNT IS STILL SHOWN, because it is real information and this
    // is the one place that can afford to compute it.
    //
    // A first attempt at the above deleted this too, and
    // `the_report_states_the_bar_a_result_must_clear` caught it: that test
    // asserts `distinct < raw`, and on the shipped fixture 3,591 of 3,798 masks
    // have support identical to a superset's. Ninety-four per cent duplication
    // is a fact about the vocabulary an operator should see, and dropping the
    // row to save a call would have hidden it.
    //
    // It stays HERE and not in `rank::walk` because the difference is where the
    // call sits, not whether it is affordable: this runs once, at the report
    // boundary, after every parallel phase has finished. `walk` ran it BEFORE
    // the parallel phase, which is what left thirteen cores idle.
    let distinct = crate::significance::effective_trials(sweep);
    let _ = writeln!(out, "SIGNIFICANCE");
    row(
        out,
        "hypotheses tested",
        &raw.to_string(),
        "support counted against bars",
    );
    row(
        out,
        "  distinct tests among them",
        &distinct.to_string(),
        "exact duplicates removed -- same support, same test",
    );
    if n < 2 {
        row(
            out,
            "threshold",
            "-",
            "too few hypotheses to have a noise floor",
        );
        let _ = writeln!(out);
        return;
    }
    row(
        out,
        "best t-stat by luck alone",
        &format!("{:.2}", crate::significance::expected_max_bailey(n)),
        "Bailey & Lopez de Prado, if every hypothesis were worthless",
    );
    row(
        out,
        "  the sqrt(2 ln N) approximation",
        &format!("{:.2}", crate::significance::expected_max_t(n)),
        "the figure usually quoted -- it OVERSTATES the floor",
    );
    row(
        out,
        "t required (Bonferroni 5%)",
        &format!("{:.2}", crate::significance::bonferroni_t(n)),
        "Harvey, Liu & Zhu: their floor is 3.0 for a HANDFUL of trials",
    );
    let _ = writeln!(out);
}

/// A statistical mean, rendered as the paisa integer §7 keeps money in.
///
/// `f64 as i64` truncates silently outside the integer range. No mean of paisa
/// closes can reach that, and printing a wrapped number rather than refusing is
/// the class of defect this repository keeps finding, so the conversion
/// saturates and says so.
#[allow(
    clippy::cast_possible_truncation,
    reason = "saturated at the i64 bounds immediately below; a paisa mean \
              outside them cannot arise from a price series."
)]
fn paisa(mean: f64) -> i64 {
    // 2^62 as a literal, comfortably inside both f64 exactness and i64 range,
    // so the comparison needs no lossy i64 -> f64 cast of its own. A paisa mean
    // beyond it is 46 quadrillion rupees and cannot arise from a price series.
    const UPPER: f64 = 4_611_686_018_427_387_904.0;
    // Written as a negative literal, not as `-UPPER`: negation is arithmetic,
    // and this module takes no float-arithmetic exception -- the one place that
    // rule bites is where a bound is expressed rather than computed.
    const LOWER: f64 = -4_611_686_018_427_387_904.0;
    let r = mean.round();
    if r >= UPPER {
        i64::MAX
    } else if r <= LOWER {
        i64::MIN
    } else {
        r as i64
    }
}

/// The kept combinations, each beside the bar it had to clear.
///
/// # Why the bar is printed on every row and not once at the top
///
/// The whole failure this guards against is a reader seeing `t = 4.1`, recalling
/// that three is the threshold they have always used, and stopping there. The
/// bar is a property of **how many hypotheses this run tested** — above six on a
/// sixty-one-million sweep — and it is printed against each row so the
/// comparison cannot be skipped.
///
/// A row that does not clear it is not a weak finding. It is **indistinguishable
/// from the best of pure noise**, and the verdict column says exactly that.
///
/// # What this table looks like on the synthetic fixture, and why that is a trap
///
/// `synthetic::sessions` generates a deterministic upward DRIFT. Every condition
/// that fires therefore "predicts" a rise, and the whole table clears the bar at
/// t values above twenty. That is the pipeline working -- it measures what it is
/// given -- and it is emphatically **not** an edge. On bars with no drift these
/// collapse toward zero, which is the entire reason the bar is printed.
///
/// A reader who sees t = 23 here and concludes the engine found gold has made
/// the mistake every part of this module exists to prevent.
#[must_use]
pub fn render_findings(ranked: &Ranked, sweep: &Sweep) -> String {
    let mut out = String::with_capacity(1_024);
    // THE BAR THE ORDERING ACTUALLY USED, READ RATHER THAN RECOMPUTED.
    //
    // This recomputed it from `effective_trials`, and once `rank::walk` began
    // ordering on `bonferroni_t(trials(..))` the two disagreed BY CONSTRUCTION
    // wherever a redundant support set exists -- `effective_trials <= trials`
    // and `bonferroni_t` is monotone, so the report could print "clears" for a
    // row the ordering had already demoted. `Ranked::bar` was added to end that
    // disagreement and nothing read it; this is the read.
    //
    // It also deletes one of the FOUR `closed::redundant_count` walks a single
    // `audit-range` run was paying -- each roughly 40M map inserts and 200-240M
    // lookups against a multi-gigabyte set. The trial count itself was never
    // printed here; it existed only to feed `bonferroni_t`, so reading the bar
    // removes the call outright rather than moving it.
    let bar = ranked.bar;

    let _ = writeln!(out, "FINDINGS");
    row(
        &mut out,
        "combinations weighed",
        &ranked.considered.to_string(),
        "",
    );
    row(
        &mut out,
        "kept",
        &ranked.top.len().to_string(),
        "best by |t|",
    );
    row(
        &mut out,
        "bar every row must clear",
        &format!("{bar:.2}"),
        "Bonferroni 5% on this run's own trial count",
    );
    // A TRUNCATED LADDER MAKES EVERY ROW BELOW CONDITIONAL, AND THIS SECTION
    // DID NOT SAY SO.
    //
    // `render` prints the halt in its own block, but `render_findings` is
    // rendered on its own by `cli::screen` and reads as a complete answer there.
    // A bar computed from a trial count the walk never finished collecting is
    // not wrong — it is the honest bar for what WAS tested — but a reader
    // comparing rows against it needs to know the frontier stopped early, which
    // is exactly the "failure wearing a success's clothes" §4 bans.
    //
    // This is also the only remaining use of `sweep` in this function. The
    // parameter used to feed `effective_trials`; reading `ranked.bar` removed
    // that call, and rather than take a `&Sweep` and ignore it — a signature
    // that lies, and one this crate cannot change because two of its callers are
    // in a file another session holds — it now carries the one fact the findings
    // cannot be read without.
    if let Some(halt) = sweep.halted.as_ref() {
        row(
            &mut out,
            "ladder HALTED",
            &format!("k={}", halt.k),
            "the frontier stopped short, so these are the best of a PARTIAL search",
        );
    }
    let _ = writeln!(out);

    if ranked.top.is_empty() {
        let _ = writeln!(out, "  nothing kept — the sweep produced no combination");
        let _ = writeln!(out);
        return out;
    }

    let _ = writeln!(
        out,
        "  {:<6}{:>10}{:>10}{:>14}{:>9}  verdict",
        "rank", "hits", "n", "mean paisa", "t"
    );
    for (index, s) in ranked.top.iter().enumerate() {
        // A t-statistic from n observations is Student-t, and the bar is a
        // NORMAL quantile. The two converge as n grows and diverge sharply
        // below about thirty -- an audit measured the bar understated by 4.89
        // at n = 13, which is 729 times the family-wise budget. Rather than
        // compare across distributions, a row with too few observations is not
        // judged at all.
        let judgeable = s.edge.n >= MIN_OBSERVATIONS;
        let clears = judgeable && s.edge.t.abs() >= bar;
        let _ = writeln!(
            out,
            "  {:<6}{:>10}{:>10}{:>14}{:>9.2}  {}",
            index.saturating_add(1),
            s.hits,
            s.edge.n,
            // The mean is a paisa figure and is printed as one: §7 keeps money
            // in integers, and a fractional paisa is not a price. `saturating`
            // rather than a bare cast: a mean outside i64 cannot arise from
            // paisa closes, and a truncating cast would print a wrapped number
            // rather than refuse.
            paisa(s.edge.mean_paisa),
            s.edge.t,
            // MISPAIRED OUTRANKS EVERY OTHER VERDICT, and it is checked first.
            //
            // `Edge::mismatched` counts hits whose bar the `Forward` could not
            // answer for because the two were built from DIFFERENT slices. Its
            // own doc says a non-zero count makes the whole struct meaningless
            // -- the mean and the t are then computed from returns belonging to
            // other bars. It was computed, documented that way, and never
            // rendered: `render_findings` printed rank, hits, n, mean, t and a
            // verdict, and an operator had no way to see it at all. A row that
            // says "clears" on fabricated numbers is the worst output this
            // report can produce, so it is now impossible to print one.
            if s.edge.mismatched > 0 {
                "MISPAIRED — the forward outcomes belong to other bars; \
                 this row's mean and t mean nothing"
            } else if !judgeable {
                "TOO FEW OBSERVATIONS to judge -- a normal bar cannot rule on a t"
            } else if clears {
                "clears"
            } else {
                "BELOW THE BAR — indistinguishable from luck"
            }
        );
        // THE COMBINATION, IN WORDS, ON ITS OWN LINE UNDER THE NUMBERS.
        //
        // Indented past the rank column so the table's own columns stay
        // scannable and the names read as a continuation of the row above
        // rather than as a sixth column that would wrap unpredictably -- a
        // combination at k=8 is longer than any terminal is wide, and a name
        // broken across a column boundary is worse than no name.
        //
        // Under the numbers and not above them, because a reader scanning for
        // `clears` reads the verdict first and only then asks what the row was.
        let _ = writeln!(out, "        {}", conditions_line(&s.mask));
    }
    // AND SAID ONCE MORE, IN A LINE OF ITS OWN. A per-row tag is easy to miss
    // in a table an operator scans for `clears`; a mispairing is a fault in the
    // RUN, not a property of one combination, and it is reported as one.
    let mispaired = ranked.top.iter().filter(|s| s.edge.mismatched > 0).count();
    if mispaired > 0 {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "  MISPAIRED: {mispaired} of {} row(s) measured a column against a \
             forward built from a different slice. Nothing in this section is \
             trustworthy. Rebuild the forward from the same bars the column \
             was built from.",
            ranked.top.len()
        );
    }
    // AND THE OTHER REASON A SAMPLE IS SMALLER, WHICH HAD NO LINE AT ALL.
    //
    // `Edge::refused` counts hits whose outcome was dropped because the exit bar
    // failed the engine's own bar check. Until it existed, those were swallowed
    // into the same branch as the TAIL — a documented, expected absence — so a
    // run over corrupt data reported a slightly smaller `n` and looked exactly
    // like a run over clean data with a slightly longer horizon.
    //
    // Reported per RUN and not per row, like the mispairing above: it is a fault
    // in the DATA, and every row measured on that data carries it.
    let dropped: u64 = ranked
        .top
        .iter()
        .fold(0_u64, |a, s| a.saturating_add(s.edge.refused));
    if dropped > 0 {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "  REFUSED EXITS: {dropped} outcome(s) across the rows above were \
             DROPPED because the exit bar failed the engine's own bar check. \
             Every `n` here is over a smaller sample, not a corrected one -- the \
             store handed this run records it does not consider bars, and the \
             BARS section's refusal count is where they were first seen."
        );
    }
    let _ = writeln!(out);
    out
}

/// Where every offered bar went.
fn bars(out: &mut String, census: Census) {
    let _ = writeln!(out, "BARS");
    row(out, "offered", &census.offered.to_string(), "");
    row(
        out,
        "swept",
        &census.swept.to_string(),
        &permille(census.swept, census.offered),
    );
    row(
        out,
        "warming (bits correct, not swept)",
        &census.warming.to_string(),
        &permille(census.warming, census.offered),
    );
    row(out, "refused", &census.refused().to_string(), "");
    // Every refusal reason, named. A total alone cannot tell a corrupt feed from
    // a single bad bar, which is the distinction §4 is about.
    for (name, n) in [
        ("  high below low", census.high_below_low),
        ("  range overflows", census.range_overflows),
        ("  price outside range", census.price_outside_range),
        (
            "  timestamp not increasing",
            census.timestamp_not_increasing,
        ),
        ("  negative volume", census.negative_volume),
        ("  accumulator too large", census.accumulator_too_large),
    ] {
        if n > 0 {
            row(out, name, &n.to_string(), "");
        }
    }
    row(
        out,
        "every bar accounted for",
        if census.reconciles() { "yes" } else { "NO" },
        if census.reconciles() {
            ""
        } else {
            "<- a bar was lost; this is a defect, not data"
        },
    );
    let _ = writeln!(out);
}

/// What the ladder did with the bars that survived.
fn ladder(out: &mut String, sweep: &Sweep) {
    let _ = writeln!(out, "LADDER");
    row(out, "min_hits applied", &sweep.min_hits.to_string(), "");
    row(
        out,
        "support that requires",
        &permille(sweep.min_hits, sweep.bars),
        "of swept bars",
    );
    row(out, "depth reached", &sweep.depth().to_string(), "");
    row(
        out,
        "combinations found",
        &sweep.all_frequent().count().to_string(),
        "",
    );
    row(
        out,
        "positions excluded before k=1",
        &sweep.excluded.len().to_string(),
        "always-true, always-false or not live",
    );
    let _ = writeln!(out);
    excluded_by_name(out, sweep);

    // ── per level ───────────────────────────────────────────────────────────
    // Every column `Frontier::reconciles` sums, in the order it sums them, so a
    // reader can add the row up on the page. Dropping `excluded` — which is only
    // ever non-zero at k=1 — would make that one row appear to lose 161
    // candidates, which is precisely the kind of unexplained gap this whole
    // report exists to prevent.
    let _ = writeln!(
        out,
        "  {:<6}{:>12}{:>12}{:>10}{:>10}{:>12}{:>10}",
        "level", "generated", "duplicate", "excluded", "pruned", "infrequent", "frequent"
    );
    for level in &sweep.levels {
        let _ = writeln!(
            out,
            "  k={:<4}{:>12}{:>12}{:>10}{:>10}{:>12}{:>10}{}",
            level.k,
            level.generated,
            level.duplicates,
            level.excluded,
            level.pruned,
            level.infrequent,
            level.frequent.len(),
            if level.reconciles() {
                ""
            } else {
                "  <- LOST A CANDIDATE"
            }
        );
    }
    let _ = writeln!(out);
}

/// Whether the answer may be believed as a whole.
fn verdict(out: &mut String, outcome: &Outcome) {
    let sweep = &outcome.sweep;
    let _ = writeln!(out, "VERDICT");
    match sweep.halted {
        // Extinction is a MEASUREMENT, and it may only be reported when one was
        // taken. `halted == None` is true for a sweep that finished and equally
        // true for one that never started: a cold column, a column where every
        // bar was refused, and the `Sweep::default()` that `Sweeper::auto`
        // substitutes when its search keeps no rung. All three used to print
        // "the frontier went extinct, which is the answer" -- an answer nobody
        // computed, which is the §4 breach this module's own header claims to
        // exist to prevent, and which no test here caught.
        None if sweep.levels.is_empty() || sweep.bars == 0 => row(
            out,
            "outcome",
            "NOTHING MEASURED",
            "no ladder was walked -- this is not extinction",
        ),
        None => row(
            out,
            "outcome",
            "complete",
            "the frontier went extinct, which is the answer",
        ),
        Some(halt) => {
            row(out, "outcome", "REFUSED", "the walk stopped short");
            row(
                out,
                "  budget spent",
                match halt.breach {
                    engine::Breach::Candidates => "candidates",
                    engine::Breach::Pairs => "pairs",
                    // Not a budget anyone set. The machine refused.
                    engine::Breach::Memory => "MEMORY",
                },
                "",
            );
            row(out, "  at level", &format!("k={}", halt.k), "");
            row(
                out,
                "  candidates admitted",
                &halt.candidates.to_string(),
                &format!("of {}", halt.ceiling),
            );
            row(
                out,
                "  pairs walked",
                &halt.pairs.to_string(),
                &format!("of {}", halt.pair_budget),
            );
        }
    }
    row(
        out,
        "trustworthy as a whole answer",
        if outcome.is_complete() { "yes" } else { "NO" },
        "",
    );
}

/// The same, for a run the engine tuned for itself.
///
/// Adds what the search did — which thresholds it tried and which it refused —
/// because a threshold chosen by machine needs more explaining than one a caller
/// passed in, not less.
#[must_use]
pub fn render_auto(auto: &Auto, id: Option<&RunId>) -> String {
    let mut out = render(&auto.outcome, id);
    let _ = writeln!(out);
    let _ = writeln!(out, "SEARCH");
    row(
        &mut out,
        "threshold chosen",
        &auto
            .min_hits
            .map_or_else(|| "NONE".to_owned(), |m| m.to_string()),
        if auto.affordable {
            ""
        } else {
            "<- no rung measured anything"
        },
    );
    row(
        &mut out,
        "ladders walked",
        &auto.attempts.to_string(),
        "one per halving",
    );
    row(
        &mut out,
        "refused below",
        &auto
            .refused_below
            .map_or_else(|| "-".to_owned(), |m| m.to_string()),
        "this column cannot afford lower",
    );
    out
}

/// The same names, from the six raw words a stored row carries.
///
/// # Why `cli` cannot call `condition_names` directly
///
/// It takes a `&ConditionMask`, and that type lives in `vocab`. `CLAUDE.md` §5
/// lists `cli`'s arrows and `vocab` is **not among them** -- adding one so a
/// listing could print a name would be the silent scope change §3 rule 2
/// forbids, and it would be invisible in review because `Cargo.toml` is the
/// only file that changes.
///
/// So the conversion lives here, in a crate that already holds the arrow
/// legitimately, and `cli` passes the `[u64; WORDS]` it read off disk. The
/// caller needs no vocabulary type at all.
///
/// # Why the ledger stores WORDS and not NAMES
///
/// A name is a `vocab_version` fact, not a run fact. Bit 143 means whatever
/// today's table says it means; freezing the string at write time would make an
/// old row render a claim the current table no longer makes, and nothing would
/// flag it. Storing the bits and resolving late means a bit that has moved
/// renders as `?143` -- visibly wrong rather than quietly stale. That is the
/// same reason `vocab_version` is one of the nine terms in §3 rule 3.
#[must_use]
pub fn names_from_words(words: [u64; vocab::mask::WORDS]) -> Vec<String> {
    condition_names(&ConditionMask::from_words(words))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail, and `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind."
)]
mod tests {
    use super::{Outcome, condition_names, conditions_line, permille, render, render_auto};
    use crate::identity::{Direction, Params, Run, data_digest, identity};
    use crate::{Sweeper, synthetic};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use engine::{Frontier, Ladder, Sweep};
    use indicators::column::Census;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use vocab::ConditionMask;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("both pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    /// AN EXCLUDED POSITION IS NAMED IN THE REPORT, NOT MERELY COUNTED.
    ///
    /// # What D-0080 asked for, and what the report gave
    ///
    /// D-0080's requirement is that a position excluded before k=1 be *"named in
    /// the run output rather than silently dropped, so that a reader of a result
    /// can tell 'this condition was never tried' apart from 'this condition was
    /// tried and lost'"*. `Sweep::excluded` carried the position, its measured
    /// support and a `Why` for it — and the report printed `excluded.len()`.
    ///
    /// A count cannot make that distinction, which is the one D-0080 exists for.
    ///
    /// # Why this runs a real sweep instead of hand-building a `Sweep`
    ///
    /// A fixture with a hand-written `excluded` vector would prove the renderer
    /// formats a struct. It would not prove that the ladder actually populates
    /// that struct on a real column, which is the half that was silently
    /// unexercised — and a test that passes on a fixture the production path
    /// never produces is the shape this whole audit kept finding.
    #[test]
    fn an_excluded_position_reaches_the_report_by_name_and_by_reason() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert!(
            !out.sweep.excluded.is_empty(),
            "the fixture must exclude something or this test proves nothing; a \
             generated column always has always-true and always-false positions"
        );
        assert!(
            text.contains("EXCLUDED BEFORE k=1, by name"),
            "the section exists: {text}"
        );

        // EVERY EXCLUDED POSITION APPEARS, not a sample and not a head.
        for e in &out.sweep.excluded {
            let name = u16::try_from(e.position)
                .ok()
                .and_then(vocab::table::name)
                .unwrap_or("?");
            assert!(
                text.contains(name),
                "position {} ({name}) was excluded and must be named: {text}",
                e.position
            );
        }

        // AND THE REASON IS WORDS, NOT A DISCRIMINANT. "excluded: 20" tells a
        // reader nothing; "never true on any loaded bar" tells them the
        // condition was measured and lost, which is the distinction D-0080 is
        // about.
        assert!(
            text.contains("never true on any loaded bar")
                || text.contains("partitions nothing")
                || text.contains("retired, void, or outside the table"),
            "the reason is stated in words: {text}"
        );

        // A SUPPORT THAT WAS NEVER TAKEN SAYS SO rather than printing 0. §3
        // rule 6 forbids naming a measurement that was not made, and 0 is a
        // real measurement.
        if out.sweep.excluded.iter().any(|e| e.support.is_none()) {
            assert!(
                text.contains("not measured"),
                "an unmeasured support must not render as a number: {text}"
            );
        }
    }

    /// A MASK RENDERS ONE NAME PER SET BIT, INCLUDING BITS THE TABLE CANNOT NAME.
    ///
    /// # Why the unnameable case is tested rather than argued away
    ///
    /// `ConditionMask` is 384 wide and the table defines 280 rows, so a mask can
    /// carry a bit no name exists for. On today's production path it cannot —
    /// every mask reaching a report is built from `Evaluator::positions`, all of
    /// which are in the table — so the `?N` arm is unreachable *from production*
    /// and would be an uncovered region under `CLAUDE.md` §9's 100% floor. That
    /// makes it exactly the kind of line the operator's standing rule says
    /// blocks the coverage floor, and the answer is a direct test rather than a
    /// deletion: the arm is what keeps the printed arity equal to `popcount`, so
    /// a reader counting names against the `k` the ladder reports cannot be
    /// misled by a silent omission.
    ///
    /// # The property, not the strings
    ///
    /// The assertion that matters is the COUNT: one rendered name per set bit,
    /// whatever the table says. A version that skipped unnameable positions
    /// would still produce plausible output and would fail here.
    #[test]
    fn every_set_bit_is_named_and_an_unnameable_one_is_shown_rather_than_dropped() {
        // Position 0 is the first row of the table; `NEXT_FREE` is the first
        // position past its end, and `with_bit` accepts anything below 384.
        //
        // DERIVED, BECAUSE IT MOVED THREE TIMES. It was 200 until the
        // forming-pivot block reached it, 300 until the crossing family did, and
        // 350 until the ordinal family did -- each time the test went red
        // because the "unnameable" position had acquired a name. That is the
        // gate working, but it is also a literal that has to be re-typed on
        // every append, and a fixture that needs maintenance is a fixture that
        // will eventually be maintained wrongly.
        //
        // `NEXT_FREE` IS the first unallocated position, by definition, so it
        // can never acquire a name without this line moving with it. The day the
        // table fills the mask entirely there is no unnameable position left and
        // this test fails loudly, which is the correct answer at that point.
        let unallocated = u32::from(vocab::table::NEXT_FREE);
        let both = ConditionMask::default().with_bit(0).with_bit(unallocated);
        let names = condition_names(&both);

        assert_eq!(
            names.len(),
            usize::try_from(both.popcount()).unwrap_or(usize::MAX),
            "one name per set bit, or a reader counting names against k is \
             silently misled: {names:?}"
        );
        assert_eq!(
            names,
            vec!["close_above_ema20".to_owned(), format!("?{unallocated}")],
            "the named position renders its name and the unnameable one renders \
             its index rather than vanishing"
        );

        // AND THE JOINED FORM SEPARATES THEM WITH A CHARACTER THE VOCABULARY
        // NEVER USES, so a name is never mistaken for two.
        assert_eq!(
            conditions_line(&both),
            format!("close_above_ema20 · ?{unallocated}")
        );

        // THE EMPTY MASK IS NAMED, NOT BLANK. A blank where a combination should
        // be is indistinguishable from a rendering bug, and an empty mask is the
        // one that matches every bar — the most dangerous thing to render silently.
        assert!(condition_names(&ConditionMask::ZERO).is_empty());
        assert!(
            conditions_line(&ConditionMask::ZERO).contains("matches every bar"),
            "the empty set says what it means"
        );
    }

    fn bounded() -> Ladder {
        Ladder::with_min_hits(600).with_ceiling(50_000)
    }

    /// The VALUE cell of a labelled row — what a reader's eye lands on.
    ///
    /// `text.contains("NO")` is satisfied by "NOT RECORDED", which every report
    /// rendered with `id = None` prints at the top; `text.contains("pairs")` is
    /// satisfied by the "  pairs walked" row, which `verdict` prints for EVERY
    /// halt whatever budget was breached. Two assertions in this file were
    /// therefore tautologies and a third could not distinguish the two halt
    /// kinds — an adversarial audit found all three, and none of them could have
    /// failed on any input. Reading the one cell under test is the fix.
    fn cell(text: &str, label: &str) -> String {
        text.lines()
            .find(|l| l.contains(label))
            .and_then(|l| l.split_once(label))
            .map(|(_, rest)| rest.split_whitespace().next().unwrap_or("").to_owned())
            .unwrap_or_default()
    }

    #[test]
    fn permille_is_integer_arithmetic_and_handles_the_empty_case() {
        assert_eq!(permille(0, 0), "-", "no denominator, no percentage");
        assert_eq!(permille(1, 2), "50.0%");
        assert_eq!(permille(1, 3), "33.3%");
        assert_eq!(permille(0, 100), "0.0%");
        assert_eq!(permille(100, 100), "100.0%");
    }

    #[test]
    fn a_complete_sweep_reports_every_section() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        for section in ["SWEEP", "BARS", "LADDER", "VERDICT"] {
            assert!(text.contains(section), "missing section {section}");
        }
        assert!(text.contains("NOT RECORDED"), "an absent identity is named");
        assert!(text.contains("complete"), "an extinct ladder is complete");
        assert!(text.contains("every bar accounted for"));
        // Every level is a row.
        for level in &out.sweep.levels {
            assert!(
                text.contains(&format!("k={}", level.k)),
                "level {} is missing from the table",
                level.k
            );
        }
    }

    #[test]
    fn every_level_row_adds_up_when_read_off_the_page() {
        // The report's claim is not "the counters reconcile" -- `reconciles()`
        // already says that inside the engine. The claim is that a HUMAN reading
        // the rendered table can verify it without trusting the code. So this
        // test parses the rendered text back, exactly as a reader would, and
        // adds the row up. A column silently dropped from the format string
        // passes every other test in this file and fails this one.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        let mut rows = 0_u32;
        for line in text.lines().filter(|l| l.trim_start().starts_with("k=")) {
            let n: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|f| f.parse().ok())
                .collect();
            assert_eq!(n.len(), 6, "a level row must print six counters: {line}");
            let (generated, rest) = n.split_first().expect("six is not zero");
            assert_eq!(
                *generated,
                rest.iter().sum::<u64>(),
                "this row does not add up on the page, so a reader auditing it \
                 would see candidates vanish: {line}"
            );
            rows = rows.saturating_add(1);
        }
        assert!(
            rows > 1,
            "the fixture must produce a real ladder, not one rung"
        );
    }

    #[test]
    fn a_refusal_names_the_budget_it_spent() {
        let bars = synthetic::sessions(8);
        let tight = Ladder::with_min_hits(2).with_pair_budget(50_000);
        let out = Sweeper::new(tight).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert_eq!(cell(&text, "outcome"), "REFUSED", "a halt must be loud");
        assert_eq!(
            cell(&text, "budget spent"),
            "pairs",
            "and must name the budget that was actually spent -- the ceiling was \
             never approached here"
        );
        assert_eq!(
            cell(&text, "trustworthy as a whole answer"),
            "NO",
            "a halted sweep is not trustworthy"
        );
    }

    #[test]
    fn a_refused_bar_is_named_by_its_reason() {
        let mut bars = synthetic::sessions(8);
        let last = bars.last().copied().expect("the run is not empty");
        bars.push(crate::candle(
            last.ts_micros + 60_000_000,
            last.close,
            last.close - 10,
            last.close + 10,
            last.close,
        ));
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert!(
            text.contains("high below low"),
            "a refusal reason must appear by name, not just in a total"
        );
    }

    /// A census that has lost a bar, built by hand.
    ///
    /// `Column::build` cannot produce one — every bar takes exactly one arm of an
    /// exhaustive match — which is why the branch is unreachable through the
    /// normal path and why it still has to be tested. The report's whole job is
    /// to be loud when an invariant breaks, and a detector nobody has ever seen
    /// fire is a detector nobody knows works.
    fn broken(census: Census, levels: Vec<Frontier>) -> Outcome {
        Outcome {
            census,
            first_swept: Some(0),
            sweep: Sweep {
                levels,
                excluded: Vec::new(),
                bars: census.swept,
                min_hits: 1,
                halted: None,
            },
        }
    }

    #[test]
    fn a_lost_bar_is_reported_as_a_defect_and_not_as_data() {
        let census = Census {
            offered: 10,
            swept: 4,
            warming: 1,
            ..Census::default()
        };
        assert!(!census.reconciles(), "the fixture must actually be broken");
        let text = render(&broken(census, Vec::new()), None);

        assert!(
            text.contains("a bar was lost; this is a defect, not data"),
            "a census that does not add up must say so in the report, not \
             silently render a smaller total"
        );
        assert_eq!(
            cell(&text, "every bar accounted for"),
            "NO",
            "the BARS cell must say NO -- the earlier form asserted \
             `text.contains(\"NO\")`, which \"NOT RECORDED\" satisfies on every \
             report rendered without an identity"
        );
        assert_eq!(
            cell(&text, "trustworthy as a whole answer"),
            "NO",
            "and the verdict must carry it, not just the BARS section"
        );
    }

    #[test]
    fn a_lost_candidate_is_reported_on_the_level_that_lost_it() {
        // generated 99, accounted for 0.
        let level = Frontier {
            k: 1,
            generated: 99,
            ..Frontier::default()
        };
        assert!(!level.reconciles(), "the fixture must actually be broken");
        let text = render(&broken(Census::default(), vec![level]), None);

        assert!(
            text.contains("LOST A CANDIDATE"),
            "a level whose counters do not add up must be marked on its own row, \
             so a reader sees WHICH level rather than only that something is wrong"
        );
    }

    #[test]
    fn a_candidate_ceiling_breach_names_candidates_and_not_pairs() {
        let bars = synthetic::sessions(8);
        // Ten admitted candidates is far below the live position count, so the
        // ceiling binds at k=1 and the pair budget is never approached.
        let tight = Ladder::with_min_hits(2).with_ceiling(10);
        let out = Sweeper::new(tight).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert_eq!(cell(&text, "outcome"), "REFUSED");
        assert_eq!(
            cell(&text, "budget spent"),
            "candidates",
            "the report must name the budget that was actually spent; the pair \
             budget was untouched and saying 'pairs' here would misdirect. The \
             earlier form asserted `text.contains(\"candidates\")`, which the \
             unconditional \"  candidates admitted\" row satisfies for a PAIR \
             breach too -- it could not fail either way"
        );
    }

    #[test]
    fn a_machine_refusal_is_reported_as_memory_and_not_as_a_budget() {
        // `Breach::Memory` is not a number anyone chose, and the report must not
        // dress it as one. Built by hand because a genuine allocation failure is
        // unreachable from a fixture -- the same reason `Ladder::exhausted` takes
        // a growth amount.
        let mut out = broken(Census::default(), Vec::new());
        out.sweep.bars = 1;
        out.sweep.halted = Some(engine::Halt {
            k: 7,
            candidates: 12_345,
            ceiling: engine::DEFAULT_CEILING,
            pairs: 99,
            pair_budget: engine::DEFAULT_PAIR_BUDGET,
            breach: engine::Breach::Memory,
        });
        let text = render(&out, None);

        assert_eq!(cell(&text, "outcome"), "REFUSED");
        assert_eq!(
            cell(&text, "budget spent"),
            "MEMORY",
            "the machine refusing an allocation is not a budget being spent, and \
             calling it `candidates` would name a ceiling that was never reached"
        );
        assert_eq!(cell(&text, "trustworthy as a whole answer"), "NO");
    }

    #[test]
    fn the_report_states_the_bar_a_result_must_clear() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert!(text.contains("SIGNIFICANCE"));
        let n: u64 = cell(&text, "hypotheses tested").parse().unwrap_or(0);
        assert!(
            n > 1_000,
            "this fixture tests thousands of hypotheses, got {n}"
        );
        // The bar must be well above the t>3 a reader would bring from a
        // hand-built strategy -- that is the entire reason it is printed.
        let required: f64 = cell(&text, "t required (Bonferroni 5%)")
            .parse()
            .unwrap_or(0.0);
        assert!(
            required > 3.0,
            "a sweep of {n} hypotheses cannot have a bar at or below the \
             literature's floor for a handful of trials; got {required}"
        );
        let luck: f64 = cell(&text, "best t-stat by luck alone")
            .parse()
            .unwrap_or(0.0);
        assert!(
            luck > 0.0 && luck < required,
            "the noise floor must be positive and sit BELOW the Bonferroni bar: \
             {luck} against {required}"
        );
        // THE DEFLATION IS ON THE PAGE. This fixture is 94% exact duplicates --
        // 3,591 of 3,798 are masks whose support is identical to a superset's --
        // so the distinct-test count must be far below the raw one. Asserting
        // the relationship rather than a fixed number, because the fixture's
        // redundancy is a property of the vocabulary and will move.
        let distinct: u64 = cell(&text, "distinct tests among them")
            .parse()
            .unwrap_or(0);
        assert!(
            distinct > 0 && distinct < n,
            "distinct tests must be positive and below the raw count: \
             {distinct} of {n}"
        );
    }

    #[test]
    fn a_run_with_no_hypotheses_states_that_rather_than_a_threshold() {
        // A cold column tested nothing, and "the noise floor of zero trials" is
        // not a quantity. It must say so rather than print 0.00, which reads
        // like an easy bar rather than an absent one.
        let auto = Sweeper::new(bounded()).auto(&synthetic::sessions(1), &mut evaluator());
        let text = render_auto(&auto, None);
        assert_eq!(cell(&text, "hypotheses tested"), "0");
        assert_eq!(cell(&text, "threshold"), "-");
    }

    #[test]
    fn every_finding_is_printed_beside_the_bar_it_had_to_clear() {
        // The failure this guards: a reader sees t = 4.1, remembers that three
        // is the threshold they have always used, and stops. The bar is a
        // property of how many hypotheses THIS run tested, so it goes on the
        // page beside the rows rather than in a paper the reader has not read.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let column = indicators::column::Column::build(&bars, &mut evaluator());
        let f = crate::outcome::forward(&bars, crate::outcome::Horizon::DEFAULT);
        let ranked = crate::rank::rank(&out.sweep, &column, &f, 10);
        let text = crate::report::render_findings(&ranked, &out.sweep);

        assert!(text.contains("FINDINGS"));
        assert_eq!(cell(&text, "kept"), "10");
        let bar: f64 = cell(&text, "bar every row must clear")
            .parse()
            .unwrap_or(0.0);
        assert!(bar > 3.0, "a sweep of thousands cannot have a bar at 3.0");

        // Every data row carries a verdict, and the verdict agrees with the bar.
        let mut rows = 0_u32;
        for line in text
            .lines()
            .filter(|l| l.trim_start().starts_with(char::is_numeric))
        {
            let clears = line.contains("clears");
            let below = line.contains("BELOW THE BAR");
            assert!(
                clears ^ below,
                "every row must carry exactly one verdict: {line}"
            );
            let t: f64 = line
                .split_whitespace()
                .nth(4)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            assert_eq!(
                t.abs() >= bar,
                clears,
                "the verdict disagrees with the bar on this row: {line}"
            );
            rows = rows.saturating_add(1);
        }
        assert_eq!(rows, 10, "ten kept means ten rows");
    }

    #[test]
    fn an_empty_ranking_says_so_rather_than_printing_an_empty_table() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let column = indicators::column::Column::build(&bars, &mut evaluator());
        let f = crate::outcome::forward(&bars, crate::outcome::Horizon::DEFAULT);
        let ranked = crate::rank::rank(&out.sweep, &column, &f, 0);
        let text = crate::report::render_findings(&ranked, &out.sweep);

        assert!(text.contains("nothing kept"));
        assert!(!text.contains("rank"), "no header for a table with no rows");
        // And the count of what was weighed is still stated: "kept none of four"
        // and "kept none of sixty-one million" are different facts.
        assert_ne!(cell(&text, "combinations weighed"), "0");
    }

    #[test]
    fn a_paisa_mean_saturates_rather_than_wrapping() {
        // `f64 as i64` truncates silently outside the range, printing a wrapped
        // number instead of refusing -- the class of defect this repository
        // keeps finding. No price series reaches these values; the arms exist
        // so that if one ever did the output would be visibly wrong rather than
        // quietly wrong.
        assert_eq!(super::paisa(0.0), 0);
        assert_eq!(super::paisa(47.4), 47, "rounds to the nearest paisa");
        assert_eq!(super::paisa(-47.6), -48);
        assert_eq!(super::paisa(f64::MAX), i64::MAX, "saturates, never wraps");
        assert_eq!(super::paisa(f64::MIN), i64::MIN);
        assert_eq!(super::paisa(f64::INFINITY), i64::MAX);
        assert_eq!(super::paisa(f64::NEG_INFINITY), i64::MIN);
    }

    #[test]
    fn a_row_below_the_bar_is_named_as_indistinguishable_from_luck() {
        // The synthetic fixture has an upward DRIFT, so every real row on it
        // clears -- which leaves the verdict that matters most untested. Built
        // by hand: one strong row and one weak one against a real sweep's bar.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let weak = crate::rank::Scored {
            mask: vocab::ConditionMask::default().with_bit(3),
            hits: 500,
            edge: crate::outcome::Edge {
                n: 500,
                mismatched: 0,
                refused: 0,
                mean_paisa: 1.0,
                t: 0.4,
                ..crate::outcome::Edge::default()
            },
        };
        let ranked = crate::rank::Ranked {
            top: vec![weak],
            considered: 3_689,
            halted: None,
            // THE REAL BAR THIS SWEEP DEMANDS, not a literal.
            //
            // `render_findings` reads `ranked.bar` rather than recomputing, so a
            // fixture carrying `0.0` would make every row clear and this test
            // would assert nothing. Deriving it the way `rank::walk` does keeps
            // the fixture honest and keeps the two in step if the trial count
            // ever changes again.
            bar: crate::significance::bonferroni_t(crate::significance::trials(&out.sweep)),
        };
        let text = crate::report::render_findings(&ranked, &out.sweep);

        assert!(
            text.contains("BELOW THE BAR"),
            "a t of 0.4 against a bar above four must be named, not merely \
             ranked last"
        );
        assert!(
            text.contains("indistinguishable from luck"),
            "and named in words a reader cannot misread as 'weak but real'"
        );
        assert!(!text.contains("  clears"), "nothing here clears");
    }

    #[test]
    fn one_sweep_prints_one_bar_and_the_rows_are_judged_against_it() {
        // The FINDINGS section computed its bar from `trials` while SIGNIFICANCE
        // computed one from `effective_trials`. A single report carried two
        // different Bonferroni figures and the per-row verdict used the harsher,
        // rejecting rows the page above had already said were allowed.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let column = indicators::column::Column::build(&bars, &mut evaluator());
        let f = crate::outcome::forward(&bars, crate::outcome::Horizon::DEFAULT);
        let ranked = crate::rank::rank(&out.sweep, &column, &f, 10);

        let stats = render(&out, None);
        let findings = crate::report::render_findings(&ranked, &out.sweep);
        let from_significance: f64 = cell(&stats, "t required (Bonferroni 5%)")
            .parse()
            .unwrap_or(0.0);
        let from_findings: f64 = cell(&findings, "bar every row must clear")
            .parse()
            .unwrap_or(-1.0);
        assert!(from_significance > 0.0);
        assert!(
            (from_significance - from_findings).abs() < 1e-9,
            "two sections of one report printed different bars: {from_significance} \
             and {from_findings}"
        );
    }

    #[test]
    fn a_row_with_too_few_observations_is_refused_rather_than_judged() {
        // `Edge::t` is Student-t; every bar here is a NORMAL quantile. They
        // diverge sharply below thirty observations -- an audit measured the bar
        // understated by 4.89 at n = 13. Comparing across distributions produces
        // a confident wrong answer, which §4 ranks below a named refusal.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let thin = crate::rank::Scored {
            mask: vocab::ConditionMask::default().with_bit(7),
            hits: 13,
            edge: crate::outcome::Edge {
                n: 13,
                mismatched: 0,
                refused: 0,
                mean_paisa: 50.0,
                // Enormous, and it must STILL not be called a finding.
                t: 40.0,
                ..crate::outcome::Edge::default()
            },
        };
        let ranked = crate::rank::Ranked {
            top: vec![thin],
            considered: 3_689,
            halted: None,
            bar: 0.0,
        };
        let text = crate::report::render_findings(&ranked, &out.sweep);
        assert!(
            text.contains("TOO FEW OBSERVATIONS"),
            "thirteen observations cannot be ruled on by a normal bar, however \
             large the statistic"
        );
        assert!(
            !text.contains("  clears"),
            "and it must not read as a finding"
        );
    }

    #[test]
    fn a_healthy_report_raises_neither_alarm() {
        // The negative direction, which nothing asserted. Both defect alarms
        // could have been wired permanently ON -- rendering "LOST A CANDIDATE"
        // on every row of every run -- and the two broken-fixture tests would
        // still pass, because they only prove an alarm CAN fire.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let text = render(&out, None);

        assert_eq!(
            cell(&text, "every bar accounted for"),
            "yes",
            "a clean census must say so in the cell, not merely fail to complain"
        );
        assert!(
            !text.contains("LOST A CANDIDATE"),
            "no level of a healthy sweep may carry the candidate alarm"
        );
        assert!(!text.contains("a bar was lost"), "nor the census alarm");
        assert_eq!(cell(&text, "trustworthy as a whole answer"), "yes");
    }

    #[test]
    fn a_search_that_kept_no_rung_says_nothing_was_measured() {
        // `Sweeper::auto` substitutes `Sweep::default()` when no threshold was
        // affordable: bars 0, no levels, and `halted: None` because nothing ran
        // far enough to breach anything. `Sweep::completed()` reads that as a
        // clean finish, so the report printed "the frontier went extinct, which
        // is the answer" and "trustworthy: yes" for a search that walked nothing.
        // A fallback wearing a success's clothes -- `CLAUDE.md` §4.
        let bars = synthetic::sessions(1);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
        let text = render_auto(&auto, None);

        assert!(!auto.affordable, "the fixture must keep no rung");
        assert_eq!(
            cell(&text, "outcome"),
            "NOTHING",
            "an absent ladder must not be reported as an extinct one"
        );
        assert!(
            !text.contains("went extinct"),
            "extinction is a measurement, and none was taken"
        );
        assert_eq!(
            cell(&text, "trustworthy as a whole answer"),
            "NO",
            "and the verdict must carry it"
        );
    }

    #[test]
    fn the_identity_is_rendered_when_there_is_one() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("swept");
        let id = identity(&Run {
            mask: ConditionMask::default(),
            direction: Direction::Undirected,
            instrument: &key,
            timeframe: "1min",
            params: Params::of(bounded()),
            data_digest: data_digest(&bars),
            commit: "0123456789abcdef",
            feed: "groww",
        });
        let text = render(&out, Some(&id));

        assert!(text.contains(&id.hex()), "the digest must be readable");
        assert!(!text.contains("NOT RECORDED"));
    }

    #[test]
    fn a_grid_that_ends_vacuous_then_refused_still_finds_the_band() {
        // THE CASE THE FIRST BISECTION WAS DEAD CODE ON.
        //
        // The grid probes `swept - 1` first, and that rung is ALWAYS vacuous:
        // D-0080 excludes `support == bars` as AlwaysTrue, so it completes with
        // depth 0 and never sets `best`. When the next rung down REFUSES, a
        // bracket keyed on `best` has `hi == lo` and the loop never runs --
        // `auto` then reports "nothing affordable" with the answer sitting
        // between the two probes it already took.
        //
        // Measured before the fix, this exact fixture: affordable=false,
        // min_hits=None, attempts=2, 0 combinations -- while t=592 completes
        // with 4,722 frequent sets at the same ceiling. Found by an adversarial
        // audit; the existing bracket test below opens with `.expect()` on
        // `min_hits`, so it can only ever exercise the path that already worked.
        let bars = synthetic::sessions(8);
        let tight = Ladder::with_min_hits(1).with_ceiling(5_000);
        let auto = Sweeper::new(tight).auto(&bars, &mut evaluator());

        assert!(
            auto.affordable,
            "a band exists at this ceiling and the search must reach it"
        );
        assert!(
            auto.attempts > 2,
            "attempts == 2 is the signature of the defect: two grid probes and \
             zero bisection probes. got {}",
            auto.attempts
        );
        assert!(
            auto.outcome.sweep.all_frequent().count() > 0,
            "and the kept sweep must actually contain combinations"
        );

        // A TIGHTER CEILING PUSHES THE BAND HIGHER, so the bisection's first
        // midpoint lands in the VACUOUS region above it. That probe completes
        // and finds nothing: it must narrow the bracket without being kept, and
        // it is the only way the `found == false` arm is ever reached -- the
        // wider fixture above takes the true arm on all 31 of its probes.
        let tighter = Ladder::with_min_hits(1).with_ceiling(200);
        let narrow = Sweeper::new(tighter).auto(&bars, &mut evaluator());
        assert!(narrow.affordable, "a band exists here too");
        let chosen = narrow.min_hits.unwrap_or(0);
        assert!(
            chosen > auto.min_hits.unwrap_or(0),
            "a tighter ceiling can only afford a HIGHER threshold: {chosen} vs {:?}",
            auto.min_hits
        );
    }

    #[test]
    fn the_search_closes_the_bracket_the_halving_grid_leaves_open() {
        // The halving grid can only ever bracket the answer to a factor of two:
        // it probes swept-1, /2, /4 ... and stops at the first refusal, so the
        // true crossover is somewhere in (refused_below, chosen). An audit
        // measured what that costs -- 6,631 combinations chosen where 51,778 were
        // affordable at the same budget, because the grid jumped 48,374 -> 24,187
        // and never tried 30,961.
        //
        // Bisection must close the bracket to a single threshold. Asserting the
        // WIDTH rather than a particular value is what makes this scale-free:
        // there is no fixture size to get wrong and no number to re-pin when the
        // column changes.
        let bars = synthetic::sessions(8);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());

        let chosen = auto.min_hits.expect("this fixture must find a threshold");
        let refused = auto
            .refused_below
            .expect("and must find an edge, or the bracket below is vacuous");
        assert!(
            chosen > refused,
            "the chosen threshold must sit above the refused one: {chosen} vs {refused}"
        );
        assert!(
            chosen.saturating_sub(refused) <= 1,
            "the bracket must be closed to one. chosen {chosen}, refused {refused} \
             -- a gap wider than one means bisection did not converge and the \
             search is still reporting a power-of-two grid point as the answer"
        );
    }

    #[test]
    fn the_search_is_reported_for_a_tuned_run() {
        let bars = synthetic::sessions(8);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
        let text = render_auto(&auto, None);

        assert!(text.contains("SEARCH"));
        assert!(text.contains("ladders walked"));
        assert!(text.contains("threshold chosen"));
    }

    #[test]
    fn a_cold_run_reports_no_threshold_and_says_why() {
        let bars = synthetic::sessions(1);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
        let text = render_auto(&auto, None);

        assert!(text.contains("NONE"), "no threshold is stated as NONE");
        assert!(
            text.contains("no rung measured anything"),
            "and the reason is given rather than left to be inferred"
        );
        assert!(text.contains("warming"), "the warm-up loss is visible");
    }

    #[test]
    fn an_empty_run_renders_without_dividing_by_zero() {
        let auto = Sweeper::new(bounded()).auto(&[], &mut evaluator());
        let text = render_auto(&auto, None);
        assert!(text.contains('-'), "an empty denominator renders as a dash");
        assert!(!text.is_empty());
    }
}

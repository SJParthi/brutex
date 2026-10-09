//! **`pool-oos`: the pool's discovery, judged on months it never saw.**
//!
//! # The gap this closes
//!
//! `pool` finds the combination that names a stock before it moves, and its
//! own header says what that finding is worth: every figure is in sample, and
//! the pooled table is the largest of `instruments × candidates` comparisons.
//! `CLAUDE.md` §1 says the same thing about the whole surface: an in-sample
//! result across 210 instruments means nothing until it is validated out of
//! sample. Nothing in this crate did that for the pool (audit-20261003
//! gaps-5).
//!
//! # What it does
//!
//! 1. **DISCOVERY, on the training months only.** Pass 1 and the union are
//!    `pool`'s own: every instrument on the surface is screened exactly as
//!    `range-rung` screens one, and the union of their frontier rows, each
//!    with the side it was priced at, is the candidate family. Nothing here
//!    reads a later bar.
//! 2. **THE SPLIT.** Every instrument's training span and later span are
//!    prepared through [`crate::pool::prepare_span`], the screen's own
//!    sequence. The later months must start strictly after the training
//!    months end; the arm refuses an overlap before any bar is read.
//! 3. **FROZEN EXITS.** Each candidate is walked with
//!    [`runner::trade::walk`] at the TRAINING span's holding period. The later
//!    prices choose nothing: no exit level, no horizon, no candidate.
//! 4. **THE CORRECTION.** Each candidate's pooled later return per IST
//!    session is one strategy in a Romano-Wolf stepdown
//!    ([`runner::bootstrap::romano_wolf_receipt`]) over the WHOLE union, at
//!    the same 5% family-wise error rate and stationary-bootstrap block the
//!    audit stack uses. A candidate HELD only when the stepdown rejects its
//!    null of a non-positive mean. White's Reality Check over the same family
//!    is printed beside it with the measured calibration for the number of
//!    later sessions, because a short later span is badly miscalibrated and
//!    `runner::bootstrap::Verdict::calibration` says by how much.
//!
//! # Why returns, not paisa
//!
//! The pooled series adds trades from a ₹20,000 index and a ₹100 share. In
//! paisa the index would decide every day's sign. Each trade's pessimistic
//! P&L ([`runner::trade::Trade::worst`]) is therefore taken as parts per
//! million of its entry bar's open before it is pooled, truncated toward zero
//! in integer arithmetic (§7). D-1576.
//!
//! # What it does not do
//!
//! * No cost of any kind is charged. The opening says so beside its totals,
//!   exactly as `pool`'s does, and a held equity candidate is gross research,
//!   never Selection V6 or execution authority (§1).
//! * Fills are on the signal rung's bars, as the audit stack's bootstrap
//!   family's are; on a rung above one minute that is coarser than the exit
//!   grid's minute replay. `docs/06-limits.md` records it.
//! * One later span is one draw. A held candidate is evidence about those
//!   months, not a promise about the next ones.
//!
//! # The handoff to qualification
//!
//! The held candidates are written, before the verb returns, as an explicit
//! program catalog at `CATALOG_OUT` — one AND of decimal bit ids per line, the
//! exact syntax `crate::boolean_catalog_command::catalog` reads — and read
//! back through that same reader before the page says it was written. So
//! `boolean-qualified-campaign-stored` and the other `CATALOG_FILE` verbs take
//! the discovery's output with no hand transcription (audit-20261003
//! gaps-11). The file is created new and never overwritten. D-1577.
//!
//! # Cost
//!
//! Discovery costs what `pool`'s pass 1 costs. The judging is
//! `instruments × candidates` walks per span, each O(bars), plus one
//! stepdown of O(draws × candidates × sessions). Neither is a §3 rule-4
//! operation; `docs/06-limits.md` records the bound.

use core::fmt::Write as _;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use crate::frontier::Direction;
use crate::pool::{Candidate, PreparedSpan};

/// A month range, first and last month inclusive, as `(year, month)`.
type Months = ((u16, u8), (u16, u8));

/// One instrument's two prepared spans. The training span's holding period is
/// the one both are walked at. The verb streams through [`walk_all`] instead;
/// this holds both spans and exists for the judge's fixtures.
#[cfg(test)]
pub(crate) struct Instrument<'a> {
    pub(crate) training: &'a PreparedSpan,
    pub(crate) later: &'a PreparedSpan,
}

/// One candidate's figures on one span, pooled across instruments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    /// Round trips taken.
    pub(crate) trades: u64,
    /// The sum of every trade's pessimistic return, in ppm of its entry open.
    pub(crate) sum_ppm: i128,
}

/// One candidate's verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Judgement {
    /// Index into the union.
    pub(crate) candidate: usize,
    pub(crate) training: Tally,
    pub(crate) later: Tally,
    /// The Romano-Wolf stepdown rejected this candidate's null on the later
    /// span: it HELD out of sample.
    pub(crate) held: bool,
}

/// The whole family's out-of-sample verdict.
#[derive(Debug)]
pub(crate) struct Judged {
    /// One row per union candidate: the held ones, then the failed ones, each
    /// in discovery (union) order.
    pub(crate) rows: Vec<Judgement>,
    pub(crate) training_sessions: usize,
    pub(crate) later_sessions: usize,
    pub(crate) draws: usize,
    /// White's Reality Check over the same later family.
    pub(crate) reality: Option<runner::bootstrap::Verdict>,
}

/// One span walked for every union candidate, with the span itself dropped.
///
/// This is what survives an instrument's preparation: its IST session days,
/// one tally per candidate, and -- on the later span only -- one booking per
/// trade. The bars and the condition column are gone once this exists, so a
/// pool over 210 instruments never holds more than one lane's spans at once
/// (sweep audit OS-1, D-2300).
/// `(candidate, exit day, ppm)` for one trade.
pub(crate) type Booking = (usize, i64, i64);

#[derive(Debug, Default)]
pub(crate) struct Walked {
    /// The span's IST session days, ascending.
    pub(crate) days: Vec<i64>,
    /// One tally per union candidate, in union order.
    pub(crate) tallies: Vec<Tally>,
    /// `(candidate, exit day, ppm)` per trade, in candidate then trade order.
    /// Empty when the span was walked for its tallies only.
    pub(crate) bookings: Vec<Booking>,
}

/// Walk every union candidate over one span at `horizon`.
///
/// Each trade's pessimistic P&L is taken as ppm of its entry open, truncated
/// toward zero (§7). The candidates are walked in parallel and gathered in
/// union order, so the result does not depend on the thread count. With
/// `book` false only the tallies are kept: the training span needs no series
/// (sweep audit OS-2, D-2300).
///
/// # Errors
///
/// A trade naming a bar outside its span, a non-positive entry open, or a
/// return that does not fit an `i64`. Nothing is clamped.
pub(crate) fn walk_span(
    span: &PreparedSpan,
    horizon: runner::outcome::Horizon,
    union: &[Candidate],
    book: bool,
) -> Result<Walked, String> {
    let bars = span.bars.as_slice();
    let per_candidate: Vec<Result<(Tally, Vec<Booking>), String>> = union
        .par_iter()
        .enumerate()
        .map(|(at, candidate)| {
            let mask = vocab::ConditionMask::from_words(candidate.words);
            let walked =
                runner::trade::walk(bars, &span.column, &mask, horizon, candidate.direction);
            let mut tally = Tally::default();
            let mut bookings = Vec::new();
            if book {
                bookings.reserve_exact(walked.trades.len());
            }
            for trade in &walked.trades {
                let (Some(entry), Some(exit)) =
                    (bars.get(trade.entry_bar), bars.get(trade.exit_bar))
                else {
                    return Err("a walked trade names a bar outside its span".to_owned());
                };
                if entry.open <= 0 {
                    return Err(format!(
                        "a trade entered at a non-positive open ({} paisa); its return is undefined and no series was built",
                        entry.open
                    ));
                }
                let ppm = i128::from(trade.worst)
                    .checked_mul(1_000_000)
                    .map(|scaled| scaled / i128::from(entry.open))
                    .and_then(|ppm| i64::try_from(ppm).ok())
                    .ok_or("a trade's return in ppm does not fit an i64")?;
                tally.trades = tally.trades.saturating_add(1);
                tally.sum_ppm = tally.sum_ppm.saturating_add(i128::from(ppm));
                if book {
                    bookings.push((at, indicators::ist_day(exit.ts_micros), ppm));
                }
            }
            Ok((tally, bookings))
        })
        .collect();
    let mut out = Walked {
        days: crate::session_index(bars),
        tallies: Vec::with_capacity(union.len()),
        bookings: Vec::new(),
    };
    for result in per_candidate {
        let (tally, mut bookings) = result?;
        out.tallies.push(tally);
        out.bookings.append(&mut bookings);
    }
    Ok(out)
}

/// Every candidate's tally summed over the walked spans, and the number of
/// distinct IST sessions they cover.
fn summed(walked: &[&Walked], candidates: usize) -> Result<(Vec<Tally>, Vec<i64>), String> {
    let mut tallies = vec![Tally::default(); candidates];
    let mut days = std::collections::BTreeSet::new();
    for span in walked {
        if span.tallies.len() != candidates {
            return Err(
                "a walked span carries a different candidate count than the union".to_owned(),
            );
        }
        for (sum, one) in tallies.iter_mut().zip(&span.tallies) {
            sum.trades = sum.trades.saturating_add(one.trades);
            sum.sum_ppm = sum.sum_ppm.saturating_add(one.sum_ppm);
        }
        days.extend(span.days.iter().copied());
    }
    Ok((tallies, days.into_iter().collect()))
}

/// The pooled later series: one row per candidate, one column per IST session
/// in `days`, each slot the sum of the trades that exited that day.
///
/// The matrix is reserved fallibly, so a family too large for memory is
/// refused by name rather than aborting the process (§4, sweep audit OS-2).
fn pooled_later(
    walked: &[&Walked],
    candidates: usize,
    days: &[i64],
) -> Result<Vec<Vec<i64>>, String> {
    let index: HashMap<i64, usize> = days
        .iter()
        .enumerate()
        .map(|(slot, day)| (*day, slot))
        .collect();
    let mut series: Vec<Vec<i64>> = Vec::new();
    series.try_reserve_exact(candidates).map_err(|why| {
        format!("the later series of {candidates} candidate(s) cannot be held: {why}; nothing was judged")
    })?;
    for _ in 0..candidates {
        let mut row = Vec::new();
        row.try_reserve_exact(days.len()).map_err(|why| {
            format!(
                "the later series of {candidates} candidate(s) x {} session(s) cannot be held: {why}; nothing was judged",
                days.len()
            )
        })?;
        row.resize(days.len(), 0_i64);
        series.push(row);
    }
    for span in walked {
        for &(candidate, day, ppm) in &span.bookings {
            let slot = index
                .get(&day)
                .and_then(|slot| series.get_mut(candidate)?.get_mut(*slot))
                .ok_or("a trade exited on a day outside the session index")?;
            *slot = slot
                .checked_add(ppm)
                .ok_or("a pooled session return overflowed i64; no series was clamped")?;
        }
    }
    Ok(series)
}

/// Judge every union candidate on the later spans, with exits frozen at each
/// instrument's training holding period, under one Romano-Wolf stepdown.
///
/// The spans are walked here; [`judge_walked`] is the same judgement over
/// spans already walked, which is how the verb streams its instruments.
///
/// # Errors
///
/// When a series cannot be built without clamping, or when the stepdown has
/// no complete receipt for this family. No verdict is invented for either.
#[cfg(test)]
pub(crate) fn judge(instruments: &[Instrument<'_>], union: &[Candidate]) -> Result<Judged, String> {
    if instruments.is_empty() || union.is_empty() {
        return Err("no instrument or no candidate to judge out of sample".to_owned());
    }
    let mut walked = Vec::with_capacity(instruments.len());
    for instrument in instruments {
        // FROZEN: the later span is walked at the TRAINING horizon.
        let horizon = instrument.training.horizon;
        walked.push((
            walk_span(instrument.training, horizon, union, false)?,
            walk_span(instrument.later, horizon, union, true)?,
        ));
    }
    judge_walked(&walked, union)
}

/// [`judge`] over instruments whose spans were already walked by
/// [`walk_span`]: training without bookings, later with them.
///
/// # Errors
///
/// As [`judge`].
pub(crate) fn judge_walked(
    walked: &[(Walked, Walked)],
    union: &[Candidate],
) -> Result<Judged, String> {
    if walked.is_empty() || union.is_empty() {
        return Err("no instrument or no candidate to judge out of sample".to_owned());
    }
    let training: Vec<&Walked> = walked.iter().map(|(training, _)| training).collect();
    let later: Vec<&Walked> = walked.iter().map(|(_, later)| later).collect();
    let (training_tallies, training_days) = summed(&training, union.len())?;
    let (later_tallies, later_days) = summed(&later, union.len())?;
    let training_sessions = training_days.len();
    let later_sessions = later_days.len();
    if later_sessions == 0 {
        return Err(
            "the later span holds no session; nothing can be judged out of sample".to_owned(),
        );
    }
    let later_series = pooled_later(&later, union.len(), &later_days)?;
    let draws = crate::bootstrap_draws(later_sessions);
    let receipt = runner::bootstrap::romano_wolf_receipt(
        &later_series,
        draws,
        crate::BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
        crate::BOOTSTRAP_ALPHA_PPM,
    )
    .ok_or("the Romano-Wolf stepdown has no complete receipt for this family; no candidate is called held or failed")?;
    let reality = runner::bootstrap::reality_check(
        &later_series,
        draws,
        crate::BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
    );
    let mut held = Vec::with_capacity(union.len());
    let mut failed = Vec::with_capacity(union.len());
    for (candidate, (training, later)) in
        training_tallies.into_iter().zip(later_tallies).enumerate()
    {
        let row = Judgement {
            candidate,
            training,
            later,
            held: receipt
                .is_rejected(candidate)
                .ok_or("the stepdown receipt omits a candidate of its own family")?,
        };
        if row.held {
            held.push(row);
        } else {
            failed.push(row);
        }
    }
    // HELD FIRST, each half in discovery order. No ranking by the later
    // figures: the stepdown already decided which ones held.
    held.append(&mut failed);
    let rows = held;
    Ok(Judged {
        rows,
        training_sessions,
        later_sessions,
        draws,
        reality,
    })
}

/// The mean of a tally per session, in ppm, truncated toward zero; zero over
/// no session.
fn per_session(tally: Tally, sessions: usize) -> i128 {
    i128::try_from(sessions)
        .ok()
        .filter(|n| *n > 0)
        .map_or(0, |n| tally.sum_ppm / n)
}

/// The decimal bit ids a mask requires, ascending.
fn bits_of(words: [u64; 6]) -> Vec<u32> {
    let mut bits = Vec::new();
    for (w, word) in (0_u32..).zip(words) {
        for b in 0..64_u32 {
            if (word >> b) & 1 == 1 {
                bits.push(w * 64 + b);
            }
        }
    }
    bits
}

/// One catalog line: the AND of a mask's bits, as `Expression::parse` reads it.
fn program_line(words: [u64; 6]) -> String {
    bits_of(words)
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(" & ")
}

/// Write the held candidates as an explicit program catalog at `path` and read
/// it back through the reader every `CATALOG_FILE` verb uses.
///
/// One line per distinct mask, in the order given; a mask held on both sides
/// is one program, because a catalog program is priced on both sides by the
/// verbs that read it. The file is created new: an existing path refuses and
/// is not touched. Returns the number of programs written.
///
/// # Errors
///
/// A mask with no bit, a line the parser refuses (a retired bit, a capacity
/// breach), a catalog over the readers' byte bound, a path that exists, an
/// I/O failure, or a read-back that disagrees with what was written. Every one
/// names its reason; none writes a partial catalog it then calls complete.
pub(crate) fn write_catalog(
    path: &Path,
    held: &[Candidate],
    heading: &str,
) -> Result<usize, String> {
    let mut seen = HashSet::with_capacity(held.len());
    let mut text = String::new();
    for line in heading.lines() {
        let _ = writeln!(text, "# {line}");
    }
    let mut programs = Vec::new();
    for candidate in held {
        if !seen.insert(candidate.words) {
            continue;
        }
        let line = program_line(candidate.words);
        if line.is_empty() {
            return Err("a held candidate requires no condition; it is not a program".to_owned());
        }
        let program = vocab::expression::Expression::parse(&line)
            .map_err(|why| format!("held mask `{line}` is not a catalog program: {why:?}"))?;
        let _ = writeln!(text, "{line}");
        programs.push(program.encode());
    }
    if programs.is_empty() {
        return Err(
            "no held candidate to write; an empty catalog is refused by every reader".to_owned(),
        );
    }
    let bound = crate::boolean_catalog_command::CATALOG_BYTES;
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > bound {
        return Err(format!(
            "the held catalog is {} bytes, over the {bound}-byte bound every CATALOG_FILE reader \
             enforces; nothing was written",
            text.len()
        ));
    }
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|why| format!("cannot create catalog {}: {why}", path.display()))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|why| format!("catalog {} was not written whole: {why}", path.display()))?;
    }
    let read = crate::boolean_catalog_command::catalog(path, bound)
        .map_err(|why| format!("catalog {} does not read back: {why}", path.display()))?;
    let back: Vec<_> = read
        .iter()
        .map(vocab::expression::Expression::encode)
        .collect();
    if back != programs {
        return Err(format!(
            "catalog {} reads back as different programs than were written",
            path.display()
        ));
    }
    Ok(programs.len())
}

/// The verb, with every refusal rendered the way the CLI prints one.
#[must_use]
pub fn pool_oos(
    vendor_word: &str,
    rung: &'static str,
    training: Months,
    later: Months,
    support_ppm: Option<u64>,
    catalog_out: &Path,
) -> String {
    match run(vendor_word, rung, training, later, support_ppm, catalog_out) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

fn run(
    vendor_word: &str,
    rung: &'static str,
    training: Months,
    later: Months,
    support_ppm: Option<u64>,
    catalog_out: &Path,
) -> Result<String, String> {
    if later.0 <= training.1 || later.0 > later.1 || training.0 > training.1 {
        return Err(
            "the later months must be ordered and start strictly after the training months end; \
             nothing was read"
                .to_owned(),
        );
    }
    crate::commit_stamp().ok_or_else(|| {
        "this build carries no verified commit stamp, so §3 rule 3's run identity cannot be \
         recorded and no instrument will be screened"
            .to_owned()
    })?;
    let vendor = crate::parse_vendor(vendor_word)?;
    crate::swept_rung(rung)?;
    let root = crate::store_root()?;
    run_under(
        &root,
        vendor,
        vendor_word,
        rung,
        (training, later),
        support_ppm,
        catalog_out,
    )
}

fn run_under(
    root: &Path,
    vendor: brutex_core::vendor::Vendor,
    vendor_word: &str,
    rung: &'static str,
    (training, later): (Months, Months),
    support_ppm: Option<u64>,
    catalog_out: &Path,
) -> Result<String, String> {
    // BEFORE ANY BAR IS READ: a catalog path that exists would be refused at
    // the end, after hours of screening.
    if std::fs::symlink_metadata(catalog_out).is_ok() {
        return Err(format!(
            "CATALOG_OUT {} already exists; a catalog is created new and never overwritten",
            catalog_out.display()
        ));
    }
    let (mut out, surface, unread) =
        crate::pool::head_under(root, vendor_word, rung, training.0, training.1, support_ppm)?;
    let _ = writeln!(
        out,
        "\nOUT OF SAMPLE. Discovery reads {}-{:02}..{}-{:02} only. The union is then judged on \
         {}-{:02}..{}-{:02}, months it never saw,\nwith exits frozen at the training holding \
         period, under one Romano-Wolf stepdown at a 5% family-wise error rate.",
        training.0.0,
        training.0.1,
        training.1.0,
        training.1.1,
        later.0.0,
        later.0.1,
        later.1.0,
        later.1.1
    );
    if surface.is_empty() {
        return Ok(out);
    }
    // PASS 1 IS `pool`'s, CALLED, NOT COPIED: one instrument at a time in
    // surface order. This was a rayon parallel map around `one_rung` (G1-1,
    // D-4700), so every screen's durable rows landed in thread-completion
    // order and each concurrent sweep took the whole machine.
    let screened = crate::pool::screen_pass_one(
        root,
        crate::commit_stamp(),
        vendor_word,
        &surface,
        rung,
        training,
        support_ppm,
    );
    crate::pool::at_least_one_screened(&screened, &unread)?;
    crate::pool::render_per_symbol(&mut out, &screened);
    let (union, unread) = crate::pool::union_of(root, &screened);
    for (symbol, why) in &unread {
        let _ = writeln!(out, "  FRONTIER ROWS NOT READ for {symbol}: {why}");
    }
    if union.is_empty() {
        let _ = writeln!(
            out,
            "\n  OUT OF SAMPLE: nothing to judge. No screened instrument left a frontier row. \
             No catalog was written."
        );
        return Ok(out);
    }
    let streamed = walk_all(root, vendor, rung, &surface, (training, later), &union);
    let mut instruments = Vec::new();
    for (symbol, walked) in surface.iter().zip(streamed) {
        match walked {
            Ok(both) => instruments.push(both),
            Err(why) => {
                let _ = writeln!(out, "  NOT IN THE OUT-OF-SAMPLE POOL: {symbol}: {why}");
            }
        }
    }
    let judged = judge_walked(&instruments, &union)?;
    render(&mut out, &union, &judged, instruments.len());
    let heading = format!(
        "brutex pool-oos held candidates: feed {vendor_word}, rung {rung}, training \
         {}-{:02}..{}-{:02}, later {}-{:02}..{}-{:02}.\nGross of every charge; research, \
         not Selection V6 or execution authority.",
        training.0.0,
        training.0.1,
        training.1.0,
        training.1.1,
        later.0.0,
        later.0.1,
        later.1.0,
        later.1.1
    );
    hand_off(&mut out, &union, &judged, catalog_out, &heading, later.1);
    Ok(out)
}

/// Every surface instrument's training and later spans, prepared through the
/// screen's own sequence and walked for every union candidate, in surface
/// order. A refusal names its span.
///
/// STREAMED: each lane prepares one span, walks it, and drops the bars and
/// the condition column before preparing the next, so peak memory is one
/// span per running lane plus the walked tallies and bookings, not every
/// instrument's two spans at once (sweep audit OS-1, D-2300). The later span
/// is walked at the TRAINING span's holding period: the later prices choose
/// nothing.
fn walk_all(
    root: &Path,
    vendor: brutex_core::vendor::Vendor,
    rung: &'static str,
    surface: &[String],
    (training, later): (Months, Months),
    union: &[Candidate],
) -> Vec<Result<(Walked, Walked), String>> {
    surface
        .par_iter()
        .map(|symbol| {
            let (horizon, training) = {
                let span =
                    crate::pool::prepare_span(root, vendor, symbol, rung, training.0, training.1)
                        .map_err(|why| format!("training span: {why}"))?;
                (span.horizon, walk_span(&span, span.horizon, union, false)?)
            };
            let later = {
                let span = crate::pool::prepare_span(root, vendor, symbol, rung, later.0, later.1)
                    .map_err(|why| format!("later span: {why}"))?;
                walk_span(&span, horizon, union, true)?
            };
            Ok((training, later))
        })
        .collect()
}

/// The held candidates, written to `catalog_out` as the qualification verbs'
/// `CATALOG_FILE`, and the page's line saying so, or saying why not.
fn hand_off(
    out: &mut String,
    union: &[Candidate],
    judged: &Judged,
    catalog_out: &Path,
    heading: &str,
    later_end: (u16, u8),
) {
    let held: Vec<Candidate> = judged
        .rows
        .iter()
        .filter(|row| row.held)
        .filter_map(|row| union.get(row.candidate).copied())
        .collect();
    if held.is_empty() {
        let _ = writeln!(
            out,
            "\n  NO CATALOG WRITTEN: no candidate held out of sample, so there is nothing to qualify."
        );
        return;
    }
    match write_catalog(catalog_out, &held, heading) {
        Ok(programs) => {
            let _ = writeln!(
                out,
                "\n  CATALOG WRITTEN: {programs} program(s) at {}. It is the CATALOG_FILE of \
                 boolean-qualified-campaign-stored as it stands;\n  qualify it on months after \
                 {}-{:02}, which this page has already used.",
                catalog_out.display(),
                later_end.0,
                later_end.1
            );
        }
        Err(why) => {
            let _ = writeln!(out, "\nrefused: the held catalog was not written: {why}");
        }
    }
}

/// The out-of-sample table, under the family's own statistics.
fn render(out: &mut String, union: &[Candidate], judged: &Judged, instruments: usize) {
    use crate::columns::{left, right};
    let held = judged.rows.iter().filter(|row| row.held).count();
    let _ = writeln!(
        out,
        "\n  OUT OF SAMPLE -- {} candidate(s) pooled across {instruments} instrument(s); {} \
         training and {} later IST session(s); {} bootstrap draws.\n  {held} HELD under \
         Romano-Wolf at 5% FWER. Returns are each trade's pessimistic P&L in ppm of its entry \
         open, GROSS of every charge.",
        union.len(),
        judged.training_sessions,
        judged.later_sessions,
        judged.draws
    );
    match judged.reality {
        Some(verdict) => {
            let _ = writeln!(
                out,
                "  White's Reality Check over the same family: p = {:.4} ({}).",
                verdict.p_value,
                verdict.calibration()
            );
        }
        None => {
            let _ = writeln!(
                out,
                "  White's Reality Check: NOT COMPUTED for this family."
            );
        }
    }
    let columns = [
        right(4),
        left(9).after(1),
        left(5),
        right(8),
        right(14),
        right(8),
        right(14),
    ];
    let header = [
        "rank",
        "verdict",
        "side",
        "is_trd",
        "is_ppm/sess",
        "oos_trd",
        "oos_ppm/sess",
    ];
    let shown: Vec<&Judgement> = judged
        .rows
        .iter()
        .take(held.max(crate::Rules::operator().top.max(1)))
        .collect();
    let cells: Vec<Vec<String>> = shown
        .iter()
        .enumerate()
        .map(|(rank, row)| {
            vec![
                (rank + 1).to_string(),
                if row.held { "HELD" } else { "FAILED" }.to_owned(),
                match union.get(row.candidate).map(|c| c.direction) {
                    Some(Direction::Long) => "long",
                    Some(Direction::Short) => "short",
                    None => "?",
                }
                .to_owned(),
                row.training.trades.to_string(),
                per_session(row.training, judged.training_sessions).to_string(),
                row.later.trades.to_string(),
                per_session(row.later, judged.later_sessions).to_string(),
            ]
        })
        .collect();
    let laid = crate::columns::with_header(&columns, &header, cells);
    let _ = writeln!(out, "  {}  mask", laid.header);
    for (row, line) in shown.iter().zip(&laid.rows) {
        let mask = union.get(row.candidate).map_or_else(
            || "missing candidate".to_owned(),
            |c| crate::pool::mask_hex(c.words),
        );
        let _ = writeln!(out, "  {line}  {mask}");
    }
    let hidden = judged.rows.len().saturating_sub(shown.len());
    if hidden > 0 {
        let _ = writeln!(
            out,
            "  ... and {hidden} more FAILED candidate(s), in discovery order, not shown."
        );
    }
    let _ = writeln!(
        out,
        "  A HELD row is evidence about these later months only. It is gross research: no equity \
         result enters Selection V6 or execution authority (CLAUDE.md §1)."
    );
}

#[cfg(test)]
#[path = "pool_oos_tests.rs"]
mod tests;

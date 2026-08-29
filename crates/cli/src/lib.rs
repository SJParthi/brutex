//! The sweep, driven from a command line.
//!
//! # What this closes
//!
//! Until this crate existed, nothing that could be RUN reached the sweep. The
//! only binary in the workspace was `api`, and its dependency set names neither
//! [`runner`] nor anything beneath it. So the Apriori ladder, the 280-position
//! vocabulary, the exit grid, the walk-forward and the significance bar were
//! compiled, tested, benchmarked — and unreachable from any entry point. The
//! three render surfaces that display them had no caller at all.
//!
//! # Every bar this crate sweeps is GENERATED, and it says so in the output
//!
//! The operator's standing rule is that no vendor pull may originate here and
//! that the bars already on disk may not be used either. So the only honest
//! input is [`runner::synthetic`], and this crate takes no other. That is a
//! real limit, not a placeholder: **a report rendered from generated bars is
//! not a backtest**, and a reader who mistook one for the other would be making
//! exactly the error `CLAUDE.md` §4 bans a program from inviting.
//!
//! It is therefore stated in the rendered output itself, not only here — see
//! [`PROVENANCE`]. A banner in a doc comment protects nobody reading a
//! terminal.
//!
//! # What a caller must decide
//!
//! `min_hits` and the session count. Neither has a default in [`runner`],
//! because `CLAUDE.md` §3 rule 1 will not let that crate invent one, and this
//! crate does not invent one either — it requires them on the command line and
//! refuses without them. The vocabulary's own [`Widths`], [`Availability`] and
//! [`Thresholds`] are the pinned set, which is the only configuration this
//! build ships.
//!
//! # Cost
//!
//! This crate makes no cost claim of its own. It generates bars, hands them to
//! [`runner::Sweeper`] and renders the result; the bounds are `runner`'s and
//! `engine`'s, measured by their own benches.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Reading real bars out of the store — the join `CLAUDE.md` §5 calls the live
/// gap. `crates/api` declared `store` and no `runner`; this crate declared
/// `runner` and no `store`, so nothing in the workspace connected a pulled bar
/// to a ranked result.
pub mod batch;
pub mod frontier;
pub mod knobs;
pub mod results;
pub mod stability;
pub mod stored;
/// Every round trip a recorded run took, keyed by that run's identity.
///
/// The backtest page has drawn a per-trade table since it was written and every
/// cell of it is a padlock, because nothing ever wrote the file it reads. See
/// the module for why the store that existed in `crates/api` could not be that
/// writer.
pub mod trades;

use brutex_core::vendor::Vendor;
use costs::fill::Direction;
use engine::Ladder;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use rayon::prelude::*;
use runner::excursion::Side;
use runner::identity::{Direction as RunDirection, Params, Run, data_digest, identity};
use runner::outcome::Horizon;
use runner::{Sweeper, audit, closed, grid, synthetic, trade};
use std::fmt::Write as _;

/// Everything went as asked.
pub const OK: u8 = 0;
/// It was asked for something reasonable and could not do it.
pub const FAILED: u8 = 1;
/// It was asked for something it does not understand.
pub const MISUSED: u8 = 2;

/// The sentence every rendered report carries above it.
///
/// # Why this is a constant and not a comment
///
/// A report from this crate is byte-identical in shape to a report from real
/// bars, and the difference is the only thing that decides whether a number in
/// it means anything. `CLAUDE.md` §4 bans a fallback that hides a failure;
/// rendering a complete, confident-looking sweep over invented data without
/// saying so is that failure with better typography.
///
/// It is asserted by `the_report_always_declares_its_bars_are_generated`, so it
/// cannot be dropped by an edit that only touches the rendering.
pub const PROVENANCE: &str = "\
=== THESE BARS ARE GENERATED, NOT MARKET DATA ===
Produced by runner::synthetic in this process. Nothing was pulled from a vendor
and nothing was read from the store. Every figure below describes the generator,
NOT any instrument. This proves the pipeline runs end to end.
It is not a backtest, and no result in it is evidence about any market.
";

/// Usage, printed on every refusal so the reader never has to guess.
pub const USAGE: &str = "\
usage: cli sweep    SESSIONS MIN_HITS   walk the ladder at one threshold
       cli auto     SESSIONS            let the search choose the threshold
       cli auto-stored VENDOR UNDERLYING RUNG FROM_Y FROM_M TO_Y TO_M
                                   the same search, over REAL stored bars. There
                                   is no threshold argument: it bisects for the
                                   deepest one this machine can finish on this
                                   span, and prints what it settled on. Use it
                                   BEFORE sweep-stored or range-all rather than
                                   guessing a number and watching it refuse.
       cli audit    SESSIONS MIN_HITS   sweep, then trade the best combination
       cli sweep-stored VENDOR UNDERLYING RUNG YEAR MONTH MIN_HITS
                                   sweep REAL bars read from the store
       cli audit-stored VENDOR UNDERLYING RUNG YEAR MONTH MIN_HITS
                                   sweep REAL bars, then trade them: exit grid,
                                   walk-forward, PBO and bootstrap p-values
       cli verify       VENDOR UNDERLYING
                                   prove the engine correct on YOUR data: joins,
                                   determinism, no look-ahead, ledger round trip
                                   and the refusal surface. Exits non-zero on any
                                   failure. Nothing is pulled, nothing is written
                                   to the bar store.
       cli top          [VENDOR UNDERLYING]
                                   the ranked TOP COMBINATIONS of the best
                                   complete recorded run, read back from the
                                   store with their condition names, mean
                                   forward move, t and payoff. This is the list
                                   the ledger could not hold before: it stored
                                   one combination per run and folded the rest
                                   into two counters.
       cli results      [VENDOR UNDERLYING]
                                   list every recorded run, newest first, and
                                   name the best COMPLETE one
       cli screen       VENDOR UNDERLYING RUNG FROM_Y FROM_M TO_Y TO_M
                        SUPPORT_PPM MAX_POINTS MIN_RR TOP
                                   sweep, then report ONLY the combinations that
                                   satisfy YOUR rules. MAX_POINTS is the stop in
                                   index points -- no single trade may run more
                                   than that against entry. MIN_RR is hundredths:
                                   200 demands the SMALLEST win be twice the
                                   LARGEST loss, 0 drops the rule. TOP is how many
                                   to print.
       cli elite        VENDOR UNDERLYING RUNG FROM_Y FROM_M TO_Y TO_M
                        MAX_POINTS TOP
                                   THE RARE-WINNER HUNT, and it takes NO support
                                   threshold. Whatever number you type for that,
                                   you have already decided how often the answer
                                   may fire before anything is measured -- type it
                                   high and a rare setup is pruned at level one,
                                   type it low and the frontier explodes. There is
                                   no correct value, which is section 6's own
                                   argument against a depth parameter.
                                   So it WALKS the threshold: halving down from a
                                   cheap ceiling toward the floor one trade a week
                                   implies, stopping at the first support that
                                   admits a row. Cheap answers arrive first.
                                   Every rule is on: 80%% of trades won AND 80%% on
                                   the 95%% lower bound, the smallest win at least
                                   3x the largest loss, total profit at least 5x
                                   the worst peak-to-trough fall, and the weakest
                                   calendar grain still half positive. There is NO
                                   trade floor -- the assurance bound refuses a
                                   sample too thin, which is what lets a rare
                                   40-trade setup through.
                                   If NOTHING passes at any support, it prints the
                                   most permissive step it walked, so the answer is
                                   what IS there rather than a blank page.
                                   MAX_POINTS is still yours: it is the one number
                                   only you can mean.
       cli descend      VENDOR UNDERLYING RUNG FROM_Y FROM_M TO_Y TO_M
                        CEILING_PPM PER_WEEK
                                   sweep ONE rung at successively LOWER supports,
                                   from CEILING_PPM down to the floor PER_WEEK
                                   trades a week implies on that rung's own bar
                                   count. A rare setup is pruned by a high
                                   support before it is ever priced, so a fixed
                                   threshold cannot find one -- this walks it.
       cli range-all    VENDOR UNDERLYING FROM_Y FROM_M TO_Y TO_M SUPPORT_PPM
                                   sweep the span on ALL EIGHT INTRADAY RUNGS and
                                   table comparing them. SUPPORT_PPM is parts per
                                   million -- 200000 is 20% -- and each rung's
                                   min_hits comes from its OWN bar count, because
                                   81 months holds 1,671 daily bars and 623,546
                                   one-minute ones. Every rung is recorded.
       cli audit-range  VENDOR UNDERLYING RUNG FROM_Y FROM_M TO_Y TO_M MIN_HITS
                                   sweep a CONTIGUOUS SPAN of months as ONE
                                   series -- the seven-year question, not twelve
                                   monthly ones. Missing months are named, never
                                   skipped quietly.
       cli sweep-all    VENDOR RUNG MIN_HITS
                                   sweep EVERY stored instrument-month at that
                                   feed and rung, one report for all of them

SESSIONS  how many generated trading days to sweep, 1..=3650
MIN_HITS  bars a combination must fire on to be kept, 1 or more
VENDOR    the feed that wrote them -- groww, dhan, truedata, gdfl, zerodha
UNDERLYING  the index, e.g. NIFTY or BANKNIFTY
RUNG      the bar length as its directory word -- 1min, 1day

The two stored commands read a run identity off the build, so they refuse
unless it was stamped:
    BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli
";

/// The `verify` arm, lifted out of [`run`] for the reason [`audit_range_arm`]
/// gives.
///
/// Exits [`FAILED`] and not [`MISUSED`] on a failing check: the ARGUMENTS were
/// understood and the ENGINE did not hold, which are different facts and a
/// script distinguishing them is the point of having two codes.
fn verify_arm(out: &mut String, feed: &str, underlying: &str) -> u8 {
    let report = verify(feed, underlying);
    let failed = report.contains("FAIL");
    out.push_str(&report);
    if failed { FAILED } else { OK }
}
/// The `results` arm, lifted out of [`run`] for the reason [`audit_range_arm`]
/// gives.
///
/// # Two shapes and not three
///
/// `results zerodha` — a feed with no instrument — is deliberately NOT a shape.
/// It would have to either list everything, which reads as a filter that
/// silently did nothing, or filter on the feed alone, which is a third rule an
/// operator has to learn. It falls through to the usage refusal instead.
///
/// Listing is never `MISUSED`: an empty ledger is an ordinary state on a fresh
/// store, not an operator error, and returning a failure code for it would make
/// a first run look broken.
///
/// # But an UNREADABLE ledger is `FAILED`, and it used to be `OK`
///
/// Measured across six hostile stores: a ledger whose bytes are not this format,
/// whose version this build does not write, whose tail is a part-record, and one
/// whose path is a DIRECTORY all printed a clear refusal and then exited **0**.
/// A bad ARGUMENT exited 2 the whole time, so the two halves of the same command
/// disagreed about whether a refusal is a failure.
///
/// The message was never the problem — an operator reading the terminal saw it.
/// A script did not: `cli results && deploy` ran the second half after the
/// ledger refused to open, which is the failure wearing a success's clothes that
/// §4 bans. `FAILED` and not `MISUSED` because the arguments were fine; what
/// could not be done was the work.
/// The `auto-stored` arm, lifted out of [`run`] for the reason [`audit_range_arm`]
/// gives.
///
/// Parses the four date parts and nothing else: there is no threshold argument,
/// which is the whole point of the command.
fn auto_stored_arm(
    out: &mut String,
    vendor: &str,
    underlying: &str,
    rung: &str,
    dates: (&str, &str, &str, &str),
) -> u8 {
    let (from_y, from_m, to_y, to_m) = dates;
    let parsed = (
        from_y.parse::<u16>(),
        from_m.parse::<u8>(),
        to_y.parse::<u16>(),
        to_m.parse::<u8>(),
    );
    let (Ok(fy), Ok(fm), Ok(ty), Ok(tm)) = parsed else {
        out.push_str("refused: FROM_Y FROM_M TO_Y TO_M must all be whole numbers\n");
        return MISUSED;
    };
    let text = auto_stored(vendor, underlying, rung, (fy, fm), (ty, tm));
    let refused = text.starts_with("refused:");
    out.push_str(&text);
    if refused { MISUSED } else { OK }
}

/// The `top` arm, beside [`results_arm`] because it reads the same store.
///
/// Lifted out of [`run`] for the reason every other arm was: the dispatch is a
/// command LIST, and an inline body makes the list harder to read as one.
fn top_arm(out: &mut String, filter: Option<(&str, &str)>) -> u8 {
    let (feed, underlying) = match filter {
        None => (None, None),
        Some((feed, underlying)) => (Some(feed), Some(underlying)),
    };
    let listing = top_list(feed, underlying);
    // The refusal is read back off the rendered page for the reason
    // `results_arm` gives: a second refusal path can disagree with the one an
    // operator actually sees.
    let refused = listing.contains("refused:");
    out.push_str(&listing);
    if refused { FAILED } else { OK }
}

fn results_arm(out: &mut String, filter: Option<(&str, &str)>) -> u8 {
    let (feed, underlying) = match filter {
        None => (None, None),
        Some((feed, underlying)) => (Some(feed), Some(underlying)),
    };
    let listing = results_list(feed, underlying);
    // `results_list` renders its own refusal into the page rather than returning
    // one, so the code is read back off the rendered text. Ugly, and deliberately
    // so: the alternative is a second refusal path that can disagree with the
    // one an operator actually sees.
    let refused = listing.contains("refused:");
    out.push_str(&listing);
    if refused { FAILED } else { OK }
}

/// The `screen` arm, lifted out of [`run`] for the reason [`audit_range_arm`]
/// gives.
///
/// # The stop is given in POINTS and converted against the instrument
///
/// An operator says "twenty points", not "eight hundred parts per million". The
/// conversion needs a price, and the honest one is the instrument's own — a rule
/// stated in points on a 25,000 index and applied unchanged to a 52,000 one
/// would be a different rule. `NIFTY_REFERENCE` is a stated approximation and
/// is named as one on the page.
/// The `elite` arm — [`Rules::elite`] over a stored span.
///
/// # Why a command and not a flag
///
/// The operator's requirement is six numbers, and `screen` takes three. Typing
/// the other three every run is how a policy drifts between runs and stops being
/// comparable; leaving them off is how `screen` came to have four of its six
/// rules switched to zero. A NAMED profile is neither: the numbers live in one
/// documented place, the banner prints them, and two runs a month apart applied
/// the same policy because they named the same one.
///
/// It takes the stop in POINTS, exactly as `screen` does, because that is the
/// one number that is genuinely per-operator and per-instrument — a five-point
/// stop is a different intention on NIFTY than on BANKNIFTY, and only the person
/// running it knows which they mean.
fn elite_arm(
    out: &mut String,
    vendor: &str,
    underlying: &str,
    rung: &str,
    span: (&str, &str, &str, &str),
    limits: (&str, &str),
) -> u8 {
    let (max_points, top) = limits;
    let (from_y, from_m, to_y, to_m) = span;
    let numbers = (
        from_y.parse::<u16>(),
        from_m.parse::<u8>(),
        to_y.parse::<u16>(),
        to_m.parse::<u8>(),
    );
    let rules = (max_points.parse::<i64>(), top.parse::<usize>());
    match (numbers, rules) {
        ((Ok(fy), Ok(fm), Ok(ty), Ok(tm)), (Ok(pts), Ok(n))) => {
            if pts <= 0 {
                return refuse(
                    out,
                    "MAX_POINTS must be a whole number of index points, 1 or more",
                );
            }
            if n == 0 {
                return refuse(out, "TOP must be 1 or more");
            }
            // NO SUPPORT ARGUMENT. `elite_descend` walks the threshold from a
            // cheap ceiling down to the floor one trade a week implies, and
            // stops at the first support that admits a row. See its doc for why
            // a typed threshold cannot find a rare setup at any value.
            // POINTS ALL THE WAY IN, converted against THESE bars.
            //
            // This read `points_to_ppm(pts)`, which converts against
            // `NIFTY_REFERENCE` — a 25,000 constant — whatever instrument the
            // operator named. `elite_descend_in_points`'s own doc states the
            // cost: *"a NIFTY constant applied to BANKNIFTY does not
            // approximate the operator's rule, it doubles it"*, and understates
            // it on a 2020 low. So `elite NIFTY 20` and `elite BANKNIFTY 20`
            // asked for different stops while printing the same number.
            //
            // The correctly-converting variant already existed and
            // `crates/api/src/sweeprun.rs` already calls it — this argument
            // parser was the one caller left on the constant. It reads the
            // reference off the span it is about to sweep, so nothing here
            // needs to know a price.
            let text =
                elite_descend_in_points(vendor, underlying, rung, (fy, fm), (ty, tm), pts, n);
            let refused = text.starts_with("refused");
            out.push_str(&text);
            if refused { MISUSED } else { OK }
        }
        ((Err(_), _, _, _) | (_, _, Err(_), _), _) => {
            refuse(out, "FROM_YEAR and TO_YEAR must be whole years")
        }
        ((_, Err(_), _, _) | (_, _, _, Err(_)), _) => {
            refuse(out, "FROM_MONTH and TO_MONTH must be 1 to 12")
        }
        (_, (Err(_), _)) => refuse(
            out,
            "MAX_POINTS must be a whole number of index points, 1 or more",
        ),
        (_, (_, Err(_))) => refuse(out, "TOP must be a whole number, 1 or more"),
    }
}

fn screen_arm(
    out: &mut String,
    vendor: &str,
    underlying: &str,
    rung: &str,
    // The span as one tuple and the four rule words as another: the arm above
    // is a positional list and `clippy::too_many_arguments` is right that ten of
    // them is unreadable.
    span: (&str, &str, &str, &str),
    // Four rule words as one tuple: the arm above is a positional list and
    // `clippy::too_many_arguments` is right that ten of them is unreadable.
    limits: (&str, &str, &str, &str),
) -> u8 {
    let (support, max_points, min_rr, top) = limits;
    let (from_y, from_m, to_y, to_m) = span;
    let numbers = (
        from_y.parse::<u16>(),
        from_m.parse::<u8>(),
        to_y.parse::<u16>(),
        to_m.parse::<u8>(),
        parse_support_ppm(support),
    );
    let rules = (
        max_points.parse::<i64>(),
        min_rr.parse::<i64>(),
        top.parse::<usize>(),
    );
    match (numbers, rules) {
        ((Ok(fy), Ok(fm), Ok(ty), Ok(tm), Ok(sup)), (Ok(pts), Ok(rr), Ok(n))) => {
            // VALIDATED HERE AND NOT IN A GUARD. A match guard on the happy arm
            // leaves the compiler unable to prove the match exhaustive, and the
            // arms it then demands are shapes no parse can produce.
            if pts <= 0 {
                return refuse(
                    out,
                    "MAX_POINTS must be a whole number of index points, 1 or more",
                );
            }
            if n == 0 {
                return refuse(out, "TOP must be 1 or more");
            }
            let text = screen_range(
                vendor,
                underlying,
                rung,
                (fy, fm),
                (ty, tm),
                sup,
                Policy {
                    rules: Rules {
                        max_mae_ppm: points_to_ppm(pts),
                        min_rr_bp: rr,
                        // `screen` takes three numbers today, so the two new rules are
                        // off rather than guessed. A win-rate floor an operator did
                        // not type is a policy the engine invented.
                        min_win_rate_bp: 0,
                        min_assurance_bp: 0,
                        min_weakest_bp: 0,
                        min_trades: 0,
                        // Off for the same reason as the four above, and stated
                        // separately because it is the one rule about the PATH
                        // rather than the trade list. `cli elite` is where an
                        // operator turns it on without typing six numbers.
                        min_ret_over_dd_bp: 0,
                        top: n,
                    },
                    // The historical cut. `screen` is the command an operator
                    // uses to apply their OWN three numbers; changing which
                    // combinations it considers would change its answer
                    // without them having asked.
                    lens: runner::rank::Lens::Detectability,
                    // `screen` is a single answer, not a walk, so it pays for
                    // the full stack once -- BY DEFAULT. It hardcoded `true`,
                    // and MEASURED that meant it returned nothing at all: on
                    // NIFTY 60min over twelve months the sweep takes 14 seconds
                    // and this command printed zero bytes after fifty minutes,
                    // twice. `validate_from_env` lets an operator take the
                    // candidates now, marked `UNVALIDATED` on their face.
                    validate: validate_from_env(),
                },
            );
            let refused = text.starts_with("refused: ");
            out.push_str(&text);
            if refused { MISUSED } else { OK }
        }
        ((Err(_), _, _, _, _) | (_, _, Err(_), _, _), _) => {
            refuse(out, "YEAR must be a number like 2026")
        }
        ((_, Err(_), _, _, _) | (_, _, _, Err(_), _), _) => refuse(out, "MONTH must be 1..=12"),
        ((_, _, _, _, Err(why)), _) => refuse(out, why),
        (_, (Err(_), _, _)) => refuse(
            out,
            "MAX_POINTS must be a whole number of index points, 1 or more",
        ),
        (_, (_, Err(_), _)) => refuse(out, "MIN_RR is hundredths; 200 is 1:2, 0 drops the rule"),
        (_, (_, _, Err(_))) => refuse(out, "TOP must be 1 or more"),
    }
}

/// The index level a points-stated stop is converted against.
///
/// **A stated approximation, and it is printed as one.** NIFTY has traded
/// between roughly 7,500 and 26,000 across the span this store holds, so no
/// single number converts points to a fraction exactly. Twenty-five thousand is
/// the recent level; a rule of twenty points against it is 800 ppm, and at the
/// 2020 low the same 800 ppm is six points.
///
/// `CLAUDE.md` §3 rule 1 forbids inventing a figure, so this is not used to
/// MEASURE anything — every excursion in the report is ppm, taken against the
/// entry price of its own trade. It converts the OPERATOR'S RULE into the unit
/// the engine measures in, and the report prints both so the assumption is
/// visible rather than buried.
const NIFTY_REFERENCE: i64 = 25_000;

/// The price a rule stated in POINTS is converted against — read off the bars.
///
/// # This was a constant, and on any other instrument it was simply wrong
///
/// `NIFTY_REFERENCE` is 25,000. A rule of "twenty points" against it is 800 ppm,
/// and 800 ppm on a 52,000 index is FORTY-ONE points. Sweeping BANKNIFTY with a
/// NIFTY constant does not approximate the operator's rule, it doubles it — and
/// the same constant is wrong in the other direction on the 2020 low, where 800
/// ppm is six points.
///
/// The bars carry the price. Taking the MIDPOINT of the swept range — the mean
/// of the lowest low and the highest high — converts a rule against the
/// instrument that was actually swept, at the level it actually traded, on every
/// instrument and every span, with no constant to be wrong.
///
/// # Midpoint and not the mean of the closes
///
/// A mean is pulled by wherever the series spent most of its time, so a span
/// that ranged 15,000 to 26,000 but sat at 16,000 would convert a rule as though
/// the high years did not happen. The midpoint of the range treats both ends
/// alike, which is what a rule spanning the whole period needs.
///
/// # Refusal
///
/// An empty slice, or one whose extremes are not positive, falls back to
/// `NIFTY_REFERENCE` — the only remaining use of that constant, and it is
/// reached exactly when there is no price to read. It is scaled to PAISA on the
/// way out, because the branch above it returns paisa and a function whose two
/// arms disagree about the unit is the defect this whole block documents.
///
/// # THE UNIT IS PAISA, AND IT USED TO BE BOTH
///
/// `b.low` and `b.high` are paisa — `CLAUDE.md` §7 puts every price in this
/// system on a paisa `i64`, and `docs/02-store-format.md` writes it into the
/// record layout. The midpoint of two paisa is paisa. The fallback returned
/// `NIFTY_REFERENCE` unscaled, which is 25,000 — an INDEX LEVEL, a hundred
/// times smaller than the 2,500,000 paisa the same index is worth.
///
/// **One function, two branches, two units**, and the converters below trusted
/// the wrong one. `points_to_ppm_at(20, 2_500_000)` returned 8 where the
/// paragraph above states in writing that twenty points is 800 ppm, so the
/// shipped stop ladder came out at 2..10 ppm — 0.05 to 0.25 index points,
/// one to five NIFTY ticks — instead of 200..1000. Every stop the exit grid has
/// ever priced was inside the entry bar's own range, which is exactly what
/// [`STOP_FLOOR_POINTS`] exists to prevent.
///
/// The fallback is now scaled and the converters take paisa, so both arms and
/// every caller agree. `a_rule_in_points_converts_at_the_documented_rate` fails
/// the build if the rate ever drifts from the figure this doc names.
fn reference_price(bars: &[indicators::Candle]) -> i64 {
    let lo = bars.iter().map(|b| b.low).filter(|&l| l > 0).min();
    let hi = bars.iter().map(|b| b.high).filter(|&h| h > 0).max();
    match (lo, hi) {
        (Some(l), Some(h)) if h >= l => l.saturating_add(h) / 2,
        _ => NIFTY_REFERENCE.saturating_mul(PAISA_PER_POINT),
    }
}

/// Paisa in one index point.
///
/// `CLAUDE.md` §7 puts the tick grid at two decimal places, so a one-point move
/// on a quote is a hundred paisa — the same ratio
/// [`brutex_core::price::PAISA_PER_RUPEE`] names for money, taken from there
/// rather than written again so the two cannot drift apart.
const PAISA_PER_POINT: i64 = brutex_core::price::PAISA_PER_RUPEE;

/// Index points as parts per million against a price read off the bars.
///
/// `reference` is **paisa**, the unit [`reference_price`] returns and the unit
/// every price in this workspace is in. The points are lifted to paisa before
/// the ratio is taken, which is the step whose absence made this a hundredfold
/// error.
const fn points_to_ppm_at(points: i64, reference: i64) -> i64 {
    if reference <= 0 {
        return 0;
    }
    points
        .saturating_mul(PAISA_PER_POINT)
        .saturating_mul(1_000_000)
        / reference
}

/// Parts per million back to index points, at a price read off the bars.
///
/// `reference` is **paisa**. The quotient is paisa, so it is brought back down
/// to points — the inverse of [`points_to_ppm_at`], and it must stay the exact
/// inverse or a ladder built in one unit is printed in another.
///
/// # ROUNDED TO NEAREST, AND FLOORING WAS WRONG BY A WHOLE POINT
///
/// Two truncating divisions sit between a ppm rung and the point it is printed
/// as. A five-point rung at a reference of 2,512,500 paisa is 199 ppm exactly;
/// flooring it back gives 499 paisa and then **4 points**. The `TIGHTEST`
/// column is the one an operator reads to decide whether a stop is reachable,
/// and it was understating every rung by up to a point — always in the
/// direction that makes a stop look tighter than the engine can actually place.
///
/// Rounding to nearest makes `ppm_to_points_at(points_to_ppm_at(p, r), r) == p`
/// hold on every rung of the shipped ladder, which
/// `a_rule_in_points_converts_at_the_documented_rate` asserts.
const fn ppm_to_points_at(ppm: i64, reference: i64) -> i64 {
    if reference <= 0 {
        return 0;
    }
    let paisa = ppm.saturating_mul(reference) / 1_000_000;
    // Away from zero on a tie, and sign-aware so a negative excursion does not
    // round the wrong way. Integer arithmetic only -- §7 keeps floats off any
    // path whose output is compared.
    let half = PAISA_PER_POINT / 2;
    if paisa >= 0 {
        (paisa + half) / PAISA_PER_POINT
    } else {
        (paisa - half) / PAISA_PER_POINT
    }
}

/// Index points as parts per million against [`NIFTY_REFERENCE`].
///
/// **Superseded by [`points_to_ppm_at`].** Kept for the two `const` contexts
/// that cannot call a function taking a runtime price; every path that has bars
/// in hand uses the measured reference instead.
const fn points_to_ppm(points: i64) -> i64 {
    points.saturating_mul(1_000_000) / NIFTY_REFERENCE
}

/// Parts per million back to index points, the unit a stop is spoken in.
const fn ppm_to_points(ppm: i64) -> i64 {
    ppm.saturating_mul(NIFTY_REFERENCE) / 1_000_000
}

/// The `sweep-all` arm, lifted out of [`run`] for the reason
/// [`audit_range_arm`] gives: the dispatch is a command LIST and every inline
/// arm makes the list harder to read as one.
fn sweep_all_arm(out: &mut String, vendor: &str, rung: &str, min_hits: &str) -> u8 {
    match parse_min_hits(min_hits) {
        Ok(h) => {
            let text = batch::sweep_all(vendor, rung, h);
            let refused = text.starts_with("refused: ");
            out.push_str(&text);
            if refused { MISUSED } else { OK }
        }
        Err(why) => refuse(out, why),
    }
}
/// `descend`'s argument parsing, in the shape [`range_all_arm`] uses.
///
/// `PER_WEEK` is the operator's cadence and the only argument here that is not
/// also on `range-all`. It is a whole number of trades per week, refused at
/// zero -- a cadence of zero trades has no floor and would sweep to a support
/// of one hit, which is every combination that ever fires once.
#[expect(
    clippy::too_many_arguments,
    reason = "the command takes eight words and each is a separate parse that \
              refuses with its own sentence. Bundling them into a struct would \
              move the parsing, not remove it, and `range_all_arm` beside it \
              takes six for the same reason."
)]
fn descend_arm(
    out: &mut String,
    vendor: &str,
    underlying: &str,
    rung: &str,
    from: (&str, &str),
    to: (&str, &str),
    ceiling_ppm: &str,
    per_week: &str,
) -> u8 {
    match (
        from.0.parse::<u16>(),
        from.1.parse::<u8>(),
        to.0.parse::<u16>(),
        to.1.parse::<u8>(),
        parse_support_ppm(ceiling_ppm),
        per_week.parse::<u64>(),
    ) {
        (Ok(fy), Ok(fm), Ok(ty), Ok(tm), Ok(h), Ok(w)) => {
            // Checked HERE and not as a match guard: a guard on this arm makes
            // the match non-exhaustive, and the compiler is right -- the zero
            // case would fall through to a later arm that says nothing about
            // cadence.
            if w == 0 {
                return refuse(
                    out,
                    "PER_WEEK must be a whole number of trades per week, at \
                     least 1. A cadence of zero has no support floor and would \
                     sweep every combination that fires even once.",
                );
            }
            let text = descend(vendor, underlying, rung, (fy, fm), (ty, tm), h, w);
            let refused = text.starts_with("refused: ");
            out.push_str(&text);
            if refused { MISUSED } else { OK }
        }
        (_, _, _, _, _, Err(_)) => refuse(
            out,
            "PER_WEEK must be a whole number of trades per week, at least 1.",
        ),
        (Err(_), _, _, _, _, _) | (_, _, Err(_), _, _, _) => {
            refuse(out, "YEAR must be a number like 2026")
        }
        (_, Err(_), _, _, _, _) | (_, _, _, Err(_), _, _) => {
            refuse(out, "MONTH must be a number from 1 to 12")
        }
        (_, _, _, _, Err(why), _) => refuse(out, why),
    }
}

/// The `range-all` arm, lifted out of [`run`] for the reason
/// [`audit_range_arm`] gives.
///
/// The same four number parses and the same four refusals, in the same order,
/// because two commands taking one shape of argument must reject a bad one
/// identically or an operator learns two rules.
fn range_all_arm(
    out: &mut String,
    vendor: &str,
    underlying: &str,
    from: (&str, &str),
    to: (&str, &str),
    support_ppm: &str,
) -> u8 {
    match (
        from.0.parse::<u16>(),
        from.1.parse::<u8>(),
        to.0.parse::<u16>(),
        to.1.parse::<u8>(),
        parse_support_ppm(support_ppm),
    ) {
        (Ok(fy), Ok(fm), Ok(ty), Ok(tm), Ok(h)) => {
            let text = range_all(vendor, underlying, (fy, fm), (ty, tm), Some(h));
            let refused = text.starts_with("refused: ");
            out.push_str(&text);
            if refused { MISUSED } else { OK }
        }
        (Err(_), _, _, _, _) | (_, _, Err(_), _, _) => {
            refuse(out, "YEAR must be a number like 2026")
        }
        (_, Err(_), _, _, _) | (_, _, _, Err(_), _) => refuse(out, "MONTH must be 1..=12"),
        (_, _, _, _, Err(why)) => refuse(out, why),
    }
}
/// The `audit-range` arm, lifted out of [`run`].
///
/// # Why it is a function and not five more lines in the match
///
/// `run` is a dispatch table and `clippy::too_many_lines` caps it at a hundred.
/// The cap is doing real work here: an arm that parses FOUR numbers plus a
/// threshold is the largest in the table, and inlining it pushed the whole
/// dispatch past the point where a reader can see the command list at all.
///
/// The refusals are the same four `audit-stored` gives and in the same order,
/// because two commands taking one shape of argument must reject a bad one
/// identically or an operator learns two rules.
fn audit_range_arm(
    out: &mut String,
    vendor: &str,
    underlying: &str,
    rung: &str,
    from: (&str, &str),
    to: (&str, &str),
    min_hits: &str,
) -> u8 {
    match (
        from.0.parse::<u16>(),
        from.1.parse::<u8>(),
        to.0.parse::<u16>(),
        to.1.parse::<u8>(),
        parse_min_hits(min_hits),
    ) {
        (Ok(fy), Ok(fm), Ok(ty), Ok(tm), Ok(h)) => {
            let text = audit_range(vendor, underlying, rung, (fy, fm), (ty, tm), h);
            let refused = text.starts_with("refused: ");
            out.push_str(&text);
            if refused { MISUSED } else { OK }
        }
        (Err(_), _, _, _, _) | (_, _, Err(_), _, _) => {
            refuse(out, "YEAR must be a number like 2026")
        }
        (_, Err(_), _, _, _) | (_, _, _, Err(_), _) => refuse(out, "MONTH must be 1..=12"),
        (_, _, _, _, Err(why)) => refuse(out, why),
    }
}

/// Parses one command and runs it, returning the code the shell reads.
///
/// # Why this takes the arguments rather than reading them
///
/// The same reason `api::server::run` does: a function that reads
/// `std::env::args` has arms no unit test can enter, and `CLAUDE.md` §9's
/// coverage floor is not something to work around with a comment.
#[must_use]
pub fn run(args: &[String], out: &mut String) -> u8 {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["sweep", sessions, min_hits] => match (parse_sessions(sessions), parse_min_hits(min_hits))
        {
            (Ok(s), Ok(m)) => {
                out.push_str(&sweep(s, m));
                OK
            }
            (Err(why), _) | (_, Err(why)) => refuse(out, why),
        },
        ["audit", sessions, min_hits] => {
            match (parse_sessions(sessions), parse_min_hits(min_hits)) {
                (Ok(s), Ok(m)) => {
                    out.push_str(&audit_run(s, m));
                    OK
                }
                (Err(why), _) | (_, Err(why)) => refuse(out, why),
            }
        }
        [
            "sweep-stored",
            vendor,
            underlying,
            rung,
            year,
            month,
            min_hits,
        ] => {
            match (
                year.parse::<u16>(),
                month.parse::<u8>(),
                parse_min_hits(min_hits),
            ) {
                (Ok(y), Ok(m), Ok(h)) => {
                    let text = sweep_stored(vendor, underlying, rung, y, m, h);
                    let refused = text.starts_with("refused: ");
                    out.push_str(&text);
                    if refused { MISUSED } else { OK }
                }
                (Err(_), _, _) => refuse(out, "YEAR must be a number like 2026"),
                (_, Err(_), _) => refuse(out, "MONTH must be 1..=12"),
                (_, _, Err(why)) => refuse(out, why),
            }
        }
        [
            "audit-stored",
            vendor,
            underlying,
            rung,
            year,
            month,
            min_hits,
        ] => {
            // The same parse, the same order and the same three refusals as
            // `sweep-stored` above. Two commands taking one shape of argument
            // must reject a bad one identically, or an operator learns two rules.
            match (
                year.parse::<u16>(),
                month.parse::<u8>(),
                parse_min_hits(min_hits),
            ) {
                (Ok(y), Ok(m), Ok(h)) => {
                    let text = audit_stored(vendor, underlying, rung, y, m, h);
                    let refused = text.starts_with("refused: ");
                    out.push_str(&text);
                    if refused { MISUSED } else { OK }
                }
                (Err(_), _, _) => refuse(out, "YEAR must be a number like 2026"),
                (_, Err(_), _) => refuse(out, "MONTH must be 1..=12"),
                (_, _, Err(why)) => refuse(out, why),
            }
        }
        ["audit-range", v, u, r, fy, fm, ty, tm, mh] => {
            audit_range_arm(out, v, u, r, (fy, fm), (ty, tm), mh)
        }
        ["screen", v, u, r, fy, fm, ty, tm, sup, pts, rr, n] => {
            screen_arm(out, v, u, r, (fy, fm, ty, tm), (sup, pts, rr, n))
        }
        ["elite", v, u, r, fy, fm, ty, tm, pts, n] => {
            elite_arm(out, v, u, r, (fy, fm, ty, tm), (pts, n))
        }
        ["range-all", v, u, fy, fm, ty, tm, mh] => range_all_arm(out, v, u, (fy, fm), (ty, tm), mh),
        ["descend", v, u, r, fy, fm, ty, tm, sup, pw] => {
            descend_arm(out, v, u, r, (fy, fm), (ty, tm), sup, pw)
        }
        ["verify", feed, underlying] => verify_arm(out, feed, underlying),
        ["auto-stored", v, u, r, fy, fm, ty, tm] => auto_stored_arm(out, v, u, r, (fy, fm, ty, tm)),
        ["top"] => top_arm(out, None),
        ["top", feed, underlying] => top_arm(out, Some((feed, underlying))),
        ["results"] => results_arm(out, None),
        ["results", feed, underlying] => results_arm(out, Some((feed, underlying))),
        ["sweep-all", vendor, rung, min_hits] => sweep_all_arm(out, vendor, rung, min_hits),
        ["auto", sessions] => match parse_sessions(sessions) {
            Ok(s) => {
                out.push_str(&auto(s));
                OK
            }
            Err(why) => refuse(out, why),
        },
        [] => refuse(out, "no command given"),
        [word, rest @ ..] => refuse(out, &unmatched(word, rest.len())),
    }
}

/// Why a word reached [`run`]'s last arm, said in the terms that were wrong.
///
/// # A KNOWN COMMAND WITH THE WRONG ARITY IS NOT AN UNKNOWN COMMAND
///
/// Every dispatch arm is a slice pattern of a FIXED length, so `cli screen` with
/// no arguments matches none of them and falls through. The message there was
/// *"`screen` is not a command this build knows"* — which is false, and sends a
/// reader hunting for a typo or a stale binary instead of reading the argument
/// list printed two lines below it in the same output.
///
/// `CLAUDE.md` §4 requires a refusal to name what was wrong. When the word is
/// real, the thing that was wrong is the COUNT, so that is what it says.
fn unmatched(word: &str, given: usize) -> String {
    if COMMANDS.contains(&word) {
        format!(
            "`{word}` is a command, but not with {given} argument{}. Its argument \
             list is in the usage below.",
            if given == 1 { "" } else { "s" }
        )
    } else {
        format!("`{word}` is not a command this build knows")
    }
}

/// Every word [`run`]'s dispatch answers to.
///
/// # Why a list and not a parse of [`USAGE`]
///
/// The refusal above needs to tell a known command with the wrong arity apart
/// from a typo, and the only other source for "is this a command" is the usage
/// text. Parsing prose for that is the shape this workspace refuses elsewhere:
/// the text wraps, its argument lists run onto continuation lines, and a
/// reworded line would silently change which words the binary claims to know.
///
/// So it is written down, and `every_command_is_listed_in_both_places` asserts
/// the list, the dispatch and the usage all name the same set. The duplication
/// is real; the test is what makes it safe.
const COMMANDS: [&str; 15] = [
    "audit",
    "audit-range",
    "audit-stored",
    "auto",
    "auto-stored",
    "descend",
    "elite",
    "range-all",
    "results",
    "screen",
    "sweep",
    "sweep-all",
    "sweep-stored",
    "top",
    "verify",
];

/// Writes a named refusal and the usage, and returns [`MISUSED`].
///
/// Named rather than bare: `CLAUDE.md` §4 requires a refusal to say what was
/// wrong, and "usage:" alone leaves the reader to work out which of their words
/// this build objected to.
fn refuse(out: &mut String, why: &str) -> u8 {
    out.push_str("refused: ");
    out.push_str(why);
    out.push('\n');
    out.push_str(USAGE);
    MISUSED
}

/// Sessions must be at least one and at most ten years of them.
///
/// The upper bound is not a performance guess: `synthetic::sessions` allocates
/// one `Candle` per minute per day, so an unbounded count is an unbounded
/// allocation reached from the command line, which is the shape
/// `docs/06-limits.md` §5 is about.
fn parse_sessions(text: &str) -> Result<i64, &'static str> {
    match text.parse::<i64>() {
        Ok(n) if (1..=3650).contains(&n) => Ok(n),
        Ok(_) => Err("SESSIONS is outside 1..=3650"),
        Err(_) => Err("SESSIONS is not a whole number"),
    }
}

/// `SUPPORT_PPM` as parts per million, or the sentence that refuses it.
///
/// # Bounded at both ends, and both bounds are real
///
/// Zero would make every candidate frequent, which disables extinction — the
/// defect `engine::Ladder::with_min_hits` raises zero to one to prevent, arriving
/// through a different door.
///
/// A million is 100% support: a combination that fires on EVERY bar. Above that
/// nothing can be frequent and the sweep is guaranteed to find nothing, so it is
/// refused rather than run — a report of zero combinations that took an hour to
/// produce is a waste, not a finding.
///
/// # Errors
///
/// A non-number, zero, or anything above 1,000,000.
fn parse_support_ppm(text: &str) -> Result<u64, &'static str> {
    match text.parse::<u64>() {
        Err(_) => Err("SUPPORT_PPM is not a whole number"),
        Ok(0) => Err("SUPPORT_PPM must be 1 or more; 0 would disable extinction"),
        Ok(ppm) if ppm > 1_000_000 => Err(
            "SUPPORT_PPM is parts per million, so 1000000 is 100%. Above \
                 that nothing can be frequent",
        ),
        Ok(ppm) => Ok(ppm),
    }
}
/// `min_hits` must be at least one.
///
/// Zero is refused HERE as well as clamped in [`Ladder::with_min_hits`], and
/// the duplication is deliberate: the clamp keeps the ladder correct, and this
/// keeps the operator from believing a zero they typed was honoured.
fn parse_min_hits(text: &str) -> Result<u64, &'static str> {
    match text.parse::<u64>() {
        Ok(n) if n >= 1 => Ok(n),
        Ok(_) => Err("MIN_HITS must be 1 or more; 0 would disable extinction"),
        Err(_) => Err("MIN_HITS is not a whole number"),
    }
}

/// The pinned evaluator, which is the only configuration this build ships.
///
/// One line, so that the half a test CANNOT drive is as small as it can be.
fn evaluator() -> Result<Evaluator, &'static str> {
    evaluator_from(Widths::pinned().ok())
}

/// The evaluator, from widths the caller supplies.
///
/// # Why the widths are a parameter rather than a read
///
/// The same reason `api::autopilot::stays_paused_from` takes its value instead
/// of fetching it: `Widths::pinned()` reads two pinned constants and cannot fail
/// in this build, so a function that called it inline would carry a refusal arm
/// no test could enter — and `CLAUDE.md` §9's coverage floor is not something to
/// work around with a comment. `None` is the shape a future widening of the
/// tolerance table could produce, and this is where it is answered.
fn evaluator_from(widths: Option<Widths>) -> Result<Evaluator, &'static str> {
    let widths = widths.ok_or("the pinned tolerances are not valid")?;
    Ok(Evaluator::new(
        widths,
        // ABSENT, AND THE CALLER MAY NOT DERIVE IT. `vwap::availability_of`
        // reads the WHOLE slice, so a mask at bar 0 would depend on bar N --
        // look-ahead, which §3 rule 7 forbids outright. `runner` refuses to
        // compute it for exactly this reason and takes the verdict from its
        // caller; this crate declares it absent rather than inventing one.
        Availability::Absent,
        Thresholds::CLASSICAL,
    ))
}

/// One sweep at one threshold, rendered.
#[must_use]
pub fn sweep(sessions: i64, min_hits: u64) -> String {
    sweep_with(evaluator(), sessions, min_hits)
}

/// One sweep, from an evaluator the caller supplies.
///
/// Split for the same reason [`evaluator_from`] is: `evaluator()` cannot fail in
/// this build, so a refusal arm reached only through it is a region no test can
/// enter — and `CLAUDE.md` §9 asks for 100%, which an unreachable arm makes
/// unattainable rather than merely unmet. The arm still has to EXIST, because a
/// later widening of the tolerance table would reach it; taking the `Result`
/// here is what lets a test prove it says something useful when it does.
#[allow(
    clippy::needless_pass_by_value,
    reason = "`Evaluator` is 1.7 KB of indicator state and this function CONSUMES \
              it -- `Sweeper::run` takes `&mut`, and the value must outlive the \
              call, so a reference would only move the ownership problem to the \
              caller and put the unreachable refusal arm back in a public \
              function. One move of 1.7 KB happens once per process."
)]
// `Evaluator` measures 1,744 bytes (`docs/10-shared-core.md`), so this trips
// `large_types_passed_by_value` at its 256-byte limit. Taken by value anyway, and the
// allow is narrowed to the two functions that do it rather than relaxed at the crate
// root: the body needs OWNERSHIP — `Sweeper::run` takes `&mut ev` — so a reference
// would only move the move to the caller, and `Box` would buy an allocation and an
// indirection to avoid a memcpy that happens ONCE per CLI invocation, before any bar is
// read. This is not the sweep's hot path; `ConditionMask::hits` is, and nothing here is
// on it. If either function is ever called in a loop, this allow is the thing to revisit.
#[allow(
    clippy::large_types_passed_by_value,
    reason = "the body needs ownership; one 1,744-byte move per CLI invocation"
)]
fn sweep_with(ev: Result<Evaluator, &'static str>, sessions: i64, min_hits: u64) -> String {
    let mut ev = match ev {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    let bars = synthetic::sessions(sessions);
    let ladder = match ladder_for(min_hits) {
        Ok(l) => l,
        Err(why) => return format!("refused: {why}\n"),
    };
    let outcome = Sweeper::new(ladder).run(&bars, &mut ev);
    let mut out = String::from(PROVENANCE);
    out.push('\n');
    out.push_str(&runner::report::render(&outcome, None));
    out
}

/// What a run over REAL bars says about itself.
///
/// The counterpart to [`PROVENANCE`], and it exists for the same reason: a sweep
/// over stored bars and a sweep over generated ones are byte-identical in shape,
/// so the report has to say which it was or the reader cannot tell. This one
/// names the feed, the instrument and the month, because "real data" is not a
/// provenance — *whose* data, of *what*, for *when* is.
pub const STORED_PROVENANCE: &str = "\
=== THESE BARS ARE REAL MARKET DATA, READ FROM THE STORE ===
Nothing was pulled from a vendor by this process. The bars below were read from
a file some earlier pull wrote, and the run identity beneath names the exact
column they came from. A figure here describes that instrument and that month.
";

/// The commit this binary was BUILT from, if the build was stamped.
///
/// # Why `option_env!` and not a `build.rs`, and not `.git/HEAD`
///
/// `CLAUDE.md` §2 forbids a `build.rs` that invokes an external process, so
/// `git rev-parse` at build time is not available and is not wanted.
///
/// Reading `.git/HEAD` at RUN time would compile, and it would be wrong. §3
/// rule 3 identifies a run by the commit **the computation ran at**, and the
/// computation is this binary — which was compiled from one commit and may be
/// executed long after the tree moved to another. A runtime read would stamp a
/// result with a commit whose source never produced it, which is worse than
/// recording nothing: it is a reproducibility claim that cannot be honoured.
///
/// `option_env!` resolves at compile time, in the binary, with no process
/// spawned. `None` means the build was not stamped, and that is refused rather
/// than filled in.
#[must_use]
pub const fn commit_stamp() -> Option<&'static str> {
    option_env!("BRUTEX_COMMIT")
}

/// The store root: `$BRUTEX_STORE`, else `$HOME/.brutex/store`.
///
/// The same two-step every other root in this workspace uses, so an operator who
/// has moved one has moved them all.
fn store_root() -> Result<std::path::PathBuf, stored::Refusal> {
    root_from(std::env::var_os("BRUTEX_STORE"), std::env::var_os("HOME"))
}

/// [`store_root`]'s decision, with the environment passed in.
///
/// Split so the three outcomes are testable without mutating process
/// environment, which `cargo test` runs threads against in parallel: a test that
/// set `HOME` would change it under every other test in the binary.
fn root_from(
    explicit: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<std::path::PathBuf, stored::Refusal> {
    if let Some(explicit) = explicit {
        return Ok(std::path::PathBuf::from(explicit));
    }
    home.map_or_else(
        || Err("neither BRUTEX_STORE nor HOME is set, so the store cannot be found".to_owned()),
        |home| Ok(std::path::PathBuf::from(home).join(".brutex").join("store")),
    )
}

/// Where the sweep writes its events: `$BRUTEX_LOG_DIR`, else `<store>/logs/cli`.
///
/// # `cli`, NOT `logs`, and the subdirectory is a correctness fix
///
/// This resolved `<store>/logs` — the same directory `api` appends to. Both
/// write `events.ndjson` because `telemetry::sink::BASENAME` is a constant, so
/// **two live processes were appending to one file.** A long `range-all` and a
/// running server interleaving lines is a corrupted record of the one thing that
/// exists to say what happened.
///
/// The measured consequence was worse than a race. To avoid it, a whole
/// nine-rung run over 81 months was pointed at a scratch directory instead —
/// forty events describing every span load, every derived threshold and every
/// month found or missing, written somewhere `/logs` does not read and the
/// operator would never see. `<store>/logs/events.ndjson` stayed **zero bytes**
/// while the run produced its entire history elsewhere.
///
/// A subdirectory removes the race without a `telemetry` change: `Config` takes
/// a DIRECTORY, so each writer owning one means each owns its own
/// `events.ndjson`, its own rotation and its own byte budget. No lock, no
/// coordination, nothing to get wrong under concurrency — which is the only
/// design that is `O(1)` per event with two writers as with one.
///
/// **`/logs` must read both.** `api` serves whatever directory it resolved, so
/// until it walks `logs/` and `logs/cli/` the page shows the server's half. That
/// is recorded in `HANDOVER-web-backtest.md` rather than left for the next
/// operator to find by seeing an empty page.
///
/// `BRUTEX_LOG_DIR` still wins outright and is used exactly as given, because an
/// operator who names a directory means that directory and not a child of it.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn log_dir_from(
    explicit: Option<std::ffi::OsString>,
    store: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    explicit
        .map(std::path::PathBuf::from)
        .or_else(|| store.map(|s| s.join("logs").join("cli")))
}

/// Installs the process-wide event sink, or says why it could not.
///
/// # Why this returns a warning instead of refusing
///
/// A sweep with no log is still a correct sweep. Refusing to compute because a
/// directory is not writable would trade a whole answer for an audit trail,
/// which is not the bargain `CLAUDE.md` §4 asks for — §4 bans a fallback that
/// **hides** a failure, and this one names it. The caller prints the returned
/// line above the report, so an operator who expected events and got none is
/// told why on the same screen rather than discovering an empty log later.
///
/// # SUCCESS IS ALSO A LINE NOW, AND THE MEASUREMENT IS WHY
///
/// This returned `None` on success and printed nothing. So an operator ran a
/// sweep, opened `/logs`, saw no sweep events, and had nothing to go on —
/// because `api` and this crate resolve DIFFERENT default directories and
/// neither ever says which it chose:
///
/// | | default when `BRUTEX_LOG_DIR` is unset |
/// |---|---|
/// | `api::served_log_dir` | `<cwd>/logs` when cwd is the workspace root |
/// | this function | `<store>/logs` |
///
/// Measured on the operator's machine, with `api` launched from the workspace
/// root as `.claude/launch.json` runs it:
///
/// ```text
/// <workspace>/logs/events.ndjson    905,539 bytes   <- what /logs serves
///     grep -c 'cli.sweep'  ->  0
/// ~/.brutex/store/logs/events.ndjson      0 bytes   <- what this writes
/// ```
///
/// D-0226 added the three `cli.sweep` events precisely so `/logs` would cover
/// the READ half of the data path, and they land in a file the page does not
/// open. Nothing was wrong with either default; what was missing is that
/// **neither end said where**, which is the silent half of the same
/// `CLAUDE.md` §4 failure this function's `Err` half already refuses.
///
/// Naming the directory is the honest fix available from THIS crate. Making the
/// two agree is an `api` change and `api` is not this crate's to edit.
///
/// # Errors
///
/// The refusal in `telemetry`'s own words — an unwritable directory, or a sink
/// already installed, which `telemetry::install` refuses rather than ignores
/// because two sinks on one path each roll the other's file away. Or, before
/// either can be tried, that no directory could be resolved at all.
///
/// # The `?` that was here was itself the silent failure this function warns about
///
/// The first version read `let dir = log_dir_from(..)?;` — so when neither
/// `BRUTEX_LOG_DIR` nor a store root resolved, it returned `None` early. `None`
/// is the value that means *installed successfully*, so a run with nowhere to
/// write reported itself as fully recorded, and the operator got no events and
/// no reason. That is exactly the `CLAUDE.md` §4 fallback-that-hides-a-failure
/// this function exists to avoid, introduced by the function avoiding it.
///
/// The two failures are now separate sentences because they need different
/// actions: an unresolvable directory is fixed by setting a variable, an
/// unwritable one by changing permissions.
#[must_use]
pub fn install_log() -> String {
    let Some(dir) = log_dir_from(std::env::var_os("BRUTEX_LOG_DIR"), store_root().ok()) else {
        return "events are NOT being recorded: neither BRUTEX_LOG_DIR nor a store root \
                is set, so there is nowhere to write them. Set BRUTEX_LOG_DIR, or set \
                BRUTEX_STORE or HOME so the log can sit beside the store."
            .to_owned();
    };
    let shown = dir.display().to_string();
    match telemetry::install(&telemetry::Config::new(dir)) {
        Err(why) => format!("events are NOT being recorded: {why}"),
        // THE DIRECTORY, NOT A TICK. "recorded successfully" would leave the
        // operator exactly where the measurement above found them: told it
        // worked, and unable to find the file.
        Ok(_installed) => format!(
            "events -> {shown}\n  \
             The /logs page walks BOTH halves -- the server's directory and \
             this `cli/` beside it -- and merges them newest-first on the \
             clock, so these events appear there. D-0301."
        ),
    }
}

/// One structural event about a run. **Never called per bar or per candidate.**
///
/// Gate 17's rule is that the innermost loop calls nothing at all; this crate
/// holds no loop over bars and none over candidates, so every call site here is
/// a boundary — one per run, or one per instrument-month in a batch. That is the
/// granularity gate 17's own comment prescribes as the affordable one.
pub(crate) fn note(event: &telemetry::Event<'_>) {
    // The result is deliberately discarded HERE and only here. `emit` returns
    // `NotInstalled` rather than panicking when nothing was installed, which is
    // what makes these call sites safe in a test binary that never installs a
    // sink — and `install_log` above is the one place that reports the absence,
    // so reporting it again per event would be noise on every line.
    let _ = telemetry::emit(event);
}

/// A vendor's directory word to the vendor, or the list of words that work.
///
/// Walks [`Vendor::ALL`], which is five entries and a compile-time constant, so
/// no word can be accepted here that the store cannot then address.
fn parse_vendor(word: &str) -> Result<Vendor, stored::Refusal> {
    Vendor::ALL
        .into_iter()
        .find(|v| v.as_str() == word)
        .ok_or_else(|| {
            let known: Vec<&str> = Vendor::ALL.iter().map(|v| v.as_str()).collect();
            format!(
                "`{word}` is not a feed this build knows: {}",
                known.join(", ")
            )
        })
}

/// One sweep over REAL bars, with the run identity §3 rule 3 requires.
///
/// # What this closes
///
/// `crates/api` declared `store` and no `runner`; `crates/cli` declared `runner`
/// and no `store`, so no crate in the workspace could do both and a pulled bar
/// could not reach a ranked result. `e302d99` added [`stored::load`] as the
/// join — and nothing called it, which is the same capability-with-no-caller gap
/// `CLAUDE.md` §5 says this crate exists to close. This is the caller.
///
/// # Why the identity is recorded HERE and not in `sweep`
///
/// §3 rule 3 identifies a run by eight terms, and four of them — instrument,
/// timeframe, data digest and the commit — **do not exist for generated bars**.
/// `runner::synthetic` invents a column that is of no instrument, at no bar
/// length, from no pull. That is why `sweep` renders `NOT RECORDED` and is right
/// to: there is nothing to record. Real bars supply all four, so this function
/// records them, and refuses rather than inventing one it cannot supply.
#[must_use]
pub fn sweep_stored(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> String {
    match sweep_stored_inner(vendor_word, underlying, rung, year, month, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// [`sweep_stored`]'s body, so every refusal is one `?` rather than a nest.
fn sweep_stored_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> Result<String, stored::Refusal> {
    // THE COMMIT IS CHECKED FIRST, BEFORE ANY BAR IS READ. §3 rule 3 is "no
    // computation without that identity recorded", so a build that cannot be
    // identified must refuse BEFORE it computes, not sweep and then apologise.
    let commit = commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and the sweep will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;

    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let loaded = stored::load(&root, vendor, underlying, rung, year, month)?;

    // THE FILE WAS OPENED AND THIS IS WHERE AN OPERATOR LEARNS IT. The question
    // after a sweep that found nothing is "did it even read my month?", and
    // until this line nothing in the workspace could answer it.
    note(
        &telemetry::Event::info("cli.sweep", "stored month loaded")
            .with("feed", loaded.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", loaded.timeframe)
            .with("year", u64::from(year))
            .with("month", u64::from(month))
            .with("bars", u64::try_from(loaded.bars.len()).unwrap_or(u64::MAX))
            .with("min_hits", min_hits),
    );

    let mut ev = evaluator().map_err(str::to_owned)?;
    let ladder = ladder_for(min_hits)?;
    // RANKED, NOT MERELY COUNTED. This was `Sweeper::run`, whose report ends at
    // "combinations found 3,689" -- a count with no way to learn what any of the
    // 3,689 are. `run_ranked` builds the forward from the same slice the column
    // was built from, so the mispairing `Edge::mismatched` guards against cannot
    // arise, and `report::render_findings` below names every kept combination.
    let run = Sweeper::new(ladder).run_ranked(&loaded.bars, &mut ev, Horizon::DEFAULT, STORED_KEEP);
    let (outcome, ranked) = (run.outcome, run.ranked);

    // The identity, over the bars actually swept and the ladder actually
    // applied. `Params::of` reads the ladder rather than the argument, so a
    // `min_hits` the ladder raised is recorded as what ran, not as what was asked.
    let id = identity(&Run {
        // `Default::default()` AND NOT `ConditionMask::default()`, which clippy asks
        // for and this crate cannot give it. The named path needs `use vocab::…`,
        // and `vocab` is not among `cli`'s dependencies -- `CLAUDE.md` §5 lists
        // them, and adding an arrow to satisfy a lint would be the silent scope
        // change §3 rule 2 forbids. The struct field types this value already.
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        direction: RunDirection::Undirected,
        instrument: &loaded.key,
        timeframe: loaded.timeframe,
        params: Params::of(ladder),
        data_digest: data_digest(&loaded.bars),
        commit,
        // THE FEED, READ OFF THE LOAD RATHER THAN OFF THE ARGUMENT.
        //
        // `loaded.vendor` is the first path segment of the file that was
        // actually opened — "never inferred", as its own doc says — so an
        // identity built from it names the column on disk rather than the word
        // the operator typed. Those agree today because `stored::load` resolves
        // one from the other, and reading the load keeps them agreeing if that
        // ever stops being true.
        feed: loaded.vendor.as_str(),
    });

    let mut out = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "feed {} · {} · {} · {year}-{month:02} · {} bars · built at {commit}",
        vendor.as_str(),
        underlying,
        loaded.timeframe,
        loaded.bars.len(),
    );
    // THE ANSWER, KEYED BY THE IDENTITY THAT NAMES IT. Emitted after the walk
    // and before the render, so a run killed while formatting a large report
    // still leaves its result in the log. `halted` is the field that separates
    // "the ladder went extinct" from "a budget stopped it short", which the
    // count alone cannot say.
    note(
        &telemetry::Event::info("cli.sweep", "ladder walked")
            .with("identity", id.hex().as_str())
            .with("feed", loaded.vendor.as_str())
            .with(
                "depth",
                u64::try_from(outcome.sweep.depth()).unwrap_or(u64::MAX),
            )
            .with("kept", ranked.considered)
            .with(
                "ranked",
                u64::try_from(ranked.top.len()).unwrap_or(u64::MAX),
            )
            .with("completed", outcome.sweep.completed())
            .with("halted", outcome.sweep.halted.is_some()),
    );

    out.push('\n');
    out.push_str(&runner::report::render(&outcome, Some(&id)));
    // THE ANSWER, NOT JUST THE SEARCH. `render` reports how MANY combinations
    // survived at each level; this reports WHICH, by condition name, with the
    // evidence for each and the bar that evidence must clear. Without it the
    // whole ladder is a counter.
    out.push_str(&runner::report::render_findings(&ranked, &outcome.sweep));
    Ok(out)
}

/// The candidate ceiling a THRESHOLD SEARCH probes with.
///
/// # Why this is not the sweep's ceiling, and what it cost to leave it so
///
/// `Sweeper::auto` probes each rung with **the caller's** ceiling — deliberately,
/// because an earlier version silently discarded whatever budget the `Sweeper`
/// was built with. This crate then handed it `Ladder::with_min_hits(1)`, whose
/// ceiling is `engine::DEFAULT_CEILING` = `1 << 26` = 67,108,864. That is the
/// budget for producing an ANSWER, and the search runs roughly seventeen probes
/// with it.
///
/// Measured on this machine with nothing else running, `cli auto 20`, whole
/// search end to end:
///
/// | probe ceiling | search time | threshold chosen | depth |
/// |---|---|---|---|
/// | 67,108,864 (`DEFAULT_CEILING`, what shipped) | **> 240 s, killed** | — | — |
/// | 50,000 | 0.8 s | 1,831 | 14 |
/// | **500,000** | **3.7 s** | **1,036** | **17** |
/// | 5,000,000 | 35.6 s | 496 | 19 |
///
/// This is the knee, and it is where the CURVE TURNS rather than where the
/// numbers are round: 50,000 to 500,000 costs 4.6x the time and buys three more
/// levels, while 500,000 to 5,000,000 costs 9.6x and buys two. The step after
/// the one taken is the expensive one -- the same shape `PROBE_PAIRS` records for
/// the pair budget.
///
/// A probe only has to answer "is this threshold cheap" — the same argument
/// `runner`'s `PROBE_PAIRS` already makes for the pair budget, and the same safe
/// direction: a threshold needing more than this is rejected, so the chosen
/// threshold may be higher than strictly necessary. The sweep that is finally
/// KEPT is the probe's own, walked at this ceiling, so the report never claims a
/// budget it did not use.
const SEARCH_CEILING: usize = 500_000;

// A PROBE BUDGET AT OR ABOVE THE ANSWER BUDGET IS NOT A PROBE, and this is a
// BUILD failure rather than a test one -- the same form the streaming indicators
// use for their size ceilings. `Sweeper::auto` probes with the CALLER's ceiling,
// so the caller is what decides whether the search is a search; this crate handed
// it `DEFAULT_CEILING` and `cli auto 250` then produced no output in twenty
// minutes. Asserted as a RATIO against the engine's own default rather than
// against a literal, so raising that default cannot quietly undo the fix.
const _: () = assert!(SEARCH_CEILING < engine::DEFAULT_CEILING);
const _: () = assert!(engine::DEFAULT_CEILING / SEARCH_CEILING >= 100);

/// The threshold search, rendered.
#[must_use]
pub fn auto(sessions: i64) -> String {
    auto_with(evaluator(), sessions)
}

/// The threshold search, from an evaluator the caller supplies. See
/// [`sweep_with`] for why this is split.
#[allow(
    clippy::needless_pass_by_value,
    reason = "`Evaluator` is 1.7 KB of indicator state and this function CONSUMES \
              it -- `Sweeper::run` takes `&mut`, and the value must outlive the \
              call, so a reference would only move the ownership problem to the \
              caller and put the unreachable refusal arm back in a public \
              function. One move of 1.7 KB happens once per process."
)]
// Same trade as `sweep_with`, and the same reasoning — see the comment there.
#[allow(
    clippy::large_types_passed_by_value,
    reason = "the body needs ownership; one 1,744-byte move per CLI invocation"
)]
fn auto_with(ev: Result<Evaluator, &'static str>, sessions: i64) -> String {
    let mut ev = match ev {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    let bars = synthetic::sessions(sessions);
    let found =
        Sweeper::new(Ladder::with_min_hits(1).with_ceiling(SEARCH_CEILING)).auto(&bars, &mut ev);
    let mut out = String::from(PROVENANCE);
    out.push('\n');
    out.push_str(&runner::report::render_auto(&found, None));
    out
}

/// The threshold search, over REAL stored bars across a span of months.
///
/// # The gap this closes
///
/// `auto` searches for the deepest threshold a machine can finish, by bisection,
/// and it is the only dynamic thing in the whole surface. It took a count of
/// SYNTHETIC sessions. Every command that touched the store — `sweep-stored`,
/// `audit-stored`, `screen`, `range-all` — took a threshold as an ARGUMENT, so
/// on real data an operator had to guess a number, watch it refuse or grind, and
/// guess again.
///
/// That is backwards. The right threshold is not a preference; it is a property
/// of how much the machine can afford on THIS span, and the search already knows
/// how to find it. Wiring it to the store is the whole of this function:
/// `Sweeper::auto` takes `&[Candle]` and has never cared where the bars came
/// from.
///
/// # Why a span and not a month
///
/// `sweep-stored` sweeps one month, and one month of a coarse rung is too few
/// bars for a threshold to mean anything — 60-minute bars over June 2024 are
/// about 150 rows. The search's answer is only useful over the span that will
/// actually be swept, which is why this takes the same four date arguments
/// `screen` does.
///
/// # What it refuses
///
/// The same things `sweep_stored` refuses and for the same reasons: an
/// unstamped build before any bar is read, an unknown vendor, an unknown rung,
/// a backwards span, and a store that does not hold the months asked for.
#[must_use]
pub fn auto_stored(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
) -> String {
    match auto_stored_inner(vendor_word, underlying, rung, from, to) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// [`auto_stored`]'s body, so every refusal is one `?` rather than a nest.
fn auto_stored_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
) -> Result<String, stored::Refusal> {
    // Before any bar is read, for the reason `sweep_stored_inner` gives: §3
    // rule 3 forbids computation without a recordable identity, so a build that
    // cannot be identified refuses first rather than sweeping and apologising.
    commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and the search will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;
    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let span = stored::load_span(&root, vendor, underlying, rung, from, to)?;

    note(
        &telemetry::Event::info("cli.auto", "threshold search over stored bars")
            .with("feed", vendor.as_str())
            .with("underlying", underlying)
            .with("rung", rung)
            .with("bars", u64::try_from(span.bars.len()).unwrap_or(u64::MAX))
            .with("months_found", u64::from(span.found))
            .with("months_asked", u64::from(span.asked)),
    );

    let mut ev = evaluator().map_err(str::to_owned)?;
    // `min_hits(1)` is the search's FLOOR, not its answer: `Sweeper::auto`
    // brackets from `swept - 1` downward and reports what it settled on. The
    // ceiling is `SEARCH_CEILING` because the search's whole job is to find what
    // fits under it.
    let found = Sweeper::new(Ladder::with_min_hits(1).with_ceiling(SEARCH_CEILING))
        .auto(&span.bars, &mut ev);

    let mut out = String::from(STORED_PROVENANCE);
    out.push('\n');
    // The span, before the search, because "which threshold" is unanswerable
    // without "over how many bars" -- and a span with a HOLE gives a threshold
    // for a shorter sample than the operator asked for.
    let _ = writeln!(
        out,
        "  SPAN  {underlying} {rung} {}-{:02}..{}-{:02}  {} bars over {} of {} months",
        from.0,
        from.1,
        to.0,
        to.1,
        span.bars.len(),
        span.found,
        span.asked
    );
    if span.found < span.asked {
        let _ = writeln!(
            out,
            "  A HOLE: {} month(s) were asked for and not found, so the threshold \
             below is for a SHORTER sample than the span names.",
            span.asked.saturating_sub(span.found)
        );
    }
    out.push('\n');
    out.push_str(&runner::report::render_auto(&found, None));
    Ok(out)
}

/// Resamples the bootstrap takes.
///
/// A stated assumption, in the form `bootstrap::DEFAULT_BLOCK` and
/// `validate::DEFAULT_RUNGS` already use. A thousand is the conventional floor
/// for a test read at 5%: the p-value is a proportion of draws, so its own
/// resolution is `1/draws`, and below about a thousand the answer is quantised
/// coarser than the threshold it is compared against. More draws narrow the
/// Monte Carlo error and cost linearly; nothing in the data says where that
/// trade sits, so this is the assumption and the report prints it.
/// Superseded by [`bootstrap_draws`], which derives the count from the alpha
/// the answer is read at. Kept as the FLOOR that derivation clamps to: a
/// thousand is the conventional minimum for a test read at 5%, and a derived
/// figure below it would be quantised coarser than the threshold it is compared
/// against.
const BOOTSTRAP_DRAWS: usize = 1_000;

/// Resamples to take, DERIVED from the alpha the answer is compared against.
///
/// # A thousand was conventional, and convention is not a derivation
///
/// The p-value is a proportion of draws, so its resolution is `1/draws`. That
/// is the whole constraint, and it is expressible: to read an answer at
/// `alpha`, the resolution must be finer than `alpha` by enough that the
/// quantisation cannot move a verdict across the threshold.
///
/// [`BOOTSTRAP_ALPHA_PPM`] is 50,000 ppm — 5% — so `1/alpha` is 20 draws for a
/// resolution of exactly one alpha, and a hundred times that puts the
/// quantisation two orders of magnitude below the threshold it is compared
/// against. **2,000 draws at 5%**, and an alpha of 1% would take 10,000 without
/// anyone editing a constant.
///
/// # Why it also follows the SAMPLE
///
/// Monte Carlo error falls as `1/sqrt(draws)` and sampling error falls as
/// `1/sqrt(n)`. Draws far below `n` make the resample the binding uncertainty —
/// the test would then be measuring its own resolution rather than the data. So
/// the floor above is raised to the observation count when that is larger, and
/// the two sources of error stay comparable.
///
/// # The ceiling is stated
///
/// 100,000, because the cost is linear and a million draws on a 650,000-bar
/// column is hours spent moving a p-value in its fifth decimal. That bound is
/// chosen; everything above it is derived.
fn bootstrap_draws(observations: usize) -> usize {
    let per_alpha = 1_000_000_usize
        .checked_div(usize::try_from(BOOTSTRAP_ALPHA_PPM).unwrap_or(50_000))
        .unwrap_or(20);
    per_alpha
        .saturating_mul(100)
        .max(observations)
        .clamp(BOOTSTRAP_DRAWS, 100_000)
}

/// The family-wise error rate the stepdown is judged at, in parts per million.
///
/// Fifty thousand ppm is 5%, and it is 5% because the two rows printed beside it
/// — White's Reality Check and Hansen's SPA — already render a `clears 5%`
/// column. One report carrying two different alphas would let a reader compare
/// three numbers that are not comparable, which is the kind of quiet mismatch
/// `crate::report`'s own Bonferroni figure was corrected for.
///
/// Parts per million rather than a float because `CLAUDE.md` §7 keeps this
/// workspace's arithmetic in integers wherever a decision depends on it, and a
/// rejection threshold is such a decision.
const BOOTSTRAP_ALPHA_PPM: u64 = 50_000;

/// How many ranked combinations a stored sweep prints.
///
/// # Why a bound at all, and why this number
///
/// A sweep retains every survivor and there can be tens of millions of them —
/// `crate::rank` exists precisely so that memory is a function of how many
/// results you look at rather than how many exist. That makes this constant the
/// only thing standing between an operator and a report longer than any
/// terminal scrollback, so it is a bound rather than "all of them".
///
/// **Twenty-five is a stated assumption, not a derivation.** No document names a
/// number and nothing in the data implies one, so under `CLAUDE.md` §3 rule 1
/// this is the operator's choice with a default: enough rows that a reader can
/// see where the evidence falls off, few enough that the whole section fits on
/// one screen beside the bar it must clear. `crate::rank::rank` orders by |t|,
/// so these are the twenty-five with the strongest evidence — which is emphatically
/// **not** the twenty-five that are true. The bar printed above them is what
/// decides that, and on a sweep of sixty-one million hypotheses it sits above 6.
const STORED_KEEP: usize = 25;

/// How many ranked combinations the audit weighs before choosing one to trade.
///
/// # Why larger than [`STORED_KEEP`], which only has to be readable
///
/// `STORED_KEEP` bounds a table a person reads. This bounds a SEARCH: the audit
/// takes the strongest candidate that is also closed, so the heap has to be deep
/// enough that a closed combination is still in it after the redundant supersets
/// are filtered out. Too small and the filter empties the list, and the audit
/// reports extinction on a sweep that found plenty — a refusal that would be a
/// lie about the market rather than a fact about it.
///
/// # It was 250, and that number capped the whole search
///
/// [`screen_cap`] decides how many combinations get an exit grid, and this
/// decides how many the screen can even see. Raising the first without the
/// second changes nothing: on a real 15-minute run over 81 months the pipeline
/// read **1,024,058 found, 250 kept, 21 priced**, and the 250 was this constant.
///
/// The cut is made by `|t|` over a LEVEL-LESS return, so it is not a
/// tie-break among equals — it is a different question from the one being
/// asked. A combination that is unremarkable unstopped and excellent under a
/// ten-point stop ranks low here and is discarded before any grid exists to
/// show otherwise.
///
/// So this follows [`screen_cap`] rather than standing beside it, and the two
/// can no longer disagree. `crate::rank` is a bounded min-heap at 80 bytes per
/// entry, so ten thousand is 0.80 MB and a million is 80 MB — O(keep) whatever
/// the sweep produced, which is the property that makes raising it safe.
fn audit_keep() -> usize {
    // At least the screen's own cap: keeping fewer than the screen will price
    // is the defect this replaces, expressed as an invariant rather than as two
    // constants an editor has to remember to move together.
    screen_cap().max(250)
}

/// The exit grid's step, in ppm — ONE INDEX POINT, and the engine chooses it.
///
/// # What this replaces
///
/// Every exit level used to be a quantile of the excursions the combination
/// itself produced. That makes each level relative to how loose the signal is,
/// so on a loose signal the tightest stop the engine would ever try was the
/// 20th percentile — measured at 312 index points on a real 15-minute NIFTY
/// run. Across 1,024,058 combinations, not one was asked how it behaves under
/// anything tighter, and a rule stated in POINTS had no cell to land in.
///
/// # Why one point, and why nothing smaller
///
/// The tick grid is two decimal places (§7), but an index point is the unit a
/// rule is stated in — "no trade beyond twenty points" — and a step finer than
/// the unit multiplies the grid without adding a level anybody would name.
/// Every level is a multiple of it, so a rule at any whole number of points
/// lands exactly on a rung rather than between two.
///
/// # It is derived, not typed
///
/// [`points_to_ppm`] converts against [`NIFTY_REFERENCE`], which is a stated
/// approximation the report names on its own page. No operator supplies this
/// and no percentage appears in it: `grid_rungs(bars)` alone decides how far the
/// ladder reaches, so raising the depth extends 1pt…4pt to 1pt…25pt without
/// moving a single level that was already tried.
/// The exit ladder's step, in ppm, DERIVED from the bars it will sweep.
///
/// # It was `points_to_ppm(1) / 2` against a constant
///
/// Half an index point sounds instrument-neutral and is not: half a point is
/// 20 ppm on a 25,000 index and 9.6 ppm on a 52,000 one, so the same constant
/// gives BANKNIFTY a ladder twice as fine as NIFTY's in the unit the engine
/// actually measures in. The reach differs with it.
///
/// Reading the reference off the bars fixes both: half a point is half a point
/// on whatever was swept, at the level it traded.
///
/// # Half a point, and why the fraction is not derived too
///
/// A point is the unit a rule is SPOKEN in -- "no trade beyond ten points" --
/// and half of it is the finest division that still lands on a level anyone
/// would name. Finer multiplies the grid without adding a rung an operator
/// would state, and the tick is finer still at 0.05. This is the one figure
/// here that is a choice about language rather than about data, and it is
/// stated as one.
/// The tightest stop worth testing, in index points.
///
/// # Why five and not a half
///
/// A derived step reached down to fractions of a point, and a half-point stop on
/// one-minute execution is not a stop — it is inside the bar it would be placed
/// on. Both legs fill at the extremes the bar PRINTED, so a level closer than a
/// typical bar's own range is hit by the entry bar itself, and the grid spends
/// its cells on trades that could not have been taken.
///
/// The operator's own reading, and it is a statement about the instrument rather
/// than about preference: on one-minute NIFTY the worst case is ten to
/// twenty-five points, so a floor of five is the tightest level that survives
/// contact with a real fill.
///
/// # Stated, not derived, and that is the honest label
///
/// This is domain knowledge about how the instrument fills. No amount of
/// arithmetic over the bars produces "five points" — the median bar range
/// produces a RESOLUTION, which is a different quantity, and using it as a floor
/// was the mistake this replaces.
const STOP_FLOOR_POINTS: i64 = 5;

/// The stop ladder in ppm: `5, 7.5, 10 … 25` points, at the measured price.
///
/// # Why the stops no longer share the general step
///
/// Every axis used one derived step, which made the stop ladder a function of
/// the instrument's median bar range. That is the right quantity for a
/// RESOLUTION and the wrong one for a FLOOR: it reached below a point on a quiet
/// instrument, and those rungs are unfillable on one-minute execution.
///
/// The stop is the one axis with a real physical floor, so it gets its own
/// ladder. Targets and trails keep the derived step, because there is no reason
/// a target cannot be far and every reason to resolve it finely.
///
/// # Halves without floating point
///
/// Points are counted in HALVES so 7.5 is expressible in integers — §7 keeps
/// floats out of anything compared, and a ladder printed beside a rule is
/// compared by the person reading it.
fn stop_ladder_ppm(bars: &[indicators::Candle]) -> Vec<i64> {
    let reference = reference_price(bars);

    // IN HALVES DERIVED FROM PAISA, NOT FROM ROUNDED POINTS.
    //
    // These read `max_stop_points(bars) * 2` and `stop_floor_points(bars) * 2`,
    // and both of those divide paisa by `PAISA_PER_POINT` FIRST -- a hundred to
    // one -- so the whole range distribution was collapsed into whole points
    // before the span was measured. On a series whose 25th percentile bar range
    // is 120 paisa and whose 90th is 195, both round to ONE point, floor equals
    // cap, and the loop below runs exactly once: the one-rung collapse, arriving
    // by a third route after the identical-expressions route and the
    // `MAX_STOP_POINTS`-clamp route were closed. `synthetic::bar` produces
    // precisely that shape, so every synthetic fixture in this crate was still
    // exercising a one-rung ladder.
    //
    // Reading the percentiles in paisa and converting to halves here keeps the
    // resolution the instrument actually has. The points-rounded functions stay
    // for the callers that speak to an operator in points.
    let halves_of = |paisa: i64| {
        paisa
            .saturating_mul(2)
            .checked_div(PAISA_PER_POINT)
            .unwrap_or(0)
    };
    let cap_halves = range_percentile(bars, 9, 10)
        .map_or_else(|| max_stop_points(bars).saturating_mul(2), halves_of)
        .max(1);
    let floor_halves = range_percentile(bars, 1, 4)
        .map_or_else(|| stop_floor_points(bars).saturating_mul(2), halves_of)
        .max(1)
        // A floor above its own cap would leave the loop empty. Both come from
        // one sorted distribution so this cannot fire on real bars; it is the
        // guard that makes that argument checkable rather than assumed.
        .min(cap_halves);

    // THE STEP IS THE SPAN OVER THE RUNG COUNT, AND BOTH COME FROM THE BARS.
    //
    // This advanced by `STOP_STEP_POINTS_HALVES` -- a fixed 2.5 index points,
    // the same distance on NIFTY at 25,000 and on BANKNIFTY at 52,000, and the
    // same distance in 2020 as in 2026. A ladder whose SPAN is derived from the
    // instrument's own bar ranges but whose STEP is a constant does not scale
    // with the instrument: on a quiet series it emits one or two rungs, and on
    // a violent one it emits dozens, none of which anybody chose.
    //
    // `grid_rungs` already answers "how many rungs can this grid afford", and
    // it is derived -- cap over the median-range step. Dividing the span by it
    // gives a step that moves with the instrument AND keeps the cell count
    // where the budget put it, which is the only thing a constant step was ever
    // protecting.
    //
    // `max(1)` because the span can be smaller than the rung count on a very
    // tight series, and a zero step is an infinite loop rather than a fine
    // ladder.
    let rungs = i64::try_from(grid_rungs(bars).max(1)).unwrap_or(1);
    let step_halves = cap_halves
        .saturating_sub(floor_halves)
        .checked_div(rungs)
        .unwrap_or(1)
        .max(1);

    let mut out: Vec<i64> = Vec::with_capacity(16);
    let mut halves = floor_halves;
    while halves <= cap_halves {
        // `points_to_ppm_at` takes whole points, so the halves are converted by
        // taking the ppm of a whole point and halving it -- exact in integers.
        let ppm = points_to_ppm_at(halves, reference) / 2;
        if ppm > 0 {
            out.push(ppm);
        }
        halves = halves.saturating_add(step_halves);
    }
    // A cap below the floor leaves nothing; one rung at the floor is still a
    // ladder and refusing here would drop the whole grid for a quiet
    // instrument.
    if out.is_empty() {
        out.push(points_to_ppm_at(stop_floor_points(bars), reference).max(1));
    }
    out
}

/// The tightest stop worth trying, DERIVED from how far this instrument moves.
///
/// # Why five points was a NIFTY number wearing a general name
///
/// [`STOP_FLOOR_POINTS`] is 5, and its own doc gives the reason a floor exists:
/// *"a level closer than a typical bar's own range is hit by the entry bar
/// itself, and the grid spends its cells on trades that could not have been
/// taken."* That reason is exactly right — and it is a statement about the
/// INSTRUMENT'S BAR RANGE, which is measurable.
///
/// The doc then concluded *"no amount of arithmetic over the bars produces five
/// points"*, and that is the step this replaces. Five is the answer for NIFTY
/// one-minute bars. **BANKNIFTY can travel twenty-five points inside a single
/// minute**, so on BANKNIFTY a five-point stop is the very thing the floor was
/// built to refuse: inside the entry bar, hit before the trade has begun. The
/// constant did not generalise, and nothing said so.
///
/// # What is measured
///
/// The **median** bar range. A stop at the median is hit by the entry bar about
/// half the time, which is the boundary between a stop that can be placed and
/// one that cannot — so it is the floor, not a recommendation. Median rather
/// than mean for the reason [`max_stop_points`] gives: one 2020 session moved
/// further than a hundred ordinary ones, and a mean lets that session set the
/// ladder for the other eighty months.
///
/// # Floors on the floor
///
/// At least one point, because a ladder needs somewhere to start and a stop
/// below the tick grid is not a price.
///
/// **There is no upper clamp any more, and its removal is the point.** This
/// sentence used to promise "at most [`MAX_STOP_POINTS`]'s own cap" and the
/// code applied one — which is exactly how the ladder collapsed by a second
/// route: on a violently wide instrument the 25th AND 90th percentiles both
/// exceeded twenty-five points, both clamped to twenty-five, floor equalled cap,
/// and the `while halves <= cap_halves` loop ran once. A cap expressed in NIFTY
/// points cannot bound a ladder whose ends are percentiles of whatever
/// instrument is in front of it.
///
/// A floor above its own ceiling is impossible without the clamp rather than
/// because of it: both ends come from the same sorted distribution, so the 25th
/// percentile cannot exceed the 90th. [`MAX_STOP_POINTS`] survives only as the
/// fallback for a slice where no bar has any range at all, where there is no
/// distribution to read a percentile from.
/// One point of the sorted bar-range distribution, in POINTS.
///
/// # Why this exists, and what it fixes
///
/// [`stop_floor_points`] and [`max_stop_points`] were **byte-identical**: same
/// filter, same sort, same median index, same divisor, same clamp, differing
/// only in the fallback for an empty slice. So on every real series
/// `floor == cap`, [`stop_ladder_ppm`]'s `while halves <= cap_halves` ran
/// exactly ONCE, and the shipped stop ladder had a single rung —
/// a fixed 2.5-point step never advancing it. The doc above
/// [`stop_ladder_ppm`] claimed a span of *"5, 7.5, 10 … 25"* that no run has
/// ever walked, and the test named for that span asserts nothing about its
/// length: its ordering check is `rungs.windows(2).all(..)`, which is vacuously
/// true on one element.
///
/// A floor and a ceiling must come from DIFFERENT places in the distribution or
/// they are not a floor and a ceiling. Both now do, and both are still derived
/// from this instrument's own bars rather than from a constant.
///
/// `numerator/denominator` is the fraction of the way through the sorted
/// ranges. Percentiles rather than multiples of the median, because a multiple
/// is a figure somebody chose and a percentile is a figure the data reports.
fn range_percentile(
    bars: &[indicators::Candle],
    numerator: usize,
    denominator: usize,
) -> Option<i64> {
    let mut ranges: Vec<i64> = bars
        .iter()
        .map(|b| b.high.saturating_sub(b.low))
        .filter(|&r| r > 0)
        .collect();
    if ranges.is_empty() {
        return None;
    }
    ranges.sort_unstable();
    // `len - 1` so the last percentile addresses the last element rather than
    // one past it; the `min` is belt and braces for a caller passing n > d.
    let at = ranges
        .len()
        .saturating_sub(1)
        .saturating_mul(numerator)
        .checked_div(denominator)
        .unwrap_or(0)
        .min(ranges.len().saturating_sub(1));
    ranges.get(at).copied()
}

/// [`range_percentile`] rounded to whole index points.
///
/// **The rounding is the reason the two are separate.** A bar range is paisa,
/// and points are paisa over [`PAISA_PER_POINT`] — a hundred to one. Dividing
/// FIRST and comparing after collapses the distribution: on a series whose
/// 25th percentile bar range is 120 paisa and whose 90th is 195, both round to
/// ONE point, floor equals cap, and `stop_ladder_ppm`'s
/// `while halves <= cap_halves` runs exactly once. That is the one-rung collapse
/// this whole family was rewritten to fix, arriving by a third route after the
/// identical-expressions route and the `MAX_STOP_POINTS`-clamp route were both
/// closed.
///
/// It is not hypothetical: `synthetic::bar` produces exactly that shape, which
/// is why `the_stop_ladder_spans_the_points_its_constants_name` cannot express
/// a ladder and a separate fixture had to be built for the span assertion.
///
/// So the LADDER reads paisa and keeps its resolution, and callers that speak
/// to an operator in points — the report, `grid_rungs`' reach — round here,
/// once, at the edge.
fn range_percentile_points(
    bars: &[indicators::Candle],
    numerator: usize,
    denominator: usize,
) -> Option<i64> {
    range_percentile(bars, numerator, denominator).map(|paisa| paisa / PAISA_PER_POINT)
}

/// The TIGHTEST stop worth testing: the 25th percentile of the bar range.
///
/// A stop inside the quarter of bars that barely move is a stop the instrument's
/// ordinary noise takes out, and pricing it teaches nothing. Below that the
/// ladder would spend every rung measuring the same stop-out.
fn stop_floor_points(bars: &[indicators::Candle]) -> i64 {
    let Some(points) = range_percentile_points(bars, 1, 4) else {
        // No bar has a range, so nothing about this instrument is measurable.
        // The stated NIFTY figure is the honest fallback and is named as one.
        return STOP_FLOOR_POINTS;
    };
    points.max(1)
}

fn grid_step_ppm(bars: &[indicators::Candle]) -> i64 {
    // NO PRICE, NO POINTS, NO REFERENCE -- THE LADDER IS SIZED IN THE UNIT THE
    // ENGINE ALREADY MEASURES IN.
    //
    // This was `points_to_ppm_at(1, reference_price(bars)) / 2`: half an index
    // point, converted at a price read off the bars. Better than a constant and
    // still asking the wrong question -- a POINT is a unit of the instrument's
    // quote, and the engine measures in ppm of each trade's own entry.
    //
    // The scale that needs no translation is the instrument's own movement.
    // Each bar's range as a fraction of its own close IS a ppm figure, and the
    // median of those is what a typical bar does, on any instrument, at any
    // price level, with nothing converted.
    //
    // Divided so the ladder resolves WITHIN a typical bar rather than stepping
    // past it: a stop coarser than one bar's usual move can only sit outside
    // it. Twenty is the divisor, and it is the one figure here that is chosen
    // -- chosen as a RESOLUTION, not as a price, so it is right on an
    // instrument that moves eight points a day and on one that moves four
    // hundred.
    let mut ranges: Vec<i64> = bars
        .iter()
        .filter(|b| b.close > 0)
        .map(|b| {
            let span = i128::from(b.high.saturating_sub(b.low));
            let ppm = span.saturating_mul(1_000_000) / i128::from(b.close);
            i64::try_from(ppm).unwrap_or(i64::MAX)
        })
        .filter(|&r| r > 0)
        .collect();
    if ranges.is_empty() {
        // No bar moved, so there is no scale to read. One ppm is the finest
        // non-zero rung and the sweep will find nothing either way -- a refusal
        // here would stop a report that has other things to say.
        return 1;
    }
    ranges.sort_unstable();
    let median = ranges.get(ranges.len() / 2).copied().unwrap_or(1);
    // THE DIVISOR IS THE ONE CHOSEN FIGURE HERE, AND IT IS NO LONGER BAKED.
    //
    // Everything above this line is measured: each bar's range as a fraction of
    // its own close is already a ppm figure, and the median of those is what a
    // typical bar does on any instrument at any price level. Only the divisor
    // is a decision, and the comment above states exactly what kind — a
    // RESOLUTION, "so the ladder resolves WITHIN a typical bar rather than
    // stepping past it".
    //
    // Twenty means twenty steps across a typical bar's range. Finer resolves
    // stops a trader could not distinguish and multiplies the grid; coarser
    // steps past the bar the stop has to sit inside. Nothing in the data
    // decides between them, which is exactly why it must be movable rather
    // than argued once and frozen: `BRUTEX_GRID_RESOLUTION` moves it at runtime
    // and every ladder on every rung re-derives around the new value, with no
    // rebuild.
    //
    // Refused at zero because a zero divisor is a panic, and refused below one
    // for the same reason `.max(1)` guards the result: a step of zero is an
    // infinite ladder, not a fine one.
    let resolution = crate::knobs::var("BRUTEX_GRID_RESOLUTION")
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(20);
    median.checked_div(resolution).unwrap_or(median).max(1)
}

/// The reward-to-risk ratios the grid pairs each stop with, in hundredths.
///
/// # Why these and not a cross product
///
/// Pairing every stop with every target at a half-point step out to twenty
/// points is 27,637,321 cells per combination -- 226 trillion over the 8.19
/// million combinations 81 months produce. Almost all of it is trades nobody
/// would take: a twenty-point stop against a half-point target is not a
/// strategy, it is a cell.
///
/// An operator states a RATIO. So each stop is paired with the ratios worth
/// trading and nothing else, which is about 1,440 cells for the same reach.
///
/// # The range, and why it reaches 1:8
///
/// 1:1 is the floor a reader needs for comparison -- a strategy at 1:1 is what
/// the others are better than. The top is 1:8 because the goal is the MINIMAL
/// stop against the MAXIMUM win: at a half-point stop, 1:8 is a four-point
/// target, and a combination that hits that reliably is exactly the rare shape
/// being searched for. Stopping at 1:3 would refuse to look for it.
///
/// Ten values, so the grid is ten targets per stop rather than forty, and the
/// collapse is 4x rather than 6.7x -- bought back by reaching further into the
/// region where the answer would be.
/// THE STOP IS BOUNDED AND NOTHING ELSE IS.
///
/// A stop is a promise about the WORST case, so it has a ceiling: an operator
/// who says "no trade may run more than twenty-five points against me" means it
/// absolutely, and a grid rung past that is a rung that breaks the promise.
///
/// A target is the opposite. There is no reason to cap what a winner may make,
/// and capping it is how a search stops looking for the shape being searched
/// for -- a half-point stop against a hundred-point target is exactly the rare
/// asymmetry worth finding, and a ratio ladder stopping at 1:8 cannot express
/// it. So `grid_ratios` reaches 1:150 and the stop ladder is capped here.
const MAX_STOP_POINTS: i64 = 25;

/// The furthest a stop rung may sit, in points, DERIVED from the bars.
///
/// # A typed cap is a rule about an instrument nobody named
///
/// Twenty-five points is a sensible ceiling on NIFTY and a meaningless one on
/// an instrument that moves eight points a day or four hundred. The cap exists
/// to stop the ladder wasting rungs on stops no trade would ever reach — so the
/// figure that decides it is how far this instrument actually moves.
///
/// # One day's typical range
///
/// The median of `high - low` over the swept bars, in points. A stop wider than
/// a typical bar's whole range is a stop that only fires on the rare bar, and a
/// ladder reaching past it prices rungs against nothing.
///
/// Median rather than mean: one 2020 session moved further than a hundred
/// ordinary ones, and a mean lets that session set the ladder for the other
/// eighty months.
///
/// # Floors and ceilings, both stated
///
/// At least 1 point, so a quiet instrument still gets a ladder. At most
/// [`MAX_STOP_POINTS`], because a stop is a promise about the worst case and an
/// operator who says twenty-five means it — the derivation may narrow that
/// promise, never widen it.
/// The LOOSEST stop worth testing: the 90th percentile of the bar range.
///
/// A stop wider than nine bars in ten is one that a single ordinary bar cannot
/// reach, so above this the ladder is measuring the time exit rather than the
/// stop. The tenth percentile left outside is the tail of violent bars, and
/// sizing a stop to survive those is a different question from the one the
/// ladder asks.
///
/// # What this used to be
///
/// A BAR RANGE IS ALREADY PAISA, SO NO REFERENCE IS INVOLVED. This once read
/// `ppm_to_points_at(points_to_ppm_at(median, reference), reference)` — a round
/// trip through both converters against the same reference, which is the
/// identity up to integer truncation. So the result was the median bar range IN
/// PAISA, and a NIFTY minute bar's few-hundred-paisa range clamped to
/// [`MAX_STOP_POINTS`] on every real series: a function documented as derived
/// from the data returned the constant it was meant to replace.
///
/// That was fixed to `median / PAISA_PER_POINT` — correct arithmetic, and
/// **the same expression [`stop_floor_points`] had**, which is how the ladder
/// came to have one rung. See [`range_percentile`].
fn max_stop_points(bars: &[indicators::Candle]) -> i64 {
    let Some(points) = range_percentile_points(bars, 9, 10) else {
        return MAX_STOP_POINTS;
    };
    points.max(1)
}

/// Reward-to-risk ratios, in hundredths, DENSE where strategies live and
/// SPARSE where the exceptional ones do.
///
/// # Why it reaches 1:150
///
/// At a one-point stop, 1:8 is an eight-point target. The operator is asking
/// for a minimal stop against a MAXIMISED win -- a hundred or a hundred and
/// fifty points -- and no ratio under 1:100 can express that against a stop
/// that small. A ladder that stops at 1:8 has decided the answer is not there.
///
/// # Why the spacing widens
///
/// Between 1:1 and 1:3 a quarter-step is a different strategy. Between 1:100
/// and 1:110 it is nothing -- no combination distinguishes them, and pricing
/// both spends the grid on a rung nobody would name. So the step grows with
/// the ratio, which is the same reasoning `win_rate_rungs` uses.
///
/// Generated rather than listed: a list is a set of numbers somebody chose, and
/// every one of them would have to be retyped to reach further.
fn grid_ratios() -> Vec<i64> {
    let mut out: Vec<i64> = Vec::with_capacity(32);
    let mut r = 100_i64;
    while r <= 15_000 {
        out.push(r);
        r += if r < 300 {
            25
        } else if r < 1_000 {
            100
        } else if r < 3_000 {
            250
        } else {
            2_500
        };
    }
    out
}

/// What the grid costs at each depth, measured rather than assumed.
///
/// The exit grid is a CROSS PRODUCT of three ladders plus the armed
/// trailing-profit rows, so its width grows far faster than the ladder does.
/// Counted with `grid::variants(r, r, r)`:
///
/// | rungs | variants per combination | reach at a half-point step |
/// |---|---|---|
/// | 4 | 625 | 2 pt |
/// | 8 | 12,393 | 4 pt |
/// | 10 | 34,606 | 5 pt |
/// | 16 | 319,345 | 8 pt |
/// | 20 | 935,361 | 10 pt |
/// | 40 | **27,637,321** | 20 pt |
///
/// This is the wall, and it is arithmetic rather than an implementation limit.
/// A half-point step reaching twenty points needs forty rungs, and forty rungs
/// is twenty-seven million cells FOR ONE COMBINATION. The engine prices 250 of
/// them per run today, which would be seven billion cells.
///
/// So depth is a real choice with a real cost, and the constant above is the
/// step rather than the reach precisely because the two must be decided
/// separately: the step is what makes a level nameable, and the reach is what
/// the machine can afford.
const _GRID_COST_TABLE: () = ();

/// How many exit settings the audit's grid actually evaluates.
///
/// `grid::variants` is `(S+1)·[ (T+1) + R·(T+1)(T+2)/2 ]`, which at four rungs
/// is **325 cells and not 256**.
///
/// # This number moved TWICE, and the build caught it both times
///
/// D-0241 added the ARMING axis — a trailing take profit, being a trail that
/// does not start until the move has already paid. The axis indexes the target
/// ladder, so a naive fourth factor reads 625.
///
/// The first attempt refused one family and landed on 525: an arm with no trail
/// to arm is inert, so those 100 cells duplicate the trail-less ones. **An
/// adversarial read then found a second family**, and it was the dangerous one —
/// an arm at or above the target rung can never fire, because the target closes
/// the position first. Those 200 cells were duplicates too, and
/// `Grid::best`'s `max_by_key` returns the LAST maximum, so a tie was won by the
/// highest arm index: the audit would have printed a trailing take profit for a
/// run in which nothing armed. Refusing both leaves 325.
///
/// Nothing here had to be noticed either time. The assertion below **failed the
/// build** the moment `grid::variants` changed, which is the whole reason it is
/// written as a literal proved against the function rather than computed from
/// it: the workspace denies `as` casts and `u64::try_from` is not `const`, so
/// deriving a `u64` from that `usize` inside a `const` is not expressible — but a
/// compile-time equality is. A stale figure here would make the printed exposure
/// charge for a grid that was never run.
/// The exit grid's width, DERIVED from the rung count rather than restated.
///
/// # Why this stopped being a constant
///
/// It was `625`, then `12_393`, each pinned to `grid_rungs(bars)` by a `const`
/// assertion — a good guard, and it forced the two to move together. But the
/// rung count is the REACH: the ladder is stepped at half an index point, so
/// four rungs stop at 2.0 points and eight reach 4.0, and no fixed number is
/// the right one. An operator whose rule is "no trade beyond ten points" needs
/// twenty rungs; one exploring needs four. Choosing for them is the defect
/// `CLAUDE.md` §6 describes in a different place — a parameter that can be set
/// wrongly and silently.
///
/// So the width follows the depth at runtime and cannot drift from it, which is
/// what the assertion was protecting.
fn grid_variants(bars: &[indicators::Candle]) -> u64 {
    let n = grid_rungs(bars);
    u64::try_from(grid::variants(n, n, n)).unwrap_or(u64::MAX)
}

/// How many rungs each exit ladder carries — the REACH, in half-points.
///
/// # The cost, measured with the engine's own counter
///
/// The grid is a cross product, so width grows roughly cubically in this while
/// reach grows linearly:
///
/// | rungs | reach | cells / combination | cells over 8.19M combinations |
/// |---|---|---|---|
/// | 4 | 2.0 pt | 625 | 5.1 billion |
/// | 8 | 4.0 pt | 12,393 | 101.5 billion |
/// | 10 | 5.0 pt | 34,606 | 283 billion |
/// | 20 | 10.0 pt | 935,361 | 7.6 trillion |
/// | 40 | 20.0 pt | 27,637,321 | 226 trillion |
///
/// Every row is affordable in MEMORY — grid width is per-combination work, not
/// retained state, and `crate::rank`'s heap is sized by [`screen_cap`] alone.
/// What it costs is TIME, and that is the operator's to spend.
///
/// `BRUTEX_GRID_RUNGS` sets it. The default is eight because it is the deepest
/// rung count whose full-search cost was actually measured on this machine;
/// it is a starting point, not a ceiling, and nothing in the engine treats it
/// as one.
fn grid_rungs(bars: &[indicators::Candle]) -> usize {
    // THE REACH IS THE CAP, SO THE RUNG COUNT FOLLOWS IT.
    //
    // # The contradiction this closes
    //
    // `max_stop_points` says the stop may reach twenty-five points. The rung
    // count was a fixed eight, and the step is derived from the bars, so the
    // ladder's actual reach was `8 * step` -- whatever that happened to be.
    // On 15-minute NIFTY the step derives to about 3.75 points, so eight rungs
    // reached thirty and blew past the cap; on a quiet instrument the same
    // eight would have stopped far short of it.
    //
    // Either way the reach was an ACCIDENT of two numbers that never spoke to
    // each other. The cap is the statement about how far a stop may sit, the
    // step is the resolution the data supports, and the rung count is simply
    // one divided by the other.
    //
    // # An override still exists, and it overrides the REACH
    //
    // `BRUTEX_GRID_RUNGS` sets the count outright for an operator who wants a
    // deeper grid than the cap implies -- the cost table on `Levels::rungs`
    // says what that buys and what it costs. Absent it, nothing here is chosen.
    if let Some(n) = crate::knobs::var("BRUTEX_GRID_RUNGS")
        // `knobs::var` already answers `Option`, so there is no `Result` to unwrap.
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|&n| n > 0)
    {
        return n;
    }
    let reference = reference_price(bars);
    let step_points = ppm_to_points_at(grid_step_ppm(bars), reference).max(1);
    let cap = max_stop_points(bars);
    // At least two rungs, so a ladder always offers a tighter and a looser
    // choice rather than one level dressed as a grid.
    let from_the_data = usize::try_from(cap / step_points).unwrap_or(2).max(2);
    // AND NO MORE THAN THE MACHINE CAN HOLD. See `rungs_within_cell_budget`:
    // `cap / step_points` was bounded only by `MAX_STOP_POINTS` clamping `cap`,
    // and removing that clamp -- correct in itself, it was collapsing the stop
    // ladder -- left this division with no upper bound at all.
    from_the_data.min(rungs_within_cell_budget())
}

/// The most rungs whose grid still fits the memory this machine can spare.
///
/// # The blow-up this bounds, which two separate fixes composed into
///
/// [`runner::grid::variants`] is `(S+1)·[(T+1)(R+1) + (T(T+1)/2)·(R(R+1)/2)]` —
/// **quadratic in targets AND in trails** — and `ratio_targets` grows the
/// target ladder with the stop ladder, so the true shape is about `n^5/4`.
/// `grid::evaluate` reserves that count UP FRONT with `Vec::with_capacity`, so
/// the reservation is the allocation whether or not the cells are ever pushed.
///
/// | rungs | cells | at ~152 B each |
/// |---|---:|---:|
/// | 4 | 625 | 95 KB |
/// | 8 | 12,393 | 1.9 MB |
/// | 16 | 319,345 | 49 MB |
/// | 40 | 27,637,321 | **4.2 GB, per candidate** |
///
/// Two changes made forty reachable and neither considered the other. Removing
/// the `MAX_STOP_POINTS` clamp from [`max_stop_points`] was right — the clamp
/// was collapsing the stop ladder to one rung — but that clamp was also the
/// only thing bounding `cap / step_points` above. And `screen` was
/// parallelised, so `available_parallelism` of these grids are live at once.
/// The candidate ceiling covers none of it: [`derived_ceiling`] counts APRIORI
/// candidates, and an exit grid is transient per-candidate work it never sees.
///
/// # Derived, not typed
///
/// [`derived_ceiling`] already answers *"how many 146-byte records fit in this
/// machine's share"* — scaled by core count and divided among concurrent rungs
/// by `SharedBy`. A `Cell` is about the same width, and a grid is TRANSIENT
/// where a candidate is RETAINED, so a grid gets a small fraction of that: one
/// part in 1,024, split again across the threads each holding one.
///
/// On the reference machine that is roughly nine thousand cells per candidate,
/// solving to seven or eight rungs — the range the old constant already sat in,
/// now reached by arithmetic rather than by a number that happened to hold. A
/// larger machine earns more; a smaller one is protected.
///
/// Solved by walking upward rather than inverting a quintic: the answer is
/// small, the walk is bounded, and `variants` is a `const fn` of a few
/// multiplies.
fn rungs_within_cell_budget() -> usize {
    /// A grid is transient per-candidate work rather than retained state, so it
    /// takes a thousandth of the retained budget instead of a share of it.
    const GRID_SHARE: usize = 1_024;
    /// Past this the quintic makes each further rung absurd, and a bound that
    /// cannot terminate is not a bound.
    const SEARCH_LIMIT: usize = 64;
    /// A ladder always offers a tighter and a looser choice, whatever the
    /// budget says — one level is not a grid.
    const FLOOR: usize = 2;

    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let budget = derived_ceiling()
        .checked_div(GRID_SHARE)
        .and_then(|share| share.checked_div(threads))
        .unwrap_or(0);

    let mut best = FLOOR;
    for n in FLOOR..=SEARCH_LIMIT {
        if runner::grid::variants(n, n, n) > budget {
            break;
        }
        best = n;
    }
    best
}

/// The strongest combination by evidence that is also **closed**.
///
/// # Why not simply the first one, which is what this replaced
///
/// The audit used to trade `closed::closed(&sweep).kept.first()`. `closed`
/// builds `kept` from `Sweep::all_frequent`, which is level-ascending and then
/// discovery order within a level — an ordering `crates/engine` states outright
/// is **not a ranking**. So `first()` was normally whichever k=1 bit happened to
/// survive first, and every figure downstream of it — the trades, the 125-cell
/// exit grid, the walk-forward, the PBO, the bootstrap — described that
/// arbitrary singleton.
///
/// # Why the closed set is still applied, as a filter
///
/// Evidence order alone is not enough. A superset with the same support as its
/// subset adds a condition that changed nothing, so trading it would report a
/// k=3 result that is really a k=1 one wearing two extra names. `closed` removes
/// exactly those. Ranking first and filtering second gives the strongest
/// candidate that is also irredundant — which neither ordering gives alone.
///
/// # Cost — and the half of it this comment used to deny
///
/// **Only the second half is bounded by `keep`.** This block first read: *"Both
/// are bounded by `keep`, not by how many combinations the sweep produced."* The
/// filter over `ranked.top` is — that is `keep` probes, 250 today. The
/// `closed::closed` call is **not**, and it is the expensive one: `closed.rs`
/// builds a `HashSet` pre-sized to *every* frequent itemset, a `HashMap` per
/// level, and a `kept` Vec of every closed set, then this function builds a
/// second `HashSet` over that. Its own doc states the shape — `O(Σ |F_k| · k)`,
/// and `UNVERIFIED as a measured figure`.
///
/// The scale that makes the difference matter: `crate::rank`'s header prices
/// 61,125,295 retained combinations at 13 GB, and the ladder's own per-level
/// ceiling is `1 << 26`. A comment claiming a 12 KB bound on a path that can
/// allocate gigabytes is exactly the shape `CLAUDE.md` §4 refuses — the number
/// was not measured, it was assumed from the wrong term.
///
/// So, honestly: **the closed walk is `O(sum of |F_k| times k)` in time and
/// `O(|F|)` in space**,
/// then O(`keep`) probes. It runs **once per audit run**, between the findings
/// table and the traded line — not per bar, not per candidate, and not on any
/// HTTP path, because `CLAUDE.md` §5 gives `api` no `runner` arrow.
fn closed_by_evidence<'a>(
    ranked: &'a runner::rank::Ranked,
    sweep: &engine::Sweep,
) -> Vec<&'a runner::rank::Scored> {
    let kept: std::collections::HashSet<_> = closed::closed(sweep)
        .kept
        .iter()
        .map(|item| item.mask)
        .collect();
    ranked
        .top
        .iter()
        .filter(|scored| kept.contains(&scored.mask))
        .collect()
}

/// How many exit settings the audit also searched, and what that costs the bar.
///
/// # The axis the significance section does not charge for
///
/// `report::render`'s SIGNIFICANCE block computes its Bonferroni bar from
/// `significance::trials`, which counts one thing: how many condition
/// COMBINATIONS had their support measured. That is the whole search for
/// `sweep-stored`, which ranks on forward returns and builds no grid.
///
/// It is not the whole search here. This command evaluates the chosen
/// combination at [`grid_variants(bars)`] stop/target/trail settings and keeps the
/// best of them, and selecting a maximum over 125 cells is 125 more chances to
/// look good by luck. None of it entered the bar printed above.
///
/// # Why a second bar rather than a replacement
///
/// Because the true correction is **unknown and this one is only a ceiling**.
/// The cells share a single trade walk, so they are heavily correlated and the
/// effective trial count is somewhere between 1 and [`grid_variants(bars)`] —
/// unmeasured.
/// Replacing the printed bar with the ceiling would reject real findings;
/// leaving it alone accepts noise. Printing both, and saying which is which,
/// hands the reader the range that is actually known. `CLAUDE.md` §3 rule 6
/// asks for exactly that when a bound cannot be met, and §3 rule 1 forbids
/// inventing the discount that would collapse the range to a point.
fn grid_exposure(sweep: &engine::Sweep, bars: &[indicators::Candle]) -> String {
    let plain = runner::significance::effective_trials(sweep);
    // THE SAME `plain` ON BOTH SIDES OF THE SENTENCE.
    //
    // This read `trials_with_grid(sweep, grid_variants(bars))`, which multiplied the
    // RAW trial count while the line beside it printed the DUPLICATE-DEFLATED
    // one -- so the sentence "with N exit settings each, at most {ceiling}"
    // claimed the only difference was the grid, and the measured ratio was
    // 929,577x where it promised 325x. The upper Bonferroni bar was overstated
    // by 1.27 t-units as a result.
    //
    // Passing `plain` makes the claim true by construction rather than by two
    // calls happening to agree, and `trials_with_grid` now takes a count for
    // exactly that reason.
    let ceiling = runner::significance::trials_with_grid(plain, grid_variants(bars));
    let mut out = String::with_capacity(512);
    let _ = writeln!(
        out,
        "\nEXIT-GRID EXPOSURE\n  \
         combinations weighed {plain}\n  \
         with {width} exit settings each, at most {ceiling}\n  \
         t must clear {:.2} on the combination axis alone\n  \
         t must clear {:.2} if every exit setting were an independent trial\n\
         \n  \
         The truth is between them and is NOT measured: the {width} cells \
         share one trade walk, so they are correlated rather than independent. \
         The upper figure cannot be cleared by luck; the lower one can.",
        runner::significance::bonferroni_t(plain),
        runner::significance::bonferroni_t(ceiling),
        width = grid_variants(bars),
    );
    out
}

/// Sessions below which the institutional stack reports at a resolution it does
/// not have.
///
/// # Where the number comes from
///
/// It is not invented, and it is not a preference. It is the arithmetic the
/// three consumers of the series force:
///
/// * the walk-forward splits into [`WALK_FORWARD_SPLITS`] anchored folds, so a
///   fold's TEST window is roughly `sessions / splits`;
/// * the bootstrap resamples in stationary blocks of
///   [`runner::bootstrap::DEFAULT_BLOCK`], so a draw is roughly
///   `sessions / block` blocks;
/// * a `t` is not judged at all below `MIN_OBSERVATIONS` in `runner::report`,
///   for the same reason a normal quantile cannot rule on a small-sample
///   Student-t.
///
/// At 50 sessions a fold tests on ~10 and a draw is ~5 blocks: thin, and
/// arguably reportable. Below it, a draw is one or two blocks and a fold tests
/// on a handful of days, at which point the resampling has almost no
/// independent structure left to resample.
///
/// One stored instrument-month is **about 20 trading days**, so `audit-stored`
/// on a single month is always below this. That is not a reason to hide the
/// number — it is the reason to print the warning.
const MIN_AUDIT_SESSIONS: usize = 50;

/// What the sample is worth, said beside the figures rather than after them.
///
/// # Why this warns rather than refuses
///
/// `CLAUDE.md` §3 rule 6 requires an unmeetable bound to be said out loud, and
/// §4 requires a degradation to name its reason. Neither asks for a refusal
/// here: the trades, the exit grid and the excursions are all perfectly
/// meaningful on twenty sessions — it is the *p-values and the PBO* that are
/// not, and refusing the whole report would throw away the honest half to
/// suppress the dishonest half.
///
/// What was actually wrong before this existed is narrower and worse: a p-value
/// computed over ~20 sessions rendered in **exactly the same format** as one
/// computed over 3,650, with nothing on the page to tell them apart. The number
/// was not wrong; the impression it gave was.
fn sample_warning(sessions: usize) -> String {
    if sessions >= MIN_AUDIT_SESSIONS {
        return String::new();
    }
    let block = runner::bootstrap::DEFAULT_BLOCK;
    let mut out = String::with_capacity(512);
    let _ = writeln!(
        out,
        "\nSAMPLE\n  sessions {sessions} · walk-forward folds {WALK_FORWARD_SPLITS} · \
         bootstrap block {block}\n  \
         THIN. Below {MIN_AUDIT_SESSIONS} sessions each fold tests on roughly \
         {} day(s) and each bootstrap draw is roughly {} block(s), so the \
         p-values and the PBO figure above are computed at a resolution this \
         sample does not have. They render in the same format they would over \
         ten years; they do not mean the same thing. The trades, the exit grid \
         and the excursions are unaffected — those measure what happened.",
        sessions / WALK_FORWARD_SPLITS.max(1),
        sessions / block.max(1),
    );
    out
}

/// Why the audit has nothing to trade — and the two reasons are not the same.
///
/// # The sentence that covered both
///
/// The audit printed *"no closed combination survived, so there is nothing to
/// trade. This is extinction, not a failure."* unconditionally. Its input is
/// `closed_by_evidence`, the strongest [`audit_keep()`] by |t| intersected with
/// the closed set, so it can be empty two ways:
///
/// * the sweep genuinely found nothing — that **is** extinction, and §6 says so:
///   depth is decided by extinction and an empty answer is the answer;
/// * the sweep found plenty and none of the strongest 250 happened to be closed.
///
/// The second is not extinction and saying so is a claim about the market that
/// the code cannot support. [`audit_keep()`]'s own doc block predicts this exact
/// failure — *"the audit reports extinction on a sweep that found plenty, a
/// refusal that would be a lie about the market rather than a fact about it"* —
/// and the constant was raised to make it unlikely while the message was left
/// covering both. Guarding the cause and not the claim is how a report ends up
/// asserting something it does not know.
///
/// The second arm also tells the operator what to change, which `CLAUDE.md` §4
/// asks of a refusal: it must name what was wrong, not merely decline.
fn nothing_to_trade(frequent: usize) -> String {
    if frequent == 0 {
        return "\nAUDIT\n  no combination met the threshold, so there is nothing \
                to trade. This is extinction, not a failure.\n"
            .to_owned();
    }
    let mut out = String::with_capacity(384);
    let _ = writeln!(
        out,
        "\nAUDIT\n  REFUSED. The sweep kept {frequent} combination(s), and none \
         of the strongest {kept} by |t| is closed — each is a superset of \
         a subset with the same support, so trading one would report a result of \
         one arity that is really another.\n  This is NOT extinction. Raise \
         BRUTEX_SCREEN_CAP, or raise min_hits and run again.",
        kept = audit_keep(),
    );
    out
}

/// Which side the evidence points, for a combination the ranker chose.
///
/// # Why this has to be derived rather than assumed
///
/// `crate::rank` orders by **|t|**, and says why in its own header: a
/// combination that reliably precedes a fall is as tradeable as one that
/// precedes a rise, so ranking on a signed `t` would discard every short setup.
/// The sign is not lost — it is carried in `Edge::mean_paisa` *"for a reader to
/// see"*. The audit was not a reader: it traded `Direction::Long`
/// unconditionally, so a strong downward signal was bought.
///
/// # The sign is taken from `mean_paisa`, not from `t`
///
/// They agree in sign by construction — `t` is the mean over its standard
/// error, and a standard error is non-negative. `mean_paisa` is used because it
/// is the quantity with units: the mean forward move in paisa, which is what a
/// side actually means. Reading `t` would work and would say less.
///
/// # Exactly zero is Long, and that is a stated convention
///
/// A mean of exactly 0.0 has no side. It cannot survive the significance bar —
/// a zero mean gives `t = 0` — so which way it is taken changes no verdict, and
/// picking one keeps the function total rather than adding a third arm no
/// report can reach. `f64` is used because `Edge` keeps statistical values at
/// full precision, which `CLAUDE.md` §7 permits and requires for exactly this.
fn side_of_evidence(scored: &runner::rank::Scored) -> Side {
    if scored.edge.mean_paisa < 0.0 {
        Side::Short
    } else {
        Side::Long
    }
}

/// The trade direction matching a side.
///
/// Two enums for one concept — `costs::fill::Direction` decides which way a
/// fill is adverse, `runner::excursion::Side` decides which way an excursion is
/// favourable — and every call site that takes both must map them consistently
/// or a long is priced as a short. `runner::validate` carries its own copy of
/// this mapping for the same reason.
const fn direction_of(side: Side) -> Direction {
    match side {
        Side::Long => Direction::Long,
        Side::Short => Direction::Short,
    }
}

/// The combination the audit traded, in words, above its own P&L.
///
/// `CLAUDE.md` §4: a number whose subject is unstated is a number that cannot be
/// checked. Every figure in the sections below belongs to this one combination,
/// and until it was printed the report gave the reader no way to learn which.
fn traded_line(scored: &runner::rank::Scored) -> String {
    let mut out = String::with_capacity(256);
    let _ = writeln!(
        out,
        "\nTRADED COMBINATION\n  {}\n  hits {} · n {} · mean {} paisa · t {:.2}",
        runner::report::condition_names(&scored.mask).join(" · "),
        scored.hits,
        scored.edge.n,
        scored.edge.mean_paisa,
        scored.edge.t,
    );
    out
}

/// The seed the resampler is started from.
///
/// # Why a constant and not a clock
///
/// `CLAUDE.md` §3 rule 5 requires the same inputs to give the same outputs byte
/// for byte. A bootstrap seeded from the clock would produce a different p-value
/// on every run and reruns would stop being safe — so the seed is fixed, stated
/// here, and folded into the run identity like every other parameter. An
/// operator who wants a different draw changes this deliberately rather than
/// getting one by accident.
///
/// The value carries no meaning. It is `0xB2_07_E8` — "brutex" in the only
/// digits that spell it — chosen so nobody mistakes it for a measurement.
const BOOTSTRAP_SEED: u64 = 0x00B2_07E8;

/// How many combinations the bootstrap compares.
///
/// White's Reality Check asks whether the BEST of a set beats what the same
/// search would find on resampled data, so the set is the point: comparing one
/// strategy against itself answers nothing. Sixteen is a stated assumption —
/// enough that the maximum is a real maximum over a family, small enough that
/// sixteen full trade walks stay affordable beside the sweep that produced them.
const BOOTSTRAP_CANDIDATES: usize = 16;

/// One combination's pessimistic return per SESSION, aligned to the day index.
///
/// # Why sessions and not bars
///
/// A bootstrap resamples periods, and a period has to be a unit over which a
/// return means something. A one-minute bar is not: most bars hold no trade at
/// all, so a per-bar series is almost entirely zeros and the block resampling
/// would be shuffling emptiness. A session is the natural unit for an intraday
/// strategy that squares off daily, and `DEFAULT_BLOCK`'s own documentation
/// reasons in sessions too.
///
/// # Why every series is the same length by construction
///
/// `bootstrap::aligned` refuses a set whose members differ in length, and it is
/// right to: two series of different lengths are not two views of one period
/// set. The day index is built ONCE from the bars and every combination is
/// bucketed into it, so alignment is a property of how this is built rather than
/// something a caller has to check.
///
/// A trade lands in the session its EXIT falls in, because that is when its
/// result is known. `Trade::worst` and not `best`: the pessimistic fill, the
/// same side every other figure in this report is taken on.
///
/// # Why a probe and not a `binary_search`
///
/// This ran a `binary_search` over the day slice, once per trade. (Written
/// without the receiver, because CI gate 11 rule 1 greps SOURCE and would
/// otherwise match this very sentence — a gate that reads comments is a gate
/// that stays red for a line of prose.) It was correct and it was
/// against the law: `docs/07-o1-architecture.md` layer 4 is "No search of any
/// kind … **Never `binary_search`**", and CI gate 11 rule 1 refuses the
/// construct workspace-wide with an allowlist whose own comment reads "AND IT
/// STAYS EMPTY". The gate was RED on this line, while `docs/06-limits.md` §11
/// asserted the last such call had been removed by D-0065.
///
/// `crates/runner/src/excursion.rs` goes to real trouble — a two-cursor merge
/// over monotone sequences — specifically to honour that ban, so leaving this
/// here made one crate's discipline pay for another's convenience.
///
/// The index is a `HashMap` built ONCE per report from the same `days` slice
/// the caller already owns, and every trade is one probe. O(sessions) to build,
/// O(1) per trade, and nothing searches.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn session_returns(
    index: &std::collections::HashMap<i64, usize>,
    sessions: usize,
    bars: &[indicators::Candle],
    taken: &trade::Trades,
) -> Vec<i64> {
    let mut series = vec![0_i64; sessions];
    for t in &taken.trades {
        let Some(bar) = bars.get(t.exit_bar) else {
            continue;
        };
        let day = indicators::ist_day(bar.ts_micros);
        if let Some(cell) = index.get(&day).and_then(|slot| series.get_mut(*slot)) {
            *cell = cell.saturating_add(t.worst);
        }
    }
    series
}

/// The distinct IST days the bars span, ascending.
///
/// Built from the bars rather than assumed, so a half-day, a holiday gap or a
/// Muhurat session changes the index instead of shifting every later bucket by
/// one — which is the defect a fixed 375-bar stride would have.
///
/// # Why there is no sort here, and there was one
///
/// The doc block above celebrates removing a `binary_search` from
/// `session_returns` because CI gate 11 rule 1 refuses the construct
/// workspace-wide. The replacement introduced a `sort_unstable` HERE, which
/// fails gate 11 **rule 4** with no allowlist entry — one red gate traded for
/// another, in the same commit, three lines apart.
///
/// It was never needed. `bars` comes from the store, and the store enforces
/// strictly increasing timestamps: `survey` refuses a batch that is not
/// ordered, and `Header::advance` refuses an append that does not follow. So
/// the days derived from them are already non-decreasing, and a single pass
/// keeping each day that differs from the last is the whole of the work.
///
/// The order is CHECKED rather than assumed. A day that goes backwards means
/// the store's own invariant has broken upstream, and this returns what it has
/// rather than silently building an index that maps trades to the wrong
/// session — the bound stays O(bars) either way.
///
/// # Cost
///
/// One pass, one comparison per bar, one push per distinct day. No sort, no
/// search, no allocation per bar beyond the days kept.
fn session_index(bars: &[indicators::Candle]) -> Vec<i64> {
    let mut days: Vec<i64> = Vec::new();
    for bar in bars {
        let day = indicators::ist_day(bar.ts_micros);
        match days.last() {
            Some(&last) if last == day => {}
            Some(&last) if last > day => {
                // OUT OF ORDER, WHICH THE STORE FORBIDS. Reported rather than
                // sorted around: sorting here would paper over a broken
                // invariant one layer down and produce a plausible index.
                eprintln!(
                    "session index: {day} follows {last}, which the store's \
                     monotonic timestamps forbid — the index stops here rather \
                     than reordering bars it did not order"
                );
                return days;
            }
            _ => days.push(day),
        }
    }
    days
}

/// How many anchored folds the walk-forward uses.
///
/// # A stated assumption, in the form this crate already uses for one
///
/// `bootstrap::DEFAULT_BLOCK` and `validate::DEFAULT_RUNGS` are both constants
/// their own documentation calls "a stated assumption and not a derivation", and
/// this is the third. `CLAUDE.md` §3 rule 1 forbids PRETENDING a number is
/// derived; it does not forbid choosing one and saying so.
///
/// Five is the anchored-walk-forward count in common use, and the trade it makes
/// is legible: each additional fold buys another independent out-of-sample
/// verdict and costs one more full sweep, while shortening every training window.
/// On a 91,874-bar column that is roughly 18,000 test bars per fold — enough that
/// a fold's verdict is not one afternoon.
///
/// Nothing in the data says where that trade sits, and no charter source names a
/// fold count, so this is the assumption and the report prints it beside the
/// result rather than burying it.
const WALK_FORWARD_SPLITS: usize = 5;

/// How many anchored folds to split a column into, DERIVED from its length.
///
/// # Five was a number, and five is the wrong number twice
///
/// A fold count is a trade: more folds mean more independent verdicts and fewer
/// bars behind each. Five is roughly 18,000 test bars per fold on a 91,874-bar
/// column, which is reasonable — and on a 1,700-bar column it is 340 bars a
/// fold, where a fold's verdict is one afternoon and means nothing.
///
/// The figure that decides it is not a preference, it is how many bars there
/// are. This targets a floor of test bars per fold and takes as many folds as
/// that allows, so a long column gets many verdicts and a short one gets few
/// rather than many worthless ones.
///
/// # The floor
///
/// Two thousand test bars is about five trading days at one minute, or a full
/// quarter at sixty. Below it a fold is measuring a week and calling it
/// out-of-sample evidence. It is the one figure here that is chosen rather than
/// derived, and it is chosen against the SAMPLE the fold needs rather than
/// against a fold count somebody liked.
///
/// # Bounds
///
/// At least 2 — one fold is not a walk-forward, it is a single split, and the
/// PBO calculation needs more than one placement to rank. At most 20, because
/// the anchored design gives the earliest fold the least data and a twenty-first
/// slice of a column is training on almost nothing.
fn walk_forward_splits(bars: usize) -> usize {
    const TEST_BARS_FLOOR: usize = 2_000;
    // Anchored folds put roughly `bars / (splits + 1)` in each test window, so
    // the count that hits the floor is `bars / floor - 1`.
    (bars / TEST_BARS_FLOOR).saturating_sub(1).clamp(2, 20)
}

/// The sweep, then what its best combination actually did.
///
/// # What this reaches that `sweep` does not
///
/// `report::render` shows the census, the ladder and the significance bar —
/// everything the SEARCH produced. It says nothing about money, because the
/// engine has no notion of it: `Itemset` carries a mask and a hit count.
///
/// `audit::render` is the other half — trades, the exit grid, the walk-forward,
/// PBO and the bootstrap — and **until this function it had no caller anywhere
/// in the workspace.** The whole institutional stack was reachable only from its
/// own tests.
///
/// # What is filled in, and what is honestly `None`
///
/// Trades and the exit grid, from the first CLOSED combination the sweep kept —
/// closed rather than merely frequent, because `closed::closed` removes the
/// combinations that carry no information a larger one does not, and the first
/// of those is a better subject than the first of everything.
///
/// The walk-forward, PBO and bootstrap are passed as `None`. They are not
/// unavailable — `validate::walk_forward` and the three bootstrap tests all
/// work — but each needs a fold count, a draw count and a seed that
/// `CLAUDE.md` §3 rule 1 will not let this crate invent, and no charter source
/// supplies them. `audit::render` prints an explicit absence for each rather
/// than a zero, which is the difference between "not measured" and "measured as
/// nothing".
#[must_use]
pub fn audit_run(sessions: i64, min_hits: u64) -> String {
    audit_with(evaluator(), sessions, min_hits, None)
}

/// [`audit_run`], under a candidate ceiling the caller names.
///
/// # Why this exists, and what an hour of nothing cost
///
/// `ladder_for` reads `engine::DEFAULT_CEILING` -- 134,217,728 candidates --
/// which is the right budget for a real instrument-month and the wrong one for a
/// caller that needs a sweep only to EXIST.
///
/// `the_audit_renders_every_stage_of_the_institutional_stack` asserts that every
/// SECTION of the report renders. It does not need a deep ladder to do that, and
/// it inherited the full ceiling in a DEBUG build: measured at over an hour
/// before it was killed. `cargo test --workspace` therefore never terminated,
/// so §9's green-suite requirement was unverifiable -- and a genuinely red test
/// sat behind it undetected from `a12192b` onward.
///
/// `crates/runner/tests/join_answer_is_unchanged.rs` had the answer all along:
/// it bounds its own fixture at `with_ceiling(50_000)` and completes in 0.04s.
/// A test that needs a sweep should say how big a sweep it needs.
#[must_use]
pub fn audit_run_within(sessions: i64, min_hits: u64, ceiling: usize) -> String {
    audit_with(evaluator(), sessions, min_hits, Some(ceiling))
}

/// The full audit stack over **one real instrument-month read from the store**.
///
/// # The gap this closes
///
/// `sweep-stored` reads real bars and ranks the combinations it finds. It stops
/// there. Everything that turns a combination into money — trades, worst-case
/// fills, the 125-cell exit grid, walk-forward folds, PBO and the bootstrap
/// p-values — lived behind `cli audit`, whose bars were **hardcoded** to
/// `synthetic::sessions`. So real bars could be swept and could never be traded,
/// and every P&L, PBO figure and p-value the workspace could print described a
/// generated series with a deterministic upward drift.
///
/// # Errors
///
/// The same refusals `sweep_stored` returns, in the same order and for the same
/// reasons: an unstamped build first (§3 rule 3 — no computation without a
/// recordable identity), then an unknown feed, then a store root that is not
/// configured, then a month that is not held.
fn audit_stored_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> Result<String, stored::Refusal> {
    // COMMIT FIRST, BEFORE A BAR IS READ — the order `sweep_stored` uses and for
    // the identical reason: a build that cannot be identified must refuse BEFORE
    // it computes, not compute and then apologise.
    let commit = commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and the audit will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;

    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let loaded = stored::load(&root, vendor, underlying, rung, year, month)?;

    // THE FILE WAS OPENED AND THIS IS WHERE AN OPERATOR LEARNS IT.
    //
    // MEASURED, on a real month: `audit-stored groww BANKNIFTY 15min 2026 03`
    // completed, printed 795 lines, and wrote a log file of ZERO BYTES. The
    // banner above it said `events -> <dir>` because `telemetry::install`
    // succeeded, so the run reported that it was being recorded and recorded
    // nothing -- the failure wearing a success's clothes `CLAUDE.md` section 4
    // bans, and the exact defect D-0226 added `telemetry` to this crate's
    // dependency set to remove.
    //
    // `sweep_stored` carried both of these calls and this function carried
    // neither, so the LESSER command was observable and the one that produces
    // the exit grid, the walk-forward, the PBO and the bootstrap was dark. An
    // operator watching /logs during a long audit saw a blank page and could not
    // tell a running sweep from a dead process.
    //
    // Gate 17's granularity holds: one event per run, at a boundary. No loop
    // over bars and no loop over candidates reaches this line.
    //
    // NOT REACHED BY `cargo test`, AND THAT IS STATED RATHER THAN PAPERED OVER.
    // `commit_stamp()` is `option_env!`, so an unstamped test build refuses at
    // the gate above before the store is touched --
    // `the_stored_audit_refuses_for_the_same_cause_and_names_itself` documents
    // the same limit for the refusal path, and `sweep_stored`'s two events sit
    // in the identical region. The verification is therefore a MEASUREMENT on a
    // stamped build, recorded in `docs/05-decisions.md` and `docs/06-limits.md`
    // rather than claimed here: zero lines before, two after, on
    // `groww BANKNIFTY 15min 2026-03`.
    note(
        &telemetry::Event::info("cli.audit", "stored month loaded")
            .with("feed", loaded.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", loaded.timeframe)
            .with("year", u64::from(year))
            .with("month", u64::from(month))
            .with("bars", u64::try_from(loaded.bars.len()).unwrap_or(u64::MAX))
            .with("min_hits", min_hits),
    );

    let ladder = ladder_for(min_hits)?;
    let id = identity(&Run {
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        // UNDIRECTED, and deliberately so even though this command DOES trade.
        // The identity names the SWEEP that produced the candidates; the
        // direction a trade is taken in is chosen per combination further down,
        // and stamping one of them here would name a decision the sweep did not
        // make. `Direction::Long`/`Short` re-key the identity for the day a
        // directional sweep exists, which is what that field is reserved for.
        direction: RunDirection::Undirected,
        instrument: &loaded.key,
        timeframe: loaded.timeframe,
        params: Params::of(ladder),
        data_digest: data_digest(&loaded.bars),
        commit,
        feed: loaded.vendor.as_str(),
    });

    // THE BANNER LEADS, and the feed line rides inside it rather than above the
    // report. `audit_bars` writes its banner first and the report second, so a
    // feed line written here would land BETWEEN them — pushing the provenance
    // claim away from the numbers it qualifies. `sweep_stored` puts the two
    // together for the same reason.
    let mut header = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        header,
        "feed {} · {} · {} · {year}-{month:02} · {} bars · built at {commit}",
        vendor.as_str(),
        underlying,
        loaded.timeframe,
        loaded.bars.len(),
    );
    let bars = u64::try_from(loaded.bars.len()).unwrap_or(u64::MAX);
    let report = audit_bars(
        evaluator(),
        loaded.bars,
        &header,
        min_hits,
        Some(&id),
        AuditOptions {
            execution: None,
            recording: None,
            rules: Rules::BASELINE,
            // The historical cut, unchanged. `Payoff` is reached only by an
            // operator who typed `elite`.
            // The environment's ceiling: an operator-facing command must not
            // silently narrow its own search.
            ceiling: None,
            // THE FULL STACK, UNLESS THE OPERATOR SAYS OTHERWISE.
            //
            // This was `true`, hardcoded, under a comment reading "every
            // operator-facing command validates" — so walk-forward, PBO and the
            // bootstrap ran on EVERY browser sweep and `BRUTEX_VALIDATE` could
            // not reach the one path the operator actually uses.
            //
            // MEASURED, and it is why nothing has ever completed: one rung over
            // ONE YEAR of 60-minute bars — about 1,700 bars, against D-0258
            // completing 5,249 bars in 4.26 seconds — ran twelve minutes and
            // recorded nothing, with the sampler in `grid::realised` and
            // `one_variant` throughout. `screen_cap`'s own doc already recorded
            // this stack returning "nothing at all after fifty minutes, twice".
            //
            // `validate_from_env` defaults to ON, so nothing changes unless the
            // operator sets `BRUTEX_VALIDATE=0` — and a run taken that way
            // carries the `UNVALIDATED` banner `CLAUDE.md` §5 requires, so a
            // candidate can never be mistaken for a finding.
            validate: validate_from_env(),
            lens: runner::rank::Lens::Detectability,
        },
    );
    // THE IDENTITY REACHES THE LOG, which is the half section 3 rule 3 cares
    // about. A report names its identity in text that scrolls past; an operator
    // asking "which run produced the grid I am looking at" needs it in a line
    // `/logs` can search.
    //
    // Emitted AFTER the render rather than before, so the event means the audit
    // finished. The pair is then readable as a span: `stored month loaded`
    // opened it, this closes it, and a `loaded` with no matching `rendered` is a
    // run that died in between -- which is the one thing a single event could
    // not have said.
    //
    // The sweep's own figures -- depth, kept, ranked -- are NOT here, and that is
    // a stated gap rather than an oversight: `audit_bars` returns rendered text
    // and keeps its `Outcome` private, so reporting them would mean widening its
    // signature. `sweep_stored`'s `ladder walked` event carries them for the
    // command that does expose them.
    note(
        &telemetry::Event::info("cli.audit", "audit rendered")
            .with("identity", id.hex().as_str())
            .with("feed", loaded.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", loaded.timeframe)
            .with("bars", bars),
    );
    Ok(report)
}

/// The provenance banner for one span, including any months it is missing.
///
/// Split out of [`audit_range_inner`] to keep it under
/// `clippy::too_many_lines`. It is one idea: what this run was over, said
/// before any figure computed from it.
fn span_banner(
    span: &stored::Span,
    underlying: &str,
    from: (u16, u8),
    to: (u16, u8),
    commit: &str,
) -> String {
    let mut header = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        header,
        "feed {} · {underlying} · {} · {}-{:02}..{}-{:02} · {} of {} months · {} bars · built at {commit}",
        span.vendor.as_str(),
        span.timeframe,
        from.0,
        from.1,
        to.0,
        to.1,
        span.found,
        span.asked,
        span.bars.len(),
    );
    // A HOLE IS NAMED, NEVER SKIPPED. A span missing three months is a shorter
    // sample and not a corrected one, and every figure below is computed over
    // what was actually there. `CLAUDE.md` §4 bans the fallback that would let
    // it read like a whole span.
    if !span.complete() {
        let names: Vec<String> = span
            .missing
            .iter()
            .map(|&(y, m)| format!("{y}-{m:02}"))
            .collect();
        let _ = writeln!(
            header,
            "MONTHS MISSING FROM THIS SPAN ({}): {}\n\
             Every figure below is over a SHORTER sample, not a corrected one. \
             Pull those months and rerun to close the gap.",
            span.missing.len(),
            names.join(" ")
        );
    }
    header
}

/// The full audit over a CONTIGUOUS SPAN of months, as one series.
///
/// # Why this is not `audit_stored` in a loop
///
/// Looping the single-month audit produces N answers about N months. This
/// produces ONE answer about the whole span, and the difference is the question
/// itself: a combination frequent in every month separately is not a combination
/// frequent over seven years, a trade may open in one month and close in the
/// next, and a walk-forward split across a span tests against REGIMES rather
/// than against days inside one month.
///
/// The month is a STORAGE unit — `crates/store` keeps one file per
/// instrument-month — and it was never meant to be the analysis unit. It became
/// one only because the loader could open a single file.
///
/// # Errors
///
/// Every arm of [`stored::load_span`], plus the commit-stamp refusal §3 rule 3
/// requires before any bar is read.
/// The rung every position opens and closes on, whatever the signal rung is.
///
/// One minute is the finest series this store carries for an index, so it is the
/// most resolution a fill can be measured at. It is a constant and not a
/// parameter because it is not a choice: `CLAUDE.md` §6 explains why a knob that
/// can be set can be set wrongly and silently, and an execution rung coarser
/// than the data allows would quietly widen every intra-bar ambiguity in the
/// report.
const EXECUTION_RUNG: &str = "1min";

fn audit_range_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    min_hits: u64,
) -> Result<String, stored::Refusal> {
    // COMMIT FIRST, BEFORE A BAR IS READ, for the reason `audit_stored_inner`
    // gives: a build that cannot be identified must refuse BEFORE it computes.
    let commit = commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and the audit will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;

    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let span = stored::load_span(&root, vendor, underlying, rung, from, to)?;

    note(
        &telemetry::Event::info("cli.audit", "stored span loaded")
            .with("feed", span.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", span.timeframe)
            .with("from", format!("{}-{:02}", from.0, from.1).as_str())
            .with("to", format!("{}-{:02}", to.0, to.1).as_str())
            .with("months_asked", u64::from(span.asked))
            .with("months_found", u64::from(span.found))
            .with(
                "months_missing",
                u64::try_from(span.missing.len()).unwrap_or(u64::MAX),
            )
            .with("bars", u64::try_from(span.bars.len()).unwrap_or(u64::MAX))
            .with("min_hits", min_hits),
    );

    // BOUND ONCE, USED TWICE: by the run identity below and by the
    // `AuditOptions` this function goes on to build.
    //
    // They were written apart -- the identity taking the ladder alone, the
    // options constructing their policy inline further down -- so there was
    // no single place holding what this run's policy IS, and the identity
    // simply did not carry it. Naming them here makes the two physically the
    // same values rather than two spellings that happen to agree.
    let rules = Rules::operator();
    let lens = runner::rank::Lens::Payoff;
    let validate = validate_from_env();
    let ladder = ladder_for(min_hits)?;
    let id = identity(&Run {
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        // Undirected for the reason `audit_stored_inner` states: the identity
        // names the SWEEP, and direction is chosen per combination below.
        direction: RunDirection::Undirected,
        instrument: &span.key,
        timeframe: span.timeframe,
        // THE POLICY IS PART OF THE IDENTITY, AND ON THIS PATH IT WAS NOT.
        //
        // This read `Params::of(ladder)` — the threshold and the two budgets,
        // and nothing about what was DONE with the sweep's output.
        // `screen_range_inner` has folded the policy in since D-0294, on the
        // principle its doc states: *"an identity two different results can
        // share is not an identity."* This function is the one that RECORDS,
        // and it was the one without it.
        //
        // The gap became a trap the moment `validate` stopped being hardcoded.
        // The `UNVALIDATED` banner instructs the reader to *"Unset
        // BRUTEX_VALIDATE to price them in full"* — and following that
        // instruction produced the identical nine terms, so
        // `Results::append_locked` refused the validated run as a duplicate
        // with *"this run has nothing new to add"*. That justification was
        // false: the inputs differed and so did the outputs. The candidate row
        // could never be replaced by the finding row, and `one_rung` reported
        // the rung as a failure.
        //
        // Same `policy_of` and the same argument order as
        // `screen_range_inner`'s call, so the two paths key identically and a
        // knob added to one cannot be missed by the other.
        params: Params::of(ladder).with_policy(&policy_of(&span.bars, rules, lens, validate)),
        // THE DIGEST IS OVER THE WHOLE SPAN, which is what makes this identity
        // correct without a new field. `Run` carries no year or month, so a
        // span and any single month inside it hash differently purely because
        // their bars differ — and two runs over the same span agree, which is
        // §3 rule 5.
        data_digest: data_digest(&span.bars),
        commit,
        feed: span.vendor.as_str(),
    });

    let signal_length = stored::rung_length_micros(rung)?;

    // THE EXECUTION SERIES, LOADED ALONGSIDE THE SIGNAL ONE.
    //
    // Always one-minute, and always the SAME span, feed and instrument -- a
    // position opened on one instrument's signal and filled on another's would
    // be a different strategy wearing this one's name.
    //
    // When the signal rung IS `1min` this is skipped rather than loaded twice:
    // projecting a series onto itself is the identity, and `align`'s own test
    // `a_one_minute_signal_on_one_minute_bars_is_simply_the_next_bar` pins that
    // it degenerates to "enter on the next bar" -- which is exactly what the
    // engine did before this layer existed. Loading it anyway would double the
    // read for no change in the answer.
    let execution_bars = if rung == EXECUTION_RUNG {
        None
    } else {
        match stored::load_span(&root, vendor, underlying, EXECUTION_RUNG, from, to) {
            Ok(exec) => Some(exec),
            // A REFUSAL HERE STOPS THE RUN. It would be easy to fall back to
            // executing on the signal rung and print a note, and that is exactly
            // the `CLAUDE.md` §4 fallback: the numbers would be a different
            // model's, rendered identically. If the one-minute bars are not
            // there, the answer this command promises cannot be computed.
            Err(why) => {
                return Err(format!(
                    "the {EXECUTION_RUNG} execution series is required and could \
                     not be loaded: {why} Every entry and exit fills on \
                     {EXECUTION_RUNG} bars, so without them there is no run to \
                     make. Pull that rung for this span, or sweep \
                     {EXECUTION_RUNG} directly."
                ));
            }
        }
    };
    let execution = execution_bars.as_ref().map(|exec| Execution {
        bars: &exec.bars,
        signal_length_micros: signal_length,
    });

    let header = span_banner(&span, underlying, from, to, commit);
    Ok(audit_bars(
        evaluator(),
        span.bars,
        &header,
        min_hits,
        Some(&id),
        AuditOptions {
            execution,
            recording: Some(Recording {
                root: &root,
                feed: vendor.as_str(),
                underlying,
                timeframe: span.timeframe,
                from,
                to,
                months_asked: span.asked,
                months_found: span.found,
            }),
            rules,
            // The environment's ceiling: an operator-facing command must not
            // silently narrow its own search.
            ceiling: None,
            // THE FULL STACK, UNLESS THE OPERATOR SAYS OTHERWISE.
            //
            // This was `true`, hardcoded, under a comment reading "every
            // operator-facing command validates" — so walk-forward, PBO and the
            // bootstrap ran on EVERY browser sweep and `BRUTEX_VALIDATE` could
            // not reach the one path the operator actually uses.
            //
            // MEASURED, and it is why nothing has ever completed: one rung over
            // ONE YEAR of 60-minute bars — about 1,700 bars, against D-0258
            // completing 5,249 bars in 4.26 seconds — ran twelve minutes and
            // recorded nothing, with the sampler in `grid::realised` and
            // `one_variant` throughout. `screen_cap`'s own doc already recorded
            // this stack returning "nothing at all after fifty minutes, twice".
            //
            // `validate_from_env` defaults to ON, so nothing changes unless the
            // operator sets `BRUTEX_VALIDATE=0` — and a run taken that way
            // carries the `UNVALIDATED` banner `CLAUDE.md` §5 requires, so a
            // candidate can never be mistaken for a finding.
            validate,
            // THE LENS THAT DECIDES WHICH COMBINATIONS ARE EVER PRICED, AND IT
            // WAS ANSWERING A DIFFERENT QUESTION FROM THE ONE BEING ASKED.
            //
            // This read `Detectability`, described as "the historical cut" --
            // and this function is what the browser's Run button reaches, so it
            // is the ordering behind every sweep the operator has ever started.
            //
            // `screen_cap`'s doc states the consequence in its own words, with
            // its own measurement: "The pipeline is: Apriori produces the
            // combinations, they are ranked by |t|, and the first SCREEN_CAP of
            // that ordering get an exit grid. On a real 15-minute run over 81
            // months that read: 1,024,058 combinations found, 250 kept, 21
            // priced." One in fifty thousand reached a grid, and the ordering
            // that chose them ranks a LEVEL-LESS forward return -- no stop, no
            // target, no trail. It asks "how far does this signal run
            // UNSTOPPED".
            //
            // The operator's question is the other one, stated in that same
            // doc: "which signal keeps every loser inside ten points and every
            // winner past thirty". And it names the cost exactly: "A
            // combination that is unremarkable unstopped and excellent under a
            // tight stop scores low on the first question, is cut at 60, and
            // never meets an exit grid at all. No tier ladder, no rule and no
            // report can recover that. They all filter cells, and the cells
            // were never computed."
            //
            // So the standing requirement -- massive win rate, very small stop,
            // very small drawdown, top 10 to 25 -- was being systematically
            // discarded before anything could measure it. `Rules` could not
            // save it and neither could the exit grid: both filter cells that
            // this cut prevented from existing.
            //
            // `Payoff` ranks by `Edge::payoff_bp`, mean win against mean loss,
            // with `|t|` as the tie-break so a two-trade fluke cannot outrank a
            // measured edge. That is the small-losers-large-winners shape, and
            // it is what `elite_descend` and `descend` already select. Note
            // that `payoff_bp` was inflated by every flat bar until the same
            // day this changed: ranking by it before that fix would have
            // ordered by a wrong number, so the two changes belong together.
            lens,
        },
    ))
}

/// The trade-quality block: the question "minimal stop, maximum profit" as
/// numbers.
///
/// # These were stored from the first row and shown by nothing
///
/// The listing printed totals — worst, best, trades — and every one of them
/// answers *how much did it make*. None answered *what did it risk to make it*,
/// which is the operator's actual question and the one the exit grid exists to
/// price. `winner_mae`, `winner_mfe`, `all_mae`, `worst_trade` and
/// `max_drawdown` have been in every record since the ledger landed; nothing
/// rendered them.
///
/// **`winner MAE` is the headline.** It is how far the winning trades went
/// AGAINST the position before they worked — so it is the tightest stop that
/// would not have killed a winner. A strategy whose winners never dip 0.2%
/// can be run with a 0.2% stop; that is "very minimal stop loss" as a measured
/// figure rather than a wish.
///
/// **`per trade` is the other one.** A total of five lakh over eleven thousand
/// trades is forty-five rupees a trade, which is a different proposition from
/// five lakh over twelve. Division the reader should not have to do.
fn quality_block(record: &crate::results::Record) -> String {
    let mut out = String::from("\n  TRADE QUALITY -- what it risked to make it\n");
    let per_trade = if record.trades == 0 {
        0
    } else {
        record.pessimistic / i64::try_from(record.trades).unwrap_or(1)
    };
    for (label, value, note) in [
        // THESE THREE ARE MEANS, AND TWO OF THEM USED TO BE LABELLED AS BOUNDS.
        //
        // `winner_mae` was headed "tightest stop that keeps every winner". It
        // is the MEAN of the winners' adverse excursions, so a stop placed
        // there stops out roughly half of them -- the opposite of what the
        // label promised. `runner::audit` prints the same field correctly, as
        // "mean MAE, winners only", and prints `worst_mae` beside it under
        // "THE BOUND". Measured on June 2024: the mean read 0.00% and the bound
        // read 0.24%, which at NIFTY 23,000 is 55 points.
        //
        // `worst_mae` is NOT in the ledger record, so this surface cannot show
        // it however it is labelled -- see `docs/06-limits.md`. Until it is,
        // the honest thing is to stop calling a mean a maximum.
        (
            "mean adverse move, winners only",
            as_percent(record.winner_mae),
            "the AVERAGE winner's dip -- NOT a stop that keeps them all",
        ),
        (
            "mean favourable move, winners only",
            as_percent(record.winner_mfe),
            "how far the average winner ran",
        ),
        (
            "mean adverse move, all trades",
            as_percent(record.all_mae),
            "losers included -- always worse than winners alone",
        ),
        (
            "worst single round trip",
            rupees(record.worst_trade),
            "the largest loss one trade took",
        ),
        (
            "worst peak-to-trough",
            rupees(record.max_drawdown),
            "the deepest the equity curve fell",
        ),
        (
            "per trade, worst-case fills",
            rupees(per_trade),
            "total divided by round trips",
        ),
    ] {
        let _ = writeln!(out, "  {label:<40}{value:>16}  {note}");
    }
    // THE RATIO THE WHOLE THING IS FOR. Reward against the risk actually taken,
    // both measured on the same trades. `winner_mae` of zero means no winner
    // ever dipped -- a real answer on a small sample and not a division to make.
    // INTEGER ARITHMETIC, because `CLAUDE.md` section 7 keeps floats out of any
    // value that is compared. Tenths of a multiple: 47 renders as 4.7x, which is
    // all a reader needs and all the sample supports.
    let ratio = if record.winner_mae == 0 {
        "no winner dipped".to_owned()
    } else {
        let tenths = record.winner_mfe.saturating_mul(10) / record.winner_mae;
        format!("{}.{}x", tenths / 10, tenths % 10)
    };
    let _ = writeln!(
        out,
        "  {:<40}{ratio:>16}  mean winner run vs mean winner dip -- not a bound",
        "reward per unit of risk (means)"
    );
    // THE BOUND IS NOT IN THIS FILE, and saying so is the only honest option.
    // `grid::Cell::worst_mae` answers "did ANY single trade run further against
    // than X" and `cli screen` filters on it, but `results::Record` never
    // persisted it, so no relabelling of the rows above can produce it here.
    let _ = writeln!(
        out,
        "\n  Every excursion above is a MEAN. The BOUND -- the worst any single\n  \
         trade ran against -- is not recorded in this ledger. Run `audit-stored`\n  \
         for that month to see it; on June 2024 the mean read 0.00% and the\n  \
         bound read 0.24%."
    );
    let stop = record.exit_rungs.first().copied().unwrap_or(-1);
    let target = record.exit_rungs.get(1).copied().unwrap_or(-1);
    let tsl = record.exit_rungs.get(2).copied().unwrap_or(-1);
    let arm = record.exit_rungs.get(3).copied().unwrap_or(-1);
    let trail = record.exit_rungs.get(4).copied().unwrap_or(-1);
    let rung = |r: i16| {
        if r < 0 {
            "none".to_owned()
        } else {
            r.to_string()
        }
    };
    let _ = writeln!(
        out,
        "  {:<40}{:>16}  stop/target/trailing-stop/ttp-arm/ttp-trail, `none` is \
         that axis switched OFF",
        "the exit variant that won",
        format!(
            "{}/{}/{}/{}/{}",
            rung(stop),
            rung(target),
            rung(tsl),
            rung(arm),
            rung(trail)
        )
    );

    append_condition_names(&mut out, record);
    out
}

/// The winning combination, BY NAME, appended to a quality block.
///
/// # Why this is its own function
///
/// It was inline in [`quality_block`] and pushed it past the hundred-line
/// ceiling clippy enforces. Splitting on that boundary is the right cut
/// anyway: everything above it is MONEY, derived from figures the record
/// already held, and this is the one part that decodes a stored field back
/// through the vocabulary. The two fail for different reasons and read
/// better apart.
fn append_condition_names(out: &mut String, record: &crate::results::Record) {
    // THE CONDITIONS THEMSELVES, which no stored surface has ever printed.
    //
    // Every figure above is money, and money without the combination that
    // earned it is a number an operator cannot act on, cannot reproduce and
    // cannot argue with. The winner scrolled past in the live run's output and
    // the ledger kept only its P&L, so a row read back a week later said what
    // was made and never what made it. Version 3 of the ledger carries the six
    // mask words for exactly this line.
    //
    // `runner::report::names_from_words` and not `vocab` directly: see its own
    // comment -- `CLAUDE.md` §5 does not give `cli` a `vocab` arrow.
    let names = runner::report::names_from_words(record.mask_words);
    if names.is_empty() {
        // Distinguishable from "the names are missing". An all-zero mask means
        // the run recorded no winning combination at all -- a halt, or a sweep
        // whose frontier emptied at k=1 -- and saying that is not the same as
        // printing nothing, which reads as a rendering bug.
        let _ = writeln!(
            out,
            "  {:<40}{:>16}  no combination was recorded for this row",
            "the conditions it required", "NONE"
        );
    } else {
        let _ = writeln!(
            out,
            "  {:<40}{:>16}  every one must hold on the same bar, ANDed",
            "the conditions it required",
            format!("{} of them", names.len())
        );
        for (ordinal, name) in names.iter().enumerate() {
            let _ = writeln!(out, "        {:>2}. {name}", ordinal + 1);
        }
    }
}

/// The best COMPLETE run among these rows, as the line the listing ends on.
///
/// Separate from the table because a reader scanning forty rows for the largest
/// number is a reader who will miss it — and because `done: NO` rows must not
/// win. A halted ladder's total is not comparable with a complete one's: it
/// covers less of the search while its combination count looks larger.
fn best_complete_line(rows: &[crate::results::Record]) -> String {
    let complete: Vec<&crate::results::Record> = rows.iter().filter(|r| r.halted == 0).collect();
    let Some(best) = complete.iter().max_by_key(|r| r.pessimistic) else {
        return "  NO COMPLETE RUN to rank: every matching row halted on a \
                budget, so no total here is comparable with another. Raise \
                MIN_HITS and rerun.\n"
            .to_owned();
    };
    format!(
        "  BEST COMPLETE RUN: {} {} {} {}-{:02}..{}-{:02} at min_hits {} -- {} \
         worst-case over {} trades.\n  Ranked on the WORST-case total, \
         which is the figure every other surface selects on. Halted rows are \
         excluded: a truncated ladder's total is not comparable with a complete \
         one's.\n",
        crate::results::read_field(&best.feed),
        crate::results::read_field(&best.underlying),
        crate::results::read_field(&best.timeframe),
        best.from_year,
        best.from_month,
        best.to_year,
        best.to_month,
        best.min_hits,
        rupees(best.pessimistic),
        best.trades,
    )
}

/// A count with thousands separators, so a reader can see its magnitude.
///
/// # 54895691 and 54,895,691 are not equally readable
///
/// The combination count is the single number this engine exists to produce and
/// the one an operator most wants to size at a glance. Unseparated it takes
/// deliberate counting to tell fifty-four million from five hundred and forty
/// million, which is the difference between a search that finished and one that
/// hit the ceiling.
///
/// Plain 3-3-3 grouping and not the Indian 2-2-3 that [`rupees`] uses: this is a
/// COUNT and not money, and the two conventions are for different things.
fn grouped(n: u64) -> String {
    let digits: Vec<char> = n.to_string().chars().collect();
    let mut out = String::with_capacity(digits.len().saturating_add(digits.len() / 3));
    for (i, ch) in digits.iter().enumerate() {
        if i > 0 && digits.len().saturating_sub(i) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    out
}

/// Paisa as rupees, with two decimals and thousands separators.
///
/// # An accounting unit is not an answer
///
/// Every money figure in this workspace is a paisa `i64`, because
/// `CLAUDE.md` §7 forbids a float anywhere a price is compared. That is right
/// for the ENGINE and wrong for the operator: a listing that prints `2459160`
/// makes a reader do the division before they can tell whether the run made
/// twenty-four thousand rupees or two hundred and forty-six thousand.
///
/// The conversion happens HERE, at the render boundary, and nowhere else — the
/// same rule the tick grid follows. Integer arithmetic throughout: the rupees
/// and the paise are separated by division and remainder, never by a float.
fn rupees(paisa: i64) -> String {
    let negative = paisa < 0;
    // `unsigned_abs` and not `abs`: `i64::MIN` has no positive counterpart, and
    // it is exactly the value `saturating_add` produces for a variant that lost
    // without bound. A sort that panicked on the worst possible result would be
    // the report killing the process over the answer.
    let magnitude = paisa.unsigned_abs();
    let whole = magnitude / 100;
    let fraction = magnitude % 100;

    // Thousands separators, built right to left. Indian grouping is 2-2-3 rather
    // than 3-3-3, so a crore reads as 1,00,00,000 and not 10,000,000 -- which is
    // the grouping the operator reads prices in.
    let digits: Vec<char> = whole.to_string().chars().collect();
    let mut grouped = String::new();
    for (i, ch) in digits.iter().enumerate() {
        let from_right = digits.len().saturating_sub(i);
        // A separator before the last three digits, then every two after.
        if i > 0 && (from_right == 3 || (from_right > 3 && from_right % 2 == 1)) {
            grouped.push(',');
        }
        grouped.push(*ch);
    }
    format!(
        "{}{}{}.{:02}",
        if negative { "-" } else { "" },
        "\u{20b9}",
        grouped,
        fraction
    )
}

/// Parts per million as a percentage, to two decimals, in integers.
///
/// The excursion columns are ppm because a rung must mean the same distance on
/// NIFTY and BANKNIFTY. A reader wants the percentage.
fn as_percent(ppm: i64) -> String {
    let negative = ppm < 0;
    let magnitude = ppm.unsigned_abs();
    // ppm to hundredths of a percent: 10,000 ppm is 1%, so 100 ppm is 0.01%.
    let hundredths = magnitude / 100;
    format!(
        "{}{}.{:02}%",
        if negative { "-" } else { "" },
        hundredths / 100,
        hundredths % 100
    )
}

/// One checked property, and what it measured.
struct Check {
    /// What is being proved, in the operator's words.
    claim: &'static str,
    /// Whether it held.
    held: bool,
    /// The number that decided it, so a reader can see the evidence rather
    /// than a tick.
    evidence: String,
}

/// Proves the engine correct on the operator's OWN data, and prints what it
/// measured.
///
/// # Why this exists
///
/// `docs/09-verify.md` lists six gates and every one of them is a BUILD gate:
/// the workspace compiles, the tests pass, clippy is quiet, the bundle is
/// current. They prove the code is well-formed. **None of them proves the sweep
/// produces a correct answer on this store.**
///
/// That gap is the whole reason an operator has to take a report on trust, and
/// taking a report on trust is the thing this repository exists not to ask for.
/// Every check below runs against the REAL store, in seconds, and prints the
/// figure it measured beside the verdict.
///
/// # What each check would catch
///
/// | check | the defect it refuses |
/// |---|---|
/// | determinism | a sweep whose answer moves between runs, so no result is reproducible |
/// | suffix independence | look-ahead: a bar's verdict changing because LATER bars exist |
/// | monotone join | a span assembled out of order, folding a later bar into an earlier state |
/// | paired statistics | `Edge::mismatched` — a t and a mean computed from other bars' returns |
/// | ledger round trip | a fixed-stride file whose reader and writer disagree, which still parses |
/// | refusal surface | a bad argument answered with a report instead of a refusal |
///
/// Every one of those has actually occurred in this repository. This is the
/// button that would have caught them.
#[must_use]
pub fn verify(vendor_word: &str, underlying: &str) -> String {
    let mut out = String::from("VERIFYING THE ENGINE ON YOUR OWN DATA\n");
    let _ = writeln!(
        out,
        "  feed {vendor_word} · {underlying} · nothing is pulled, nothing is \
         written to the bar store\n"
    );
    let mut checks: Vec<Check> = Vec::new();

    let root = match store_root() {
        Ok(root) => root,
        Err(why) => return format!("refused: {why}\n"),
    };
    let vendor = match parse_vendor(vendor_word) {
        Ok(vendor) => vendor,
        Err(why) => return format!("refused: {why}\n"),
    };

    // 1. THE SPAN LOADS, AND ITS JOIN IS MONOTONE.
    let span = stored::load_span(&root, vendor, underlying, "1day", (2019, 12), (2026, 8));
    checks.extend(span_checks(&span));

    // 2. DETERMINISM. §3 rule 5: same inputs, same outputs, byte for byte.
    // Two sweeps of the same slice, compared as bytes.
    if let Ok(span) = &span {
        let short: Vec<indicators::Candle> = span.bars.iter().take(600).copied().collect();
        // The BASELINE options: no execution series, no ledger row, the stated
        // rules. Determinism is a property of the ENGINE and must not depend on
        // which options were passed, so the two runs differ in nothing at all.
        let plain = AuditOptions {
            execution: None,
            recording: None,
            rules: Rules::BASELINE,
            // The historical cut, unchanged. `Payoff` is reached only by an
            // operator who typed `elite`.
            // The environment's ceiling: an operator-facing command must not
            // silently narrow its own search.
            ceiling: None,
            // THE FULL STACK, UNLESS THE OPERATOR SAYS OTHERWISE.
            //
            // This was `true`, hardcoded, under a comment reading "every
            // operator-facing command validates" — so walk-forward, PBO and the
            // bootstrap ran on EVERY browser sweep and `BRUTEX_VALIDATE` could
            // not reach the one path the operator actually uses.
            //
            // MEASURED, and it is why nothing has ever completed: one rung over
            // ONE YEAR of 60-minute bars — about 1,700 bars, against D-0258
            // completing 5,249 bars in 4.26 seconds — ran twelve minutes and
            // recorded nothing, with the sampler in `grid::realised` and
            // `one_variant` throughout. `screen_cap`'s own doc already recorded
            // this stack returning "nothing at all after fifty minutes, twice".
            //
            // `validate_from_env` defaults to ON, so nothing changes unless the
            // operator sets `BRUTEX_VALIDATE=0` — and a run taken that way
            // carries the `UNVALIDATED` banner `CLAUDE.md` §5 requires, so a
            // candidate can never be mistaken for a finding.
            validate: validate_from_env(),
            lens: runner::rank::Lens::Detectability,
        };
        let first = audit_bars(evaluator(), short.clone(), "", 120, None, plain);
        let second = audit_bars(evaluator(), short, "", 120, None, plain);
        checks.push(Check {
            claim: "two runs of one slice agree byte for byte",
            held: first == second,
            evidence: format!(
                "{} bytes vs {} bytes, {}",
                first.len(),
                second.len(),
                if first == second {
                    "identical"
                } else {
                    "DIFFER"
                }
            ),
        });
    }

    // 3. SUFFIX INDEPENDENCE -- the measurable face of no-look-ahead. A bar's
    // conditions must not change because LATER bars exist, so a column built on
    // a prefix must agree with the same rows of a column built on the whole.
    if let Ok(span) = &span {
        let whole: Vec<indicators::Candle> = span.bars.iter().take(900).copied().collect();
        let prefix: Vec<indicators::Candle> = whole.iter().take(600).copied().collect();
        let mut ev_a = match evaluator() {
            Ok(ev) => ev,
            Err(why) => return format!("refused: {why}\n"),
        };
        let mut ev_b = ev_a;
        let on_whole = indicators::column::Column::build(&whole, &mut ev_a);
        let on_prefix = indicators::column::Column::build(&prefix, &mut ev_b);
        let shared = on_prefix.len();
        let disagreements = on_prefix
            .bits()
            .iter()
            .zip(on_whole.bits().iter().take(shared))
            .filter(|(a, b)| a != b)
            .count();
        checks.push(Check {
            claim: "a bar's conditions do not change because later bars exist",
            held: disagreements == 0,
            evidence: format!("{disagreements} of {shared} rows differ"),
        });
    }

    checks.push(ledger_round_trip());
    checks.push(refusal_surface(vendor_word, underlying));

    let passed = checks.iter().filter(|c| c.held).count();
    for check in &checks {
        let _ = writeln!(
            out,
            "  {:<6}{:<62}{}",
            if check.held { "PASS" } else { "FAIL" },
            check.claim,
            check.evidence
        );
    }
    let _ = writeln!(
        out,
        "\n  {passed} of {} checks passed.{}",
        checks.len(),
        if passed == checks.len() {
            " Every property above was MEASURED on your store just now, not \
             recalled."
        } else {
            " A FAILURE ABOVE MEANS A RESULT FROM THIS ENGINE CANNOT BE \
             TRUSTED UNTIL IT IS FIXED."
        }
    );
    out
}

/// The two properties a joined span must have before anything is computed on it.
///
/// `Column::build` folds bar by bar and §3 rule 7's no-look-ahead property is
/// held by that shape, so a series that stepped backwards at a month boundary
/// would fold a later bar into an earlier state with nothing downstream to
/// notice. A hole is the other half: a span missing months is a SHORTER sample
/// and every figure taken over it is over that shorter sample.
fn span_checks(span: &Result<stored::Span, stored::Refusal>) -> Vec<Check> {
    match span {
        Err(why) => vec![Check {
            claim: "the span loads at all",
            held: false,
            evidence: why.lines().next().unwrap_or(why).to_owned(),
        }],
        Ok(span) => {
            let steps_back = span
                .bars
                .windows(2)
                .filter(|w| {
                    let then = w.first().map_or(i64::MIN, |b| b.ts_micros);
                    let now = w.last().map_or(i64::MIN, |b| b.ts_micros);
                    now <= then
                })
                .count();
            vec![
                Check {
                    claim: "every month the range asked for is present",
                    held: span.complete(),
                    evidence: format!("{} of {} months", span.found, span.asked),
                },
                Check {
                    claim: "the joined series is strictly increasing in time",
                    held: steps_back == 0,
                    evidence: format!(
                        "{steps_back} backward steps across {} bars",
                        span.bars.len()
                    ),
                },
            ]
        }
    }
}

/// Six hostile requests against THIS feed, all of which must be refused.
///
/// # Feed-scoped, not literal
///
/// The set is built from the feed and instrument under test rather than from
/// hard-coded names. A verification that always attacked `zerodha NIFTY` would
/// prove nothing about the feed the operator actually asked about — and this
/// whole surface is feed-scoped by design: the ledger keys on `feed`, the
/// listing filters on it, and `feed` is the NINTH term of the run identity
/// precisely so two vendors redistributing one exchange's data are never
/// collapsed into one answer.
///
/// The property is not merely "exit non-zero". It is **exit non-zero AND print
/// no provenance banner** — `cli`'s own header calls the two banners the only
/// thing separating a real sweep from a generated one, so a refusal that still
/// printed one would be the failure this check exists to catch.
fn refusal_surface(vendor_word: &str, underlying: &str) -> Check {
    let v = vendor_word.to_owned();
    let u = underlying.to_owned();
    let ppm = "200000".to_owned();
    let hostile: [[String; 8]; 6] = [
        // A feed no build knows.
        [
            "range-all".into(),
            "nosuchfeed".into(),
            u.clone(),
            "2019".into(),
            "12".into(),
            "2026".into(),
            "8".into(),
            ppm.clone(),
        ],
        // This feed, range inverted.
        [
            "range-all".into(),
            v.clone(),
            u.clone(),
            "2026".into(),
            "8".into(),
            "2019".into(),
            "12".into(),
            ppm.clone(),
        ],
        // This feed, a month that is not a month.
        [
            "range-all".into(),
            v.clone(),
            u.clone(),
            "2019".into(),
            "13".into(),
            "2026".into(),
            "8".into(),
            ppm.clone(),
        ],
        // This feed, an instrument shaped like a path.
        [
            "range-all".into(),
            v.clone(),
            "../../etc".into(),
            "2019".into(),
            "12".into(),
            "2026".into(),
            "8".into(),
            ppm.clone(),
        ],
        // This feed, no instrument at all.
        [
            "range-all".into(),
            v.clone(),
            String::new(),
            "2019".into(),
            "12".into(),
            "2026".into(),
            "8".into(),
            ppm,
        ],
        // This feed, a support that would disable extinction.
        [
            "range-all".into(),
            v,
            u,
            "2019".into(),
            "12".into(),
            "2026".into(),
            "8".into(),
            "0".into(),
        ],
    ];
    let mut refused = 0_usize;
    for args in &hostile {
        let mut sink = String::new();
        if run(args.as_ref(), &mut sink) == MISUSED && !sink.contains("REAL MARKET DATA") {
            refused = refused.saturating_add(1);
        }
    }
    Check {
        claim: "a request nothing can serve is refused, no provenance banner",
        held: refused == hostile.len(),
        evidence: format!("{refused} of {} refused cleanly", hostile.len()),
    }
}

/// Writes one record to a scratch ledger, reads it back, and compares.
///
/// Uses a temporary directory rather than the operator's ledger: a verification
/// that appended a fake row to the real history would corrupt the thing it
/// exists to protect.
///
/// # The directory names this process, and it used to be a constant
///
/// It read `temp_dir().join("brutex-verify-ledger")`, one fixed path shared by
/// every `cli verify` on the machine — and the very next line is
/// `remove_dir_all`. Two verifications running at once is not exotic: it is one
/// operator in two terminals, or a terminal beside the server's own. The second
/// to start would delete the first's scratch mid-write, and the first would
/// report a round-trip failure against a ledger the second had removed. A
/// self-check that fails because another self-check is running teaches the
/// operator the opposite of what it exists to say.
///
/// `process::id()` is what makes them disjoint, and it is the discriminator
/// rather than a clock because two processes CAN start in the same microsecond
/// and cannot share a pid. Gate 23 clause C refuses a fixed temporary path for
/// exactly this reason.
fn ledger_round_trip() -> Check {
    let dir = std::env::temp_dir().join(format!("brutex-verify-ledger-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let want = crate::results::Record {
        identity: [0xAB; 32],
        finished_micros: 1_787_000_000_000_000,
        feed: crate::results::field("zerodha"),
        underlying: crate::results::field("NIFTY"),
        timeframe: crate::results::field("10min"),
        from_year: 2019,
        from_month: 12,
        to_year: 2026,
        to_month: 8,
        months_asked: 81,
        months_found: 81,
        bars: 623_546,
        min_hits: 124_709,
        combinations: 54_895_691,
        depth: 16,
        halted: 0,
        trades: 11_209,
        pessimistic: 2_459_160,
        optimistic: 3_649_640,
        worst_trade: -4_694,
        max_drawdown: 29_163,
        winner_mae: 300,
        winner_mfe: 700,
        all_mae: 500,
        exit_rungs: [-1, -1, 3, 0, 0],
        mask_words: [0xDEAD_BEEF, 0, 0, 0, 0, 0x1234],
    };
    let outcome = crate::results::Results::open(&dir)
        .and_then(|mut store| {
            let at = store.append(&want)?;
            store.read(at)
        })
        .map(|got| got == want);
    let _ = std::fs::remove_dir_all(&dir);
    match outcome {
        Err(why) => Check {
            claim: "a recorded run reads back exactly as written",
            held: false,
            evidence: why,
        },
        Ok(same) => Check {
            claim: "a recorded run reads back exactly as written",
            held: same,
            evidence: format!(
                "24 fields at stride {}, {}",
                crate::results::STRIDE,
                if same { "all equal" } else { "MISMATCH" }
            ),
        },
    }
}

/// Rows the listing prints before it says how many it dropped.
///
/// A bound, not a page size: there is no cursor and no second call. A listing
/// that silently showed the best fifty of four thousand would read as the whole
/// ledger, so whatever this drops is stated on the line below the table —
/// `crate::report`'s rule for the exit grid, applied to the same problem.
const LIST_ROWS: usize = 40;

/// The ranked frontier of one recorded run, read back from the store.
///
/// # The question this answers, which nothing could answer before
///
/// *"Show me the top twenty-five combinations."* Until [`crate::frontier`]
/// existed, the ledger held ONE combination per run and everything else the
/// ladder found was folded into `depth` and `combinations` and dropped — so the
/// answer could be printed once, on the run that produced it, and never again.
///
/// Which run: the **best complete** one matching the filter, chosen exactly as
/// [`best_complete_line`] chooses it — highest `pessimistic` among rows that did
/// not halt. A halted run's totals are not comparable with a complete one's, so
/// ranking them together would be the defect `range-all`'s `complete` column
/// exists to prevent.
///
/// # It reports the run it chose, and why
///
/// A frontier printed without saying which run it belongs to is a list of masks.
/// The banner names the feed, the instrument, the rung, the span and the
/// identity, so a reader can put the rows beside the ledger row they came from.
#[must_use]
pub fn top_list(feed: Option<&str>, underlying: Option<&str>) -> String {
    match store_root() {
        Ok(root) => top_at(&root, feed, underlying),
        Err(why) => format!("refused: {why}\n"),
    }
}

/// [`top_list`], against a root the caller names.
///
/// Split out so the rendering can be tested against a scratch store. The env
/// lookup is the only part that cannot be, and it is one line above.
#[must_use]
pub fn top_at(root: &std::path::Path, feed: Option<&str>, underlying: Option<&str>) -> String {
    let rows = match newest_complete(root, feed, underlying) {
        Ok(Some(record)) => record,
        Ok(None) => {
            return "  NO COMPLETE RUN matches. Every matching row halted on a \
                    budget, or nothing has been recorded yet — `cli results` \
                    lists what is there.\n"
                .to_owned();
        }
        Err(why) => return format!("refused: {why}\n"),
    };

    let mut store = match crate::frontier::Frontier::open(root) {
        Ok(store) => store,
        Err(why) => return format!("refused: {why}\n"),
    };
    let (found, damaged) = match store.of_run(&rows.identity) {
        Ok(pair) => pair,
        Err(why) => return format!("refused: {why}\n"),
    };

    let mut out = String::from(STORED_PROVENANCE);
    out.push('\n');
    let _ = writeln!(
        out,
        "\nTOP COMBINATIONS\n  feed {} · {} · {} · {}-{:02}..{}-{:02}\n  run {}",
        crate::results::read_field(&rows.feed),
        crate::results::read_field(&rows.underlying),
        crate::results::read_field(&rows.timeframe),
        rows.from_year,
        rows.from_month,
        rows.to_year,
        rows.to_month,
        rows.identity_hex()
    );

    if found.is_empty() {
        let _ = writeln!(
            out,
            "\n  This run recorded NO frontier. Rows are written by runs made \
             after `cli frontier` landed; an older run has a ledger row and no \
             ranked list, which is a gap in the record rather than an empty \
             result. Re-run it to fill one in."
        );
        return out;
    }

    let _ = writeln!(
        out,
        "\n  {:<5}{:>10}{:>9}{:>14}{:>10}{:>10}  conditions",
        "rank", "hits", "n", "mean", "t", "payoff"
    );
    for row in &found {
        let _ = writeln!(
            out,
            "  {:<5}{:>10}{:>9}{:>14}{:>10}{:>10}  {}",
            row.rank,
            row.hits,
            row.n,
            // MEAN IS PAISA AND IS SHOWN AS RUPEES, like every other money
            // column in this binary. It is stored in thousandths of a paisa, so
            // it comes back to whole paisa first.
            rupees(row.mean_milli_paisa / 1_000),
            hundredths_of(row.t_milli / 10),
            if row.payoff_bp == i64::MAX {
                "inf".to_owned()
            } else {
                hundredths_of(row.payoff_bp)
            },
            runner::report::names_from_words(row.mask_words).join(" · ")
        );
    }

    let _ = writeln!(
        out,
        "\n  `mean` is the average forward move over the run's horizon, per ONE \
         unit of the index, gross of the statutory charge stack.\n  `payoff` is \
         the mean WIN over the mean LOSS, in hundredths -- 300 reads 3.00. It \
         carries no stop, no target and no path,\n  so it does not say what a \
         stop would have done: it says which combinations are worth asking. \
         `cli elite` ranks the cut on it.\n  `inf` means the combination never \
         lost on this span, which is a fact about the sample and not a promise."
    );
    if let Some(why) = damaged {
        let _ = writeln!(
            out,
            "\n  PART OF THE FRONTIER COULD NOT BE READ, and the rows above are \
             what survived: {why}"
        );
    }
    out
}

/// The best COMPLETE recorded run matching the filter, newest wins a tie.
fn newest_complete(
    root: &std::path::Path,
    feed: Option<&str>,
    underlying: Option<&str>,
) -> Result<Option<crate::results::Record>, crate::results::Refusal> {
    let mut store = crate::results::Results::open(root)?;
    let count = store.len()?;
    let mut best: Option<crate::results::Record> = None;
    for index in 0..count {
        let record = store.read(index)?;
        // HALTED ROWS ARE NOT CANDIDATES. A halted run's total covers less of
        // the ladder than its combination count suggests, so ranking it against
        // a complete one compares two different searches.
        if record.halted != 0 {
            continue;
        }
        if feed.is_some_and(|f| crate::results::read_field(&record.feed) != f) {
            continue;
        }
        if underlying.is_some_and(|u| crate::results::read_field(&record.underlying) != u) {
            continue;
        }
        // `>=` so a later run wins a tie: two runs with identical totals are
        // the same answer, and the newer one is the one an operator just made.
        if best.is_none_or(|b| record.pessimistic >= b.pessimistic) {
            best = Some(record);
        }
    }
    Ok(best)
}

/// Every recorded run, newest first, optionally narrowed to one feed and
/// instrument.
///
/// # This is the half of the ledger that was missing
///
/// Runs have been recorded since the results store landed, and nothing could
/// READ them. A ledger that can only be appended to is a write-only file: the
/// operator's actual questions — *which threshold did best on NIFTY 15-minute*,
/// *did the 7-year run ever complete*, *what did I already try* — all need the
/// rows back out.
///
/// # Ranked by the WORST-case total, deliberately
///
/// Selection ranks on the pessimistic figure everywhere else in this workspace,
/// for the reason `crate::validate` gives: a search ranked on the flattering
/// reading picks whatever the flattering assumption helped most. A listing that
/// ordered by the best case would quietly propose a different winner from the
/// one every other surface names.
///
/// # Cost
///
/// `O(rows)` — the size of the answer, and every individual read is `O(1)` at
/// `HEADER + i·STRIDE`. There is no scan of anything larger than the ledger and
/// no index to maintain, which is `CLAUDE.md` §4's *"the path is the index"*
/// applied to a file that is one array.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn results_list(feed: Option<&str>, underlying: Option<&str>) -> String {
    let root = match store_root() {
        Ok(root) => root,
        Err(why) => return format!("refused: {why}\n"),
    };
    let mut store = match crate::results::Results::open(&root) {
        Ok(store) => store,
        Err(why) => return format!("refused: {why}\n"),
    };
    let count = match store.len() {
        Ok(count) => count,
        Err(why) => return format!("refused: {why}\n"),
    };

    let mut out = String::from("RECORDED RUNS\n");
    let _ = writeln!(
        out,
        "  file                                    {}",
        crate::results::Results::path(&root).display()
    );
    let _ = writeln!(out, "  rows                                    {count}");
    if count == 0 {
        let _ = writeln!(
            out,
            "\n  Nothing has been recorded yet. `cli audit-range` and `cli \
             range-all` write a row each; the other commands do not, because a \
             synthetic sweep has no feed or instrument to name."
        );
        return out;
    }

    // NEWEST FIRST, read backwards. The ledger is append-only, so the last row
    // is the most recent and no sort is needed to say so.
    let mut rows: Vec<crate::results::Record> = Vec::new();
    for back in 1..=count {
        match store.read(count.saturating_sub(back)) {
            Err(why) => return format!("refused: {why}\n"),
            Ok(record) => {
                let keep = feed.is_none_or(|f| crate::results::read_field(&record.feed) == f)
                    && underlying
                        .is_none_or(|u| crate::results::read_field(&record.underlying) == u);
                if keep {
                    rows.push(record);
                }
            }
        }
    }
    if let (Some(f), Some(u)) = (feed, underlying) {
        let _ = writeln!(out, "  filtered to                             {f} {u}");
    }
    let _ = writeln!(
        out,
        "  matching                                {}",
        rows.len()
    );
    let _ = writeln!(out);

    let _ = writeln!(
        out,
        "  {:<9}{:<8}{:>16}{:>16}{:>7}{:>6}{:>6}{:>17}{:>17}{:>9}",
        "feed",
        "rung",
        "COMBINATIONS",
        "min_hits",
        "months",
        "depth",
        "done",
        "worst",
        "best",
        "trades"
    );
    for record in rows.iter().take(LIST_ROWS) {
        let _ = writeln!(
            out,
            "  {:<9}{:<8}{:>16}{:>16}{:>7}{:>6}{:>6}{:>17}{:>17}{:>9}",
            crate::results::read_field(&record.feed),
            crate::results::read_field(&record.timeframe),
            // THE NUMBER THE WHOLE ENGINE EXISTS TO PRODUCE, and the listing did
            // not print it. An operator asking "how many combinations did this
            // actually weigh" had no answer on this surface at all -- the field
            // has been in every record since the ledger landed.
            grouped(record.combinations),
            grouped(record.min_hits),
            format!("{}/{}", record.months_found, record.months_asked),
            record.depth,
            if record.halted == 0 { "yes" } else { "NO" },
            rupees(record.pessimistic),
            rupees(record.optimistic),
            record.trades,
            // The first sixteen hex characters. The full digest is 64 and would
            // own the line; sixteen is enough to find a row and short enough to
            // read, and `cli results` prints the whole one when a row is asked
            // for by name.
        );
    }
    if rows.len() > LIST_ROWS {
        let _ = writeln!(
            out,
            "  ... {} further row(s) NOT SHOWN. The ledger is complete; this \
             table is not.",
            rows.len().saturating_sub(LIST_ROWS)
        );
    }

    // THE BEST ROW, BY THE FIGURE SELECTION USES.
    let _ = writeln!(out);
    out.push_str(&best_complete_line(&rows));
    // AND WHAT IT RISKED, for the row just named. A total answers "how much did
    // it make" and nothing else; these answer "what did it risk to make it",
    // which is the question the exit grid exists to price.
    if let Some(best) = rows
        .iter()
        .filter(|r| r.halted == 0)
        .max_by_key(|r| r.pessimistic)
    {
        out.push_str(&quality_block(best));
    }
    out
}

/// One rung's `min_hits`, derived from that rung's own bar count.
///
/// # Why `range-all` takes SUPPORT and not a hit count
///
/// Over 81 months of NIFTY the nine rungs hold wildly different numbers of bars:
/// **1,671 on `1day` and 623,546 on `1min`, a 373-fold spread.** One absolute
/// `min_hits` therefore means a different question on every rung — 300 hits is
/// 20% support on daily and 0.05% on one-minute — so the coarse rungs finish in
/// seconds while the fine ones blow through the candidate ceiling and report
/// `REFUSED`. A table of nine rows built that way compares nothing.
///
/// "This combination fired on 20% of bars" means the same thing on every rung.
/// "It fired 300 times" does not. So the argument is support, in parts per
/// million for the same reason every other ratio in this workspace is an
/// integer — `CLAUDE.md` §7 keeps floats out of anything compared.
///
/// Floored at one: a support so small it rounds to zero hits would disable
/// extinction entirely, which is the defect `engine::Ladder::with_min_hits`
/// raises zero to one to prevent.
const fn min_hits_for(bars: usize, support_ppm: u64) -> u64 {
    let hits = (bars as u64).saturating_mul(support_ppm) / 1_000_000;
    if hits == 0 { 1 } else { hits }
}

/// Every rung this engine SWEEPS, finest first.
///
/// # EIGHT, and `1day` is deliberately not among them
///
/// This engine is intraday: §1, and `outcome::AUTO_CLOSE_MINUTE` squares every
/// position off at 15:10. A daily bar IS one whole session, so sweeping it as a
/// signal rung produces at most one trade per day — **365 round trips in 6.75
/// years**, measured — and every one of them acts on a condition that was not
/// knowable until the session it describes had already closed.
///
/// **Daily data is an INPUT, not a rung.** The previous session's high, low and
/// close feed `indicators::daily`, which derives PDH, PDL, the CPR band and the
/// five pivot bands — and those are already in the vocabulary and already
/// firing. Every one of `close_above_pdh`, `gap_down_day`, `below_cpr_bc` and
/// `close_below_pivot_r2_band` appeared in a 15-minute sweep's own findings.
///
/// `Daily::from_previous_session` is fed by the ROLLOVER inside whatever series
/// the evaluator is given, so a 15-minute sweep already carries yesterday's OHLC
/// without a daily file being opened. Sweeping `1day` separately adds no
/// condition the intraday rungs lack and costs a full pass to produce a sample
/// too small to judge.
///
/// `1s` is absent for a different reason: a seven-year second-resolution span is
/// roughly sixty times the one-minute series and nothing here has measured what
/// that costs. §3 rule 6 — an unmeasured bound is not a bound.
pub const EVERY_RUNG: [&str; 8] = [
    "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min",
];

/// The trade walk, the exit grid and the screen, for one chosen combination.
///
/// Split from [`audit_bars`] to keep it under `clippy::too_many_lines`, and
/// because these three belong together: they are the only things in the whole
/// report that run on the EXECUTION series rather than the signal one, and
/// keeping them in one function makes that boundary visible instead of
/// scattered.
fn trade_and_screen(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    first: &runner::rank::Scored,
    by_evidence: &[&runner::rank::Scored],
    horizon: Horizon,
    rules: Rules,
    // Forwarded to `screen_cascade`: a search step must not price 960 tiers.
    validate: bool,
) -> (
    runner::trade::Trades,
    grid::Grid,
    String,
    std::collections::HashMap<[u64; 6], grid::Cell>,
) {
    let side = side_of_evidence(first);
    let taken = trade::walk(bars, column, &first.mask, horizon, direction_of(side));
    // The same forced rung the screen uses, so the headline grid and the screened
    // rows are built from one ladder. Two ladders would let the report show a
    // variant in one table that cannot exist in the other.
    //
    // `Levels` borrows the ladder, so it is bound before the call.
    let stop_rungs = stop_ladder_ppm(bars);
    let exits = grid::evaluate(
        bars,
        column,
        &first.mask,
        horizon,
        side,
        grid::Levels {
            rungs: grid_rungs(bars),
            step_ppm: Some(grid_step_ppm(bars)),
            forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
            ratios: true,
            stops_ppm: &stop_rungs,
        },
    );
    // THE CASCADE, NOT ONE POLICY. A single screen answers "0 of 21 satisfy
    // every rule" and stops -- true, and nearly useless: it says the standard
    // was not met without saying which standard WAS. The ladder walks from
    // S+++ down and reports the strictest tier that yields anything, naming
    // every tier above it as unmet.
    //
    // `rules.top` is carried through because how MANY rows to print is the
    // operator's choice and not part of the policy being relaxed.
    // THE OPERATOR'S RULES, not just their `top`. This passed `rules.top`
    // alone, so the policy an operator stated reached nothing and a generated
    // tier judged the rows instead.
    // THE CELLS COME BACK WITH THE TEXT. Every metric the operator ranks on is
    // on a `Cell`, and until now `screen` computed one per candidate and
    // returned only the rendered table -- so the frontier could store the
    // sweep's statistics and nothing about the money.
    let mut priced: std::collections::HashMap<[u64; 6], grid::Cell> =
        std::collections::HashMap::with_capacity(by_evidence.len().min(screen_cap()));
    let screened = screen_cascade(
        bars,
        column,
        by_evidence,
        horizon,
        rules,
        validate,
        &mut priced,
    );
    (taken, exits, screened, priced)
}

/// The three lines that precede the trade figures: which combination was taken,
/// how much of the grid it exposed, and how large the sample is.
///
/// Split from [`audit_bars`] to keep it under `clippy::too_many_lines`. One
/// idea: everything a reader needs BEFORE a number, so no figure below arrives
/// without the sample size and the exposure that qualify it.
fn traded_preamble(
    first: &runner::rank::Scored,
    sweep: &engine::Sweep,
    sessions: usize,
    bars: &[indicators::Candle],
) -> String {
    let mut out = traded_line(first);
    out.push_str(&grid_exposure(sweep, bars));
    out.push_str(&sample_warning(sessions));
    out
}

/// The operator's three rules, supplied at the command line and never inferred.
///
/// # Every one of these is the operator's to set, and none is the engine's to invent
///
/// `CLAUDE.md` §6 explains why a parameter that can be set wrongly and silently
/// is worse than no parameter: the predecessor's depth flag defaulted to a token
/// that was unreachable on the only vocabulary that existed, and nobody could
/// see it from the flag. These three are the opposite case — they encode a
/// TRADING POLICY the engine has no way to derive, so they must be stated, and
/// a default would be the engine inventing a risk appetite it cannot know.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// No single trade may run more than this against entry, in ppm.
    ///
    /// Checked against [`grid::Cell::worst_mae`], the MAXIMUM across every
    /// trade, because a stop is placed once and every trade must survive it.
    /// The three MAE means cannot answer it: a mean of 0.05% is entirely
    /// consistent with one trade at 1.06%, which is exactly what a real
    /// sixty-minute run produced.
    pub max_mae_ppm: i64,
    /// The average win must be at least this multiple of the average loss, in
    /// hundredths. `200` reads 2.00, which is the 1:2 an operator asks for.
    ///
    /// **This is the rule an 81%-win-rate strategy fails.** A real run scored
    /// 81.19% profitable with an average win of ₹7.57 against an average loss of
    /// ₹18.65 — a reward-to-risk of 0.41, wearing an exceptional win rate. Win
    /// rate alone cannot see that shape and this ratio is what catches it.
    pub min_rr_bp: i64,
    /// The share of trades that must WIN, in basis points. `9_000` reads 90%.
    ///
    /// # The rule that had no home
    ///
    /// An operator's requirement is usually three numbers — "out of a hundred
    /// trades, ninety win, and a loser never runs more than ten points" — and
    /// until this field the middle one could not be stated. `max_mae_ppm` caps
    /// the loss and `min_rr_bp` sets the reward-to-risk, but a combination can
    /// satisfy both while winning a third of the time: 33 winners at 3R against
    /// 67 losers at 1R is a reward-to-risk of 3.00 and a losing strategy.
    ///
    /// [`grid::Cell::win_rate_bp`] has always computed this and nothing ever
    /// filtered on it. Basis points rather than a percentage because §7 keeps
    /// floats out of anything compared, and because 90% and 90.5% are different
    /// rules an operator may well want to distinguish.
    ///
    /// Zero drops the rule, the way `min_rr_bp` of zero does.
    pub min_win_rate_bp: i64,
    /// The fewest round trips a combination may report and still be believed.
    ///
    /// # Why a rule and not a footnote
    ///
    /// "Ninety of a hundred won" is a claim about a rate, and a rate over four
    /// trades is not evidence of anything. Three of four is 75% and means
    /// nothing; 900 of 1,000 is the same arithmetic and means a great deal. The
    /// engine already refuses to SCORE a thin sample — the report prints `TOO
    /// FEW OBSERVATIONS` — but the screen would still rank a four-trade
    /// combination above a thousand-trade one if its ratios read better.
    ///
    /// Separate from the significance bar upstream on purpose: that asks
    /// "could the best of N hypotheses look this good by luck", which is about
    /// the SEARCH. This asks "is this particular row's sample big enough to
    /// mean what it says", which is about the ROW.
    ///
    /// Zero drops the rule.
    pub min_trades: u64,
    /// The **95% lower bound** on the win rate that a row must still clear, in
    /// basis points. `9_000` reads "even pessimistically, ninety percent".
    ///
    /// # The rule that lets `min_trades` come DOWN
    ///
    /// The operator's requirement is not "many trades". It is *"very minimal
    /// trades, but confirmed"* -- a rare, high-conviction setup that fires forty
    /// times in seven years and never loses. Every other rule here fights that:
    /// `min_trades` rejects it outright, and the ranking it feeds sorts by total
    /// P&L, which is volume times edge and so always prefers the grinder.
    ///
    /// [`grid::Cell::assurance_bp`] settles it without a floor anyone has to
    /// guess. Measured, from the tests that pin it:
    ///
    /// | record | raw rate | this bound |
    /// |---|---|---|
    /// | 12 of 12 | 100.00% | 75.75% |
    /// | 40 of 40 | 100.00% | 91.23% |
    /// | 900 of 1,000 | 90.00% | 87.98% |
    /// | 18,560 of 19,759 | 93.93% | 93.59% |
    ///
    /// A forty-trade perfect record clears a 90% bar. A twelve-trade one does
    /// not. **Neither number was chosen** -- the arithmetic ranked them, and it
    /// is the same arithmetic whether the sample is forty or forty thousand.
    ///
    /// # Why this does not make `min_trades` redundant
    ///
    /// It very nearly does, and that is deliberate: with this rule set, the
    /// honest `min_trades` is small or zero, because the bound already refuses
    /// what a floor was standing in for. It is kept because the two refuse for
    /// different reasons and an operator is entitled to both -- a bound of 90%
    /// admits 40 of 40, and an operator who will not trade a system on forty
    /// observations regardless of its statistics may still say so.
    ///
    /// Zero drops the rule, the way `min_rr_bp` of zero does.
    pub min_assurance_bp: i64,
    /// The share of periods that must close POSITIVE **at every grain**, in
    /// basis points. `10_000` reads "every single one, at all six".
    ///
    /// # The requirement that was reported and never enforced
    ///
    /// The operator's rule is *"every month, every week, every day, every
    /// quarter, every half and every year the same"*. The screen learned to
    /// MEASURE that and then printed it beside rows it had already admitted --
    /// so a combination positive in every year and negative in half its months
    /// was still stamped PASS, with the evidence against it in the next table
    /// down.
    ///
    /// Checked against the WEAKEST grain, not the mean. Averaging 10,000 and
    /// 5,000 reports 9,166 and clears a 90% bar; the minimum reports 5,000 and
    /// refuses, and refusing is the whole point.
    ///
    /// # Why it cannot live in `admits`
    ///
    /// [`Rules::admits`] takes a [`grid::Cell`], and a cell has no calendar --
    /// consistency needs the trades themselves, bucketed, which is only
    /// knowable once a variant has been CHOSEN. So the five cell rules gate the
    /// variant and this one gates the row, after the fact. That ordering is
    /// stated rather than hidden because it has a consequence: a combination
    /// whose chosen variant is inconsistent is refused, even if some OTHER
    /// variant of it would have been steady. Finding that variant would mean
    /// re-walking every cell trade by trade, which is the cost this deliberately
    /// does not pay.
    ///
    /// Zero drops the rule, the way every other rule here does.
    pub min_weakest_bp: i64,
    /// Total profit as a multiple of the worst peak-to-trough fall, in
    /// hundredths. `500` reads "made at least five times what it ever gave
    /// back".
    ///
    /// # The measurement that gated nothing
    ///
    /// [`grid::Cell::max_drawdown`] is the largest fall of the running total and
    /// has been on every record since the ledger landed.
    /// [`grid::Cell::return_over_drawdown`] divides the total by it, correctly,
    /// and its own doc admitted the gap in four words: **"Nothing ranks on this
    /// yet."** It appeared in no selector — not `Rules::admits`, not
    /// `Grid::best`, not `best_within`, not `sharpest`, not `validate`'s argmax.
    ///
    /// So the engine could name as winner a combination that made a hundred
    /// after twice giving back eighty, over one that ground steadily to ninety.
    /// Every other rule here is a property of the trade LIST and cannot see the
    /// order trades arrived in; this is the only one about the PATH, which is
    /// what an operator actually has to sit through.
    ///
    /// A variant that never gave anything back reports [`i64::MAX`] and clears
    /// any floor, which is the honest answer rather than a division by zero.
    ///
    /// Zero drops the rule, the way [`Self::min_rr_bp`] of zero does.
    pub min_ret_over_dd_bp: i64,
    /// How many combinations to report. Ten or twenty-five, the operator's call.
    pub top: usize,
}

impl Rules {
    /// Whether a variant satisfies every rule. All of them, not a score.
    ///
    /// A rule broken once is a disqualification and not a lower rank: a stop
    /// that one trade in ten thousand ran through is a stop that did not hold.
    #[must_use]
    pub fn admits(&self, cell: &grid::Cell) -> bool {
        // FOUR RULES, AND EACH ANSWERS A QUESTION THE OTHERS CANNOT.
        //
        // A combination can pass any three and fail the fourth, which is why
        // none of them is redundant:
        //
        //   * `worst_mae` alone: a stop that held, on twelve trades.
        //   * `reward_to_risk` alone: 33 winners at 3R against 67 losers at 1R
        //     reads 3.00 and loses money.
        //   * `win_rate` alone: this is the shape a real run already produced --
        //     81.19% profitable, average win 7.57 against average loss 18.65,
        //     a reward-to-risk of 0.41 wearing an exceptional win rate.
        //   * `trades` alone: three of four is 75% and is not evidence.
        //
        // All of them, and a rule broken once is a disqualification rather than
        // a lower rank: a stop that one trade in ten thousand ran through is a
        // stop that did not hold.
        // AND A FIFTH, WHICH IS THE ONE THAT ADMITS A SNIPER.
        //
        // The four above can all be satisfied by a rare, perfect, tiny sample --
        // and `min_trades` is the only thing that was stopping one, by refusing
        // it outright rather than by weighing it. `assurance_bp` weighs it: a
        // forty-trade perfect record clears 90%, a twelve-trade one does not,
        // and no floor had to be guessed to separate them.
        //
        // It is NOT `const`-incompatible by accident -- `assurance_bp` takes a
        // square root, so this function loses `const`. That is the price of the
        // only statistic that can rank a sniper against a grinder honestly.
        // AND A SIXTH, WHICH IS THE ONLY ONE ABOUT THE PATH.
        //
        // The five above are all properties of the TRADE LIST -- how far one
        // trade ran against you, how the average win compares to the average
        // loss, how many won, how many there were, how sure we are of the rate.
        // Not one of them can see the ORDER the trades arrived in, and order is
        // the whole of survivability: a variant that made a hundred after twice
        // giving back eighty is indistinguishable from one that ground steadily
        // to a hundred, on every rule above.
        //
        // `grid::Cell::max_drawdown` has measured that since the ledger landed
        // and NOTHING ranked or gated on it -- `return_over_drawdown`'s own doc
        // said so in words: "Nothing ranks on this yet". This is the rule that
        // makes the measurement bite, and it is the operator's actual
        // requirement: not "massive profit", but "massive profit at a drawdown
        // I could sit through".
        //
        // ZERO DROPS THE RULE, AND IT HAD TO BE SPELLED OUT TO ACTUALLY DO SO.
        //
        // The comment above already claimed this, "the way `min_rr_bp` of zero
        // does" — and the analogy is exactly where it went wrong. Every other
        // rule on this list is a `>=` against a floor, where zero is satisfied
        // by anything and the rule genuinely disappears. This one is a `<=`
        // against a CEILING, and a ceiling of zero is not absent, it is
        // impossible: `Cell::worst_mae` is a non-negative ppm excursion that
        // `grid::peak` starts at zero and only ever raises, measured from the
        // entry bar's adverse extreme, so it is strictly positive on any bar
        // with a range.
        //
        // So `max_mae_ppm == 0` rejected EVERY cell on the first conjunct, and
        // `Rules::operator()` — which the browser sweep now admits on — takes
        // exactly that default. The screen would have found nothing admitted,
        // printed `YOUR RULES: UNMET`, and fallen through to the generated tier
        // ladder on every single run: the operator's four stated criteria
        // silently replaced by a cascade they never asked for, with no error
        // anywhere. Found by audit before it reached a run.
        //
        // The other five keep their bare comparison, because for a floor the
        // analogy holds and adding a guard would be noise.
        (self.max_mae_ppm == 0 || cell.worst_mae <= self.max_mae_ppm)
            && cell.reward_to_risk_bp() >= self.min_rr_bp
            && cell.win_rate_bp() >= self.min_win_rate_bp
            && cell.trades >= self.min_trades
            && cell.assurance_bp() >= self.min_assurance_bp
            && cell.return_over_drawdown() >= self.min_ret_over_dd_bp
    }
}

impl Rules {
    /// The rules a command that does not take them uses.
    ///
    /// # A default here is not the §6 defect, and the difference matters
    ///
    /// `CLAUDE.md` §6 refuses a DEPTH default because depth is decided by
    /// extinction and a parameter that can be set wrongly is set wrongly and
    /// silently. These encode a trading POLICY, and the commands that carry this
    /// default — `sweep`, `audit`, `audit-stored`, `audit-range` — do not take a
    /// policy argument at all. Printing no screen would be worse than printing
    /// one against a STATED baseline, so long as the baseline is on the page.
    ///
    /// It IS on the page: `screen` prints all three rules above its table, in
    /// the operator's units, on every run. A reader can always see which numbers
    /// produced the PASS and FAIL column, and `cli screen` takes all three.
    ///
    /// 2,000 ppm is 0.20% — about fifty points on a 25,000 index. 200 is a 1:2
    /// reward-to-risk. Twenty-five is the count an operator asked for.
    /// The operator's own four criteria, resolved at RUNTIME, defaults stated.
    ///
    /// # Why this exists: two of the four were switched off
    ///
    /// The browser's Run button reaches `audit_range_inner`, which admitted rows
    /// on [`Self::BASELINE`]. Measured against the operator's standing rule —
    /// *"massive win rate, winning ratio, very small stop loss, very small max
    /// drawdown, top 10 to 25"* — `BASELINE` reads:
    ///
    /// | the rule | `BASELINE` | what that means |
    /// |---|---|---|
    /// | win rate | `min_win_rate_bp: 0` | **rule OFF** — any win rate admitted |
    /// | drawdown | `min_ret_over_dd_bp: 0` | **rule OFF** — any drawdown admitted |
    /// | winning ratio | `min_rr_bp: 200` | 2.0x, and the operator states 1.25 |
    /// | stop | `max_mae_ppm: 2_000` | 0.20%, typed once and never derived |
    /// | top | `top: 25` | the one that matched |
    ///
    /// So two of the four things being asked for were not being asked at all,
    /// and a third asked for something else. No amount of sweeping recovers a
    /// criterion that was never applied.
    ///
    /// # Nothing here is baked
    ///
    /// Every field is read at RUNTIME from the environment, so an operator moves
    /// any of them without a rebuild and every rung re-derives around the new
    /// value. The defaults are the operator's stated rule rather than a guess:
    /// 50% and 1.25 are their words, `top` stays 25 because that is what they
    /// asked to see, and `min_assurance_bp` is DERIVED from whatever win rate
    /// ends up in force — never typed, because a rate and a bound on that rate
    /// are different quantities and setting one without the other is how a
    /// profile comes to demand 92.5% while its table says 80%.
    ///
    /// `max_mae_ppm` defaults to zero, which DROPS the rule rather than
    /// inventing a stop: the exit grid already sweeps a derived stop ladder, and
    /// a ceiling nobody typed would silently discard variants the ladder was
    /// built to try. An operator who wants a hard stop names one.
    ///
    /// | variable | field | default |
    /// |---|---|---|
    /// | `BRUTEX_MIN_WIN_RATE_BP` | `min_win_rate_bp` | `5_000` — 50% |
    /// | `BRUTEX_MIN_RR_BP` | `min_rr_bp` | `125` — smallest win ≥ 1.25x largest loss |
    /// | `BRUTEX_MIN_RET_OVER_DD_BP` | `min_ret_over_dd_bp` | `500` — made ≥ 5x the worst fall |
    /// | `BRUTEX_MAX_MAE_PPM` | `max_mae_ppm` | `0` — no ceiling beyond the swept ladder |
    /// | `BRUTEX_MIN_TRADES` | `min_trades` | `0` — the assurance bound carries it |
    /// | `BRUTEX_TOP` | `top` | `25` |
    #[must_use]
    pub fn operator() -> Self {
        /// One runtime override, or the stated default. Negative and malformed
        /// values fall back rather than refusing: this is read on every rung of
        /// every run, and halting a sweep over a typo in an optional variable
        /// would be a worse failure than using the documented figure.
        fn at(name: &str, default: i64) -> i64 {
            crate::knobs::var(name)
                .and_then(|raw| raw.trim().parse::<i64>().ok())
                .filter(|&v| v >= 0)
                .unwrap_or(default)
        }

        let min_win_rate_bp = at("BRUTEX_MIN_WIN_RATE_BP", 5_000);
        Self {
            max_mae_ppm: at("BRUTEX_MAX_MAE_PPM", 0),
            min_rr_bp: at("BRUTEX_MIN_RR_BP", 125),
            min_win_rate_bp,
            // DERIVED, NEVER TYPED. See `assurance_floor_bp`: at or below chance
            // it returns the rate itself, which is an unsatisfiable pair — so a
            // 50% rule carries no bound and `min_trades` below is what keeps a
            // two-trade fluke out. That is the honest arrangement for a rate
            // that IS the null hypothesis, and it is why the SEARCH is sized by
            // `sizing_rate_bp` instead of by this.
            min_assurance_bp: assurance_floor_bp(min_win_rate_bp),
            min_weakest_bp: at("BRUTEX_MIN_WEAKEST_BP", 0),
            min_trades: u64::try_from(at("BRUTEX_MIN_TRADES", 0)).unwrap_or(0),
            min_ret_over_dd_bp: at("BRUTEX_MIN_RET_OVER_DD_BP", 500),
            top: usize::try_from(at("BRUTEX_TOP", 25)).unwrap_or(25),
        }
    }

    const BASELINE: Self = Self {
        max_mae_ppm: 2_000,
        min_rr_bp: 200,
        // ZERO, not a number. The baseline exists so a command that takes no
        // policy still prints a screen, and it states its three numbers on the
        // page. A win-rate floor nobody typed would be a fourth number an
        // operator never chose, silently disqualifying rows.
        min_win_rate_bp: 0,
        min_assurance_bp: 0,
        min_weakest_bp: 0,
        min_trades: 0,
        min_ret_over_dd_bp: 0,
        top: 25,
    };

    /// Every rule on, at the operator's stated requirement.
    ///
    /// # Why this is a PROFILE and not a new default
    ///
    /// [`Self::BASELINE`] switches four of its six rules off, and its reasoning
    /// is right: *a rule an operator did not type is a policy the engine
    /// invented*. Turning them on globally would be exactly that. So the
    /// requirement is written down once, given a name, and **selected** — the
    /// operator types `elite` and gets these numbers, or types their own and
    /// gets those. Nothing is invented and nothing is silently applied.
    ///
    /// # Where each number comes from
    ///
    /// The operator's requirement, in their own words, is *"if this combination
    /// occurs then minimum 80 percent of trades won, every trade's loss is very
    /// minimal, the winning side is massive, and even drawdown is less"*. That
    /// is five separate claims, and this is each one as a number:
    ///
    /// | requirement | field | value | reads |
    /// |---|---|---|---|
    /// | 80% of trades win | `min_win_rate_bp` | `8_000` | 80.00% |
    /// | ...and not by luck | `min_assurance_bp` | `8_000` | 80% even on the 95% lower bound |
    /// | minimal loss per trade | `max_mae_ppm` | caller's | the stop, in ppm |
    /// | massive winning side | `min_rr_bp` | `300` | **smallest** win ≥ 3× **largest** loss |
    /// | drawdown is less | `min_ret_over_dd_bp` | `500` | made ≥ 5× the worst fall |
    /// | every period the same | `min_weakest_bp` | `5_000` | the weakest grain still ≥ 50% positive |
    ///
    /// **`min_rr_bp` is harsher than it looks, and deliberately so.**
    /// [`grid::Cell::reward_to_risk_bp`] is `min_win / -worst_trade` — the
    /// SMALLEST winner over the LARGEST loser, not a ratio of averages. A single
    /// bad trade sets the denominator, so `300` demands that the *weakest* win
    /// still tripled the *worst* loss. Averages would let one catastrophic trade
    /// hide behind many small wins, which is the shape this profile exists to
    /// reject.
    ///
    /// **`min_assurance_bp` is what lets `min_trades` stay at zero**, and that
    /// is the point of the profile rather than an oversight. The operator wants
    /// *"not thousands of trades — the one and only"*: a rare setup that fires
    /// forty times in ten years and does not lose. A `min_trades` floor rejects
    /// that outright; the Wilson lower bound weighs it instead — 40 of 40 clears
    /// 80%, 12 of 12 does not, and neither number had to be guessed.
    ///
    /// **`min_weakest_bp` is 5,000 and not 10,000** deliberately. Demanding
    /// every single period at every one of six grains close positive is a rule
    /// almost nothing survives, and a screen that admits nothing teaches an
    /// operator nothing about why. Half is a real bar that still refuses a
    /// combination carried by one good year. An operator who wants the stricter
    /// reading sets it themselves.
    ///
    /// # This is a FILTER, not a promise
    ///
    /// Every row it admits still carries the whole-workspace caveats: figures
    /// are per ONE unit of the index, gross of the statutory charge stack, and
    /// selected out of a search whose multiplicity the screen does not correct
    /// for. A row passing `elite` is a candidate to investigate, not a result.
    #[must_use]
    pub const fn elite(max_mae_ppm: i64, top: usize) -> Self {
        Self {
            max_mae_ppm,
            min_rr_bp: 300,
            min_win_rate_bp: 8_000,
            // DERIVED FROM THE STATED RATE, and 8,000 here silently demanded
            // something else entirely.
            //
            // `min_win_rate_bp` is the OBSERVED rate: 80% of trades won.
            // `min_assurance_bp` is the 95% LOWER BOUND on the true rate, which
            // on a small sample sits far below the observed one. Setting both to
            // 8,000 does not ask for 80% twice — it asks for a rate high enough
            // that even the pessimistic reading is 80%.
            //
            // Measured on the shipped Wilson formula at n = 40:
            //
            // | record | observed | 95% lower bound | admitted at 8,000? |
            // |---|---|---|---|
            // | 32/40 | 80.0% | 65.24% | **NO** |
            // | 34/40 | 85.0% | 70.92% | NO |
            // | 36/40 | 90.0% | 76.94% | NO |
            // | 37/40 | 92.5% | 80.13% | yes |
            //
            // So an operator asking for 80% over forty trades was refused unless
            // they actually got **92.5%**. The stated rule and the enforced rule
            // were different rules, and nothing said so.
            //
            // [`assurance_floor_bp`] derives this instead: the bound must clear
            // a COIN FLIP, which is the question the bound exists to answer —
            // *is this better than chance* — while `min_win_rate_bp` carries the
            // operator's actual standard. Two rules, two jobs, neither
            // impersonating the other.
            min_assurance_bp: assurance_floor_bp(8_000),
            min_weakest_bp: 5_000,
            // ZERO ON PURPOSE. See the doc above: the assurance bound already
            // refuses a sample too thin to mean anything, and a floor here would
            // refuse the rare high-conviction setup this profile exists to find.
            min_trades: 0,
            min_ret_over_dd_bp: 500,
            top,
        }
    }

    /// The same rules with a different stated win rate, and its bound rederived.
    ///
    /// **Both fields move together or the pair is incoherent**, which is the
    /// whole reason this exists rather than a caller writing
    /// `Rules { min_win_rate_bp: x, ..elite }`. `min_win_rate_bp` is the
    /// OBSERVED rate and `min_assurance_bp` is the 95% lower bound on the true
    /// rate; setting one without the other is how a profile comes to demand
    /// 92.5% while its table says 80%, which this constructor's own doc records
    /// having happened.
    ///
    /// Used by [`statistical_support_floor`] to size a search at a rate that
    /// can carry evidence, while rows are still admitted by the operator's own
    /// rule — see [`sizing_rate_bp`] for why those cannot be one number.
    #[must_use]
    pub const fn with_win_rate(self, min_win_rate_bp: i64) -> Self {
        Self {
            min_win_rate_bp,
            min_assurance_bp: assurance_floor_bp(min_win_rate_bp),
            ..self
        }
    }
}

/// Win-rate rungs, in basis points, strictest first.
///
/// # Generated, not listed
///
/// From 95% down to 40% in steps that widen as the rate falls: the difference
/// between 95% and 93% is a different strategy, and the difference between 45%
/// and 43% is noise. Two-point steps at the top and five at the bottom is that
/// asymmetry, and it is derived from the sequence rather than typed as a list.
fn win_rate_rungs(trades: u64) -> Vec<i64> {
    // THE RESOLUTION A SAMPLE CAN CARRY, NOT A STEP SOMEBODY TYPED.
    //
    // This was three typed steps -- 200, 250 and 500 basis points -- chosen on
    // the reasoning that fine distinctions matter at the top and not at the
    // bottom. That reasoning is right and the numbers were still invented.
    //
    // A win rate is `wins / trades`, so the FINEST distinction a sample can
    // express is one trade: at 1,000 trades that is 10 basis points, and at 100
    // it is 100. A rung finer than that separates two rates the data cannot
    // tell apart; a rung coarser throws away a distinction it can. So the step
    // is the sample's own resolution.
    //
    // Multiplied by ten so the ladder is a manageable number of rungs rather
    // than one per possible outcome -- at 1,000 trades that is a rung every 1%,
    // which is 55 rungs over the range and the tier ladder's own product keeps
    // it affordable.
    let per_trade_bp = if trades == 0 {
        100
    } else {
        (10_000 / i64::try_from(trades).unwrap_or(i64::MAX)).max(1)
    };
    let step = per_trade_bp.saturating_mul(10).clamp(25, 1_000);

    // THE RANGE IS THE WHOLE RANGE. It stopped at 40% because nothing below
    // that was thought interesting, which is a judgement about the answer made
    // before the search. A combination winning 20% of the time at 1:8 is a real
    // strategy and the ladder now reaches it.
    let mut out: Vec<i64> = Vec::with_capacity(64);
    let mut bp = 9_900_i64;
    while bp > 0 {
        out.push(bp);
        bp -= step;
    }
    out
}

/// The stop levels the ladder judges against, in index points.
///
/// **The grid's own ladder, read back.** A tier that asked for a stop the grid
/// never tested would be a rule with no cell to satisfy it, so this is derived
/// from [`grid_rungs`] and [`GRID_STEP_PPM`] rather than stated: rung `i` is
/// `i * step`, the same values [`crate::grid::Levels`] hands the engine.
///
/// Rounded UP to whole points because a tier is stated in points and a rule at
/// "2.5 points" reads as a tolerance nobody set. Rounding up is the strict
/// direction: a tier asking for 2 points is satisfied only by a grid rung at or
/// under it.
fn stop_rungs_in_points(bars: &[indicators::Candle]) -> Vec<i64> {
    let reference = reference_price(bars);
    let per_point = points_to_ppm_at(1, reference).max(1);
    let step = grid_step_ppm(bars);
    (1..=grid_rungs(bars))
        .filter_map(|i| i64::try_from(i).ok())
        .map(|i| {
            let ppm = step.saturating_mul(i);
            // Ceiling division into whole points.
            ppm.saturating_add(per_point - 1) / per_point
        })
        .filter(|&pt| pt > 0 && pt <= max_stop_points(bars))
        .collect()
}

/// One rung of the tier ladder: a name, and the policy it stands for.
///
/// # Why a ladder and not one rule
///
/// An operator states the standard they WANT — "a loser never runs more than
/// ten points, a winner makes at least thirty, and ninety of a hundred win".
/// Almost nothing satisfies that, and a screen that answers `0 of 21` has told
/// them the standard is not met without telling them what IS.
///
/// The useful answer is the strictest tier that yields anything, and how far
/// down the ladder it sat. That turns "nothing passed" into "nothing passed
/// S+++ or S++; at S+ there are four, and here they are" — which is a finding
/// rather than an empty table.
///
/// # These numbers are stated, not derived
///
/// Every threshold below is a TRADING POLICY, and §3 rule 1 does not let the
/// engine invent one from the data. They descend in the three dimensions an
/// operator actually names — how far a loser may run, how far a winner must go,
/// and how often it must win — and the report prints the full ladder beside the
/// result so a reader sees exactly which standard was met and which were not.
#[derive(Clone, Copy, Debug)]
pub struct Tier {
    /// What to call it on the page.
    ///
    /// Empty on a generated tier: a name cannot be computed from the
    /// thresholds without inventing a scale, and it is the POSITION in the
    /// sorted ladder that carries meaning. `Tier::label` derives it from the
    /// rank once the sort has happened.
    pub name: &'static str,
    /// The furthest a single trade may run against entry, in index points.
    pub max_points: i64,
    /// The smallest win that counts, in index points, expressed against
    /// `max_points` as a reward-to-risk in hundredths.
    pub min_rr_bp: i64,
    /// The share of trades that must win, in basis points.
    pub min_win_rate_bp: i64,
    /// The fewest round trips for the rate above to mean anything.
    pub min_trades: u64,
}

/// The ladder, strictest first.
///
/// Read the first row as the operator's own words: **max loss 10 points,
/// minimum win 30 points — a 1:3 — and 90 of 100 winning, over at least 1,000
/// trades.**
///
/// # The trade floor, and why it never drops below 500
///
/// Seven years is about 1,700 trading days. A thousand trades is one every
/// 1.7 days and five hundred is one every 3.4 -- both genuinely SELECTIVE
/// against the 16,745 the first banked 15-minute run took, which is ten a day.
/// Below five hundred a win rate over eighty-one months is a claim about a
/// handful of weeks, and the mildest tier is the one most likely to be read
/// as a result, so it carries the floor rather than being exempted from it. Each row after it relaxes exactly one dimension at a time, so a
/// reader can see WHICH requirement the market would not meet rather than only
/// that some of them were not met together.
///
/// The last row is deliberately mild: a combination that cannot clear even that
/// is not a near miss, and saying so is more useful than printing the least-bad
/// row of a table nothing passed.
///
/// # It is GENERATED, and it used to be fourteen typed rows
///
/// A hand-written ladder is a hand-written answer: it can only ever ask the
/// questions whoever typed it thought of, and every threshold in it is a number
/// somebody chose. Fourteen rows also cannot express "one point tighter" without
/// a fifteenth.
///
/// So it is the cross product of the three axes the engine already has —
/// [`stop_rungs_in_points`], which is the grid's own stepped ladder read back;
/// [`grid_ratios`], which is what the grid pairs each stop with; and
/// [`win_rate_rungs`]. Nothing here can ask for a stop the grid never tested or
/// a ratio it never paired, because the ladder is built from the same values.
///
/// Deepening the grid deepens the ladder with it: eight rungs give 8 x 10 x 12
/// = 960 tiers and forty give 4,800, all derived.
///
/// # Ordered by a strictness score, and why it is a product
///
/// "Stricter" is three numbers moving in different directions, so a total order
/// needs a single figure. A tier is stricter when the stop is tighter, the ratio
/// higher and the win rate higher, and multiplying them ranks a tier that gives
/// up a little on each below one that gives up a lot on one — which is the
/// ordering an operator walking down actually wants.
///
/// # The trade floor rides down with it
///
/// A strict tier over 200 trades is a claim about a handful of weeks. The floor
/// is 1,000 at the top and eases to 500 at the bottom, on the same reasoning
/// that put it there: seven years is about 1,700 trading days.
fn tiers(bars: &[indicators::Candle], trades: u64) -> Vec<Tier> {
    let stops = stop_rungs_in_points(bars);
    // The win-rate resolution follows the SAMPLE, so a ladder judging a
    // thousand-trade combination is finer than one judging fifty.
    let rates = win_rate_rungs(trades);
    let ratios = grid_ratios();
    let mut out: Vec<Tier> = Vec::with_capacity(stops.len() * ratios.len() * rates.len());
    for &max_points in &stops {
        for &min_rr_bp in &ratios {
            for &min_win_rate_bp in &rates {
                out.push(Tier {
                    // NAMED BY RANK, ASSIGNED BELOW. A name cannot be computed
                    // from the thresholds without inventing a scale; it is the
                    // POSITION in the sorted ladder that means something, and
                    // that is not known until the sort has happened.
                    name: "",
                    max_points,
                    min_rr_bp,
                    min_win_rate_bp,
                    // 1,000 while the win rate is demanding, easing to 500 once
                    // it is not: a rate of 40% over 500 trades is a real
                    // measurement, and 95% over 200 is not.
                    min_trades: if min_win_rate_bp >= 8_000 { 1_000 } else { 500 },
                });
            }
        }
    }
    // Strictest first. `Reverse` on the score, then the fields themselves so the
    // order is TOTAL and identical across processes -- §3 rule 5 applies to a
    // report's row order as much as to a total.
    out.sort_by_key(|t| {
        let score = i128::from(t.min_rr_bp).saturating_mul(i128::from(t.min_win_rate_bp))
            / i128::from(t.max_points.max(1));
        (
            core::cmp::Reverse(score),
            t.max_points,
            core::cmp::Reverse(t.min_rr_bp),
            core::cmp::Reverse(t.min_win_rate_bp),
        )
    });
    out
}

impl Tier {
    /// This tier as the rules the screen applies.
    #[must_use]
    pub fn rules(&self, top: usize) -> Rules {
        crate::Rules {
            max_mae_ppm: points_to_ppm(self.max_points),
            min_rr_bp: self.min_rr_bp,
            min_win_rate_bp: self.min_win_rate_bp,
            // THE TIER'S WIN RATE, DEMANDED PESSIMISTICALLY.
            //
            // A tier says "ninety-five of a hundred win". Applied to the raw
            // rate that is satisfied by 20 of 20, which is a rate of 100% and a
            // 95% lower bound of 83.88% -- the claim is not supported and the
            // tier admitted it anyway. Applied to the bound, the same tier needs
            // roughly 120 perfect trades, or 19 of 19 will not do.
            //
            // This SUBSUMES `min_win_rate_bp` above, because the bound is never
            // greater than the observed rate -- a test pins that. The raw rule is
            // kept rather than deleted because it states the tier's intent in
            // the terms an operator wrote it in, and because a future tier may
            // legitimately want the two to differ.
            min_assurance_bp: self.min_win_rate_bp,
            // THE SAME CLAIM, ACROSS THE CALENDAR.
            //
            // A tier saying "ninety-five of a hundred win" should also mean
            // ninety-five of a hundred PERIODS closed positive, at every
            // grain. The two are near enough the same statement at the finest
            // grain -- a day holding one losing trade is a negative day -- so
            // this is not a second, harsher policy smuggled in beside the
            // first. It is the first one, asked of the calendar instead of the
            // trade list, which is where an operator actually feels it.
            //
            // The coarser grains are where it bites: a combination can win 95%
            // of its trades and still have a NEGATIVE quarter, because losses
            // clustered. That combination fails here and passed everywhere
            // else.
            min_weakest_bp: self.min_win_rate_bp,
            min_trades: self.min_trades,
            // ZERO, AND THE TIER LADDER IS THE REASON.
            //
            // A tier is generated from four axes -- max points, reward-to-risk,
            // win rate, trade floor -- and the ladder is already 960 tiers at
            // eight grid rungs. A fifth axis multiplies that by however many
            // drawdown rungs it carries, for a cascade whose job is to show an
            // operator WHERE their requirement stops being satisfiable, not to
            // enumerate every policy.
            //
            // Picking one value here instead would be worse: it would be a
            // drawdown policy attached to every tier that nobody typed, which is
            // the defect `BASELINE`'s own comment names. `cli elite` is where
            // the rule is turned on, by an operator who asked for it.
            min_ret_over_dd_bp: 0,
            top,
        }
    }

    /// The tier's name at position `rank` in the sorted ladder.
    ///
    /// The strictest few are the S grades an operator names -- `S++++++` at the
    /// top, one plus fewer each step -- and everything past them is `S`, `A`,
    /// `B`, `C` and then a bare rank. Derived rather than stored because the
    /// ladder is generated: at eight grid rungs it holds 960 tiers and at forty
    /// it holds 4,800, and no list of names could keep up.
    #[must_use]
    pub fn label(rank: usize) -> String {
        match rank {
            0..=5 => format!("S{}", "+".repeat(6 - rank)),
            6..=9 => "S".to_owned(),
            10..=19 => "A".to_owned(),
            20..=39 => "B".to_owned(),
            40..=79 => "C".to_owned(),
            _ => format!("#{rank}"),
        }
    }

    /// The tier's policy in the operator's own units, for the page.
    ///
    /// The minimum win is DERIVED rather than stored: it is the reward-to-risk
    /// applied to the loss cap, so the two can never disagree on the page the
    /// way two stored numbers could.
    #[must_use]
    pub fn describe(&self) -> String {
        let min_win = self.max_points.saturating_mul(self.min_rr_bp) / 100;
        format!(
            // INTEGER DIVISION AND A REMAINDER, not a float. §7 keeps floats out
            // of anything compared, and a ratio printed beside a rule an
            // operator reads IS compared -- by them.
            "loser <= {}pt, winner >= {}pt (1:{}.{:02}), win rate >= {}%, >= {} trades",
            self.max_points,
            min_win,
            self.min_rr_bp / 100,
            self.min_rr_bp % 100,
            self.min_win_rate_bp / 100,
            self.min_trades,
        )
    }
}

/// How many combinations the screener prices in full.
///
/// # This was 60, and it is the reason nothing was ever found
///
/// The pipeline is: Apriori produces the combinations, they are ranked by
/// `|t|`, and the first `SCREEN_CAP` of that ordering get an exit grid. On a
/// real 15-minute run over 81 months that read: **1,024,058 combinations found,
/// 250 kept, 21 priced, 13,125 grid cells evaluated.**
///
/// The ranking that makes the cut is `outcome::edge` — a LEVEL-LESS forward
/// return, with no stop, no target and no trail. So the question it answers is
/// *"how far does this signal run unstopped"*, and the question an operator
/// asks is *"which signal keeps every loser inside ten points and every winner
/// past thirty"*. A combination that is unremarkable unstopped and excellent
/// under a tight stop scores low on the first question, is cut at 60, and never
/// meets an exit grid at all.
///
/// No tier ladder, no rule and no report can recover that. They all filter
/// cells, and the cells were never computed.
///
/// # Why a cap remains, and what governs it now
///
/// Removing it outright is not free: each combination costs a full grid, which
/// is one trade walk plus `variants` cells. Measured with the engine's own
/// counter at four rungs that is 625 cells, and the whole 1,024,058 would be
/// 640,036,250 — roughly a quarter hour per rung across fourteen cores, which
/// is affordable, and 27,637,321 cells per combination at forty rungs, which is
/// not.
///
/// So the bound is stated in CELLS rather than in combinations, and the count
/// follows from the grid's own width. `BRUTEX_SCREEN_CAP` overrides it for an
/// operator who wants the full search and has the hours; the default is sized
/// so an audit still returns in seconds on a laptop.
///
/// UNVERIFIED as a measured figure: no bench row covers the screen's own cost.
fn screen_cap() -> usize {
    // `10_000` at 625 cells is 6.25 million per audit -- two orders of magnitude
    // past the 60 it replaces, and still seconds rather than hours. It is a
    // starting point an operator raises, not a ceiling anybody derived.
    const DEFAULT: usize = 10_000;
    crate::knobs::var("BRUTEX_SCREEN_CAP")
        // `knobs::var` already answers `Option`, so there is no `Result` to unwrap.
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT)
}

/// Whether an operator-facing command pays for the full validation stack.
///
/// # The default is ON, and it stays on
///
/// A search result nobody validated is a candidate, not a finding, and §4 refuses
/// a fallback that hides a failure. So this defaults to `true` and an operator
/// has to ask for less, in writing, with the answer carrying [`UNVALIDATED`] on
/// its face.
///
/// # Why asking for less had to become possible
///
/// `cli screen` hardcoded `true` under the reasoning that it *"is a single
/// answer, not a walk, so it pays for the full stack once"*. Sound, and MEASURED
/// false: on NIFTY 60min over twelve months the SWEEP takes **14 seconds** and
/// `screen` returned **nothing at all after fifty minutes**, twice, because the
/// stack — 960 generated tiers plus walk-forward, PBO and the bootstrap — runs
/// BEFORE anything is printed. A command that cannot finish gives the operator
/// no answer at all, which is strictly worse than an answer marked provisional.
///
/// `elite` already draws this line: every step of its descent runs with
/// `validate: false` and only the rung that lands is re-run with the stack. This
/// gives the same choice to the command an operator reaches for first.
fn validate_from_env() -> bool {
    validates(crate::knobs::var("BRUTEX_VALIDATE").as_deref())
}

/// The rule itself, over the raw value, so it can be tested without touching the
/// environment.
///
/// `crates/cli` is `#![forbid(unsafe_code)]` and `std::env::set_var` is unsafe,
/// so a test cannot set the variable to check the reader. Splitting the DECISION
/// from the LOOKUP is what makes the rule provable at all — and the reader above
/// is then one line with nothing left to get wrong.
///
/// Only a literal `0` turns validation off. A malformed or unexpected value
/// leaves it ON: a typo must never silently buy a weaker answer, which is the
/// same argument §4 makes against a fallback that hides a failure.
fn validates(raw: Option<&str>) -> bool {
    // NOT a `const fn`: matching a `&str` in one needs `PartialEq` as a const
    // trait, which is not stable. `trim` is the honest rule anyway -- an
    // operator who exports the variable with a trailing space meant `0`.
    raw.is_none_or(|value| value.trim() != "0")
}

/// What a report says when the validation stack did not run.
///
/// Shouted, and on its own line, for the reason `CLAUDE.md` §5 gives for the two
/// provenance banners: a screen over unvalidated candidates is byte-identical in
/// shape to one over validated findings, so the banner is the only thing
/// separating them.
const UNVALIDATED: &str = "!! NOT VALIDATED -- walk-forward, PBO and the bootstrap did NOT run.\n\
   These are CANDIDATES, not findings. Unset BRUTEX_VALIDATE to price them in full.";

/// Everything outside the `Ladder` that changes what this run records.
///
/// # Why it exists
///
/// `Params::of(ladder)` covers what the SWEEP did — the threshold and the two
/// budgets — and nothing about what was done with its output. Four choices decide
/// that, and none of them reaches a `Ladder`:
///
/// | choice | what it moves |
/// |---|---|
/// | `MAX_POINTS`, as `rules.max_mae_ppm` | which variants the operator's stop admits |
/// | the ranking [`runner::rank::Lens`] | which combination is TRADED |
/// | `BRUTEX_GRID_RUNGS`, via [`grid_rungs`] | how wide the exit grid is |
/// | `BRUTEX_SCREEN_CAP`, via [`screen_cap`] | how many combinations are priced at all |
///
/// Change any one and the recorded `Record` changes. Before this the `RunId` did
/// not, so two runs with different answers collided — and the ledger, which
/// refuses a duplicate identity, handed back whichever landed first. That is the
/// same defect `Params::pair_budget`'s own doc records having been, one layer out.
///
/// `validate` is folded in too: an unvalidated screen and a validated one over
/// the same bars are different computations, and the report says so on its face.
///
/// # The ORDER is the contract
///
/// `with_policy` hashes the slice in order and folds its length first, so adding
/// a knob re-keys every run — which is honest: those runs were computed by a
/// build that could not turn it. Append, never insert.
fn policy_of(
    bars: &[indicators::Candle],
    rules: Rules,
    lens: runner::rank::Lens,
    validate: bool,
) -> [u64; 14] {
    [
        // Negative is not expected and is not silently folded to zero: the cast
        // is saturating so a negative rule still differs from an absent one.
        u64::try_from(rules.max_mae_ppm).unwrap_or(u64::MAX),
        // THE OTHER SIX RULES, AND WHY THEY WERE NOT HERE UNTIL NOW.
        //
        // `max_mae_ppm` alone was SUFFICIENT while this path built its rules
        // from `Rules::BASELINE`, a `const`: nothing else could vary, so nothing
        // else could split two runs. `Rules::operator()` reads seven
        // environment variables, and every one of them changes which cell
        // `best_within(|c| rules.admits(c))` selects -- and therefore the
        // recorded `pessimistic`, `trades` and exit rungs.
        //
        // Making six fields runtime-variable without extending this array
        // reopened exactly the hole D-0294 closed, in the same commit that
        // widened them. They are APPENDED BELOW, after the ceiling, because
        // "append, never insert" is about POSITIONS and not about reading
        // order: putting them here first would shift the lens, the rung count,
        // the screen cap, `validate` and the ceiling down five places and
        // silently re-key every identity ever recorded. The first attempt at
        // this commit did exactly that, and
        // `every_knob_that_moves_the_answer_moves_the_identity` caught it by
        // asserting the ceiling is still the term the ladder uses.
        match lens {
            runner::rank::Lens::Detectability => 0,
            runner::rank::Lens::Payoff => 1,
        },
        grid_rungs(bars) as u64,
        screen_cap() as u64,
        u64::from(validate),
        // THE SIXTH, AND IT MOVES MORE THAN THE OTHER FIVE.
        //
        // `BRUTEX_CEILING` decides how far the ladder is allowed to WALK before
        // it halts. D-0294 folded four knobs in on the principle that "an
        // identity two different results can share is not an identity", and
        // missed this one -- which is the knob that decides whether a run
        // explored the combination space at all.
        //
        // Two runs at different ceilings are not two views of one answer: one
        // is exhaustive and one stopped early with `complete = NO` and a depth
        // that is "as far as the ladder got, not as far as it goes". Under one
        // identity the results ledger refuses the second as a duplicate, so the
        // halted run is dropped for the exhaustive one or the exhaustive one is
        // answered with the halted one's row -- and both readings are wrong.
        //
        // Read through the same `ceiling_from_env` the ladder is built with, so
        // a malformed value cannot key one thing and sweep another. Its refusal
        // is not raised here: this function has no error channel and the ladder
        // construction that does refuses first, before any bar is read.
        // `u64::MAX` marks that unreadable case distinctly rather than folding
        // it to the default's key. D-0305.
        ceiling_from_env().map_or(u64::MAX, |ceiling| ceiling as u64),
        // ---- APPENDED BELOW THIS LINE. Nothing above it may move. ----
        //
        // THE SEVENTH THROUGH THIRTEENTH: the rest of `Rules`. Each changes
        // which cell `best_within(|c| rules.admits(c))` selects, and therefore
        // the recorded `pessimistic`, `trades` and exit rungs. `top` also
        // decides how many frontier rows the run writes, so two runs at
        // different `top` produce different stored output from identical bars.
        u64::try_from(rules.min_rr_bp).unwrap_or(u64::MAX),
        u64::try_from(rules.min_win_rate_bp).unwrap_or(u64::MAX),
        u64::try_from(rules.min_assurance_bp).unwrap_or(u64::MAX),
        u64::try_from(rules.min_weakest_bp).unwrap_or(u64::MAX),
        u64::try_from(rules.min_ret_over_dd_bp).unwrap_or(u64::MAX),
        rules.min_trades,
        rules.top as u64,
        // THE FOURTEENTH: the grid resolution, which `grid_rungs` does not
        // fully carry.
        //
        // `BRUTEX_GRID_RESOLUTION` moves `grid_step_ppm`, which sets the target
        // and trail ladder SPACING. The `grid_rungs` term above captures it only
        // through `cap / step_points` -- and once `ppm_to_points_at(step, ref)`
        // rounds to zero, `.max(1)` pins `step_points` at one and `grid_rungs`
        // at `cap` for EVERY finer resolution, while `step_ppm` keeps shrinking
        // and the grid keeps changing. Two runs at 40 and at 200 can share a
        // rung count, hold different grids, produce different answers and carry
        // the same `RunId`. The step itself is folded so they cannot.
        u64::try_from(grid_step_ppm(bars)).unwrap_or(u64::MAX),
    ]
}

/// One screened combination, priced in full and judged.
struct Screened<'a> {
    /// The combination this row measured, kept so the chosen variant can be
    /// re-walked AFTER the sort rather than during the screen.
    ///
    /// # The regression this borrow exists to undo
    ///
    /// Consistency was first measured inside the screening loop, which runs
    /// over `screen_cap()` combinations -- ten thousand by default. Every one
    /// of them paid for a full trade-by-trade re-walk, and only `rules.top`
    /// of them are ever PRINTED. A single-month screen that had taken seconds
    /// stopped finishing inside 280.
    ///
    /// Measuring after the sort costs `rules.top` re-walks instead of ten
    /// thousand -- a four-hundred-fold difference for identical output.
    scored: &'a runner::rank::Scored,
    /// Its rank in the evidence ordering, so a reader can see what the screen
    /// moved.
    rank: usize,
    /// The chosen exit variant's full statistics.
    cell: grid::Cell,
    /// The tightest containment ANY variant of this combination achieved, and
    /// what it cost. A rule tighter than this cannot be met by any exit, so it
    /// tells an operator whether their number is reachable before guessing again.
    tightest: Option<grid::Cell>,
    /// The conditions, by name.
    names: String,
    /// Whether every rule held.
    admitted: bool,
    /// How the chosen variant held up across every calendar grain.
    ///
    /// `None` when the variant could not be re-walked. Distinguished from a
    /// measured zero on purpose: "not measured" and "positive in none of its
    /// periods" are opposite findings and §4 does not let them look alike.
    consistency: Option<Consistency>,
    /// Whether the consistency rule held, once it could be measured.
    ///
    /// Separate from `admitted` because the two are decided at different
    /// times: `admitted` comes from the five CELL rules during selection, and
    /// this from the calendar afterwards. Kept so `why_refused` can name which
    /// of the two refused the row.
    steady: bool,
}

impl Screened<'_> {
    /// Whether this row's chosen variant met the consistency rule.
    ///
    /// A row whose consistency could NOT be measured counts as steady, and that
    /// is deliberate: an unmeasured row must not be refused for failing a rule
    /// nothing checked. The consistency table says "not measured" for it in so
    /// many words, so the gap is visible rather than dressed up as a verdict.
    fn steady(&self) -> bool {
        self.steady
    }
}

/// The top combinations, priced in full and ranked with the PASSING ones first.
///
/// # Ordering is the whole point
///
/// The evidence ordering answers *which condition precedes a move most
/// reliably*. It does not answer *which one can I actually trade*, and those are
/// different questions: a real run's most-evident combination scored 81%
/// profitable with a reward-to-risk of 0.41 and a worst trade that ran 1.06%
/// against — evident, and untradeable under any sane stop.
///
/// So the screen re-orders: every combination that satisfies ALL the operator's
/// rules first, best net first among them; everything that broke a rule after,
/// with the rule it broke named. A failing combination is never hidden — a
/// listing that dropped them would leave a reader unable to tell "nothing
/// passed" from "nothing was tried".
/// Walk [`TIERS`] from strictest to mildest and report the first that yields
/// anything, naming every tier that did not.
///
/// # What this replaces
///
/// One screen against one policy answers `0 of 21 satisfy every rule` and
/// stops. That is true and nearly useless: it says the operator's standard was
/// not met without saying what standard WAS, so the next step is always to
/// guess a looser number by hand and run again.
///
/// # Why the grid is built once and the tiers only re-filter
///
/// Every tier reads the same [`grid::Cell`] values — `worst_mae`,
/// `reward_to_risk_bp`, `win_rate_bp`, `trades`. None of them changes what the
/// grid CONTAINS, so eight tiers cost eight passes over cells already computed
/// rather than eight sweeps. The one thing a tier does change is the forced
/// stop merged into the ladder, and that is taken from the STRICTEST tier so
/// the tightest level an operator might want is present in the grid every tier
/// then reads.
///
/// # It never invents a tier
///
/// The ladder is a stated policy, printed in full beside the answer, in index
/// points and whole percent. A reader sees which rung was met and which were
/// not — so "nothing passed" becomes "nothing passed S+++ or S++; at S+ there
/// are four, and here they are".
/// The 95% lower bound a stated win rate implies, in basis points.
///
/// # Why this is derived and not typed
///
/// A win rate and a lower bound on a win rate are different quantities, and
/// giving them the same number makes the second silently override the first. At
/// forty trades, demanding a bound of 80% refuses a genuine 80% record — it
/// admits nothing under 92.5%. An operator who typed 80 got 92.5 and was told
/// nothing.
///
/// The bound's job is to answer *is this better than chance*, so the floor is a
/// coin flip. The operator's standard is carried by `min_win_rate_bp`, which is
/// checked against the OBSERVED rate where it belongs.
///
/// # What is a convention here and what is not
///
/// **Half is not a policy about trading, it is the definition of chance** — the
/// null a two-sided bound is built to exclude. The 95% in the bound itself is a
/// convention too, and lives in `grid::Cell::assurance_bp` where it is written
/// out as `Z = 1.959964`.
///
/// What is NOT a convention, and therefore is not fixed here, is the stated rate:
/// that arrives as `min_win_rate_bp` from the caller, and this function reads it
/// so a caller demanding LESS than a coin flip is not silently raised to one.
#[must_use]
const fn assurance_floor_bp(min_win_rate_bp: i64) -> i64 {
    /// A coin flip, in basis points. Not a policy about trading — the
    /// definition of chance, and the null a two-sided bound exists to exclude.
    const CHANCE: i64 = 5_000;

    // A stated rate below chance is not a standard this bound can sharpen, so
    // the caller's own figure stands rather than being quietly raised.
    //
    // AT EXACTLY CHANCE THIS RETURNS AN UNSATISFIABLE PAIR, and the caller
    // cannot tell. `(5_000, 5_000)` asks the 95% lower bound on a rate to reach
    // the rate itself; the Wilson bound approaches from BELOW and never
    // arrives, so `grid::trades_needed_for` exhausts its search and returns
    // `TRADES_SEARCH_CEILING` — a number indistinguishable from an honest
    // answer of "five thousand round trips".
    //
    // That is the arithmetic being right about something uncomfortable: "at
    // least 50% of trades win" IS the null hypothesis of a coin flip and
    // carries no evidence on its own. It matters because the operator's stated
    // rule has a 50% leg, so anyone wiring that rule into `Rules::elite` will
    // land here and silently get the ceiling as their support floor. The
    // discriminating half of that rule is the reward-to-risk leg, not this one.
    // Pinned by
    // `the_support_floor_is_twenty_nine_trades_and_a_fifty_percent_rule_is_untestable`.
    if min_win_rate_bp <= CHANCE {
        return min_win_rate_bp;
    }
    // THE MIDPOINT BETWEEN CHANCE AND THE STATED RATE, and both ends were wrong.
    //
    // Measured on the shipped Wilson bound, for an operator asking 80% over
    // forty trades:
    //
    // | record | observed | bound | at 8,000 | at 5,000 | at 6,500 |
    // |---|---|---|---|---|---|
    // | 4/4 | 100% | 5,101 | no | **yes** | no |
    // | 12/12 | 100% | 7,575 | no | yes | yes |
    // | **32/40** | **80%** | **6,524** | **no** | yes | **yes** |
    // | 37/40 | 92.5% | 8,013 | yes | yes | yes |
    //
    // At 8,000 the operator's own case — thirty-two of forty — is REFUSED, and
    // nothing under 92.5% ever clears. At 5,000 a four-trade perfect record
    // passes, which is noise wearing a certificate.
    //
    // The midpoint admits the stated case and refuses the four-trade one. It is
    // derived from `min_win_rate_bp` rather than typed, so an operator who
    // raises or lowers their standard moves this with it and never has to know
    // the bound exists.
    CHANCE.saturating_add(min_win_rate_bp) / 2
}

/// The header a self-tuning descent leads with.
///
/// Lifted out of [`elite_descend`] to keep that function inside its line budget,
/// and because a banner is a rendering decision rather than part of the walk.
///
/// It states the FLOOR and where it came from, because a walk that stops has to
/// say what it stopped at: an operator reading rows needs to know the search
/// went as deep as the statistics allow, not as deep as somebody typed.
fn descent_banner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    span: ((u16, u8), (u16, u8)),
    bars: u64,
    floor: u64,
    steps: usize,
) -> String {
    let ((fy, fm), (ty, tm)) = span;
    let mut out = String::from(STORED_PROVENANCE);
    let trades = floor.saturating_mul(bars) / 1_000_000;
    let _ = writeln!(
        out,
        "\nELITE, SELF-TUNING\n  feed {vendor_word} · {underlying} · {rung} · \
         {fy}-{fm:02}..{ty}-{tm:02}\n  {bars} bars · floor {floor} ppm is about \
         {trades} round trip(s) — the fewest at which these rules can be \
         satisfied\n  by anything, so below it no combination passes however \
         good it is\n  {steps} step(s) from {DESCENT_CEILING_PPM} ppm down. NO \
         threshold was typed.\n  Each step asks only whether a row cleared your \
         rules; the survivor is then re-run\n  with walk-forward, PBO and the \
         bootstrap."
    );
    out
}

/// The winning support, re-run WITH the full validation stack.
///
/// # Why only the survivor pays for this
///
/// Every step of a descent runs with `validate: false`, which is what makes the
/// walk finishable: the stack costs `WALK_FORWARD_SPLITS` sweeps twice over plus
/// `BOOTSTRAP_DRAWS` x `BOOTSTRAP_CANDIDATES` — sixteen thousand full trade
/// re-walks — none of it sized by the data. MEASURED: a 60-minute audit over six
/// months did not finish in sixty seconds at a candidate ceiling of one
/// thousand; the same sweep without the stack takes 0.005s.
///
/// That is the SEARCH half. This is the other half, paid once, on the one
/// support that produced something — so the page an operator reads carries the
/// out-of-sample folds, the overfitting probability and the multiple-testing
/// p-values that decide whether the row is a finding rather than a candidate.
///
/// # A disagreement keeps the search page
///
/// If the validated re-run finds nothing where the search found something, the
/// SEARCH page is returned. The two readings differ only in which questions were
/// asked, so a silent swap would leave an operator reading one page believing it
/// was the other. Returning the search page keeps the rows visible; the missing
/// validation announces itself, because `audit::render` prints an explicit
/// absence for every unsupplied stage rather than a zero.
fn validated_at(
    vendor_word: &str,
    underlying: &str,
    rung: &'static str,
    // The span as one tuple: clippy is right that eight positional arguments is
    // a call site a reader cannot check.
    span: ((u16, u8), (u16, u8)),
    support: u64,
    policy: Policy,
    searched: &str,
) -> String {
    let (from, to) = span;
    let validated = screen_range(
        vendor_word,
        underlying,
        rung,
        from,
        to,
        support,
        Policy {
            validate: true,
            ..policy
        },
    );
    if validated.contains(YOUR_RULES_MET) {
        validated
    } else {
        searched.to_owned()
    }
}

/// The lowest support worth walking to, in ppm, DERIVED from the rules.
///
/// # The band that was never searched
///
/// A descent has to stop somewhere. It used to stop at the support one trade a
/// week implies — 561 ppm over 121 months of one-minute bars, about 526 round
/// trips. An operator hunting a setup that fires FORTY times is asking for
/// 42 ppm, so **the entire 42..561 ppm band was never searched**: the mask was
/// never built, never counted, never ranked.
///
/// That is not the Apriori prune discarding it. The prune is sound — adding a
/// required condition can only remove hits, so a set that clears `min_hits` has
/// every subset clearing it too. What discarded the setup was the THRESHOLD the
/// prune was applied under, and that threshold came from a cadence somebody
/// typed.
///
/// The win rate a SEARCH is sized by, which is not the rate a row is ADMITTED by.
///
/// # Why these are two different numbers
///
/// The operator's admission rule is *"at least 50% of trades win, and the
/// smallest win is at least 1.25x the largest loss"*. [`Rules::elite`] enforces
/// exactly that, and it is the right rule for judging a row.
///
/// It cannot size the search, and the reason is not a defect anywhere in this
/// code. [`statistical_support_floor`] asks *"below how many round trips can the
/// stated rate no longer clear its own confidence bound?"* — and "at least 50%
/// of trades win" **is the null hypothesis of a coin flip**. A 95% lower bound
/// on a 50% observation never reaches 50%: the Wilson bound approaches the
/// observed rate from below and never arrives, so no sample size satisfies it.
/// Measured, and pinned by
/// `the_support_floor_is_twenty_nine_trades_and_a_fifty_percent_rule_is_untestable`:
/// the pair `(5_000, 5_000)` exhausts `trades_needed_for` and returns its
/// ceiling, which the caller cannot tell from an honest answer.
///
/// So the search is sized by the rate at which evidence becomes *measurable*,
/// and rows are then admitted by the operator's own rule. Two questions, two
/// numbers, said out loud instead of one number quietly doing both jobs badly.
///
/// # It is derived, and it is overridable
///
/// The default is the midpoint between chance and certainty — 75%, the rate at
/// which one loss in four is the null being excluded. It is not a claim about
/// this operator's strategy; it is the point on the scale where a bound is
/// tight enough to prune and loose enough to be reachable, and it is stated
/// here rather than buried at a call site.
///
/// `BRUTEX_SIZING_RATE_BP` overrides it at runtime, in basis points, with no
/// rebuild — the operator who wants a deeper or shallower search moves this one
/// number and every rung re-derives its own floor from its own bars around it.
/// A value at or below chance is refused rather than obeyed, because it would
/// reproduce the exact unsatisfiable pair this function exists to avoid.
#[must_use]
pub fn sizing_rate_bp() -> i64 {
    /// Halfway between a coin flip and certainty. Every other point on the
    /// scale is equally arbitrary; this one is at least the midpoint, and being
    /// overridable is what keeps it from being a policy baked into a binary.
    const DEFAULT: i64 = 7_500;
    /// A coin flip. Below this there is no bound to clear.
    const CHANCE: i64 = 5_000;

    crate::knobs::var("BRUTEX_SIZING_RATE_BP")
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|&bp| bp > CHANCE && bp < 10_000)
        .unwrap_or(DEFAULT)
}

/// # What replaces it
///
/// The honest stopping point is the sample below which the stated rules cannot
/// be satisfied by anything. [`runner::grid::trades_needed_for`] answers that
/// from the rules themselves: below that many round trips the win rate can no
/// longer clear its own confidence bound, so **no combination passes however
/// good it is** — and descending further buys nothing while stopping above it
/// discards reachable answers.
///
/// Measured on the shipped Wilson bound: 80% against a coin-flip floor needs
/// **four** round trips. The cadence floor demanded 526.
/// How the banner names which rungs a run covered.
///
/// **NAMED, NOT COUNTED.** *"3 RUNGS"* beside a table of three rows is the same
/// fact twice and says nothing about WHICH three — and the banner is what a
/// reader comparing two reports reads first.
fn rungs_word(rungs: &[&str]) -> String {
    if rungs.len() == EVERY_RUNG.len() {
        "ALL EIGHT INTRADAY RUNGS".to_owned()
    } else {
        format!("RUNGS {}", rungs.join(", "))
    }
}

/// How the banner names the threshold a run used.
///
/// **The KIND of number, not just the number**, because the two are not
/// interchangeable and a reader comparing two reports has to know which they
/// are looking at. A fixed percentage is one question asked of every rung; a
/// derived floor is a DIFFERENT question asked of each — *the lowest support at
/// which a result here could still be believed* — so the same figure printed
/// without its provenance would invite comparing runs that measured different
/// things.
fn support_word(support_ppm: Option<u64>) -> String {
    support_ppm.map_or_else(
        || "DERIVED per rung from its own bars".to_owned(),
        |ppm| format!("{}.{}% (fixed)", ppm / 10_000, (ppm / 1_000) % 10),
    )
}

/// The lowest support at which a result on `bars` could still be believed.
///
/// # A number nobody typed and nobody baked in
///
/// `CLAUDE.md` §6 refuses a depth parameter because *"a parameter that can be
/// set can be set wrongly and silently"*. A CONSTANT has the same defect one
/// step earlier: it was set wrongly, once, by whoever wrote it, and then nobody
/// could see it at all. 200,000 ppm is such a constant, and `elite_descend`'s
/// doc records that a tenth of it *"cannot report a once-a-week setup no matter
/// how long it runs — the setup was pruned in the first level of the ladder,
/// and the report says nothing about it because nothing counted it"*.
///
/// So this is neither typed nor baked: it is DERIVED, per rung, per span, from
/// the only question the data can answer on its own — **below how many round
/// trips can the stated win rate no longer clear its own confidence bound?**
/// Under that count no combination can pass however good it looks, so
/// descending further buys nothing; above it, reachable answers are being
/// discarded before they are counted.
///
/// # Why the stop ceiling and the listing bound do not enter
///
/// [`statistical_floor_ppm`] reads `min_win_rate_bp` and `min_assurance_bp` and
/// nothing else, so `max_mae_ppm` and `top` cannot move the answer. They are
/// passed as `1` rather than plumbed through, and that is not laziness — a
/// caller made to supply a risk ceiling to learn a STATISTICAL floor would
/// reasonably believe the two were related, and would set it carefully for no
/// effect.
#[must_use]
pub fn statistical_support_floor(bars: u64) -> u64 {
    // SIZED BY [`sizing_rate_bp`], NOT BY THE ADMISSION RULE.
    //
    // This read `Rules::elite(1, 1)`, so whatever win rate `elite` happened to
    // hold silently decided how deep every browser sweep searched. Two
    // consequences, both measured:
    //
    // At `elite`'s old 8,000 the pair was `(8_000, 6_500)` and
    // `trades_needed_for` answered 29 -- which after the ppm round trip is the
    // `min_hits: 28` every rung of the operator's 2026-08-28 run was handed.
    // On the 1-minute rung's 617,921 bars that is 0.0045% support, three
    // orders of magnitude below the deepest figure D-0258 ever measured to
    // COMPLETE, and the run recorded nothing in five hours.
    //
    // And at the operator's own 5,000 it would be worse rather than better:
    // `assurance_floor_bp` returns the caller's figure at or below chance, so
    // the pair becomes `(5_000, 5_000)` -- a 95% lower bound asked to reach the
    // rate it is a bound on, which no sample size satisfies. `trades_needed_for`
    // would exhaust its search and return the ceiling, and nothing downstream
    // could tell that from a real answer.
    //
    // A rate that admits a row and a rate that sizes a search are different
    // questions. `Rules::elite` answers the first and this answers the second.
    let sizing = Rules::elite(1, 1).with_win_rate(sizing_rate_bp());
    statistical_floor_ppm(&sizing, bars)
}

fn statistical_floor_ppm(rules: &Rules, bars: u64) -> u64 {
    let needed = runner::grid::trades_needed_for(
        rules.min_win_rate_bp,
        rules.min_assurance_bp,
        // Bounded so an unsatisfiable pair terminates. The Wilson bound
        // approaches the observed rate from BELOW and never reaches it, so a
        // bound equal to the rate is unsatisfiable at every sample size — the
        // cap turns that into a high floor rather than a hang.
        TRADES_SEARCH_CEILING,
    );
    if bars == 0 {
        return 1;
    }
    needed.saturating_mul(1_000_000).saturating_div(bars).max(1)
}

/// The line an automated caller keys on when the OPERATOR'S OWN rules are met.
///
/// A distinct sentence rather than the word `PASS`, which appears inside the
/// phrase `NOTHING PASSED` and therefore matches on a page where nothing passed
/// at all. `elite_descend` read exactly that substring and stopped its support
/// walk on step one of ten, every time — the command's whole purpose defeated by
/// a marker that could not be absent.
pub const YOUR_RULES_MET: &str = "YOUR RULES: MET";

/// The same, when they are not.
pub const YOUR_RULES_UNMET: &str = "YOUR RULES: UNMET";

fn screen_cascade(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    by_evidence: &[&runner::rank::Scored],
    horizon: Horizon,
    rules: Rules,
    // Whether to walk the tier ladder when the stated rules find nothing. A
    // SEARCH step passes false: it needs one bit, not 960 priced tiers.
    validate: bool,
    // FORWARDED, NOT OWNED. See `screen`: the cells it prices are what the
    // operator ranks on, and this function is only a stop on the way to
    // `record_frontier`.
    priced: &mut std::collections::HashMap<[u64; 6], grid::Cell>,
) -> String {
    let top = rules.top;
    let mut out = String::with_capacity(4_096);

    // THE OPERATOR'S OWN RULES FIRST, and they were never applied at all.
    //
    // This function took only `rules.top` and then screened with `tier.rules(..)`
    // at every rung — so `Rules::admits` never saw the caller's policy, and
    // `cli elite`'s six thresholds reached nothing. What an operator actually
    // saw was a GENERATED tier whose `min_trades` is 500 or 1,000, which is the
    // exact floor `Rules::elite` sets to ZERO so a rare setup can be judged by
    // the assurance bound instead of rejected by a count.
    //
    // So the stated policy is tried first and named in the output. The tier
    // ladder below it is the fallback — the "what IS there" answer — and not a
    // replacement for the question that was asked.
    let yours = screen(bars, column, by_evidence, horizon, rules, priced);
    if yours.contains("NOTHING PASSED") {
        // A SEARCH STEP STOPS HERE, AND THAT IS THE WHOLE COST.
        //
        // Below this point the cascade walks the generated tier ladder —
        // **960 tiers at eight grid rungs, 4,800 at forty** — and each one is a
        // full `screen` over `screen_cap()` combinations. When nothing meets any
        // tier, every single one is priced.
        //
        // MEASURED, and it is the last thing that made a descent impossible: a
        // 60-minute step over six months did not finish in forty seconds at a
        // candidate ceiling of ONE HUNDRED and a screen cap of FIVE. Not the
        // sweep (0.005s), not the ladder, not the validation stack — this.
        //
        // A descent step needs ONE BIT: did anything clear the stated rules. The
        // tier ladder answers *what is the strictest standard anything DID meet*,
        // which is a question about the final answer and is asked once, on the
        // page an operator actually reads.
        if !validate {
            let _ = writeln!(
                out,
                "{YOUR_RULES_UNMET} — nothing cleared the policy you stated. The \
                 tier ladder is not walked on a search step; it is priced once, \
                 on the page that is kept."
            );
            // AND THE BANNER HERE TOO, BECAUSE "the page that is kept" IS A
            // PROMISE ONLY A DESCENT KEEPS.
            //
            // Inside `elite` that sentence is true: the rung that lands is
            // re-run with the full stack and THAT page is the one an operator
            // reads. Reached from `screen` with `BRUTEX_VALIDATE=0` there is no
            // later page at all, so the line alone reads as "a better answer is
            // coming" when none is. The banner says what did not run, which is
            // true either way.
            let _ = writeln!(out, "{UNVALIDATED}");
            return out;
        }
        let _ = writeln!(
            out,
            "{YOUR_RULES_UNMET} — nothing cleared the policy you stated. The tier \
             ladder below says what the strictest MET standard was."
        );
    } else {
        let _ = writeln!(
            out,
            "{YOUR_RULES_MET} — the rows below cleared the policy you stated, not \
             a relaxed one."
        );
        // AND THE BANNER, ON THE ONE PAGE THAT MOST NEEDS IT.
        //
        // The unmet branch above already says the ladder was not walked, so a
        // reader of THAT page knows it is partial. This branch had no marker at
        // all — rows that cleared the operator's rules, returned without the
        // walk-forward, the PBO or the bootstrap, and shaped exactly like rows
        // that had passed all three.
        //
        // That is the failure wearing a success's clothes §4 bans, and it is the
        // shape `CLAUDE.md` §5 already refuses for the two provenance banners:
        // the page is byte-identical either way, so the banner is the only thing
        // separating a candidate from a finding.
        if !validate {
            let _ = writeln!(out, "{UNVALIDATED}");
        }
        out.push_str(&yours);
        return out;
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "TIER LADDER");
    let _ = writeln!(
        out,
        "  Strictest first. The search stops at the first tier that yields \
         anything, and every tier above it is reported as unmet -- which is a \
         finding about the market, not an empty table."
    );
    // GENERATED, and printed as a SUMMARY rather than in full. Eight grid
    // rungs give 960 tiers and forty give 4,800 -- a page listing every one is
    // a page nobody reads. The strictest, the mildest and the count is what a
    // reader needs to know what was walked.
    // The trade count of the strongest candidate, which sets the win-rate
    // ladder's resolution: a rate over fifty trades cannot be judged as finely
    // as one over a thousand, and a ladder that pretended otherwise would rank
    // two rates the sample cannot separate.
    let sample = by_evidence.first().map_or(0, |s| s.hits);
    let ladder = tiers(bars, sample);
    if let (Some(first), Some(last)) = (ladder.first(), ladder.last()) {
        let _ = writeln!(
            out,
            "    {} tiers, generated from the grid's own axes",
            ladder.len()
        );
        let _ = writeln!(out, "    strictest  {}", first.describe());
        let _ = writeln!(out, "    mildest    {}", last.describe());
    }
    let _ = writeln!(out);

    for (rank, tier) in ladder.iter().enumerate() {
        let rules = tier.rules(top);
        let body = screen(bars, column, by_evidence, horizon, rules, priced);
        // `screen` prints "0 of N ... NOTHING PASSED" when the rules admit
        // nothing. Read back off the rendered text rather than recomputing the
        // predicate, so the cascade can never disagree with the table an
        // operator is looking at.
        if body.contains("NOTHING PASSED") {
            let _ = writeln!(out, "  {:<8} UNMET", Tier::label(rank));
            continue;
        }
        let _ = writeln!(
            out,
            "  {:<8} MET -- {}\n",
            Tier::label(rank),
            tier.describe()
        );
        out.push_str(&body);
        return out;
    }
    let _ = writeln!(
        out,
        "\n  NO TIER MET, INCLUDING THE MILDEST. A combination that cannot clear \
         even the mildest rung is not a near miss, and the TIGHTEST column in \
         the table below is \
         how far the closest one actually ran. This is a statement about these \
         bars, not a failure of the search."
    );
    // The mildest tier's table, so a reader still sees what was tried.
    if let Some(mildest) = ladder.last() {
        out.push_str(&screen(
            bars,
            column,
            by_evidence,
            horizon,
            mildest.rules(top),
            priced,
        ));
    }
    out
}

/// One combination's consistency across every calendar grain.
///
/// # The requirement that had no surface at all
///
/// The operator's rule is not "profitable on average". It is *"every month,
/// every week, every day, every quarter, every half and every year the same"* --
/// and a total P&L cannot express it. A combination that makes its whole return
/// in one quarter of 2020 and bleeds for six years reports the same net as one
/// that earns steadily, and until this the screen printed only the net.
///
/// [`crate::stability`] has been able to answer it since it was written, with
/// six tests, and had **zero callers**. This is the type that connects it.
///
/// Each entry is one grain and the share of that grain's periods that closed
/// positive, in basis points -- `10_000` reads "every single one".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Consistency {
    /// Positive share per grain, in the order of [`crate::stability::GRAINS`].
    shares_bp: [i64; 6],
    /// The worst single period's net, in paisa, at the FINEST grain.
    ///
    /// The finest grain is the honest one to report: a strategy positive in
    /// every year can still have a day that took a quarter of the account, and
    /// the yearly view cannot show it. A day is the smallest unit an intraday
    /// operator carries risk across.
    worst_day: i64,
    /// Periods counted at the coarsest grain, so a share can be read against a
    /// denominator. `10_000` of one year is not the same evidence as `10_000` of
    /// seven, and a share alone cannot tell them apart.
    years: usize,
}

impl Consistency {
    /// The share for one grain, in basis points, or zero past the grain list.
    fn share_bp(&self, grain: usize) -> i64 {
        self.shares_bp.get(grain).copied().unwrap_or(0)
    }

    /// The WEAKEST grain's share. One number for "consistent everywhere".
    ///
    /// The minimum and not the mean, for the same reason [`Rules::admits`] takes
    /// every rule together: a strategy positive in every year and negative in
    /// half its months is not consistent, and averaging the two grains hides
    /// exactly that.
    fn weakest_bp(&self) -> i64 {
        self.shares_bp.iter().copied().min().unwrap_or(0)
    }
}

/// Measure one chosen variant trade by trade, and bucket it by every grain.
///
/// Returns `None` when the variant cannot be re-walked -- which is not an error
/// worth failing the screen over, because the cell it came from is still a
/// valid measurement. The row simply reports no consistency rather than a
/// fabricated one.
fn consistency_of(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    scored: &runner::rank::Scored,
    horizon: Horizon,
    side: runner::excursion::Side,
    exits: &grid::Grid,
    cell: &grid::Cell,
) -> Option<Consistency> {
    // `Chosen` is `Cell`'s four exit fields and nothing else, so the variant
    // that won is re-expressed rather than re-searched. Re-searching would risk
    // walking a DIFFERENT cell than the one the row reports.
    let chosen = grid::Chosen {
        stop: cell.stop,
        target: cell.target,
        tsl: cell.tsl,
        ttp: cell.ttp,
    };
    let (_, rows) = grid::per_trade(
        bars,
        column,
        &scored.mask,
        horizon,
        side,
        runner::excursion::Ladders {
            stops: &exits.stops,
            targets: &exits.targets,
            trails: &exits.trails,
        },
        chosen,
    )?;
    if rows.is_empty() {
        return None;
    }
    let mut shares_bp = [0_i64; 6];
    let mut worst_day = 0_i64;
    let mut years = 0_usize;
    for (slot, grain) in crate::stability::GRAINS.iter().enumerate() {
        let measured = crate::stability::at(&rows, *grain);
        if let Some(share) = shares_bp.get_mut(slot) {
            *share = measured.positive_share_bp();
        }
        match *grain {
            crate::stability::Grain::Day => worst_day = measured.worst_period(),
            crate::stability::Grain::Year => years = measured.buckets.len(),
            _ => {}
        }
    }
    Some(Consistency {
        shares_bp,
        worst_day,
        years,
    })
}

fn screen(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    by_evidence: &[&runner::rank::Scored],
    horizon: Horizon,
    rules: Rules,
    // THE CELLS THIS SCREEN PRICED, HANDED BACK RATHER THAN RENDERED AND
    // DROPPED. Every metric the operator ranks on -- drawdown, losing trades,
    // win rate, smallest win, worst trade -- is on a `Cell`, and this function
    // computed one for every candidate and returned only text. An out-parameter
    // rather than a wider return type, because three callers up the chain would
    // otherwise each need their tuple widened for a value only the last one uses.
    priced: &mut std::collections::HashMap<[u64; 6], grid::Cell>,
) -> String {
    // Built ONCE for the whole screen: the same ladder judges every combination,
    // and `Levels` only borrows it.
    let stop_rungs = stop_ladder_ppm(bars);

    // AND SO ARE THESE TWO, WHICH WERE NOT, AND THAT WAS THE EXPENSIVE HALF.
    //
    // `rungs` and `step_ppm` were computed INSIDE the loop below — once per
    // candidate, up to `screen_cap()` of them. Neither depends on the
    // candidate. Each call walks every bar:
    //
    //   `grid_step_ppm`  allocates a Vec of one i64 per bar and sorts it
    //   `grid_rungs`     calls `reference_price` (two scans), `grid_step_ppm`
    //                    AGAIN, and `max_stop_points` -> another alloc and sort
    //
    // Three allocations of `bars.len()` and three sorts, per candidate. At
    // 617,921 bars that is ~15 MB allocated and freed per candidate and about
    // 3.5e7 comparisons -- ten thousand times over, and once the loop went
    // parallel, on every core at once.
    //
    // The line above already knew to do this and says so in its own words:
    // "Built ONCE for the whole screen". One of four invariants was hoisted and
    // the other three were left in the loop. Parallelising the loop hid the
    // redundancy behind more cores rather than removing it.
    let rungs = grid_rungs(bars);
    let step_ppm = grid_step_ppm(bars);

    // ACROSS EVERY CORE. This was a sequential `for` over up to `screen_cap()`
    // -- ten thousand -- combinations, each pricing a FULL exit grid over every
    // bar of the span. The repo's own measurement of what that costs is in
    // `screen_cap`'s doc: on NIFTY 60min over twelve months "the sweep takes 14
    // seconds and `screen` returned nothing at all after fifty minutes, twice".
    //
    // The body is pure per candidate: `grid::evaluate` reads `bars`, `column`
    // and `stop_rungs` immutably, `rules.admits` is a comparison, and the only
    // thing leaving the iteration is the row it builds. Nothing accumulates
    // across iterations -- the two `continue`s become `None` from a
    // `filter_map`.
    //
    // DETERMINISM (CLAUDE.md S3 rule 5): rayon's INDEXED collect preserves
    // order, so `rows` arrives in the same sequence a `for` produced. That
    // matters even though the next statement sorts it, because `sort_by_key` is
    // STABLE -- two rows with equal `(admitted, pessimistic)` keep their
    // relative order, so a shuffled input would silently reorder ties and the
    // reported top 25 could differ between runs on the same bytes. Byte-
    // identical output on any core count is the property that makes this safe.
    let mut rows: Vec<Screened<'_>> = by_evidence
        .par_iter()
        .take(screen_cap())
        .enumerate()
        .filter_map(|(rank, scored)| {
            let side = side_of_evidence(scored);
            // THE OPERATOR'S OWN STOP IS TRIED, NOT MERELY USED AS A FILTER.
            //
            // `max_mae_ppm` was a post-hoc test: build the grid from quantiles of
            // each combination's own excursions, then discard every variant whose
            // worst trade exceeded the rule. So the question the engine answered was
            // "did this signal HAPPEN to keep every trade inside twenty points",
            // which almost nothing does -- and never "does this signal work WITH a
            // twenty point stop", which is the question actually being asked.
            //
            // The difference is not small. A quantile ladder's tightest rung is the
            // 20th percentile of what the signal did, so on a loose signal the
            // engine's tightest stop was 312 index points and a 20-point rule could
            // only ever reject it. Raising the rung count subdivides the same
            // distribution and never reaches below its floor.
            //
            // Passed as a rung, the level is tried like any other: every variant
            // that could pair with a derived stop can pair with this one, and a
            // combination that is mediocre on its own quantiles but strong under the
            // operator's stop can now be found rather than filtered out unseen.
            let g = grid::evaluate(
                bars,
                column,
                &scored.mask,
                horizon,
                side,
                grid::Levels {
                    rungs,
                    step_ppm: Some(step_ppm),
                    forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
                    ratios: true,
                    stops_ppm: &stop_rungs,
                },
            );
            // THE BEST VARIANT THAT SATISFIES THE RULES, falling back to the best
            // overall only so a failing combination can still be SHOWN with the rule
            // it broke. Asking `best()` first and judging that was the error: the
            // profit-maximising cell is the one with no stop at all, so every
            // combination failed a stop rule by construction.
            let within = g.best_within(|c| rules.admits(c)).copied();
            let cell = within.or_else(|| g.best().copied())?;
            if cell.trades == 0 {
                return None;
            }
            Some(Screened {
                rank: rank.saturating_add(1),
                tightest: g.tightest_containment().copied(),
                admitted: within.is_some(),
                names: runner::report::condition_names(&scored.mask).join(" · "),
                cell,
                scored,
                consistency: None,
                // Until measured, a row is steady: a rule that has not run yet
                // cannot have been broken.
                steady: true,
            })
        })
        .collect();

    // PASSERS FIRST, then by net. `Reverse` and not a negation, for the reason
    // `audit::grid` gives: `pessimistic` saturates at `i64::MIN` and negating
    // that panics under `overflow-checks`, killing the process over a sort.
    //
    // Sorted TWICE, and the second one is not redundant. `measure_top` only
    // measures what this ordering put in the top `rules.top`, and the gate
    // below can demote some of them -- so the final order has to be taken after
    // the demotions, or a refused row sits above a passing one.
    // RECORDED BEFORE THE SORT IS USED FOR ANYTHING, so the map holds every
    // candidate this screen priced and not merely the ones that survived the
    // top cut below. `record_frontier` writes `rules.top` rows and needs a cell
    // for each; which ones those are is decided after this point.
    for row in &rows {
        priced.insert(row.scored.mask.words(), row.cell);
    }

    rows.sort_by_key(|r| (!r.admitted, core::cmp::Reverse(r.cell.pessimistic)));

    measure_top(&mut rows, bars, column, horizon, rules);

    // THE CALENDAR GATE, AFTER THE CELL GATES.
    //
    // A row that was admitted on its five cell rules and is inconsistent across
    // the calendar is refused HERE, because a cell has no calendar to check --
    // see `Rules::min_weakest_bp`. An unmeasured row is left alone: refusing it
    // would be refusing it for a rule that never ran.
    for row in rows.iter_mut().take(rules.top) {
        if let Some(ref c) = row.consistency {
            row.steady = c.weakest_bp() >= rules.min_weakest_bp;
            if !row.steady {
                row.admitted = false;
            }
        }
    }
    rows.sort_by_key(|r| (!r.admitted, core::cmp::Reverse(r.cell.pessimistic)));

    let passed = rows.iter().filter(|r| r.admitted).count();
    let mut out = rules_banner(rules, passed, rows.len());
    let _ = writeln!(
        out,
        "  {:<5}{:>8}{:>6}{:>10}{:>11}{:>6}{:>13}{:>13}{:>6}  conditions",
        "rank", "trades", "win%", "worstMAE", "TIGHTEST", "PF", "net", "exit", "rule"
    );
    for row in rows.iter().take(rules.top) {
        let pf = row.cell.profit_factor_bp();
        // `pf` is still printed; `rr` moved into the TIGHTEST column, which
        // answers a question the ratio could not: what stop is REACHABLE.
        let _ = row.cell.reward_to_risk_bp();
        let _ = writeln!(
            out,
            "  {:<5}{:>8}{:>6}{:>10}{:>11}{:>6}{:>13}{:>13}{:>6}  {}",
            row.rank,
            row.cell.trades,
            format!("{}%", row.cell.win_rate_bp() / 100),
            ppm_as_percent(row.cell.worst_mae),
            // THE TIGHTEST ANY VARIANT ACHIEVED, in points, so a reader learns
            // what is REACHABLE instead of guessing another threshold. A rule
            // below this cannot be met by any of the 625 exits.
            row.tightest.map_or_else(
                || "-".to_owned(),
                |t| format!("{}pt", ppm_to_points(t.worst_mae)),
            ),
            if pf == i64::MAX {
                "inf".to_owned()
            } else {
                hundredths_of(pf)
            },
            rupees(row.cell.pessimistic),
            exit_label(&row.cell),
            if row.admitted {
                "PASS"
            } else {
                why_refused(&row.cell, rules, row.steady())
            },
            row.names
        );
    }
    let _ = writeln!(out);

    append_consistency(&mut out, &rows, rules.top);
    out
}

/// The rules banner: every rule that is on, and `off` for every one that is not.
///
/// # Why it is its own function
///
/// It listed three rules while `admits` enforced five and the calendar gate
/// a sixth, so a reader could not tell whether a row was refused by a rule
/// they set or by one the engine applied without saying. Stating all six
/// pushed [`screen`] past the hundred-line ceiling, and the split is the
/// right cut anyway: this is the POLICY, and everything around it is the
/// measurement the policy was applied to.
fn rules_banner(rules: Rules, passed: usize, considered: usize) -> String {
    let mut out = String::from(
        "TOP COMBINATIONS, SCREENED
",
    );

    let _ = writeln!(
        out,
        // EVERY RULE THAT IS ON, and `off` for every one that is not.
        //
        // This listed three while `admits` enforced four and then five, so a
        // reader could not tell whether a row was refused by a rule they set or
        // by one the engine applied without saying. A rule that filters and
        // does not appear here is the silent policy §6 refuses to allow even in
        // a parameter.
        "  RULES, all yours and all required together:\n    \
         1. no single trade may run more than {} against entry (the WORST, not the mean)\n    \
         2. the SMALLEST win must be at least {} times the LARGEST loss\n    \
         3. at least {} of trades must WIN\n    \
         4. at least {} round trips, and the 95% LOWER BOUND on the win rate must \
         still reach {}\n    \
         5. at least {} of periods must close POSITIVE at EVERY grain -- year, \
         half, quarter, month, week, day\n    \
         6. report the top {}\n\n  \
         {passed} of {} priced combinations satisfy every rule.{}\n",
        ppm_as_percent(rules.max_mae_ppm),
        hundredths_of(rules.min_rr_bp),
        bp_as_percent(rules.min_win_rate_bp),
        if rules.min_trades == 0 {
            "no minimum".to_owned()
        } else {
            rules.min_trades.to_string()
        },
        bp_as_percent(rules.min_assurance_bp),
        bp_as_percent(rules.min_weakest_bp),
        rules.top,
        considered,
        if passed == 0 {
            " NOTHING PASSED -- the rows below are shown so a reader can see what \
             was tried and by how much each missed, which is a finding rather \
             than an empty table."
        } else {
            ""
        }
    );
    out
}

/// Measure consistency for the rows that will actually be printed.
///
/// # Why the caller does not do this inline
///
/// Two reasons, and the first is a bug that was measured. Inline, this ran
/// inside the screening loop -- over `screen_cap()` combinations, ten
/// thousand by default -- and every one paid for a full trade-by-trade
/// re-walk when only `top` are ever rendered. A single-month screen that
/// had taken seconds stopped finishing inside 280.
///
/// The second is that [`screen`] was 113 lines with it, past the hundred
/// clippy enforces.
fn measure_top(
    rows: &mut [Screened<'_>],
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    horizon: Horizon,
    rules: Rules,
) {
    // MEASURED AFTER THE SORT, AND ONLY FOR WHAT WILL BE PRINTED.
    //
    // `rules.top` rows, not `screen_cap()` -- see `Screened::scored`. The grid
    // is rebuilt here rather than carried on the row: a `Grid` holds every
    // variant's cell, and keeping ten thousand of them resident to use twenty-
    // five is the memory the bounded heap in `rank` exists to avoid.
    //
    // Rebuilding is deterministic (§3 rule 5) and produces the same grid the
    // screening pass produced, so the cell this re-walks is the cell the row
    // above reports.
    let stop_rungs_again = stop_ladder_ppm(bars);
    for row in rows.iter_mut().take(rules.top) {
        let side = side_of_evidence(row.scored);
        let g = grid::evaluate(
            bars,
            column,
            &row.scored.mask,
            horizon,
            side,
            grid::Levels {
                rungs: grid_rungs(bars),
                step_ppm: Some(grid_step_ppm(bars)),
                forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
                ratios: true,
                stops_ppm: &stop_rungs_again,
            },
        );
        row.consistency = consistency_of(bars, column, row.scored, horizon, side, &g, &row.cell);
    }
}

/// The consistency table, appended to a screen.
///
/// # Why it is its own function
///
/// It pushed [`screen`] to 175 lines, past the hundred clippy enforces.
/// The split is the right cut regardless: everything above it answers
/// "how much did this make and what did it risk", and this answers "did it
/// make it STEADILY" -- a different question, on a different table, with a
/// different failure mode.
fn append_consistency(out: &mut String, rows: &[Screened<'_>], top: usize) {
    // CONSISTENCY, and it is a SEPARATE table on purpose.
    //
    // # Why not another column
    //
    // Six grains is six numbers per row, and the table above is already nine
    // columns wide. Folded into one summary column they would answer nothing --
    // "positive in 82% of periods" cannot say WHICH periods, and a strategy
    // positive in every year and negative in half its months is the exact shape
    // this is meant to expose. Each grain gets its own column, in its own table.
    //
    // # Why it is printed even when nothing passed
    //
    // A combination that fails the stop rule may still be the most consistent
    // thing in the run, and that is a finding about these bars. The screen
    // already prints failing rows for the same reason.
    let measured = rows
        .iter()
        .take(top)
        .filter(|r| r.consistency.is_some())
        .count();
    if measured == 0 {
        let _ = writeln!(
            out,
            "  CONSISTENCY: not measured for any row. The chosen variant could \
             not be re-walked trade by trade,\n  which is a gap in the report \
             and not a statement about the strategy."
        );
        return;
    }
    let _ = writeln!(
        out,
        "  CONSISTENCY -- the share of each period that closed POSITIVE. \
         `10000` reads every single one.\n  \
         A total cannot answer this: one great quarter and six flat years \
         reports the same net as steady earning."
    );
    let _ = writeln!(
        out,
        "  {:<5}{:>8}{:>8}{:>10}{:>8}{:>7}{:>7}{:>9}{:>14}",
        "rank",
        "years",
        "yearly",
        "quarterly",
        "monthly",
        "weekly",
        "daily",
        "WEAKEST",
        "worst day"
    );
    for row in rows.iter().take(top) {
        let Some(ref c) = row.consistency else {
            let _ = writeln!(out, "  {:<5}   not measured", row.rank);
            continue;
        };
        let _ = writeln!(
            out,
            "  {:<5}{:>8}{:>8}{:>10}{:>8}{:>7}{:>7}{:>9}{:>14}",
            row.rank,
            c.years,
            c.share_bp(0),
            c.share_bp(1),
            c.share_bp(2),
            c.share_bp(3),
            c.share_bp(4),
            // THE COLUMN THE OPERATOR'S RULE ACTUALLY READS. His requirement is
            // every grain at once, so the weakest one is the whole answer and
            // the five before it are the evidence for it.
            c.weakest_bp(),
            rupees(c.worst_day),
        );
    }
    let _ = writeln!(
        out,
        "\n  WEAKEST is the minimum across all six grains, and it is the column \
         the rule reads.\n  The minimum and not the mean: positive in every year \
         and negative in half its months\n  is not consistent, and averaging the \
         two hides exactly that.\n  \
         `worst day` is the single worst DAY's net -- a strategy positive every \
         year can still\n  have a day that took a quarter of the account, and no \
         coarser grain can show it."
    );
}

/// The exit variant's rungs, as the `exit` column prints them.
///
/// `stop/target/tsl+arm@ttp`, with `-` for an axis that is switched off. A
/// screen row that says PASS without naming the variant leaves an operator
/// unable to place the trade: the stop that made it pass IS the answer.
fn exit_label(cell: &grid::Cell) -> String {
    let rung = |r: Option<usize>| r.map_or_else(|| "-".to_owned(), |v| v.to_string());
    let levels = format!(
        "{}/{}/{}",
        rung(cell.stop),
        rung(cell.target),
        rung(cell.tsl)
    );
    cell.ttp.map_or(levels.clone(), |t| {
        format!("{levels}+{}@{}", t.arm, t.trail)
    })
}
/// Which rule a variant broke, named rather than left to be inferred.
fn why_refused(cell: &grid::Cell, rules: Rules, steady: bool) -> &'static str {
    // EVERY RULE, NOT TWO OF THEM.
    //
    // This knew about `max_mae_ppm` and `min_rr_bp` and nothing else, so a row
    // refused for its win rate, its trade count or its assurance printed `-` in
    // the `rule` column -- the same glyph a PASSING row prints. `admitted` was
    // false and the reason column said nothing was wrong, which is the failure
    // wearing a success's clothes §4 bans, in a single character.
    //
    // Order matters: the first rule broken is the one named, and they are
    // checked cheapest first. A row can break several and an operator only
    // needs one to act on.
    if cell.worst_mae > rules.max_mae_ppm {
        "MAE"
    } else if cell.reward_to_risk_bp() < rules.min_rr_bp {
        "R:R"
    } else if cell.win_rate_bp() < rules.min_win_rate_bp {
        "win%"
    } else if cell.trades < rules.min_trades {
        "few"
    } else if cell.assurance_bp() < rules.min_assurance_bp {
        // The rate was observed but the sample does not SUPPORT it. Named
        // separately from `win%` because the two are opposite findings: one says
        // the strategy did not win often enough, the other that it did not win
        // often enough TIMES for the rate to mean anything.
        "conf"
    } else if !steady {
        "steady"
    } else {
        "-"
    }
}

/// Basis points as a percentage, or `off` when the rule is switched off.
///
/// `off` and not `0.00%`, because the two mean opposite things and every rule
/// in [`Rules`] documents zero as DROPPING the rule. A banner reading "at least
/// 0.00% of trades must win" states a rule that is not being applied, which is
/// the same lie as omitting it.
fn bp_as_percent(bp: i64) -> String {
    if bp == 0 {
        return "off".to_owned();
    }
    format!("{}.{:02}%", bp / 100, (bp % 100).abs())
}

/// An integer in hundredths with its decimal point. `200` is `2.00`.
fn hundredths_of(n: i64) -> String {
    let negative = n < 0;
    let m = n.unsigned_abs();
    format!(
        "{}{}.{:02}",
        if negative { "-" } else { "" },
        m / 100,
        m % 100
    )
}

/// Parts per million as a percentage AND as index points, both to two decimals.
///
/// Points because that is how a stop is spoken about — "thirty points" — and a
/// percentage because a rung must mean the same thing on NIFTY and BANKNIFTY.
/// The point figure assumes a 25,000 index, which is stated rather than implied:
/// it is a reading aid and not a measurement.
fn ppm_as_percent(ppm: i64) -> String {
    let hundredths = ppm.unsigned_abs() / 100;
    format!("{}.{:02}%", hundredths / 100, hundredths % 100)
}

/// One rung's row in the comparison table.
struct RungRow {
    rung: &'static str,
    outcome: Result<crate::results::Record, String>,
}

/// One rung's row: count its bars, derive its threshold, sweep it, read it back.
///
/// Split out of [`range_all`] to keep it under `clippy::too_many_lines`, and
/// because it is one idea — everything that happens to a single rung.
///
/// The span is COUNTED before it is swept, because `min_hits` cannot be known
/// until that rung's own bar count is. It is a second read of the same files:
/// one open per month, and it buys the only thing that makes nine rungs
/// comparable.
/// The deepest threshold this column can be swept at and still COMPLETE.
///
/// # Why a search needs this and not only a statistical floor
///
/// [`statistical_support_floor`] answers *"below how many round trips can a
/// stated rate no longer clear its own confidence bound"*, and that is a real
/// bound that must be respected — but it is a bound on the SAMPLE, and a
/// sufficient sample is roughly forty-seven trades. On a 617,921-bar rung that
/// is 0.0074% support. Any floor derived only from sample size is therefore
/// structurally incapable of bounding a long span: the trades it needs are
/// always a vanishing fraction of the bars on offer.
///
/// What actually bounds an Apriori walk is the CANDIDATE CEILING, and
/// [`runner::Sweeper::auto`] is the search that finds where the two meet. It
/// brackets downward from `swept - 1`, walking the ladder at each rung, and
/// settles on the deepest threshold whose sweep completed rather than breaching
/// the budget. The column is folded once and every probe reuses it, so the cost
/// is `log2(bars)` walks — paid once per rung per run, never per bar and never
/// per candidate.
///
/// # It is derived from the machine as well as from the data
///
/// The ceiling handed to the probe is the one [`ceiling_from_env`] resolved for
/// THIS machine and THIS run — core count, any operator override, and the
/// division among concurrently running rungs that `SharedBy` applies. So the
/// answer moves with the hardware, with the span, and with how many rungs are
/// in flight, and none of those is a number anybody typed.
///
/// `None` when the probe could not settle: an empty column, or a column where
/// even the cheapest threshold refused. The caller then falls back to the
/// statistical floor rather than inventing one, which is the honest direction
/// to fail — it searches deeper than it can afford and says so through the
/// completion flag, instead of silently reporting extinction it never reached.
fn affordable_min_hits(bars: &[indicators::Candle]) -> Option<u64> {
    let mut ev = evaluator().ok()?;
    // [`SEARCH_CEILING`], NOT THE RUN'S OWN CEILING. A PROBE IS NOT A SWEEP.
    //
    // This passed `ceiling_from_env()` — the full 134,217,720 candidates the
    // machine can hold, about 19.6 GB at 146 bytes each — to a walk whose only
    // job is to ESTIMATE a threshold. `Sweeper::auto` brackets downward, so its
    // cheap probes are at high thresholds and its expensive ones are at low
    // thresholds where almost everything is frequent; handed the whole budget,
    // the low probes are free to allocate the whole budget.
    //
    // MEASURED, and it is why nothing finished: one rung over ONE YEAR of
    // 60-minute bars — about 1,700 bars, against `docs/05-decisions.md` D-0258
    // completing 5,249 bars in 4.26 seconds — sat at 10.24 GB resident and
    // 95.8% CPU, a single core, with the sampler showing `auto` and `walk`. The
    // affordability probe added to make runs finish was itself the thing that
    // did not finish, and it was invisible because it runs BEFORE the first
    // event `one_rung` emits.
    //
    // `SEARCH_CEILING` is 500,000 — about 73 MB — and `cli auto` has always
    // used it for precisely this call, with a compile-time assertion beside it
    // that it is at least a hundred times smaller than `DEFAULT_CEILING`. The
    // answer a probe returns is a THRESHOLD, and a threshold found under a
    // smaller budget is conservative in the safe direction: it may sit higher
    // than strictly necessary, which prunes more, never less.
    Sweeper::new(Ladder::with_min_hits(1).with_ceiling(SEARCH_CEILING))
        .auto(bars, &mut ev)
        .min_hits
}

fn one_rung(
    vendor_word: &str,
    underlying: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> RungRow {
    let first_line = |why: String| why.lines().next().unwrap_or(&why).to_owned();
    let root = match store_root() {
        Ok(root) => root,
        Err(why) => {
            return RungRow {
                rung,
                outcome: Err(first_line(why)),
            };
        }
    };
    let vendor = match parse_vendor(vendor_word) {
        Ok(vendor) => vendor,
        Err(why) => {
            return RungRow {
                rung,
                outcome: Err(first_line(why)),
            };
        }
    };
    let span = match stored::load_span(&root, vendor, underlying, rung, from, to) {
        Ok(span) => span,
        Err(why) => {
            return RungRow {
                rung,
                outcome: Err(first_line(why)),
            };
        }
    };
    let bars = span.bars.len();
    // DERIVED FROM THIS RUNG'S OWN BARS WHEN NOBODY NAMES A SUPPORT.
    //
    // `None` is not a default hiding in an `Option`; it is the ABSENCE of a
    // typed threshold, which `CLAUDE.md` §6 asks for and which a CONSTANT does
    // not give. A fixed percentage is still somebody's guess baked into the
    // binary, and `elite_descend`'s own doc records that a guess a TENTH of
    // 200,000 ppm *"cannot report a once-a-week setup no matter how long it
    // runs"*.
    //
    // `statistical_support_floor` asks the only question the data alone can
    // answer: below how many round trips can the stated win rate no longer
    // clear its own confidence bound? Under that, NO combination can pass
    // however good it looks — and stopping anywhere above it discards reachable
    // answers. D-0303.
    // THE OPERATOR'S OWN SUPPORT, IF THEY NAME ONE, AND NOTHING IF THEY DO NOT.
    //
    // D-0303 removed the typed threshold because "whatever figure an operator
    // types, they have already decided how often the answer may fire, before
    // anything is measured" -- and that argument is about the DEFAULT, which is
    // still derived. What it left the operator without is any way to search
    // deeper than the derivation chose, and the first real results made the
    // cost of that concrete.
    //
    // MEASURED, 2026-08-29, zerodha NIFTY over 81 months: the affordability
    // probe settled every rung near 22% support, seven runs completed to depth
    // 15, and every one of them LOST under worst-case fills -- 1,758 trades at
    // 60min for -Rs 5,847, 13,266 trades at 3min for -Rs 84,053, the loss
    // scaling with trade count across all six rungs. That is the signature of
    // frequent setups being arbitraged into the spread, and 22% support is
    // frequent BY DEFINITION: a pattern firing on one bar in five. The rare
    // setup the operator is hunting -- once a week over seven years -- sits
    // near 0.3% and was never enumerated, so it was never counted, ranked or
    // priced.
    //
    // Expressed in ppm of the rung's OWN bars rather than as a hit count, for
    // the reason `range_over`'s banner gives: 81 months holds 1,671 daily bars
    // and 618,296 one-minute ones, so one absolute threshold would ask eight
    // different questions and the table would compare nothing. `BRUTEX_SUPPORT_PPM=100000`
    // is 10% on every rung; `50000` is 5%.
    //
    // Refused at zero and at a million: zero makes every combination frequent
    // so the frontier never empties and the walk has no end, and a million
    // demands a pattern present on every bar, which D-0080 excludes as
    // `AlwaysTrue` before k=1. Both are the same refusal `screen` already makes.
    let operator_ppm = crate::knobs::var("BRUTEX_SUPPORT_PPM")
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|&ppm| ppm > 0 && ppm < 1_000_000);

    let named_ppm = support_ppm.or(operator_ppm);
    let statistical = min_hits_for(
        bars,
        named_ppm
            .unwrap_or_else(|| statistical_support_floor(u64::try_from(bars).unwrap_or(u64::MAX))),
    );

    // TWO FLOORS, AND THE SEARCH TAKES WHICHEVER BINDS. Neither is a constant
    // and neither is typed; both are read off this rung's own bars at runtime.
    //
    // The STATISTICAL floor above answers "below how many round trips can a
    // rate no longer clear its own confidence bound". It is necessary and it is
    // not sufficient, and the reason is arithmetic rather than opinion: a
    // sufficient sample is ~47 trades, which on the 1-minute rung's 617,921
    // bars is 0.0074% support. A floor derived only from sample size is
    // STRUCTURALLY INCAPABLE of bounding a search over a long span, because the
    // sample it needs is always a vanishing fraction of the bars available.
    //
    // Measured, and this is the whole reason the operator's 2026-08-28 run
    // recorded nothing in five hours: it ran at 0.0045%, while D-0258's cost
    // curve records 4.7% as the deepest support ever measured to COMPLETE and
    // 3.3% as already refused on candidate budget. Three orders of magnitude
    // below anything observed to finish, and no rule downstream could recover
    // it -- the run simply never ended.
    //
    // `Sweeper::auto` answers the OTHER question: which is the deepest
    // threshold this column can be swept at and still COMPLETE under the
    // ceiling this machine derived. It brackets downward from `swept - 1` and
    // reports what it settled on, costing `log2(bars)` ladder walks over a
    // column folded once -- bounded, per rung, and paid once per run.
    //
    // Taking the MAXIMUM is what makes the pair honest. Below the statistical
    // floor no answer can be trusted however affordable it is; below the
    // affordable floor no answer arrives at all however trustworthy it would
    // be. A search that respects only one of them either reports noise or
    // reports nothing, and this run did the second for five hours.
    //
    // An operator who NAMES a support still gets exactly what they named --
    // `Some(_)` skips this entirely. The probe exists because `None` means
    // "derive it", and deriving it from the data alone was half an answer.
    let min_hits = match named_ppm {
        Some(_) => statistical,
        None => affordable_min_hits(&span.bars).map_or(statistical, |a| a.max(statistical)),
    };

    // ENTERING THE SWEEP IS ALSO AN EVENT, AND THE SILENCE BELOW IT IS THE LONG
    // ONE.
    //
    // "rung finished" closed the BACK half of the gap. The front half is the
    // half that hurts: `audit_range` on the next line holds the ladder walk and
    // the screen, and between the span load and its return NOTHING is emitted.
    //
    // MEASURED, 2026-08-29, on the operator's own 10% run: sixty-two minutes,
    // eleven of fourteen cores, 9.2 GB resident, eight rungs dispatched in
    // parallel by `par_iter` — and the last event of any kind was a span load
    // FIVE SECONDS after the start. There was no way to tell from outside which
    // of the eight were still running, which had finished, or at what support
    // any of them was searching. `ps` said the process was busy; nothing said
    // what it was busy with.
    //
    // `min_hits` is carried as a COUNT and as PPM OF THIS RUNG'S OWN BARS,
    // because the count alone compares nothing across rungs: 61,829 on the
    // 1-minute rung and 1,154 on the 60-minute one are the same question asked
    // of 618,296 bars and 11,545, and only the ppm says so.
    //
    // Same granularity as "rung finished" — one event per rung per run — so
    // gate 17's rule is untouched: this is `cli`, at the rung boundary, and no
    // loop over bars or candidates can reach it.
    crate::note(
        &telemetry::Event::info("cli.audit", "rung sweeping")
            .with("feed", vendor_word)
            .with("underlying", underlying)
            .with("rung", rung)
            .with("bars", u64::try_from(bars).unwrap_or(u64::MAX))
            .with("min_hits", min_hits)
            .with(
                "support_ppm",
                min_hits
                    .saturating_mul(1_000_000)
                    .checked_div(u64::try_from(bars).unwrap_or(u64::MAX))
                    .unwrap_or(0),
            )
            .with("named", u64::from(named_ppm.is_some())),
    );

    // The long report is DISCARDED on purpose: nine of them is six thousand
    // lines. The row is read back from the store, which is the point of having
    // one.
    let text = audit_range(vendor_word, underlying, rung, from, to, min_hits);
    let outcome = if let Some(why) = text.strip_prefix("refused: ") {
        Err(first_line(why.to_owned()))
    } else if let Some(why) = not_recorded_reason(&text) {
        // A ROW THAT DID NOT LAND IS A REFUSAL, NOT A LOOKUP.
        //
        // `latest_for` reads the newest row matching the KEY -- feed, underlying,
        // rung, span, min_hits -- and the key does not carry the identity. So
        // when this run's append failed, the read did not fail with it: it
        // returned an EARLIER run's row, from a different commit and possibly a
        // different ceiling, and the descent printed it under this run's banner
        // with every column plausible. `record_run` named the reason loudly and
        // the string it named it into was thrown away here.
        //
        // Refusing costs the rung its row and says why, which is what §4 asks
        // for. The alternative -- matching `record.identity` in `latest_for` --
        // is the stronger fix and needs the `RunId` computed twice or threaded
        // through; this closes the silent substitution now and does not block it.
        Err(first_line(format!("the result was not recorded: {why}")))
    } else {
        latest_for(vendor_word, underlying, rung, from, to, min_hits)
    };

    // A RUNG FINISHING IS AN EVENT, AND IT WAS NOT ONE.
    //
    // `one_rung` emitted nothing on completion. Eight rungs would finish and the
    // log stayed silent between "stored span loaded" at the start and the
    // report at the very end, so an operator watching a multi-hour run could not
    // tell a working sweep from a hung one — and could not tell which of the
    // eight had finished, or that any had.
    //
    // MEASURED, twice, over two nights: a sweep held thirteen cores for seven
    // hours with no event of any kind after the eight span loads. The run was
    // healthy. Nothing said so.
    //
    // Gate 17 silences `vocab engine indicators runner` because those hold the
    // loops, and its rule is not "each call is cheap" but "the innermost loop
    // calls nothing at all". This is `cli`, at the rung boundary — one event per
    // rung per run, which is the granularity gate 17's own comment prescribes as
    // the affordable one, and the same granularity `batch` already reports at.
    //
    // The refusal is carried when there is one: a rung that refused is the case
    // an operator most needs to see, and it is exactly the case that produced no
    // row for `/backtest.json` to show.
    crate::note(
        &telemetry::Event::info("cli.audit", "rung finished")
            .with("feed", vendor_word)
            .with("underlying", underlying)
            .with("rung", rung)
            .with("bars", u64::try_from(bars).unwrap_or(u64::MAX))
            .with("min_hits", min_hits)
            .with("recorded", u64::from(outcome.is_ok()))
            .with("why", outcome.as_ref().err().map_or("", String::as_str)),
    );

    RungRow { rung, outcome }
}

/// Trading weeks a month holds, times one hundred.
///
/// # Why a scaled integer and not 4.348
///
/// A month is 30.44 days and a trading week is five of them, so the weeks in a
/// month is 4.348 -- a number §7 will not let near a float when it feeds a
/// count. Scaled by a hundred it stays exact through the multiply and divides
/// out once, at the end, where a single rounding is visible.
const WEEKS_PER_MONTH_CENTI: u64 = 435;

/// The support a target CADENCE implies on a span, in parts per million.
///
/// # The number the operator actually has
///
/// An operator does not think in support. He thinks *"one intraday trade a
/// week"* -- and that is a claim about the calendar, not about bars. Over 81
/// months it is 352 trades, and whether that is 0.4% or 0.03% of the bars
/// depends entirely on the rung: the same cadence is 3,656 ppm on 15-minute
/// bars and 244 ppm on 1-minute ones, a fifteen-fold spread from one number.
///
/// So the floor is DERIVED, at the rung's own bar count, from a cadence the
/// operator can state. Nothing here is a constant anybody picked.
///
/// # Why this matters more than any other threshold in the crate
///
/// Measured, on the 81-month NIFTY span at 15 minutes: a support of 20,000 ppm
/// -- the value a real run was launched with -- admits only combinations firing
/// on 2% of bars or more, which is 1,926 trades, better than five a week. Every
/// cadence an operator would call *rare* sits BELOW it and is pruned before it
/// is ever priced. The engine was not failing to find rare winners; it was
/// never allowed to look at one.
///
/// Returns `None` when the span holds no bars, because a support over zero bars
/// is a division and not an answer.
#[must_use]
pub fn cadence_floor_ppm(bars: u64, months: u64, trades_per_week: u64) -> Option<u64> {
    if bars == 0 || months == 0 || trades_per_week == 0 {
        return None;
    }
    let weeks = months.saturating_mul(WEEKS_PER_MONTH_CENTI) / 100;
    let trades = weeks.saturating_mul(trades_per_week).max(1);
    // Multiply BEFORE dividing: `trades / bars` is zero for every cadence that
    // matters, and scaling a zero is still zero. At 1-minute resolution the
    // honest answer is 244 ppm and the naive order returns 0 ppm, which would
    // read as "no floor at all" and sweep everything.
    Some((trades.saturating_mul(1_000_000) / bars).max(1))
}

/// Months between two (year, month) points, inclusive of both.
#[must_use]
pub fn months_between(from: (u16, u8), to: (u16, u8)) -> u64 {
    let a = u64::from(from.0) * 12 + u64::from(from.1);
    let b = u64::from(to.0) * 12 + u64::from(to.1);
    b.saturating_sub(a).saturating_add(1)
}

/// The support ladder a descent walks, ceiling first, floor last.
///
/// # Why geometric, and why the floor is always ON it
///
/// Combination count grows by roughly an order of magnitude per halving --
/// measured on one month of NIFTY 15-minute bars: 2,272 at 43.6% support,
/// 77,384 at 21.8%, 1,442,215 at 10.9%, 15,372,419 at 5.4%. A linear ladder
/// would spend every one of its steps in the cheap region and then fall off a
/// cliff; a geometric one spends equal COST per step, which is the only spacing
/// that makes an incremental report useful.
///
/// The floor is appended explicitly rather than reached by division, because
/// halving from an arbitrary ceiling lands near it and not on it -- and the
/// floor is the whole point of the walk. Landing at 1.06× the operator's
/// cadence would prune exactly what he asked for and report a completed
/// descent, which is the failure wearing a success's clothes §4 bans.
#[must_use]
pub fn support_ladder(ceiling_ppm: u64, floor_ppm: u64) -> Vec<u64> {
    if floor_ppm == 0 || ceiling_ppm <= floor_ppm {
        return vec![floor_ppm.max(1)];
    }
    let mut rungs = Vec::new();
    let mut at = ceiling_ppm;
    while at > floor_ppm {
        rungs.push(at);
        at /= 2;
    }
    rungs.push(floor_ppm);
    rungs
}

/// Where a descent starts when nobody names a ceiling.
///
/// 200,000 ppm — 20% of the rung's own bars. The same figure
/// `crates/api/src/sweeprun.rs` hardcodes for a browser-started sweep, chosen
/// here for the opposite reason: not because it is a good threshold, but because
/// it is a CHEAP one. It is the top of a ladder that halves its way down, so the
/// first step costs almost nothing and every step after it is comparable with
/// the last.
///
/// A ceiling is not a policy about the market. It is where the walk begins.
const DESCENT_CEILING_PPM: u64 = 200_000;

/// The cadence a self-tuning search descends toward.
///
/// One trade a week. The operator's stated target is a RARE setup — *"not
/// thousands of trades, the one and only"* — and one a week over a decade is
/// roughly five hundred round trips, which is a sample a statistic can stand on
/// while still being rare enough to be worth hunting.
///
/// It is the FLOOR, not the answer: the walk stops the moment a support yields
/// an admitted row, which is usually far above it.
/// How far `trades_needed_for` searches before returning its cap.
///
/// Five thousand round trips. A pair of thresholds that cannot be satisfied at
/// five thousand cannot be satisfied at all -- the Wilson bound approaches the
/// observed rate from BELOW and never reaches it, so demanding a bound equal to
/// the rate is unsatisfiable at every sample size. The cap makes that terminate
/// instead of looping.
const TRADES_SEARCH_CEILING: u64 = 5_000;

/// `elite`, with the threshold WALKED instead of typed.
///
/// # Why this command takes no support at all
///
/// `CLAUDE.md` §6 refuses a depth parameter and gives the reason: *"A parameter
/// that can be set can be set wrongly and silently."* Support is the same
/// parameter and was left settable, which is worse — because support is the one
/// number that decides whether a rare combination is ALLOWED TO EXIST.
///
/// Whatever figure an operator types, they have already decided how often the
/// answer may fire, before anything is measured. Type it high and the rare setup
/// is pruned in the first level of the ladder. Type it low and the frontier
/// explodes past the candidate ceiling and the run returns a partial answer.
/// **There is no correct value to type**, which is precisely §6's argument.
///
/// Measured, on this operator's own store: three recorded runs used 400,000 ppm
/// and the browser hardcodes 200,000. A setup firing forty times in ten years of
/// one-minute bars is **42 ppm**. The gem was not ranked badly; it was never
/// counted.
///
/// # It stops at the first support that yields something
///
/// [`support_ladder`] halves from [`DESCENT_CEILING_PPM`] down to the floor
/// [`cadence_floor_ppm`] gives for [`DESCENT_CADENCE_PER_WEEK`]. Each step is a
/// full `elite` screen, and the walk ENDS at the first one that admits a row.
///
/// Cheap answers arrive first, each is comparable with the last, and the
/// expensive end is only reached if the cheap end had nothing — which is the
/// incremental shape [`descend`]'s own doc argues for and the reason a single
/// run at the floor is the wrong thing to do.
///
/// # Callers outside this crate should take [`elite_descend_in_points`]
///
/// This function takes `max_mae_ppm`, and parts per million of WHAT is the
/// question a caller cannot answer without opening the span. See that function
/// for the conversion and for why getting it wrong has already cost this
/// workspace a shipped stop ladder that was a hundred times too tight.
#[must_use]
pub fn elite_descend(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    max_mae_ppm: i64,
    top: usize,
) -> String {
    let Some(known) = EVERY_RUNG.iter().find(|r| **r == rung) else {
        return format!(
            "refused: `{rung}` is not a rung this engine sweeps. The eight are: \
             {}.\n",
            EVERY_RUNG.join(", ")
        );
    };

    // THE BAR COUNT IS READ, NOT SWEPT.
    //
    // This ran a whole `one_rung` at the ceiling first, purely to learn how many
    // bars the span holds — and `one_rung` is a full validated audit. MEASURED:
    // that seed alone did not finish in 110 seconds on the CHEAPEST rung, so the
    // descent never printed its first step. The walk was never reached.
    //
    // A support is a fraction of a bar count, and a bar count comes from opening
    // the span. Nothing about the floor needs a sweep, a trade or a statistic.
    let root = match store_root() {
        Ok(root) => root,
        Err(why) => return format!("refused: {why}\n"),
    };
    let vendor = match parse_vendor(vendor_word) {
        Ok(vendor) => vendor,
        Err(why) => return format!("refused: {why}\n"),
    };
    let span = match stored::load_span(&root, vendor, underlying, known, from, to) {
        Ok(span) => span,
        Err(why) => {
            return format!(
                "refused before the walk began, so no floor could be derived: \
                 {why}\n"
            );
        }
    };
    let bar_count = u64::try_from(span.bars.len()).unwrap_or(u64::MAX);
    // Dropped immediately: every step below loads its own bars, and holding a
    // second copy of a multi-year span for the length of the walk is memory
    // nothing reads.
    drop(span);
    // THE FLOOR IS DERIVED FROM WHAT THE STATISTICS CAN SUPPORT, not from a
    // cadence somebody typed.
    //
    // This was `cadence_floor_ppm(bars, months, 1 trade/week)`. Over 121 months
    // of one-minute bars that is 561 ppm — about 526 round trips. An operator
    // hunting a setup that fires FORTY times is asking for 42 ppm, so the whole
    // 42..561 ppm band was never searched: the mask was never built, never
    // counted, never ranked. Not the prune discarding it — the threshold the
    // prune was applied under.
    //
    // `trades_needed_for` answers the honest question instead: below how many
    // round trips can the stated win rate no longer clear its own confidence
    // bound? Below that, NO combination can pass however good it is, so
    // descending further buys nothing — and stopping anywhere above it discards
    // reachable answers.
    //
    // Measured on the shipped Wilson bound: 80% against a coin-flip floor needs
    // **four** round trips. The old floor demanded 526.
    if bar_count == 0 {
        return "refused: this span has no bars, so a support floor is a division \
                by zero rather than a threshold.\n"
            .to_owned();
    }
    let rules = Rules::elite(max_mae_ppm, top);
    let floor = statistical_floor_ppm(&rules, bar_count);

    let ladder = support_ladder(DESCENT_CEILING_PPM, floor);
    let policy = Policy {
        rules,
        lens: runner::rank::Lens::Payoff,
        // NO VALIDATION PER STEP. Ten supports x sixteen thousand trade re-walks
        // is what made this walk impossible to finish; the step only needs to
        // know whether anything cleared the rules. The survivor is re-run with
        // the full stack below.
        validate: false,
    };

    let mut out = descent_banner(
        vendor_word,
        underlying,
        known,
        (from, to),
        bar_count,
        floor,
        ladder.len(),
    );

    let mut last_page: Option<String> = None;
    for (step, support) in ladder.iter().enumerate() {
        // PROGRESS TO STDERR as each step lands, for the reason `descend`
        // records: a walk that buffers its whole output prints nothing for
        // minutes and an operator cannot tell a long run from a wedged one.
        eprintln!(
            "  [{}/{}] elite at {support} ppm",
            step.saturating_add(1),
            ladder.len()
        );
        let page = screen_range(vendor_word, underlying, known, from, to, *support, policy);

        // ADMITTED ROWS ARE READ BACK OFF THE RENDERED PAGE, which is the
        // pattern `results_arm` already takes and states the reason for: a
        // second decision path can disagree with the one an operator actually
        // sees, and then the walk stops somewhere the page does not explain.
        // THE UNAMBIGUOUS MARKER, not the substring `PASS`. `NOTHING PASSED`
        // contains `PASS`, so the old test matched on a page where nothing
        // passed -- the walk stopped on step one of ten, every time, and the
        // command never descended at all.
        if page.contains(YOUR_RULES_MET) {
            let _ = writeln!(
                out,
                "\n  STOPPED at {support} ppm, step {} of {} — the first support \
                 that admitted a row.\n  Re-running this support WITH the full \
                 validation stack — walk-forward, PBO and the bootstrap — \
                 because\n  a search result nobody validated is a candidate, not \
                 a finding.\n",
                step.saturating_add(1),
                ladder.len()
            );
            // THE SURVIVOR IS VALIDATED, and only the survivor — see
            // `validated_at` for why the search half cannot afford this stack.
            out.push_str(&validated_at(
                vendor_word,
                underlying,
                known,
                (from, to),
                *support,
                policy,
                &page,
            ));
            return out;
        }
        let _ = writeln!(
            out,
            "  {support:>9} ppm — nothing admitted{}",
            if page.starts_with("refused: ") {
                format!(" ({})", page.lines().next().unwrap_or_default())
            } else {
                String::new()
            }
        );
        // KEPT ONLY IF IT IS A REPORT. A refusal carries no rows and no tier
        // ladder, so keeping one would replace "what is there" with "why this
        // step could not run", which is a different answer.
        if !page.starts_with("refused") {
            last_page = Some(page);
        }
    }

    exhausted_walk(&mut out, floor, last_page.as_deref());
    out
}

/// [`elite_descend`], with the stop ceiling stated in INDEX POINTS.
///
/// # The entry point a caller outside this crate should take
///
/// `elite_descend` takes `max_mae_ppm`, and *parts per million of what* is a
/// question nobody can answer without opening the span. An operator states a
/// stop the way a stop is spoken — *"twenty points"* — and the conversion is a
/// fact about the bars, not about the request.
///
/// # Why this is not one line in the caller
///
/// Because the obvious one line is wrong, and it has already been wrong here.
/// [`points_to_ppm`] converts against `NIFTY_REFERENCE`, a stated
/// approximation, and [`reference_price`]'s own doc records what that costs:
/// *"800 ppm on a 52,000 index is FORTY-ONE points"* — so a NIFTY constant
/// applied to BANKNIFTY does not approximate the operator's rule, **it doubles
/// it**, and understates it in the other direction on a 2020 low. The same
/// family of unit slip once shipped a stop ladder at 2..10 ppm where the
/// documentation said 200..1000: every stop the exit grid priced sat inside the
/// entry bar's own range.
///
/// So the reference is READ OFF THE BARS THAT WILL BE SWEPT — the midpoint of
/// the loaded span's range, in paisa — and a caller in another crate cannot get
/// it wrong because it never sees a ppm at all.
///
/// # Cost
///
/// One extra span load, and it is deliberate. `elite_descend` opens the span
/// again for its own bar count; sharing one load means threading the bars
/// through a signature that four other callers already use, which is a change
/// to their contract for the benefit of this one. A span load is bounded by the
/// months asked for and happens once per RUN, never per bar — `CLAUDE.md` §3
/// rule 4 bounds per-operation cost, and this is not an operation on the sweep
/// path.
///
/// # Errors
///
/// Returns the same `refused: …` text every other command in this module
/// returns, so a caller tests one prefix. The store root, the feed word, the
/// rung and the span are all validated here before anything is swept.
#[must_use]
pub fn elite_descend_in_points(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    max_points: i64,
    top: usize,
) -> String {
    if max_points <= 0 {
        return "refused: the stop ceiling must be a whole number of index \
                points, 1 or more. A ceiling of zero admits no trade and a \
                negative one is not a distance.\n"
            .to_owned();
    }
    if top == 0 {
        return "refused: TOP must be 1 or more — a listing of zero rows is not \
                a shorter answer, it is no answer.\n"
            .to_owned();
    }
    let root = match store_root() {
        Ok(root) => root,
        Err(why) => return format!("refused: {why}\n"),
    };
    let vendor = match parse_vendor(vendor_word) {
        Ok(vendor) => vendor,
        Err(why) => return format!("refused: {why}\n"),
    };
    // THE REFERENCE IS READ FROM THE BARS THIS RUN WILL SWEEP, not from a
    // constant and not from a different rung. A span that refuses here refuses
    // before any threshold is derived, which is the honest order: a floor
    // computed from bars nobody could load is arithmetic on an assumption.
    let span = match stored::load_span(&root, vendor, underlying, rung, from, to) {
        Ok(span) => span,
        Err(why) => {
            return format!(
                "refused before the ceiling could be converted, so no descent \
                 began: {why}\n"
            );
        }
    };
    let reference = reference_price(&span.bars);
    // Dropped before the walk: `elite_descend` loads its own, and holding a
    // second copy of a multi-year span for the length of a descent is memory
    // nothing reads. Same reasoning `elite_descend` states for its own seed.
    drop(span);
    let max_mae_ppm = points_to_ppm_at(max_points, reference);
    if max_mae_ppm <= 0 {
        return format!(
            "refused: {max_points} point(s) against a reference of {reference} \
             paisa converts to {max_mae_ppm} ppm, which admits nothing. This is \
             the unit slip `reference_price` documents, caught rather than \
             swept with.\n"
        );
    }
    elite_descend(vendor_word, underlying, rung, from, to, max_mae_ppm, top)
}

/// [`screen_range`] with the stop ceiling in POINTS and the policy built here.
///
/// # Why a caller outside this crate cannot build the policy itself
///
/// [`Policy`] carries a [`Rules`], a `runner::rank::Lens` and a `validate`
/// flag. `Rules::elite` is private, the lens is a `runner` type `api` does not
/// depend on, and the ceiling is a ppm — which is the unit
/// [`elite_descend_in_points`] exists to keep out of other crates. Exposing
/// three types to let a caller assemble one struct would widen this crate's
/// surface to hand out a shape only this crate knows how to fill.
///
/// So the caller states the span, the support and its own risk in points, and
/// this builds the policy the same way the `screen` argv arm does — one
/// construction, one place to correct.
///
/// # Errors
///
/// The same `refused: …` text every other command here returns.
#[must_use]
pub fn screen_range_in_points(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    // ONE SPAN, ONE ARGUMENT. The two halves are never meaningful apart and the
    // pair is how `range_all_arm` and `descend`'s own dispatch already carry
    // them; splitting them here would also put this function one over the
    // workspace's argument bound, which is the lint noticing the same thing.
    span: ((u16, u8), (u16, u8)),
    support_ppm: u64,
    max_points: i64,
    top: usize,
) -> String {
    let (from, to) = span;
    if support_ppm == 0 {
        return "refused: a support of zero makes every combination frequent, \
                so the frequent frontier never empties and the walk has no \
                end.\n"
            .to_owned();
    }
    if max_points <= 0 || top == 0 {
        return "refused: the stop ceiling must be 1 index point or more and \
                TOP must be 1 row or more.\n"
            .to_owned();
    }
    let root = match store_root() {
        Ok(root) => root,
        Err(why) => return format!("refused: {why}\n"),
    };
    let vendor = match parse_vendor(vendor_word) {
        Ok(vendor) => vendor,
        Err(why) => return format!("refused: {why}\n"),
    };
    // SAME CONVERSION AS THE DESCENT, AND FOR THE SAME REASON. See
    // `elite_descend_in_points`: a points ceiling converted against a constant
    // is doubled on BANKNIFTY and halved on a 2020 low.
    let span = match stored::load_span(&root, vendor, underlying, rung, from, to) {
        Ok(span) => span,
        Err(why) => {
            return format!(
                "refused before the ceiling could be converted, so nothing was \
                 screened: {why}\n"
            );
        }
    };
    let reference = reference_price(&span.bars);
    drop(span);
    let max_mae_ppm = points_to_ppm_at(max_points, reference);
    if max_mae_ppm <= 0 {
        return format!(
            "refused: {max_points} point(s) against a reference of {reference} \
             paisa converts to {max_mae_ppm} ppm, which admits nothing.\n"
        );
    }
    let policy = Policy {
        rules: Rules::elite(max_mae_ppm, top),
        lens: runner::rank::Lens::Payoff,
        validate: validate_from_env(),
    };
    screen_range(vendor_word, underlying, rung, from, to, support_ppm, policy)
}

/// The tail of a descent that admitted nothing at any support.
///
/// Lifted out of [`elite_descend`] for its line budget. "No combination met
/// your standard" is true and useless on its own, so the walk's LAST and most
/// permissive page is kept: every row in it still carries the rule it failed,
/// and the tier ladder below it names the strictest standard anything did meet.
///
/// That is a near-miss report, not a lowered bar. Nothing is re-admitted.
fn exhausted_walk(out: &mut String, floor: u64, last_page: Option<&str>) {
    let _ = writeln!(
        out,
        "\n  NOTHING PASSED AT ANY SUPPORT, down to {floor} ppm — the fewest \
         round trips these rules can be satisfied by.\n  That is a statement \
         about this instrument, this rung and this span under THESE rules, and \
         not\n  about the market: a looser MAX_POINTS, a different rung or a \
         longer span are separate questions.\n\n  What follows is the walk's LAST \
         and most permissive step, kept so the answer is what IS there\n  rather \
         than a blank page. Every row still carries the rule it failed, and the \
         TIER\n  LADDER below it names the strictest standard anything did meet."
    );
    if let Some(page) = last_page {
        out.push('\n');
        out.push_str(page);
    }
}

/// Sweep ONE rung at successively lower supports, until the operator's cadence.
///
/// # The command that exists because a threshold was hiding the answer
///
/// `range-all` takes one `SUPPORT_PPM` and sweeps every rung at it. That is the
/// right shape for comparing rungs and the wrong shape for finding a RARE
/// combination, because the support is exactly what decides whether a rare
/// combination is allowed to exist. A run launched at 20,000 ppm cannot report
/// a once-a-week setup no matter how long it runs -- the setup was pruned in
/// the first level of the ladder, and the report says nothing about it because
/// nothing counted it.
///
/// So this walks the threshold instead of fixing it. The ceiling is where the
/// operator starts, the floor is [`cadence_floor_ppm`] for the cadence he
/// actually wants, and every step in between is reported as it completes.
///
/// # Why incremental rather than one run at the floor
///
/// The floor is the expensive end -- combination count grows by roughly an
/// order of magnitude per halving -- so a single run at the floor gives an
/// operator nothing at all until it finishes, and no way to tell a long run
/// from a stuck one. Walking down means the cheap answers arrive first, each
/// one is comparable with the last, and the walk can be stopped the moment a
/// row is good enough. That is the "dynamic incremental scalable" shape the
/// operator asked for, and it is also the only shape that lets him see the
/// combination count explode BEFORE he commits a machine to it.
///
/// Progress goes to STDERR as each step lands; the report is the returned
/// string, on stdout. A descent buffering its whole walk printed nothing for
/// 280 seconds when this was first run, which is the one thing it must not do.
///
/// # What each row proves
///
/// `trades` falling toward the cadence and `all_mae` falling toward the stop
/// ceiling is the signature being hunted. A row where `trades` stays in the
/// thousands is a grinder however good its total looks, and the descent prints
/// both so the two cannot be confused.
#[must_use]
pub fn descend(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    ceiling_ppm: u64,
    per_week: u64,
) -> String {
    let Some(known) = EVERY_RUNG.iter().find(|r| **r == rung) else {
        return format!(
            "refused: `{rung}` is not a rung this engine sweeps. The eight are: \
             {}.\n",
            EVERY_RUNG.join(", ")
        );
    };
    let months = months_between(from, to);
    let mut out = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "feed {vendor_word} · {underlying} · {known} · {}-{:02}..{}-{:02} · \
         {months} months · descending to {per_week} trade(s) per week",
        from.0, from.1, to.0, to.1
    );

    // THE FIRST STEP IS ALSO THE MEASUREMENT THE FLOOR NEEDS.
    //
    // The floor is a support, and a support is a fraction of a bar count that
    // nobody knows until the span is opened. Guessing it from the rung's name
    // would be arithmetic on an assumption -- §3 rule 1's `UNVERIFIED` -- so the
    // ceiling runs first and its `bars` is what the floor is derived from.
    let first = one_rung(vendor_word, underlying, known, from, to, Some(ceiling_ppm));
    let Ok(ref seed) = first.outcome else {
        let why = first
            .outcome
            .as_ref()
            .err()
            .map_or_else(|| "no reason given".to_owned(), Clone::clone);
        let _ = writeln!(
            out,
            "\nrefused at the ceiling, so no floor could be \
                               derived and nothing was walked.\n  {why}"
        );
        return out;
    };
    let Some(floor) = cadence_floor_ppm(seed.bars, months, per_week) else {
        let _ = writeln!(
            out,
            "\nrefused: the span reported {} bars over {months} months, so a \
             cadence floor is a division by zero rather than a threshold.",
            seed.bars
        );
        return out;
    };

    let ladder = support_ladder(ceiling_ppm, floor);
    let _ = writeln!(
        out,
        "  {} bars · one trade per week is {} of them · floor {floor} ppm · \
         {} step(s)\n",
        seed.bars,
        seed.bars.saturating_mul(floor) / 1_000_000,
        ladder.len()
    );
    let _ = writeln!(
        out,
        "  {:<10}{:>10}{:>14}{:>7}{:>10}{:>9}{:>12}{:>12}{:>14}",
        "support",
        "min_hits",
        "combinations",
        "depth",
        "complete",
        "trades",
        "worst_mae",
        "all_mae",
        "worst"
    );

    // SEQUENTIAL, and deliberately so.
    //
    // `range_all` runs its nine rungs in parallel because they are independent.
    // These are not independent in the way that matters: each step is roughly an
    // order of magnitude more expensive than the last, so running them at once
    // would hold the cheap answers hostage to the expensive one -- which is the
    // exact failure this command exists to avoid. One at a time, printed as it
    // lands.
    for (step, &support) in ladder.iter().enumerate() {
        let row = if step == 0 {
            // Already swept, above. Re-sweeping it would be a second identical
            // run that §3 rule 5 says produces identical bytes -- pure waste.
            first.outcome.clone()
        } else {
            one_rung(vendor_word, underlying, known, from, to, Some(support)).outcome
        };
        let line = match row {
            Err(why) => format!(
                "  {:<10}REFUSED: {}",
                format!("{support}ppm"),
                why.lines().next().unwrap_or(&why)
            ),
            Ok(r) => format!(
                "  {:<10}{:>10}{:>14}{:>7}{:>10}{:>9}{:>12}{:>12}{:>14}",
                format!("{support}ppm"),
                r.min_hits,
                r.combinations,
                r.depth,
                if r.halted == 0 { "yes" } else { "NO" },
                r.trades,
                r.winner_mae,
                r.all_mae,
                r.pessimistic,
            ),
        };
        // EACH ROW IS ANNOUNCED THE MOMENT IT LANDS, ON STDERR.
        //
        // # The defect this closes, found by running the command
        //
        // Every command in this crate builds a `String` and hands it to the
        // caller to print, which is right for a command that finishes. This one
        // does not: the last step of a descent is roughly an order of magnitude
        // dearer than the first, so a walk that buffers prints NOTHING for the
        // whole run and then everything at once. Measured -- a three-step
        // descent produced a zero-byte file after 280 seconds.
        //
        // That is worse than slow. An operator cannot tell a descent that is
        // working from one that is wedged, which is exactly the "cheap answers
        // first" property this command was built for, and the doc comment above
        // claimed it while the code did not do it.
        //
        // Stderr and not stdout, so the REPORT stays a single clean artifact on
        // stdout that a pipe or a file capture sees whole. Progress is for the
        // human watching; the report is for whatever reads the output.
        eprintln!("  [{}/{}] {line}", step + 1, ladder.len());
        let _ = writeln!(out, "{line}");
    }

    let _ = writeln!(
        out,
        "\n  `trades` is the whole point: a row in the thousands is a grinder \
         whatever its total reads.\n  \
         `all_mae` is every trade's adverse excursion in ppm -- the tightest \
         stop that would have held ALL of them.\n  \
         Each step is roughly an order of magnitude more expensive than the one \
         above it. A `complete` of NO\n  means a budget stopped that step's \
         ladder short and its combination count is not comparable."
    );
    out
}

/// A ledger row's return-over-drawdown, rendered for the `ret/DD` column.
///
/// # THIS COLUMN COULD NEVER PRODUCE A VALUE, AND THE TESTS WERE WHY
///
/// It was written inline as `if r.max_drawdown < 0 && r.pessimistic > 0`, then
/// divided by `-r.max_drawdown`. [`grid::Cell::max_drawdown`] is a peak-to-trough
/// FALL, documented "Always >= 0" and asserted non-negative in `grid`'s own
/// invariants, and `record_run` copies that value straight into the record. So
/// the guard was false for every real run and the column printed `-` on all
/// eight rungs of every sweep — the one column that answers *what did this rung
/// risk per unit of return*, which is the operator's stated question and the
/// reason the exit grid exists.
///
/// Nothing caught it because **four fixtures in this crate wrote the drawdown
/// negative**, so the suite covered a branch production could not reach. Those
/// fixtures now carry the engine's sign.
///
/// # It delegates rather than re-deriving
///
/// The arithmetic lives on [`grid::Cell::return_over_drawdown`] and was copied
/// here by hand, which is how the two came to disagree about a sign in the first
/// place. A `Cell` is `Default`, so the two money fields are set on one and the
/// real method answers. There is now exactly one implementation, and
/// `the_ledger_ratio_is_the_cell_ratio` fails if this ever grows a second.
fn return_over_drawdown_cell(pessimistic: i64, max_drawdown: i64) -> String {
    let cell = grid::Cell {
        pessimistic,
        max_drawdown,
        ..grid::Cell::default()
    };
    match cell.return_over_drawdown() {
        // A variant that never gave anything back. Unbounded, so it is named
        // rather than printed as a number no divisor produced.
        i64::MAX => "inf".to_owned(),
        // No profit to divide. `-` is the honest answer, not a zero.
        0 => "-".to_owned(),
        // HUNDREDTHS, RENDERED AS A DECIMAL, and the raw integer was a second
        // way to misread this column. `return_over_drawdown` is `x100` by the
        // same convention `win_rate_bp` and `profit_factor_bp` use, so a run
        // that made 84.32 times its worst fall returned `8432` — which reads as
        // eight thousand, not as eighty-four times. The `PF` column beside it
        // has always gone through `hundredths_of`; this one now does too, so
        // two ratios in one table are in one format.
        ratio => hundredths_of(ratio),
    }
}

/// Sweeps a span on EVERY rung and prints one table comparing them.
///
/// # Why this is one command and not nine invocations
///
/// Nine invocations produce nine reports of eight hundred lines each, and the
/// question an operator actually has — *which timeframe carries the edge?* — is
/// answerable only by reading all nine and lining them up by hand. The figures
/// are already comparable: every rung executes on the SAME one-minute series, so
/// the horizon means minutes on all of them and the fills come off the same
/// bars. What was missing was somewhere to put them side by side.
///
/// Every rung is also RECORDED, so the table is a view of the results store
/// rather than a thing computed and thrown away. A later run can be compared
/// against this one without re-sweeping.
///
/// A rung that refuses does NOT stop the others. A span missing one timeframe
/// is a smaller answer, not no answer — and the refusal is printed in that
/// rung's own row rather than as a footnote, so a reader cannot mistake an
/// absent row for a poor result.
///
/// Every rung, which is [`EVERY_RUNG`]. For a chosen subset see
/// [`range_over`], which this delegates to.
#[must_use]
pub fn range_all(
    vendor_word: &str,
    underlying: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> String {
    range_over(vendor_word, underlying, &EVERY_RUNG, from, to, support_ppm)
}

/// [`range_all`] over a CHOSEN subset of rungs.
///
/// # Why the subset exists
///
/// `range_all` is all-eight-or-nothing, and `/backtest` grew an instrument and
/// a timeframe menu that could express a choice no route could carry: the page
/// sent `rungs` and said, in words under the button, that the selection was not
/// acted on yet. A control whose label and behaviour disagree is what
/// `CLAUDE.md` §4 bans, and the honest sentence was a placeholder for this.
///
/// The eight are still the only rungs that may be asked for. `1day` is on disk
/// to feed `indicators::daily` — the pivot ladder and yesterday's high and low —
/// and is not a signal timeframe; an unknown rung is refused by name rather than
/// silently dropped, because a sweep that quietly covers less than it was asked
/// for is the same failure in a different coat.
///
/// An EMPTY subset is a refusal, not a no-op that prints an empty table.
///
/// # What it does not change
///
/// Determinism (§3 rule 5) is held by the same shape `range_all` relies on:
/// `map` over an indexed parallel iterator preserves order, so the rows come
/// out in the caller's rung order however the threads finish. Each rung still
/// records its own run identity, so a subset sweep is indistinguishable in the
/// ledger from the same rungs swept one at a time.
#[must_use]
pub fn range_over(
    vendor_word: &str,
    underlying: &str,
    rungs: &[&'static str],
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> String {
    if rungs.is_empty() {
        return "refused: no rung was asked for. A sweep over no timeframe is not \
                a sweep, and an empty table would report that as a result.\n"
            .to_owned();
    }
    if let Some(bad) = rungs.iter().find(|r| !EVERY_RUNG.contains(r)) {
        return format!(
            "refused: `{bad}` is not a rung this engine sweeps. The eight are: \
             {}.\n",
            EVERY_RUNG.join(", ")
        );
    }
    let mut out = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "feed {vendor_word} · {underlying} · {} · {}-{:02}..{}-{:02} · support {}",
        rungs_word(rungs),
        from.0,
        from.1,
        to.0,
        to.1,
        support_word(support_ppm)
    );
    let _ = writeln!(
        out,
        "SUPPORT, not a hit count: each rung's `min_hits` is derived from that \
         rung's OWN bar count.\n\
         81 months holds 1,671 daily bars and 623,546 one-minute ones -- a \
         373-fold spread -- so one absolute\nthreshold would ask nine different \
         questions and the table would compare nothing.\n\
         Every rung executes on the SAME 1min series, so the horizon means \
         MINUTES on all of them\nand the fills come off the same bars.\n"
    );

    // NINE RUNGS AT ONCE, ON A MACHINE WITH TEN PERFORMANCE CORES.
    //
    // Each rung is wholly independent: its own span load, its own evaluator, its
    // own ladder, its own run identity. Nothing is shared and the only write is
    // the ledger, which `LEDGER` serialises. This was a `for` loop on one core
    // while `rayon` was already in this crate's manifest for `batch::sweep_all`.
    //
    // MEASURED before this changed: 0.20 GB resident per rung at 20% support, so
    // nine at once is under 2 GB of 48. Memory is not what bounds this; the
    // 1-minute rung's 623,546 bars are.
    //
    // **THAT MEASUREMENT EXPIRED AND THE SENTENCE ABOVE IS KEPT TO SHOW HOW.**
    // It was taken at 20% support. D-0303 then deleted the 20% and gave each
    // rung a floor derived from its own bars, which on the operator's
    // 2026-08-28 run resolved to 0.0045% -- four orders of magnitude lower, and
    // candidate counts explode as support falls. Measured on that run: 13.8 GB
    // resident, not 2. The licence for running eight at once was withdrawn by a
    // change in another file and nobody re-took the reading.
    //
    // `SharedBy` is the arithmetic that makes the two agree: the ceiling is a
    // MACHINE budget, so it is divided among the rungs actually in flight
    // instead of being handed to each of them whole. Held across the map and
    // dropped after it, so a panic inside cannot leave the divisor raised for
    // the next request on a long-lived server.
    let _sharing = SharedBy::these(rungs.len());
    //
    // DETERMINISM (§3 rule 5) IS HELD BY SHAPE. `map` on an INDEXED parallel
    // iterator preserves order, so `rows` is the same sequence whatever order
    // the threads finish in -- the same argument `batch::sweep_under` makes, and
    // for the same reason. A rerun produces the same bytes.
    let rows: Vec<RungRow> = rungs
        .par_iter()
        .map(|&rung| one_rung(vendor_word, underlying, rung, from, to, support_ppm))
        .collect();
    // EVERY RUNG REFUSED IS A REFUSAL, NOT A REPORT.
    //
    // MEASURED, by attacking this command: `range-all nosuchfeed NIFTY ...`,
    // a backwards range, month 0, month 13, an empty symbol, a unicode symbol
    // and `../../etc` ALL printed the STORED_PROVENANCE banner -- "THESE BARS
    // ARE REAL MARKET DATA, READ FROM THE STORE" -- and exited ZERO. Nine rows
    // each said REFUSED underneath, which is honest, but the banner above them
    // claimed real bars had been read and the exit code told a script the run
    // succeeded.
    //
    // That is precisely the failure wearing a success's clothes `CLAUDE.md` §4
    // bans, and the provenance banner is the one line in this workspace that
    // must never be printed over nothing -- `cli`'s own header says the two
    // banners are "the only thing separating" a real sweep from a generated one.
    //
    // So: if no rung produced a row, the whole command refuses, with the first
    // rung's reason. They are all the same reason when the argument is what is
    // wrong, and when they differ the table below still prints every one.
    if rows.iter().all(|r| r.outcome.is_err()) {
        let why = rows
            .first()
            .and_then(|r| r.outcome.as_ref().err())
            .map_or_else(|| "no rung produced a row".to_owned(), Clone::clone);
        return format!(
            "refused: every one of the {} rungs refused. Nothing was read and no \
             row was recorded.\n  first reason: {why}\n",
            rows.len()
        );
    }

    let _ = writeln!(
        out,
        "  {:<8}{:>10}{:>10}{:>8}{:>14}{:>7}{:>10}{:>9}{:>14}{:>14}{:>10}",
        "rung",
        "bars",
        "min_hits",
        "months",
        "combinations",
        "depth",
        "complete",
        "trades",
        "worst",
        "best",
        "ret/DD"
    );
    for row in &rows {
        match &row.outcome {
            Err(why) => {
                let _ = writeln!(out, "  {:<8}REFUSED: {why}", row.rung);
            }
            Ok(r) => {
                let _ = writeln!(
                    out,
                    "  {:<8}{:>10}{:>10}{:>8}{:>14}{:>7}{:>10}{:>9}{:>14}{:>14}{:>10}",
                    row.rung,
                    r.bars,
                    r.min_hits,
                    format!("{}/{}", r.months_found, r.months_asked),
                    r.combinations,
                    r.depth,
                    // THE ONE COLUMN THAT DECIDES WHETHER THE REST MEAN
                    // ANYTHING. A halted sweep's depth is PARTIAL and its
                    // combination count covers less of the ladder than it looks
                    // like, so a `no` here is not a footnote.
                    if r.halted == 0 { "yes" } else { "NO" },
                    r.trades,
                    r.pessimistic,
                    r.optimistic,
                    // A PEAK-TO-TROUGH FALL IS NON-NEGATIVE, AND THIS TESTED
                    // THE OTHER SIGN.
                    //
                    // `grid::Cell::max_drawdown` is documented "Always >= 0"
                    // and asserted so; `record_run` copies that value straight
                    // in. So `r.max_drawdown < 0` was false for every real run
                    // and this column printed "-" on all eight rungs, forever
                    // -- the one column answering "what did this rung risk per
                    // unit of return", structurally dead.
                    //
                    // It survived because FOUR test fixtures in this file wrote
                    // the drawdown NEGATIVE, so the suite exercised a branch
                    // production could not reach. They now carry the engine's
                    // sign, which is what made this visible at all.
                    //
                    return_over_drawdown_cell(r.pessimistic, r.max_drawdown),
                );
            }
        }
    }
    let _ = writeln!(
        out,
        "\n  `worst` and `best` are the CHOSEN exit variant's total in paisa \
         under PRINTED-extreme and open fills -- both are prices the bar\n  actually printed, and NO tick is added to either. Selection ranks on\n  `worst`.\n  \
         `complete` NO means a budget stopped that rung's ladder short: its \
         depth is partial and its combination count is not comparable with a \
         complete one.\n  Every row above is a stored record; nothing here was \
         computed twice."
    );
    out
}

/// The record just written for this exact run, read back from the store.
///
/// Reads BACKWARDS from the newest row and stops at the first match, because the
/// row this command just appended is the last one. That is `O(1)` in the ordinary
/// case and `O(rows)` only if the run was somehow not recorded — which is
/// reported as the refusal it is rather than absorbed.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn latest_for(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    min_hits: u64,
) -> Result<crate::results::Record, String> {
    let root = store_root()?;
    let mut store = crate::results::Results::open(&root)?;
    let count = store.len()?;
    let (feed, name, tf) = (
        crate::results::field(vendor_word),
        crate::results::field(underlying),
        crate::results::field(rung),
    );
    for back in 1..=count {
        let record = store.read(count.saturating_sub(back))?;
        if record.feed == feed
            && record.underlying == name
            && record.timeframe == tf
            && record.from_year == from.0
            && record.from_month == from.1
            && record.to_year == to.0
            && record.to_month == to.1
            && record.min_hits == min_hits
        {
            return Ok(record);
        }
    }
    Err(format!(
        "the {rung} run completed but no row for it is in the results store"
    ))
}

/// `screen`: sweep a span and report only the combinations that satisfy the
/// operator's own rules.
///
/// # Every number is the operator's
///
/// `MAX_POINTS` is a stop in INDEX POINTS, because that is how a stop is
/// spoken about — *"never more than twenty points against"* — and it is
/// converted to ppm against the span's own mean price rather than against a
/// guessed index level. A rule stated in points on a 25,000 index and applied
/// unchanged to a 52,000 one would be a different rule.
///
/// `MIN_RR` is the worst-case reward-to-risk in hundredths: `200` demands the
/// SMALLEST win be twice the LARGEST loss. Set it to `0` to drop the rule
/// entirely, which is the honest way to ask "show me everything that survives
/// the stop".
///
/// `TOP` is how many to print.
///
/// # Errors
///
/// Every refusal `audit_range` makes, plus a non-numeric or zero rule.
#[must_use]
pub fn screen_range(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: u64,
    policy: Policy,
) -> String {
    match screen_range_inner(vendor_word, underlying, rung, from, to, support_ppm, policy) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// Everything about a screen that is a CHOICE rather than a span.
///
/// # Why the two travel together
///
/// [`Rules`] says which rows are ADMITTED. [`runner::rank::Lens`] says which
/// combinations were ever CONSIDERED. They are one policy in two halves, and
/// keeping them apart is how the second came to be implicit for so long: every
/// command applied rules an operator could read on the page, and a cut nobody
/// named anywhere.
///
/// Grouped rather than passed as two more positional arguments because clippy is
/// right that eight of them is a call site a reader cannot check — and because a
/// policy that is one value can be printed, compared and recorded as one thing.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    /// Which rows the screen admits.
    pub rules: Rules,
    /// Which question decides who survives the `screen_cap` cut.
    pub lens: runner::rank::Lens,
    /// Whether this screen also runs walk-forward, PBO and the bootstrap.
    ///
    /// A DESCENT sets this false on every step and true on nothing: each step
    /// asks only *did anything clear the rules*, which the screen answers. The
    /// validation stack answers *is the winner real*, a question about one
    /// combination, and paying for it ten times over to validate candidates the
    /// next step discards is what made a search impossible to finish.
    pub validate: bool,
}

/// [`screen_range`]'s work, with its refusals unrendered.
fn screen_range_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: u64,
    policy: Policy,
) -> Result<String, stored::Refusal> {
    let Policy {
        rules,
        lens,
        validate,
    } = policy;
    let commit = commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot \
         be recorded and the screen will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;
    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let span = stored::load_span(&root, vendor, underlying, rung, from, to)?;
    let signal_length = stored::rung_length_micros(rung)?;
    let bars = span.bars.len();
    let min_hits = min_hits_for(bars, support_ppm);

    let execution_bars = if rung == EXECUTION_RUNG {
        None
    } else {
        Some(stored::load_span(
            &root,
            vendor,
            underlying,
            EXECUTION_RUNG,
            from,
            to,
        )?)
    };
    let execution = execution_bars.as_ref().map(|exec| Execution {
        bars: &exec.bars,
        signal_length_micros: signal_length,
    });

    let ladder = ladder_for(min_hits)?;
    let id = identity(&Run {
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        direction: RunDirection::Undirected,
        instrument: &span.key,
        timeframe: span.timeframe,
        params: Params::of(ladder).with_policy(&policy_of(&span.bars, rules, lens, validate)),
        data_digest: data_digest(&span.bars),
        commit,
        feed: span.vendor.as_str(),
    });
    let header = span_banner(&span, underlying, from, to, commit);
    Ok(audit_bars(
        evaluator(),
        span.bars,
        &header,
        min_hits,
        Some(&id),
        AuditOptions {
            execution,
            recording: Some(Recording {
                root: &root,
                feed: vendor.as_str(),
                underlying,
                timeframe: span.timeframe,
                from,
                to,
                months_asked: span.asked,
                months_found: span.found,
            }),
            rules,
            lens,
            // The environment's ceiling -- see `AuditOptions::ceiling`.
            ceiling: None,
            validate,
        },
    ))
}

/// [`audit_range_inner`], with every refusal rendered the way the CLI prints one.
#[must_use]
pub fn audit_range(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    min_hits: u64,
) -> String {
    match audit_range_inner(vendor_word, underlying, rung, from, to, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}
/// [`audit_stored_inner`], with every refusal rendered the way the CLI prints one.
#[must_use]
pub fn audit_stored(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> String {
    match audit_stored_inner(vendor_word, underlying, rung, year, month, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// The audit, from an evaluator the caller supplies. Split for the same reason
/// [`sweep_with`] is.
#[allow(
    clippy::needless_pass_by_value,
    clippy::large_types_passed_by_value,
    reason = "the body needs ownership: `Column::build` and `Sweeper::run` both \
              take `&mut`, and the value must outlive them. One 1,744-byte move \
              per CLI invocation, in a function called once per process."
)]
fn audit_with(
    ev: Result<Evaluator, &'static str>,
    sessions: i64,
    min_hits: u64,
    ceiling: Option<usize>,
) -> String {
    // NO RECORDING FOR GENERATED BARS, and that is not an oversight. The
    // results store is a ledger of runs over REAL instrument-months; a row for
    // a synthetic sweep would carry a feed and an instrument it does not have,
    // and `CLAUDE.md` §3 rule 1 forbids inventing either.
    audit_bars(
        ev,
        synthetic::sessions(sessions),
        PROVENANCE,
        min_hits,
        None,
        AuditOptions {
            execution: None,
            recording: None,
            rules: Rules::BASELINE,
            // The historical cut, unchanged. `Payoff` is reached only by an
            // operator who typed `elite`.
            ceiling,
            validate: true,
            lens: runner::rank::Lens::Detectability,
        },
    )
}

/// Everything about an audit that is not the bars themselves.
///
/// # Three parameters that arrived one at a time
///
/// `audit_bars` took the evaluator, the bars, a banner, a threshold and an
/// identity. The one-minute execution layer added a sixth argument, the results
/// ledger a seventh, and the screening rules an eighth — at which point clippy
/// refused it, correctly. Eight positional arguments is a call site where a
/// reader cannot tell which `None` is which.
///
/// Grouping them is not cosmetic: all three are OPTIONS ABOUT THE RUN rather
/// than inputs to it, and every one of them has a meaningful absence. A
/// synthetic sweep has no execution series, no ledger row to write and no feed
/// to name; a stored month has all three.
#[derive(Clone, Copy)]
struct AuditOptions<'a> {
    /// The series positions are opened and closed on. `None` trades on the
    /// series it swept, which is what `cli sweep` and `audit-stored` do.
    execution: Option<Execution<'a>>,
    /// Where to record the run. `None` records nothing, which is right for
    /// generated bars: a row for a synthetic sweep would carry a feed and an
    /// instrument it does not have.
    recording: Option<Recording<'a>>,
    /// The operator's screening rules, always present because the screen always
    /// prints — with the rules it applied above it, so a reader can see which
    /// numbers produced the PASS and FAIL column.
    rules: Rules,
    /// Which question decides who survives the `screen_cap` cut.
    ///
    /// # This is the only place the choice can be made
    ///
    /// `keep` is a HARD boundary. Everything the ranking heap does not admit is
    /// discarded before any rule, tier or report runs, so a caller that wanted a
    /// different question answered cannot ask it downstream — it can only
    /// re-order what detectability already chose. `screen_cap`'s own comment
    /// says so: *"No tier ladder, no rule and no report can recover that. They
    /// all filter cells, and the cells were never computed."*
    ///
    /// [`runner::rank::Lens::Detectability`] is what every command did and still
    /// does. `cli elite` passes [`runner::rank::Lens::Payoff`], because the
    /// setup it hunts — small losers, large winners — has a mean near zero and
    /// is precisely what a `|t|` cut throws away.
    lens: runner::rank::Lens,
    /// A candidate ceiling this run must not exceed, or `None` for the
    /// environment's.
    ///
    /// # Why a caller may need to say, and what happened when none could
    ///
    /// `ladder_for` reads `engine::DEFAULT_CEILING` — `1 << 27`, or 134,217,728
    /// distinct candidates — unless `BRUTEX_CEILING` overrides it. That is the
    /// right budget for an operator sweeping a real instrument-month with hours
    /// to spend. It is the wrong budget for a caller that only needs a sweep to
    /// EXIST, and there was no way to say so.
    ///
    /// **Measured consequence.** `the_audit_renders_every_stage_of_the_institutional_stack`
    /// calls `audit_run(12, 300)` — 6.7% support over 328 live positions — and
    /// inherited the full ceiling in a DEBUG build. It ran for over an hour
    /// before being killed, so `cargo test --workspace` never terminated, so
    /// `CLAUDE.md` §9's green-suite requirement was unverifiable — and a
    /// genuinely red test sat behind it undetected from `a12192b` onward.
    ///
    /// This crate already knew the lesson and had applied it once:
    /// [`SEARCH_CEILING`] is 500,000 with a compile-time assert that it stay at
    /// least a hundredfold under the engine's default, added because *"`cli auto
    /// 250` then produced no output in twenty minutes."* It was given to
    /// `Sweeper::auto` and to nothing else.
    ///
    /// `None` is what every operator-facing command passes, so no command's
    /// search narrows: the default is exactly what it was.
    ceiling: Option<usize>,
    /// Whether to run walk-forward, PBO and the bootstrap.
    ///
    /// # Why a search must be able to say no
    ///
    /// These three cost `WALK_FORWARD_SPLITS` sweeps twice over plus
    /// `BOOTSTRAP_DRAWS` x `BOOTSTRAP_CANDIDATES` = sixteen thousand full trade
    /// re-walks, and none of that is sized by the data. MEASURED: a 60-minute
    /// audit over six months did not finish in sixty seconds at a candidate
    /// ceiling of ONE THOUSAND, while the same sweep without them takes 0.005s.
    ///
    /// A descent asks *did anything clear the rules* at each of ten supports;
    /// that is the screen. *Is the winner real* is a question about ONE
    /// combination and belongs after the search. Running it per step paid to
    /// validate candidates the next step was about to discard.
    ///
    /// `false` is not a weakened audit: `audit::render` prints an explicit
    /// absence for every unsupplied stage rather than a zero, so a page made
    /// this way states which questions it did not ask.
    validate: bool,
}

/// The series a position is actually opened and closed on.
///
/// # Signal and execution are two different series, and this is the second one
///
/// A bar is stamped at its OPEN (`docs/00-charter.md` §3), so a fifteen-minute
/// bar stamped 09:15 covers `[09:15, 09:30)` and its mask is not knowable until
/// 09:30. The earliest bar that mask can be acted on is the ONE-MINUTE bar
/// stamped 09:30 — the same instant the next fifteen-minute bar opens, reached
/// with fifteen times the resolution for everything that happens afterwards.
///
/// The resolution is the whole point. A stop and a target inside one bar's range
/// have no order the data can settle; a fifteen-minute bar hides fifteen minutes
/// of that path. A trailing order tracks a running peak that updates 25 times a
/// session on fifteen-minute bars and 375 times on one-minute bars, so a trail
/// measured on the coarse series never saw the peak it was supposed to trail
/// from.
#[derive(Clone, Copy)]
struct Execution<'a> {
    /// The one-minute bars, over the same span as the signal series.
    bars: &'a [indicators::Candle],
    /// How long ONE SIGNAL BAR lasts, in microseconds.
    ///
    /// Taken from the rung rather than inferred from timestamps: a gap between
    /// two signal bars is a halt or a session boundary, not a longer bar, and
    /// deriving the length from a difference would make the deadline move with
    /// the data.
    signal_length_micros: i64,
}

/// The bars and column a POSITION is taken on, given the ones a SIGNAL was found on.
///
/// Split out of [`audit_bars`] so that function stays under
/// `clippy::too_many_lines`, and because the choice it makes is one idea: the
/// search stays on the signal series and everything after the decision moves to
/// the execution series.
///
/// # Errors
///
/// The alignment refusals, as a sentence the CLI prints. Both are caller bugs
/// rather than data conditions — a non-positive bar length, or an alignment not
/// parallel to its own column — so they refuse rather than falling back to
/// executing on the signal rung. That fallback would produce a different
/// model's numbers rendered identically, which is what `CLAUDE.md` §4 bans.
fn project_onto_execution(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    execution: Option<Execution<'_>>,
    horizon: Horizon,
) -> Result<(Vec<indicators::Candle>, indicators::column::Column, String), String> {
    // NO EXECUTION SERIES IS NOT A DEGRADED RUN. `cli sweep`, `cli audit` and
    // `audit-stored` trade on the series they swept, which is what they have
    // always done and what their reports describe. Only `audit-range` supplies
    // one, and when the signal rung IS `1min` it deliberately does not -- a
    // series projected onto itself is the identity.
    let Some(execution) = execution else {
        return Ok((bars.to_vec(), column.clone(), String::new()));
    };
    let Some(alignment) = runner::align::onto_execution(
        bars,
        column.sources(),
        execution.bars,
        execution.signal_length_micros,
    ) else {
        return Err("the signal series could not be aligned onto the execution \
                    series. Nothing was traded."
            .to_owned());
    };
    let Some((projected, dropped)) = column.reproject(&alignment.onto, execution.bars.len()) else {
        return Err("the alignment is not parallel to the column it was built \
                    from. Nothing was traded."
            .to_owned());
    };
    // DROPPED SIGNALS ARE NAMED. A signal on a session's last bar has no
    // execution bar after it and cannot be taken; counting it silently would
    // make a smaller sample read like a whole one.
    let note = format!(
        "EXECUTION SERIES\n  \
         signal bars                                    {:>10}  the rung the conditions were found on\n  \
         execution bars (1min)                          {:>10}  where every entry and exit fills\n  \
         signals with no execution bar                  {:>10}  {}\n  \
         horizon                                        {:>10}  execution bars, so MINUTES\n\n",
        bars.len(),
        execution.bars.len(),
        dropped,
        if dropped == 0 {
            "every signal had a bar to act on"
        } else {
            "DROPPED -- a session's last bars have nothing after them"
        },
        horizon.as_bars(),
    );
    Ok((execution.bars.to_vec(), projected, note))
}

/// Serialises every write to the results ledger.
///
/// # Why a lock and not a lock-free append
///
/// `range_all` runs the nine rungs in PARALLEL. `Results::open` reads the whole
/// file to build its duplicate set and `append` seeks to the end and writes, so
/// two threads doing that at once can read a stale set, compute the same offset,
/// and have one record overwrite the other. A fixed-stride file makes that
/// silent: the survivor parses perfectly and the loser is simply gone.
///
/// One append per rung means nine acquisitions for a run that takes minutes, so
/// contention is nil and the O(1) append is untouched. What is bought is that
/// the ledger cannot lose a row it was told to keep.
///
/// A `Mutex` and not a file lock: every writer here is a thread of one process.
/// Two PROCESSES appending at once is a different problem and is not solved by
/// this -- which is why `cli` owns `<store>/logs/cli` rather than sharing the
/// server's directory, and why a second `range-all` should not be started while
/// one is running.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
static LEDGER: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// How many sweeps are sharing this machine right now.
///
/// # The defect this closes, and it is why a run "ran out of budget"
///
/// [`derived_ceiling`] answers *"how many candidates fit in THIS MACHINE'S
/// memory"* — `engine::DEFAULT_CEILING` at roughly 146 bytes each is about
/// **19.6 GB**, and its own doc says that is *"a fact about ONE machine"*. It is
/// therefore a budget for the machine, not for a caller.
///
/// [`range_over`] hands that whole-machine budget to **every rung at once**.
/// Its `rungs.par_iter()` runs eight independent sweeps concurrently and
/// nothing divided the ceiling between them, so eight sweeps each believed they
/// could claim 19.6 GB: **157 GB of a 48 GB machine**. The guard that exists to
/// stop the machine swapping was itself oversubscribing it eightfold.
///
/// # Why it was invisible
///
/// `range_over`'s own comment justified the concurrency with a measurement:
/// *"0.20 GB resident per rung at 20% support, so nine at once is under 2 GB of
/// 48. Memory is not what bounds this."* That was true when it was written and
/// D-0303 then deleted the 20%, replacing it with a floor each rung derives
/// from its own bars — which on the operator's 2026-08-28 run resolved to
/// **0.0045%**, four orders of magnitude lower. Candidate counts explode as
/// support falls, so the measurement that licensed running eight at once was
/// invalidated by a change in a different file, and nobody re-took it.
///
/// Two changes each correct alone, wrong together. This is the arithmetic that
/// makes them agree again: the machine's budget is DIVIDED among whoever is
/// actually running, so the total claim is constant no matter how many rungs a
/// request names.
static SWEEPS_SHARING_THIS_MACHINE: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(1);

/// Declares that `count` sweeps are about to share the machine, until dropped.
///
/// RAII rather than a pair of calls, so a panic or an early return inside the
/// parallel map cannot leave the divisor raised and silently starve every later
/// run in the same process — the server is long-lived and would carry it.
struct SharedBy;

impl SharedBy {
    fn these(count: usize) -> Self {
        SWEEPS_SHARING_THIS_MACHINE.store(count.max(1), std::sync::atomic::Ordering::Relaxed);
        Self
    }
}

impl Drop for SharedBy {
    fn drop(&mut self) {
        SWEEPS_SHARING_THIS_MACHINE.store(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The candidate ceiling this run may use, from the environment or the default.
///
/// # The last hard-coded number, and why it becomes an environment variable
///
/// `engine::DEFAULT_CEILING` is `2^27` — measured safe on a 48 GB machine at
/// roughly 146 bytes per candidate, so about 19.6 GB. That is a fact about ONE
/// machine baked into a `const`, and an operator on 16 GB would swap while one
/// on 128 GB would be capped four times below what their hardware allows.
///
/// It cannot be derived inside `crates/engine`: CI gate 22 clause A pins that
/// crate to `vocab` alone, so it can take no dependency that reads system
/// memory, and `CLAUDE.md` §2 forbids a vendored binding besides. The honest
/// place for the decision is the caller, and `Ladder::with_ceiling` has always
/// accepted one.
///
/// `BRUTEX_CEILING` follows the convention `BRUTEX_STORE` and `BRUTEX_LOG_DIR`
/// already set: an environment variable read once, at the boundary, with a
/// stated default and a loud refusal rather than a silent fallback.
///
/// # The arithmetic an operator needs to set it
///
/// | machine | safe ceiling | value |
/// |---|---|---|
/// | 16 GB | ~6 GB for the sweep | `41943040` (`2^22 * 10`) |
/// | 48 GB | ~19.6 GB | `134217728` (`2^27`, the default) |
/// | 128 GB | ~50 GB | `343597383` |
///
/// Roughly `bytes_you_can_spare / 146`. Set it too high and the machine swaps;
/// set it too low and the ladder halts early and SAYS SO — `outcome REFUSED`,
/// `trustworthy as a whole answer NO` — which is the failure mode to prefer.
///
/// A malformed value REFUSES rather than falling back to the default: an
/// operator who set the variable meant to change the ceiling, and quietly using
/// the old one is the §4 fallback that hides a failure.
/// The ceiling this machine can afford, derived rather than assumed.
///
/// # The assumption this replaces
///
/// `engine::DEFAULT_CEILING` is `2^27`, and that crate's own table sizes it
/// against **"a 48 GB machine"** — a static fact about somebody else's
/// hardware, deciding how far the ladder is allowed to walk before it halts and
/// reports `complete = NO`. On half that machine it invites the swap it was
/// chosen to avoid; on four times it, it stops the sweep well short of what the
/// operator could have explored, and the combination past the halt is not
/// ranked badly — it is never built.
///
/// # Why parallelism and not memory, stated rather than glossed
///
/// The quantity that belongs here is usable RAM, and **`cli` cannot read it**.
/// `std` exposes no memory API; `/proc/meminfo` does not exist on macOS, which
/// is the operator's platform; and `sysctl` is a process this would have to
/// spawn. Reading it properly needs a dependency, and a dependency in this
/// workspace is a `docs/05-decisions.md` matter rather than a detail — see
/// `docs/06-limits.md` §93 for what that would cost.
///
/// [`std::thread::available_parallelism`] is in `std`, answers on every
/// platform, and is a **proxy**: machines scale memory with cores, loosely and
/// not exactly. So this is honest about being a scaling and not a measurement —
/// it is calibrated at the reference machine and moves with the one it runs on,
/// which is strictly better than a constant that moves with neither.
///
/// # The reference is the machine the constant was measured on
///
/// `crates/cli`'s own `range_all` records it: *"NINE RUNGS AT ONCE, ON A MACHINE
/// WITH TEN PERFORMANCE CORES … 0.20 GB resident per rung at 20% support, so
/// nine at once is under 2 GB of 48."* Ten is therefore the divisor, and a
/// ten-core machine gets exactly `DEFAULT_CEILING` — the shipped behaviour is
/// unchanged where it was calibrated, and scales from there.
///
/// A machine that will not answer keeps the reference rather than guessing
/// downward: an unknown machine is not a small one, and halving the ladder on a
/// failed query would be a silent narrowing of the search. D-0306.
fn derived_ceiling() -> usize {
    /// What [`std::thread::available_parallelism`] answers on the machine
    /// `engine::DEFAULT_CEILING` was sized against.
    ///
    /// # It was 10 and that was a UNIT ERROR, measured on the reference machine
    /// itself
    ///
    /// `range_all`'s comment says *"A MACHINE WITH TEN PERFORMANCE CORES"*, and
    /// ten is what this constant first held. But the quantity multiplied below
    /// is not performance cores — it is `available_parallelism`, and that
    /// machine is an **Apple M4 Pro: 14 logical, 10 performance + 4
    /// efficiency, 48 GB**. It answers **14**.
    ///
    /// Dividing a ceiling calibrated for 14-core behaviour by 10 and
    /// multiplying it back by 14 inflates it by 1.4x — `187,904,808` candidates
    /// where the calibration says `134,217,728`. At the ~146 bytes per
    /// candidate `ceiling_from_env`'s own refusal quotes, that is **27 GB of
    /// 48**, and `engine::DEFAULT_CEILING`'s table says `2^28` — 39.2 GB —
    /// *"swaps on a 48 GB machine"*. The reference machine would have been
    /// pushed most of the way there **by the code meant to protect it**.
    ///
    /// The reference must be in the unit of the measurement. It is 14, so the
    /// reference machine now derives exactly `DEFAULT_CEILING` — the shipped
    /// value, unchanged where it was calibrated — and every other machine
    /// scales from there. Same class of defect as the paisa/points slip
    /// `reference_price` records: not a wrong number, a right number in the
    /// wrong unit. D-0307.
    ///
    /// # NO TEST CAN VERIFY THIS NUMBER, and pretending otherwise is worse than
    /// saying so
    ///
    /// It is a MEASURED PROPERTY of a machine that is not present at test time:
    /// `sysctl -n hw.logicalcpu` on the reference hardware. A test asserting it
    /// would either hardcode 14 — proving only that two copies of one guess
    /// agree — or read the machine it runs on, which is a different machine and
    /// answers a different number.
    ///
    /// So it is treated as `CLAUDE.md` §3 rule 1 treats a vendor fact: recorded
    /// with its source rather than derived. The source is
    /// `docs/06-limits.md` §93, which names the hardware and the command.
    /// `the_ceiling_is_derived_from_this_machine_and_not_from_an_assumed_one`
    /// bounds the CONSEQUENCE — bytes per core — but that guard is loose enough
    /// that the old value of 10 passed it at 1.96 GB against a 2 GB cap. It did
    /// not catch this and is not claimed to.
    const REFERENCE_CORES: usize = 14;

    let cores =
        std::thread::available_parallelism().map_or(REFERENCE_CORES, std::num::NonZero::get);
    // PER-CORE FIRST, so the multiply cannot overflow on a machine with many:
    // `DEFAULT_CEILING` is `2^27` and `2^27 / 10` is about 13.4 million, which
    // needs 2^38 cores to leave `usize` on a 64-bit target. Saturating anyway,
    // because a bound that wraps is not a bound.
    let whole_machine = (engine::DEFAULT_CEILING / REFERENCE_CORES)
        .saturating_mul(cores)
        // NEVER ZERO AND NEVER BELOW THE FLOOR A SINGLE CORE EARNS. A ceiling of
        // zero halts before the first candidate, which would report extinction
        // where the truth is that nothing was allowed to run.
        .max(engine::DEFAULT_CEILING / REFERENCE_CORES);

    // DIVIDED AMONG WHOEVER IS ACTUALLY RUNNING. See
    // [`SWEEPS_SHARING_THIS_MACHINE`]: the figure above is a MEMORY budget for
    // the machine, and `range_over` runs eight sweeps at once. Handing each of
    // them the whole machine claimed 157 GB of 48 -- the guard against swapping
    // oversubscribing the thing it guards.
    //
    // Dividing rather than serialising keeps the parallelism that makes eight
    // rungs finish in one rung's wall clock; what changes is only how deep each
    // is allowed to go before it must stop and say so.
    let sharing = crate::SWEEPS_SHARING_THIS_MACHINE.load(std::sync::atomic::Ordering::Relaxed);
    whole_machine
        .checked_div(sharing.max(1))
        .unwrap_or(whole_machine)
        // A SHARE IS STILL A SEARCH. Eight ways of a machine budget is millions
        // of candidates, but the floor keeps a pathological share count from
        // producing a ceiling that halts before the first level -- which reports
        // extinction where the truth is that nothing was allowed to run, the
        // same failure the `max` above refuses.
        .max(engine::DEFAULT_CEILING / REFERENCE_CORES)
}

fn ceiling_from_env() -> Result<usize, String> {
    match crate::knobs::var("BRUTEX_CEILING") {
        None => Ok(derived_ceiling()),
        Some(raw) => {
            let text = raw;
            match text.trim().parse::<usize>() {
                Ok(0) | Err(_) => Err(format!(
                    "BRUTEX_CEILING is `{text}`, which is not a candidate count \
                     of 1 or more. It is roughly the bytes you can spare divided \
                     by 146. Unset it and this machine derives {} from its own \
                     core count, which is a proxy for memory and not a \
                     measurement of it -- see docs/06-limits.md §93.",
                    derived_ceiling()
                )),
                Ok(n) => Ok(n),
            }
        }
    }
}

/// The ladder for this run: the operator's threshold and the machine's ceiling.
///
/// Every `Ladder::with_min_hits` call site in this crate goes through here, so
/// the ceiling cannot be set on one command and forgotten on another.
///
/// # Errors
///
/// A malformed `BRUTEX_CEILING`.
fn ladder_for(min_hits: u64) -> Result<Ladder, String> {
    ladder_within(min_hits, None)
}

/// [`ladder_for`], with a ceiling the caller may name.
///
/// `None` reads the environment, which is what every operator-facing command
/// passes. `Some` is for a caller that needs a sweep to EXIST rather than to be
/// exhaustive -- see [`AuditOptions::ceiling`] for the hour-long test that had
/// no way to say so.
fn ladder_within(min_hits: u64, ceiling: Option<usize>) -> Result<Ladder, String> {
    let ceiling = match ceiling {
        Some(named) => named,
        None => ceiling_from_env()?,
    };
    Ok(Ladder::with_min_hits(min_hits).with_ceiling(ceiling))
}

/// Everything the RESULTS STORE needs that only the caller knows.
///
/// The computed half — depth, combinations, the chosen exit's figures — is read
/// off the run inside [`audit_bars`]. This is the half that describes what was
/// ASKED for, and no part of the sweep can reconstruct it: the span and the feed
/// are gone by the time bars are a `Vec<Candle>`.
#[derive(Clone, Copy)]
struct Recording<'a> {
    /// Where the results file lives — the store root.
    root: &'a std::path::Path,
    /// The feed's directory word.
    feed: &'a str,
    /// The instrument.
    underlying: &'a str,
    /// The SIGNAL rung. Execution is always one-minute.
    timeframe: &'a str,
    /// First month of the span.
    from: (u16, u8),
    /// Last month of the span.
    to: (u16, u8),
    /// Months the range asked for.
    months_asked: u32,
    /// Months the store actually held.
    months_found: u32,
}

/// Writes one completed run into the results store, and says what happened.
///
/// # A failure here NEVER fails the run
///
/// A sweep with no recorded row is still a correct sweep, and refusing to report
/// an answer because a directory was unwritable would trade the whole answer for
/// an audit trail. `CLAUDE.md` §4 bans a fallback that HIDES a failure; this one
/// NAMES it, in the report, on the same screen as the numbers it failed to
/// record. That is the same rule `cli::install_log` follows for the event sink.
///
/// A duplicate identity is reported as what it is — not an error but a fact:
/// §3 rule 5 makes a rerun byte-identical, so the row is already correct.
/// Writes this run's ranked frontier, and says so on the page.
///
/// # Why `top` and not everything that survived
///
/// `by_evidence` holds up to `audit_keep()` combinations — ten thousand by
/// default, and an operator may raise it. Writing all of them per run would put
/// hundreds of megabytes on disk for a question nobody asked: `rules.top` is the
/// number the operator said they wanted to SEE, and the rest are already
/// summarised by `combinations` and `depth` on the ledger row.
///
/// The bound is stated on the page rather than left implicit, because "the top
/// twenty-five were kept" and "twenty-five survived" are different facts and a
/// reader must not have to guess which one a count is.
///
/// # A failure here does not fail the run
///
/// The ledger row is the record that a run happened and is already written. If
/// the frontier cannot be stored, the run still occurred and its numbers are
/// still on the page — so this REPORTS the refusal beside the result rather than
/// discarding a completed sweep over a detail file. `CLAUDE.md` §4 asks for the
/// reason to be named beside the answer, and that is what this does.
/// Writes this run's round trips, and never fails the run.
///
/// # Why the page has been showing padlocks
///
/// `web/src/routes/backtest/+page.svelte` draws a per-trade table — trade
/// number, entry and exit, price, net P&L, favourable and adverse excursion,
/// cumulative P&L, duration — and every cell of it renders a `Lock` carrying
/// its own sentence: *"No trade number — no trade list is recorded."* The
/// display was built and the file was never written. `crates/api/src/trades.rs`
/// held a store of the right shape with zero callers on either side; see
/// [`crate::trades`] for the three reasons it could not be the writer.
///
/// # It reports rather than refuses
///
/// Same contract as [`record_frontier`]: a `String` for the report, never a
/// `Result`. The trades are DETAIL about a run that already happened and whose
/// ledger row is already written — losing them must not turn a completed sweep
/// into a failure. A refusal is named in the returned text and the run stands.
///
/// Ordered after the ledger row and the frontier for the reason `record_frontier`
/// gives: a detail row whose run has no ledger row is an orphan, and writing the
/// ledger first makes that unreachable rather than merely unlikely.
fn record_trades(
    root: &std::path::Path,
    id: &runner::identity::RunId,
    taken: &runner::trade::Trades,
) -> String {
    if taken.trades.is_empty() {
        return String::new();
    }
    let identity = id.bytes();
    let rows: Vec<trades::Row> = taken
        .trades
        .iter()
        .enumerate()
        .map(|(seq, t)| trades::Row {
            identity,
            seq: u32::try_from(seq).unwrap_or(u32::MAX),
            signal_bar: u64::try_from(t.signal_bar).unwrap_or(u64::MAX),
            entry_bar: u64::try_from(t.entry_bar).unwrap_or(u64::MAX),
            exit_bar: u64::try_from(t.exit_bar).unwrap_or(u64::MAX),
            best: t.best,
            worst: t.worst,
        })
        .collect();

    // THE SAME LOCK THE LEDGER TAKES, AND FOR THE SAME REASON. `open` rebuilds
    // the block index by reading the file, so two rungs finishing together
    // would each index a file the other is appending to.
    let Ok(_guard) = LEDGER.lock() else {
        return "  the trades were not recorded: the results lock was poisoned\n".to_owned();
    };
    let mut file = match trades::Trades::open(root) {
        Ok(file) => file,
        Err(why) => return format!("  the trades were not recorded: {why}\n"),
    };
    match file.append_all(&rows) {
        Ok(written) => format!("  {written} trade(s) recorded for this run\n"),
        Err(why) => format!("  the trades were not recorded: {why}\n"),
    }
}

fn record_frontier(
    root: &std::path::Path,
    id: &runner::identity::RunId,
    by_evidence: &[&runner::rank::Scored],
    top: usize,
    // THE CELL EACH ROW WAS PRICED WITH, keyed by mask words.
    //
    // A row is ranked by the SWEEP and priced by the SCREEN, and those are two
    // different passes over two different questions. `screen_cap` means most
    // ranked combinations are never priced at all, so a lookup here misses for
    // exactly those -- and `Row::of` stores zeros with a `trades` of zero, which
    // is the only value that says "swept, never priced" rather than "priced and
    // it lost nothing".
    priced: &std::collections::HashMap<[u64; 6], grid::Cell>,
) -> String {
    let kept = by_evidence.len().min(top);
    let rows: Vec<frontier::Row> = by_evidence
        .iter()
        .take(top)
        .enumerate()
        .filter_map(|(at, scored)| {
            // A rank past `u16` is a `top` nobody typed -- the argument is
            // parsed as a `usize` and an operator asking for 65,536 rows has
            // asked for something this file does not carry. Dropped rather than
            // truncated to a wrong rank, and the count below says how many
            // landed.
            u16::try_from(at.saturating_add(1)).ok().map(|rank| {
                frontier::Row::of(id.bytes(), rank, scored, priced.get(&scored.mask.words()))
            })
        })
        .collect();

    let mut store = match frontier::Frontier::open(root) {
        Ok(store) => store,
        Err(why) => return format!("\n  the frontier was NOT recorded: {why}\n"),
    };
    match store.append_all(&rows) {
        Ok(total) => format!(
            "\n  frontier: {} of {} ranked combination(s) recorded, {total} row(s) \
             in the file\n",
            rows.len(),
            kept
        ),
        Err(why) => format!("\n  the frontier was NOT recorded: {why}\n"),
    }
}

fn record_run(
    into: Recording<'_>,
    id: &runner::identity::RunId,
    outcome: &runner::Outcome,
    exits: &runner::grid::Grid,
    bars: u64,
    min_hits: u64,
    // The combination this run traded, so the ledger can name it. A run identity
    // is a hash and cannot be turned back into conditions.
    mask_words: [u64; 6],
) -> String {
    let chosen = exits.best();
    let rung =
        |slot: Option<usize>| -> i16 { slot.and_then(|v| i16::try_from(v).ok()).unwrap_or(-1) };
    let record = results::Record {
        identity: id.bytes(),
        // The wall clock, taken once, after the work. `SystemTime` can precede
        // the epoch on a machine whose clock is set wrongly, and that is
        // recorded as the negative it is rather than clamped: a row stamped
        // before 1970 is a clock problem an operator should see.
        finished_micros: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_micros()).unwrap_or(i64::MAX)),
        feed: results::field(into.feed),
        underlying: results::field(into.underlying),
        timeframe: results::field(into.timeframe),
        from_year: into.from.0,
        from_month: into.from.1,
        to_year: into.to.0,
        to_month: into.to.1,
        months_asked: into.months_asked,
        months_found: into.months_found,
        bars,
        min_hits,
        combinations: outcome.sweep.all_frequent().count() as u64,
        depth: u32::try_from(outcome.sweep.depth()).unwrap_or(u32::MAX),
        // ONE BYTE THAT CHANGES HOW EVERY OTHER FIELD READS. A halted sweep's
        // `depth` is PARTIAL and its `combinations` covers less of the ladder
        // than the number suggests, so a row without this flag would rank a
        // truncated search against complete ones as though they were the same
        // kind of thing.
        halted: u8::from(outcome.sweep.halted.is_some()),
        trades: chosen.map_or(0, |c| c.trades),
        pessimistic: chosen.map_or(0, |c| c.pessimistic),
        optimistic: chosen.map_or(0, |c| c.optimistic),
        worst_trade: chosen.map_or(0, |c| c.worst_trade),
        max_drawdown: chosen.map_or(0, |c| c.max_drawdown),
        winner_mae: chosen.map_or(0, |c| c.winner_mae),
        winner_mfe: chosen.map_or(0, |c| c.winner_mfe),
        all_mae: chosen.map_or(0, |c| c.all_mae),
        exit_rungs: [
            rung(chosen.and_then(|c| c.stop)),
            rung(chosen.and_then(|c| c.target)),
            rung(chosen.and_then(|c| c.tsl)),
            rung(chosen.and_then(|c| c.ttp.map(|t| t.arm))),
            rung(chosen.and_then(|c| c.ttp.map(|t| t.trail))),
        ],
        // THE COMBINATION ITSELF, which the ledger could not name until now.
        // `identity` is a hash over the nine terms and cannot be turned back
        // into conditions; these six words can, through `vocab`.
        mask_words,
    };

    // HELD ACROSS THE OPEN AND THE APPEND, not just the append: `open` builds
    // the duplicate set by reading the whole file, and a set read before another
    // thread's append is stale by the time this one writes.
    let _guard = LEDGER.lock();
    let mut store = match results::Results::open(into.root) {
        Ok(store) => store,
        Err(why) => {
            return format!(
                "{NOT_RECORDED}: {why}\n  The figures below are correct; only the row is missing.\n\n"
            );
        }
    };
    match store.append(&record) {
        Ok(index) => format!(
            "RESULT RECORDED\n  \
             row                                            {index:>10}  in {}\n  \
             identity                                       {}\n\n",
            results::Results::path(into.root).display(),
            record.identity_hex(),
        ),
        Err(why) => format!(
            "{NOT_RECORDED}: {why}\n  The figures below are correct; only the row is missing.\n\n"
        ),
    }
}

/// The sentence [`record_run`] opens with when the row did not reach the ledger.
///
/// # Why this is a constant and not two string literals
///
/// It is written in one place and READ in another. `record_run` returns a
/// report, not a `Result` -- the failure is named into a string, which is right
/// for a reader and useless to a caller -- so `one_rung` has nothing to match on
/// except the sentence. A caller matching a literal that the producer is free to
/// reword is a check that passes for as long as nobody edits the message, which
/// is not a check. Both sides name this constant, so a reword moves both or
/// neither.
///
/// **What went wrong without it.** `one_rung` DISCARDS the long report on
/// purpose -- nine of them is six thousand lines -- and read the row back out of
/// the ledger instead. When the append failed, the read still succeeded: it
/// returned the newest row that MATCHED THE KEY, which is an earlier run at a
/// different commit, possibly a different binary and a different ceiling. The
/// descent printed it under this run's banner, feed, instrument, rung, span and
/// `min_hits`, and every column was plausible. That is the failure wearing a
/// success's clothes `CLAUDE.md` §4 bans, and it was reachable the moment a
/// ledger written by a newer build sat in the store -- `Results::append` refuses
/// a version it does not write, while `read_at` reads older rows and widens
/// them, so the two halves disagreed by design.
pub(crate) const NOT_RECORDED: &str = "RESULT NOT RECORDED";

/// The reason a report gives for a row that did not reach the ledger, if any.
///
/// `None` when the report carries no such sentence, which is the ordinary case
/// and the only one in which a row read back by key is this run's row.
fn not_recorded_reason(report: &str) -> Option<String> {
    let at = report.find(NOT_RECORDED)?;
    let rest = report[at + NOT_RECORDED.len()..].trim_start_matches([':', ' ']);
    // ONE LINE. The sentence continues "The figures below are correct; only the
    // row is missing", which is true of the long report and false of a descent
    // row -- there are no figures below, because this is the cell that would
    // have held them.
    Some(rest.lines().next().unwrap_or(rest).trim().to_owned())
}

/// The sentence a walk-forward that could not measure anything needs.
///
/// # A true zero that reads as the wrong fact
///
/// `validate::walk_forward` runs on the SIGNAL series while a run with an
/// execution series trades on the one-minute one. It cannot be moved the way the
/// ranking and the bootstrap family were: it builds its own `Column` per fold
/// and calls `grid::evaluate` on slices of whatever it is handed, so an
/// execution series must be threaded through it along with a per-fold alignment.
/// That is a change to a function with its own test suite and it is not made
/// here.
///
/// The consequence, on any rung whose bars are a whole session each: every
/// fold's forward return is refused — `outcome::forward` sets `close_at[i] = i`,
/// `exit <= i`, outcome `None` — so the walk decides nothing.
/// `Validated::decided()` prints 0, which is TRUE and reads as *"no combination
/// held up out of sample"* when the real reason is that nothing could be
/// measured. `CLAUDE.md` §4 bans a fallback that hides a failure; naming it is
/// what is left until the fix lands.
fn walk_forward_caveat(has_execution: bool, decided: usize) -> &'static str {
    if has_execution && decided == 0 {
        "  WALK-FORWARD MEASURED NOTHING, AND THAT IS NOT THE SAME AS FAILING.\n  \
         It runs on the SIGNAL series while this run trades on the 1-minute one. \
         On a rung whose bars are a whole session each, every forward return is \
         refused because a position cannot open and close inside one bar -- so \
         `that chose a combination` above is 0 for a reason about the SERIES and \
         not about the strategy. Read the EXIT GRID and the bootstrap instead; \
         both run on the execution series.\n\n"
    } else {
        ""
    }
}

/// The three multiple-testing p-values, and the family they were taken over.
///
/// Split out of [`audit_bars`] to keep it under `clippy::too_many_lines`. It is
/// one idea: build one RETURN SERIES per candidate, then run every bootstrap
/// over that same family so the three answers are about the same set.
///
/// Returns `None` when the family is empty, which is the one state the callers
/// below must not read as "the tests passed".
///
/// **Runs on the EXECUTION series**, with the trade and the exit grid.
///
/// This doc argued the opposite -- that these belong on the signal rung because
/// they are statements about how often a CONDITION precedes a move. That is true
/// of FREQUENCY and false of what this function actually builds, which is one
/// RETURN SERIES per candidate from `trade::walk`. A trade walk on a series whose
/// bars are a whole session each produces no trades at all, so the family came
/// back empty and all three tests reported on nothing.
fn bootstrap_family(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    by_evidence: &[&runner::rank::Scored],
    horizon: Horizon,
) -> Option<(
    Option<runner::bootstrap::Verdict>,
    Option<runner::bootstrap::Verdict>,
    usize,
)> {
    // THE BOOTSTRAP, WHICH NEEDED A DIFFERENT SHAPE OF DATA FROM PBO.
    //
    // PBO needed one NUMBER per candidate. This needs one SERIES per candidate —
    // a return per period — because it resamples periods and asks whether the
    // best of the family beats what the same search finds on resampled data.
    // Comparing one strategy against itself answers nothing, so a family is
    // walked rather than the winner alone.
    //
    // Every series is bucketed into the SAME day index, so `bootstrap::aligned`
    // cannot refuse the set for a length mismatch — alignment is a property of
    // how this is built.
    let days = session_index(bars);
    // BUILT ONCE, PROBED PER TRADE. `session_returns` runs once per candidate
    // and each walks its own trades, so the index is hoisted here rather than
    // rebuilt inside — one pass over the sessions for the whole family.
    let index: std::collections::HashMap<i64, usize> = days
        .iter()
        .enumerate()
        .map(|(slot, day)| (*day, slot))
        .collect();
    // THE FAMILY THE REPORT ACTUALLY SELECTS FROM, and it did not used to be.
    //
    // White's Reality Check and Hansen's SPA are FAMILY-WISE tests: they ask
    // whether the best of a set beats what the same search finds on resampled
    // data, so the set has to be the set the search considered. This walked
    // `closed.kept` truncated to the first sixteen in canonical mask order —
    // which `crates/engine` states outright is not a ranking — while the
    // combination the report chose to trade came from a different rule entirely.
    // The p-value therefore described a family the reported strategy need not
    // even have belonged to.
    //
    // `by_evidence` is the same list the traded combination is drawn from, so
    // the head of the family and the strategy under test are now one thing.
    let family: Vec<Vec<i64>> = by_evidence
        .iter()
        .take(BOOTSTRAP_CANDIDATES)
        .map(|scored| {
            // EACH CANDIDATE ON ITS OWN SIDE, for the reason the traded
            // combination above is. Walking the whole family long would give a
            // short setup a return series that is the negative of what it would
            // have earned, so the bootstrap's null would be built from returns
            // no strategy in the family would ever have taken — and the p-value
            // beside it would describe that fiction rather than the family.
            let walked = trade::walk(
                bars,
                column,
                &scored.mask,
                horizon,
                direction_of(side_of_evidence(scored)),
            );
            session_returns(&index, days.len(), bars, &walked)
        })
        .collect();
    // ALL THREE TESTS, AND THE THIRD ONE NOW ACTUALLY RUNS.
    //
    // They answer different questions: Reality Check says "something in this
    // family is real", SPA says the same with poor strategies no longer diluting
    // the null, and Romano-Wolf is the per-strategy stepdown that says WHICH.
    //
    // This comment used to read "…so it is not run rather than run and
    // discarded", and that was true of the computation and false of the REPORT:
    // `audit::bootstrap` was handed `family.len()` for its `named` column and
    // printed, in words, "Only Romano-Wolf says WHICH, and it names 16" — the
    // size of the family offered, not the count of anything rejected. A reader
    // was told a stepdown had named sixteen strategies when no stepdown had been
    // computed at all. That is the failure-wearing-a-success's-clothes shape
    // `CLAUDE.md` §4 bans, in the one section of the report whose whole job is
    // to say how much of this is luck.
    //
    // The stated reason for not running it — "needs a decision about which
    // strategies to report" — is answered by the same alpha the two rows beside
    // it are already judged at, so the decision was available; it just had not
    // been made.
    // Draws follow the alpha the answer is read at and the sample behind it, so
    // the resample is never the binding uncertainty. See `bootstrap_draws`.
    let draws = bootstrap_draws(family.first().map_or(0, Vec::len));
    let rc = runner::bootstrap::reality_check(
        &family,
        draws,
        BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
    );
    let spa = runner::bootstrap::spa(
        &family,
        draws,
        BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
    );
    // THE STEPDOWN, at the same 5% the two rows above are judged at, so one
    // report carries one alpha rather than two.
    let named = runner::bootstrap::romano_wolf(
        &family,
        draws,
        BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
        BOOTSTRAP_ALPHA_PPM,
    )
    .len();
    let boot = (!family.is_empty()).then_some((rc.as_ref(), spa.as_ref(), named));
    let _ = boot;
    if family.is_empty() {
        return None;
    }
    Some((rc, spa, named))
}

/// The whole audit stack over bars the caller supplies, under a banner it names.
///
/// # Why this exists: the audit could only ever see invented data
///
/// Everything below — trades, worst-case fills, the exit grid, walk-forward,
/// PBO and the bootstrap — was reachable from exactly one command, `cli audit`,
/// whose bars came from `synthetic::sessions` **unconditionally**. There was no
/// `audit-stored`, so no entry point in this workspace had ever produced a
/// trade, a P&L, an exit grid, a walk-forward verdict, a PBO figure or a
/// bootstrap p-value **from real market data**. Every such number this system
/// could print described the generator.
///
/// `sweep-stored` could read a real month and rank it; it stopped short of
/// trading it. This is the join, and it is the same code on both sides — a
/// second implementation for real bars would be two backtests that could
/// disagree.
///
/// # The banner is a parameter, and that is load-bearing
///
/// A sweep over invented data is byte-identical in SHAPE to one over real data,
/// so the banner is the only thing separating them — which is why
/// `the_generated_and_stored_banners_make_opposite_claims` fails the build if
/// the two ever converge. Passing it in rather than deciding it here means the
/// caller that chose the bars is the caller that names their provenance, and the
/// two cannot drift apart.
///
/// # The identity is `Option`, for the reason `report::render` takes one
///
/// A synthetic run has no instrument to name, and naming one would be the
/// invention `CLAUDE.md` §3 rule 1 forbids — so it passes `None` and the report
/// says NOT RECORDED in place of a digest. A stored run has one and passes it.
#[allow(
    clippy::needless_pass_by_value,
    clippy::large_types_passed_by_value,
    reason = "the same ownership requirement `audit_with` documents: \
              `Column::build` and `Sweeper::run` both take `&mut`, and the \
              evaluator must outlive them."
)]
fn audit_bars(
    ev: Result<Evaluator, &'static str>,
    bars: Vec<indicators::Candle>,
    banner: &str,
    min_hits: u64,
    id: Option<&runner::identity::RunId>,
    opts: AuditOptions<'_>,
) -> String {
    let AuditOptions {
        execution,
        recording,
        rules,
        lens,
        ceiling,
        validate,
    } = opts;
    let mut ev = match ev {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    let horizon = Horizon::DEFAULT;
    // ONE FOLD, NOT TWO, AND ONE EVALUATOR RATHER THAN A CALLER'S PLUS A
    // PRIVATE ONE. This was `Column::build(&bars, &mut ev)` followed by a second
    // `evaluator()` that the sweep used instead — so the `ev` parameter governed
    // the column and nothing else, and a caller passing custom widths would have
    // had masks discovered under one vocabulary indexing a column built under
    // another. `run_ranked` returns the column it measured on, so the sweep, the
    // ranking and the trade walk below cannot disagree about what they saw.
    // THE CEILING COMES FROM THE ENVIRONMENT HERE TOO. A sweep that used the
    // operator's ceiling and a walk-forward that used the compiled default
    // would be two different searches inside one report.
    // THE CALLER'S CEILING WINS WHEN IT NAMES ONE. Everything operator-facing
    // passes `None` and gets the environment's, so no command's search narrows.
    let ladder = match ladder_within(min_hits, ceiling) {
        Ok(l) => l,
        Err(why) => return format!("refused: {why}\n"),
    };
    let run = Sweeper::new(ladder).run_ranked_by(&bars, &mut ev, horizon, audit_keep(), lens);
    let (outcome, ranked, column) = (run.outcome, run.ranked, run.column);

    // THE POSITION MOVES TO THE EXECUTION SERIES; THE SEARCH DOES NOT.
    //
    // The sweep above measured which conditions are FREQUENT, and frequency is a
    // property of the signal series -- "this fired on 4% of fifteen-minute bars"
    // is the statement, and re-counting it on one-minute bars would answer a
    // different question. So `run_ranked`, `render` and `render_findings` all
    // stay on `bars`.
    //
    // What moves is everything AFTER the decision. `trade::walk` and
    // `grid::evaluate` below take the projected column and the one-minute bars,
    // so the stop, the target and both trailing orders are checked at
    // one-minute resolution instead of at the signal rung's. `Horizon` is
    // counted in bars, so it also becomes MINUTES on every signal timeframe --
    // which is the only reading under which nine timeframes are comparable at
    // all.
    let (trade_bars, trade_column, execution_note) =
        match project_onto_execution(&bars, &column, execution, horizon) {
            Ok(triple) => triple,
            Err(why) => return format!("refused: {why}\n"),
        };

    // THE RANKING IS RE-TAKEN ON THE EXECUTION SERIES, AND IT HAS TO BE.
    //
    // # MEASURED: every one of the top 250 on a daily run scored n=0, t=0.00
    //
    // `run_ranked` above computes `outcome::forward` on the SIGNAL bars, which
    // was right when the signal series was also the series positions were taken
    // on. It is not right now, and on the coarsest rung it is catastrophic.
    //
    // Trace a 1day bar through `outcome::forward`. Each daily bar is its own IST
    // session, so `close_at[i]` is `i` itself -- the forced close of that
    // session is that bar. `want = i + horizon` is past it, `is_window_end`
    // holds, so `exit = forced = i`, and `exit <= i` refuses the outcome. EVERY
    // forward return on a daily series is `None`, correctly: this engine is
    // intraday-only and squares off at 15:10, so there is no intraday movement
    // inside one daily bar to measure.
    //
    // `rank` then orders by |t| over a set where every t is 0.00, which is an
    // ordering by nothing. The audit printed 250 rows all reading "TOO FEW
    // OBSERVATIONS to judge" and the combination it traded was whichever of them
    // the heap happened to surface.
    //
    // Meanwhile `trade::walk` and `grid::evaluate` DO run on the execution
    // series and found 342 real round trips. So the run traded on one series and
    // ranked on another where trading is impossible.
    //
    // # Why frequency stays on the signal series and outcomes do not
    //
    // "This fired on 20% of 15-minute bars" is a property of the rung the
    // condition was found on, and re-counting it per minute answers a different
    // question -- which is why the sweep is left alone above. A forward RETURN
    // is not frequency: it is what happened after the signal, and what happened
    // is what the execution series records. The two halves belong on different
    // series and the split was half-made.
    // THE LENS SURVIVES THE REPROJECTION, and it did not.
    //
    // This re-ranked with `runner::rank::rank`, which is the `Detectability`
    // entry point and takes no lens. `execution` is `Some` for every rung except
    // `EXECUTION_RUNG`, so a caller that asked for `Lens::Payoff` had it honoured
    // once by `run_ranked_by` above and then SILENTLY DISCARDED here on seven of
    // the eight rungs.
    //
    // That defeated the whole of `cli elite`. `keep` is a hard boundary — the
    // ordering that decides the cut decides what the pipeline can consider at
    // all — so re-ranking on `|t|` put the cut back exactly where the payoff
    // lens was built to move it, and nothing in the report said so.
    //
    // The re-rank itself is right and stays: a forward RETURN belongs on the
    // execution series, not the signal series. Only the lens was lost.
    let ranked = match execution {
        None => ranked,
        Some(_) => runner::rank::rank_by(
            &outcome.sweep,
            &trade_column,
            &runner::outcome::forward(&trade_bars, horizon),
            audit_keep(),
            lens,
        ),
    };

    let mut out = String::from(banner);
    out.push('\n');
    out.push_str(&execution_note);
    out.push_str(&runner::report::render(&outcome, id));
    // WHICH COMBINATIONS SURVIVED, BY NAME, before anything is traded. The
    // stored SWEEP has printed this since the ranker was wired; the audit did
    // not, so the one command that produces a P&L was the one that could not say
    // what the P&L was of.
    out.push_str(&runner::report::render_findings(&ranked, &outcome.sweep));

    // RANKED BY EVIDENCE, THEN FILTERED FOR REDUNDANCY. Computed once and used
    // twice: the head is the combination this audit trades, and the first
    // `BOOTSTRAP_CANDIDATES` of it are the family the bootstrap compares. Those
    // used to be two different sets — the trade took `closed.kept.first()` and
    // the bootstrap took `closed.kept`'s first sixteen, both in canonical mask
    // order — so the report's chosen strategy was not even a member of the
    // family whose p-value the report printed beside it.
    let by_evidence = closed_by_evidence(&ranked, &outcome.sweep);
    let Some(first) = by_evidence.first().copied() else {
        // TWO DIFFERENT FACTS WORE ONE SENTENCE, AND ONLY ONE OF THEM IS
        // EXTINCTION.
        //
        // This printed "This is extinction, not a failure" unconditionally.
        // `by_evidence` is the best audit_keep() by |t| intersected with the
        // closed set — so it can be empty either because the sweep genuinely
        // found nothing, or because none of the top 250 by evidence happened to
        // be closed on a sweep that found millions.
        //
        // `audit_keep()`'s own doc block predicts this failure in words — "the
        // audit reports extinction on a sweep that found plenty, a refusal that
        // would be a lie about the market rather than a fact about it" — and
        // the constant was raised to make it unlikely while the MESSAGE was
        // left unconditional. Guarding the cause and not the claim is how a
        // report ends up asserting something it cannot know.
        out.push_str(&nothing_to_trade(outcome.sweep.all_frequent().count()));
        return out;
    };
    // THE SCREEN, before the single-combination report below. 0.20% is 20
    // basis points -- about fifty points on a 25,000 index -- and is a stated
    // default rather than a derived one: no document defines the right stop and
    // nothing in the data implies one, so the number is the operator's to move.
    out.push_str(&traded_preamble(
        first,
        &outcome.sweep,
        session_index(&bars).len(),
        &bars,
    ));

    // THE SIDE IS READ OFF THE EVIDENCE, NOT ASSUMED.
    //
    // `rank` orders candidates by |t| — the ABSOLUTE value, deliberately, because
    // a combination that reliably precedes a FALL is as tradeable as one that
    // precedes a rise; only the side differs. `rank`'s own header says so, and
    // carries the sign in `Edge::mean_paisa` "for a reader to see".
    //
    // Nothing read it. This walked `Direction::Long` unconditionally, so the
    // combination with the strongest evidence of a DOWNWARD move was traded
    // long — and every figure below it, the P&L, the 125-cell grid, the
    // walk-forward, the PBO and all three bootstrap p-values, described the
    // wrong side of it.
    //
    // Worse, 6d849ee made it more likely to bite rather than less. Before that
    // commit the audit traded `closed.kept.first()`, an arbitrary combination in
    // canonical mask order, so its sign was incidental. Selecting for the
    // largest |t| selects precisely the strongest signals of EITHER sign — so
    // the better the ranker got, the more often the side was wrong.
    let (taken, exits, screened, priced) = trade_and_screen(
        &trade_bars,
        &trade_column,
        first,
        &by_evidence,
        horizon,
        rules,
        validate,
    );
    out.push_str(&screened);
    out.push('\n');
    // THE WALK-FORWARD, WHICH USED TO BE A `None`.
    //
    // `validate::walk_forward` was built, tested and never called: this report
    // printed "NOT SUPPLIED to this render" for it on every run since the
    // function existed. What it needed was a fold count, and
    // `WALK_FORWARD_SPLITS` supplies one as a STATED ASSUMPTION -- the form
    // `bootstrap::DEFAULT_BLOCK` and `validate::DEFAULT_RUNGS` already use.
    //
    // It re-sweeps once per fold, so it costs about `WALK_FORWARD_SPLITS` times
    // the sweep above. That is what an out-of-sample verdict costs, and it is
    // paid here rather than skipped.
    // A FRESH EVALUATOR PER FOLD, AND IT IS A COPY RATHER THAN A REBUILD.
    //
    // `walk_forward` wants `FnMut() -> Evaluator` because each fold must start
    // from an unwarmed detector -- a fold that inherited the previous fold's
    // state would be reading bars it was never given. `Evaluator` is `Copy`
    // (1,744 bytes, `docs/10-shared-core.md`), so one built here and copied per
    // fold is the same value a rebuild would produce, without a fallible call
    // inside a closure that has no way to report a refusal.
    let fresh = match evaluator() {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    // THE WALK-FORWARD TAKES THE SIDE THE EVIDENCE CHOSE, and until now it did
    // not.
    //
    // `rank` orders by |t| -- the ABSOLUTE value, deliberately, because a
    // combination that reliably precedes a FALL is as tradeable as one that
    // precedes a rise. `side_of_evidence` then reads the sign, and the trade
    // walk, the fills, the 625-cell grid and the screener all follow it.
    //
    // This call did not. It passed `Direction::Long` unconditionally, so every
    // SHORT combination was validated out of sample as though it were long --
    // its out-of-sample figure was the P&L of taking the opposite side of its
    // own signal. A combination with a strong downward edge would look like a
    // strong loser and be reported as failing to hold up.
    //
    // The comment directly above `side_of_evidence` records this exact defect
    // being fixed for the audit's own trade. It was not fixed here, which is the
    // shape of most of what this session found: one call site corrected and its
    // sibling left behind.
    // BOTH WINDOW SHAPES, because neither answers the other's question.
    //
    // Anchored grows from bar zero: "does this edge survive as history
    // accumulates". Rolling slides a fixed width: "does it survive on RECENT
    // history alone". A strategy that passes anchored and fails rolling has an
    // edge that DECAYED -- the early years carried it, and the growing window
    // kept them in scope long after they stopped being informative. Neither
    // shape can show that alone, which is the whole reason both run.
    //
    // The cost is one extra walk-forward: `walk_forward_splits` sweeps, which
    // is two to twenty on any real span. That is stated rather than hidden
    // because it is a real doubling of this stage, and an operator who does not
    // want it should be able to see what he is paying for.
    // THE WHOLE VALIDATION STACK IS SKIPPABLE, AND SKIPPING IT IS WHAT MAKES A
    // SEARCH POSSIBLE AT ALL.
    //
    // Below this line are three stages whose cost is fixed by constants rather
    // than by the data: `both_shapes` runs `WALK_FORWARD_SPLITS` sweeps twice
    // over, `pbo` ranks every fold's candidates, and `bootstrap_family` draws
    // `BOOTSTRAP_DRAWS` (1,000) resamples for each of `BOOTSTRAP_CANDIDATES`
    // (16) — sixteen thousand full trade re-walks, whether the run has forty
    // trades or forty thousand.
    //
    // MEASURED, and it is the reason nothing finished: a 60-minute audit over
    // six months — seven hundred and fifty signal bars — did not complete in
    // sixty seconds at a candidate ceiling of ONE THOUSAND. The same sweep
    // without this stack takes 0.005s. The search was never the cost.
    //
    // A DESCENT ASKS ONE QUESTION PER STEP: did anything clear the rules? That
    // is the screen, which is already computed above. Walk-forward, PBO and the
    // bootstrap answer *is the winner real*, which is a question about ONE
    // combination and belongs after the search rather than inside every step of
    // it. Running them per step paid for validating candidates that the next
    // step was about to discard.
    //
    // `validate: false` is therefore not a weakened audit. It is the search
    // half, and `audit::render` prints an explicit absence for each unsupplied
    // stage rather than a zero — so a page produced this way says which
    // questions it did not ask.
    let (folds, rolling) = shapes_if(validate, &bars, horizon, first, ladder, &fresh);
    // PBO, WHICH USED TO BE A `None` FOR A REASON THAT IS NOW FIXED.
    //
    // `pbo::place` ranks a fold's candidates in-sample, finds where the winner
    // lands OUT of sample, and returns that placement. It needs both vectors,
    // and `FoldResult` kept only the chosen candidate until `in_sample_all` and
    // `out_of_sample_all` were added -- so the input did not exist to pass and no
    // constant could have supplied it.
    //
    // A fold that chose nothing yields no placement and is skipped rather than
    // counted as a failure: `place` returns `None` on an empty or mismatched
    // pair, and filtering is the honest reading of "this fold had nothing to
    // judge".
    let overfit = overfitting_of(&folds);

    // The three multiple-testing p-values, over one family. See the helper: it
    // runs on the SIGNAL series on purpose, unlike the trade and the grid above.
    // THE SAME DEFECT AS THE RANKING, IN THE SAME SHAPE. `bootstrap_family`
    // builds one return series per candidate by calling `trade::walk`, and a
    // trade walk on the SIGNAL series produces nothing on any rung whose bars
    // are a whole session each -- so the family is empty, and White's Reality
    // Check, Hansen's SPA and Romano-Wolf all report on nothing.
    //
    // Its own doc argued the opposite: that these are statements about how often
    // a CONDITION precedes a move and so belong on the rung it was found on.
    // That is true of FREQUENCY and false of a RETURN SERIES, which is what this
    // builds. The argument was right about the sweep and wrong about this.
    let boot_owned = validate
        .then(|| bootstrap_family(&trade_bars, &trade_column, &by_evidence, horizon))
        .flatten();
    let boot = boot_owned
        .as_ref()
        .map(|(rc, spa, named)| (rc.as_ref(), spa.as_ref(), *named));

    // RECORDED BEFORE IT IS RENDERED, so a process killed while formatting a
    // large report still leaves its row. The same ordering `cli.audit`'s
    // `audit rendered` event uses, and for the same reason.
    let recorded = Recorded {
        outcome: &outcome,
        exits: &exits,
        bars: u64::try_from(bars.len()).unwrap_or(u64::MAX),
        min_hits,
        mask_words: first.mask.words(),
        by_evidence: &by_evidence,
        top: rules.top,
        priced: &priced,
        taken: &taken,
    };
    if let (Some(into), Some(run_id)) = (recording, id) {
        out.push_str(&record_all(into, run_id, &recorded));
    }
    out.push_str(walk_forward_caveat(execution.is_some(), folds.decided()));
    out.push_str(&audit::render(
        Some(&taken),
        Some(&exits),
        Some(&folds),
        overfit.as_ref(),
        boot,
        12,
    ));
    out.push_str(&decay_block(&folds, &rolling));
    out
}

/// The walk-forward in BOTH window shapes, anchored first.
///
/// # Why both, and what the second one costs
///
/// Anchored grows from bar zero: *"does this edge survive as history
/// accumulates"*. Rolling slides a fixed width: *"does it survive on RECENT
/// history alone"*. A strategy that passes anchored and fails rolling has an
/// edge that DECAYED -- the early years carried it, and the growing window kept
/// them in scope long after they stopped being informative. Neither shape can
/// show that alone.
///
/// The cost is one extra walk-forward: `walk_forward_splits` sweeps, two to
/// twenty on any real span. Stated rather than hidden, because it is a real
/// doubling of this stage.
///
/// Split from `audit_bars` to keep it under `clippy::too_many_lines`, and
/// because it is one idea.
/// The probability of backtest overfitting, from a completed walk-forward.
///
/// Extracted so `audit_bars` stays inside its line budget. A fold that chose
/// nothing yields no placement and is SKIPPED rather than counted as a failure:
/// `place` returns `None` on an empty or mismatched pair, and filtering is the
/// honest reading of "this fold had nothing to judge".
///
/// `None` when no fold produced a placement, which `audit::render` prints as an
/// absence rather than as a zero probability -- the difference between "not
/// measured" and "measured as no overfitting".
fn overfitting_of(folds: &runner::validate::Validated) -> Option<runner::pbo::Pbo> {
    let placements: Vec<_> = folds
        .folds
        .iter()
        .filter_map(|f| runner::pbo::place(&f.in_sample_all, &f.out_of_sample_all))
        .collect();
    (!placements.is_empty()).then(|| runner::pbo::probability_of_overfitting(&placements))
}

/// [`both_shapes`], or a pair of empty walks when validation is off.
///
/// A one-line wrapper so `audit_bars` stays inside its line budget, and so the
/// reason for the branch sits beside the branch rather than inside a caller
/// already carrying a dozen other concerns.
///
/// `Validated::default()` is an EMPTY walk, not a failed one. `audit::render`
/// prints an explicit absence for an unsupplied stage rather than a zero, so a
/// page made this way says which question it did not ask.
fn shapes_if(
    validate: bool,
    bars: &[indicators::Candle],
    horizon: Horizon,
    first: &runner::rank::Scored,
    ladder: engine::Ladder,
    fresh: &Evaluator,
) -> (runner::validate::Validated, runner::validate::Validated) {
    if validate {
        both_shapes(bars, horizon, first, ladder, fresh)
    } else {
        (
            runner::validate::Validated::default(),
            runner::validate::Validated::default(),
        )
    }
}

fn both_shapes(
    bars: &[indicators::Candle],
    horizon: Horizon,
    first: &runner::rank::Scored,
    ladder: engine::Ladder,
    // BY REFERENCE, then copied per fold inside the closure. `Evaluator` is
    // 1,776 bytes and `Copy`, so passing it by value moves that once per call
    // for no reason -- the closure needs its own copy either way, and it takes
    // one from the borrow.
    fresh: &Evaluator,
) -> (runner::validate::Validated, runner::validate::Validated) {
    let splits = walk_forward_splits(bars.len());
    let side = direction_of(side_of_evidence(first));
    let sweeper = Sweeper::new(ladder);
    let anchored = runner::validate::walk_forward_shaped(
        bars,
        horizon,
        splits,
        side,
        &sweeper,
        &mut || *fresh,
        runner::split::Shape::Anchored,
    );
    let rolling = runner::validate::walk_forward_shaped(
        bars,
        horizon,
        splits,
        side,
        &sweeper,
        &mut || *fresh,
        runner::split::Shape::Rolling,
    );
    (anchored, rolling)
}

/// The two window shapes side by side, and what their disagreement means.
///
/// # Why a comparison and not a second table
///
/// `audit::walk_forward` already renders one `Validated` in full, and printing
/// a second identical block would leave the reader to diff twenty rows by eye
/// for the one fact that matters: whether the edge survived on RECENT history
/// as well as on all of it.
///
/// The anchored shape trains from bar zero every time, so its later folds carry
/// years that may have stopped being informative. The rolling shape slides a
/// fixed width and cannot. **Anchored passing while rolling fails is edge
/// decay**, and it is invisible in either report alone.
fn decay_block(
    anchored: &runner::validate::Validated,
    rolling: &runner::validate::Validated,
) -> String {
    let mut out = String::from("\nWINDOW SHAPE -- did the edge survive on RECENT history too\n");
    let _ = writeln!(
        out,
        "  {:<12}{:>8}{:>10}{:>12}{:>14}",
        "shape", "folds", "decided", "positive", "train bars"
    );
    for (shape, v) in [("anchored", anchored), ("rolling", rolling)] {
        // `held_up` and not a hand-rolled comparison: it already encodes what
        // counts as a fold that survived -- an exit total above zero where one
        // exists, and the worst-case-positive test otherwise -- and a second
        // definition beside it would drift from the audit table above.
        let positive = v.held_up();
        let width = v.folds.last().map_or(0, |f| f.train_bars);
        let _ = writeln!(
            out,
            "  {:<12}{:>8}{:>10}{:>12}{:>14}",
            shape,
            v.folds.len(),
            v.decided(),
            format!("{positive}/{}", v.folds.len()),
            width
        );
    }
    let anchored_positive = anchored.held_up();
    let rolling_positive = rolling.held_up();
    let _ = writeln!(
        out,
        "\n  {}",
        if anchored.folds.is_empty() || rolling.folds.is_empty() {
            "One shape produced no folds, so the two cannot be compared. The \
             slice was too short to split."
        } else if anchored_positive > rolling_positive {
            "EDGE DECAY. The anchored windows held up better than the rolling \
             ones, which means the early\n  years are carrying the result and \
             recent history alone does not reproduce it."
        } else if rolling_positive > anchored_positive {
            "The rolling windows held up BETTER, which means recent history is \
             the stronger half and the\n  early years are diluting the anchored \
             figure."
        } else {
            "Both shapes agree, so the result does not depend on how far back \
             the training window reaches."
        }
    );
    let _ = writeln!(
        out,
        "  A rolling window warms its evaluator INSIDE itself, so its first \
         bars build state and are not\n  swept. That makes it a harsher test \
         than the anchored shape, not a laxer one."
    );
    out
}

/// Everything one recorded run writes, gathered so `audit_bars` states it once.
///
/// A struct rather than nine parameters, because the workspace bounds argument
/// counts and because these nine belong together: they are one run's durable
/// output, and a caller passing eight of them has not recorded a run.
struct Recorded<'a> {
    outcome: &'a runner::Outcome,
    exits: &'a grid::Grid,
    bars: u64,
    min_hits: u64,
    mask_words: [u64; 6],
    by_evidence: &'a [&'a runner::rank::Scored],
    top: usize,
    priced: &'a std::collections::HashMap<[u64; 6], grid::Cell>,
    taken: &'a runner::trade::Trades,
}

/// The three files a run leaves behind, in the order they must be written.
///
/// LEDGER, then FRONTIER, then TRADES, and the order carries a rule rather than
/// a preference: the ledger row is the record that a run happened at all, and a
/// detail row whose run has no ledger row is an orphan. Writing the ledger first
/// makes that unreachable rather than merely unlikely.
///
/// Returns a report and never a `Result`. Detail about a completed run must not
/// turn that run into a failure — each of the three names its own refusal in the
/// text and the run stands.
fn record_all(
    into: Recording<'_>,
    run_id: &runner::identity::RunId,
    what: &Recorded<'_>,
) -> String {
    let mut out = String::with_capacity(256);
    out.push_str(&record_run(
        into,
        run_id,
        what.outcome,
        what.exits,
        what.bars,
        what.min_hits,
        what.mask_words,
    ));
    out.push_str(&record_frontier(
        into.root,
        run_id,
        what.by_evidence,
        what.top,
        what.priced,
    ));
    out.push_str(&record_trades(into.root, run_id, what.taken));
    out
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
    use super::{
        COMMANDS, MIN_AUDIT_SESSIONS, MISUSED, OK, PROVENANCE, STORED_PROVENANCE, UNVALIDATED,
        USAGE, Vendor, audit_run, audit_run_within, audit_stored, auto, auto_with, direction_of,
        evaluator_from, log_dir_from, nothing_to_trade, parse_min_hits, parse_sessions,
        parse_vendor, policy_of, root_from, run, sample_warning, side_of_evidence, sweep,
        sweep_stored, sweep_with, validates,
    };
    use super::{
        Consistency, Horizon, consistency_of, evaluator, grid, grid_step_ppm, ladder_for,
        stop_ladder_ppm,
    };
    use super::{Direction, Side};
    use super::{
        MAX_STOP_POINTS, NIFTY_REFERENCE, PAISA_PER_POINT, STOP_FLOOR_POINTS, hundredths_of,
        points_to_ppm, points_to_ppm_at, ppm_to_points_at, reference_price,
        return_over_drawdown_cell, stop_floor_points, synthetic, top_at,
    };
    use super::{cadence_floor_ppm, months_between, support_ladder};

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    /// THE ONE SENTENCE THIS BINARY MUST NEVER STOP PRINTING.
    ///
    /// A report over generated bars is byte-identical in shape to one over real
    /// bars, so the provenance line is the only thing standing between a reader
    /// and a number that means nothing. `CLAUDE.md` §4 bans a failure wearing a
    /// success's clothes; a confident sweep over invented data is exactly that,
    /// and this is the assertion that keeps it labelled.
    #[test]
    fn the_report_always_declares_its_bars_are_generated() {
        for text in [sweep(1, 1), auto(1)] {
            assert!(
                text.starts_with(PROVENANCE),
                "the provenance banner leads every render:\n{text}"
            );
            assert!(text.contains("not a backtest"), "and says so in words");
        }
    }

    #[test]
    fn a_valid_sweep_and_a_valid_auto_both_render_and_exit_zero() {
        let mut out = String::new();
        assert_eq!(run(&argv(&["sweep", "1", "1"]), &mut out), OK);
        assert!(out.contains("BARS"), "the report body rendered:\n{out}");

        let mut out = String::new();
        assert_eq!(run(&argv(&["auto", "1"]), &mut out), OK);
        assert!(!out.is_empty(), "auto rendered something");
    }

    /// Every refusal NAMES what was wrong and prints the usage.
    ///
    /// `CLAUDE.md` §4 requires a refusal to name its reason; "usage:" alone
    /// leaves the operator to work out which of their words was objected to.
    #[test]
    fn every_refusal_names_its_reason_and_prints_the_usage() {
        let cases: [(&[&str], &str); 7] = [
            (&[], "no command given"),
            (&["backtest"], "backtest"),
            (&["sweep", "0", "1"], "SESSIONS is outside"),
            (&["sweep", "3651", "1"], "SESSIONS is outside"),
            (&["sweep", "x", "1"], "SESSIONS is not a whole number"),
            (&["sweep", "1", "0"], "MIN_HITS must be 1 or more"),
            (&["sweep", "1", "x"], "MIN_HITS is not a whole number"),
        ];
        for (words, needle) in cases {
            let mut out = String::new();
            assert_eq!(
                run(&argv(words), &mut out),
                MISUSED,
                "{words:?} is a misuse, not a failure"
            );
            assert!(
                out.contains(needle),
                "{words:?} must be refused by name, expected {needle:?}:\n{out}"
            );
            assert!(
                out.contains(USAGE),
                "{words:?} must print the usage:\n{out}"
            );
        }
    }

    /// `auto` refuses a bad SESSIONS on its own arm, not only through `sweep`.
    #[test]
    fn auto_refuses_a_session_count_it_cannot_use() {
        let mut out = String::new();
        assert_eq!(run(&argv(&["auto", "0"]), &mut out), MISUSED);
        assert!(out.contains("SESSIONS is outside"), "named:\n{out}");
    }

    /// The wrong NUMBER of words is a misuse too, not a partial match.
    #[test]
    fn a_command_with_the_wrong_arity_is_refused_rather_than_guessed_at() {
        for words in [
            &["sweep"][..],
            &["sweep", "1"][..],
            &["sweep", "1", "1", "1"][..],
            &["auto"][..],
            &["auto", "1", "1"][..],
        ] {
            let mut out = String::new();
            assert_eq!(
                run(&argv(words), &mut out),
                MISUSED,
                "{words:?} does not match any command shape"
            );
        }
    }

    #[test]
    fn the_session_and_threshold_bounds_are_exactly_as_documented() {
        assert_eq!(parse_sessions("1"), Ok(1), "one session is the floor");
        assert_eq!(parse_sessions("3650"), Ok(3650), "ten years is the ceiling");
        assert!(parse_sessions("0").is_err());
        assert!(parse_sessions("3651").is_err());
        assert!(parse_sessions("-1").is_err());
        assert!(parse_sessions("").is_err());

        assert_eq!(parse_min_hits("1"), Ok(1));
        assert_eq!(parse_min_hits("18446744073709551615"), Ok(u64::MAX));
        assert!(
            parse_min_hits("0").is_err(),
            "zero would disable extinction, and the ladder's own clamp would \
             hide that the operator asked for it"
        );
        assert!(parse_min_hits("-1").is_err());
    }

    /// The refusal arm of the evaluator, which is why the widths are a parameter.
    #[test]
    fn absent_tolerances_are_refused_by_name_rather_than_defaulted() {
        assert!(evaluator_from(None).is_err(), "no widths, no evaluator");
        assert_eq!(
            evaluator_from(None).err(),
            Some("the pinned tolerances are not valid")
        );
        assert!(
            evaluator_from(indicators::evaluator::Widths::pinned().ok()).is_ok(),
            "and the shipped tolerances do build one"
        );
    }

    /// THE WHOLE INSTITUTIONAL STACK RENDERS — NOTHING IS NAMED ABSENT ANY MORE.
    ///
    /// # What this test used to assert, and why the change is the point
    ///
    /// It was `the_audit_names_the_stages_it_did_not_render`, and it asserted the
    /// report contained `NOT SUPPLIED` — because the walk-forward, PBO and the
    /// bootstrap were all passed as `None` and §4 requires an unsupplied stage to
    /// be NAMED rather than rendered as a zero. That was the right assertion for
    /// a build that could not drive them.
    ///
    /// All three are driven now, so the same assertion inverted is the honest
    /// one: every section must carry a real figure, and `NOT SUPPLIED` must not
    /// appear anywhere. A test still demanding the old string would be pinning a
    /// gap as though it were a feature.
    #[test]
    fn the_audit_renders_every_stage_of_the_institutional_stack() {
        // BOUNDED, AND THE BOUND IS THE POINT. This called `audit_run(12, 300)`
        // and inherited `engine::DEFAULT_CEILING` -- 134,217,728 candidates --
        // in a debug build. Measured at over an HOUR before it was killed, so
        // `cargo test --workspace` never terminated and section 9's green-suite
        // requirement was unverifiable. A genuinely red test
        // (`join_answer_is_unchanged`, stale since `a12192b`) sat behind it
        // undetected the whole time.
        //
        // This test asserts that every SECTION renders. It does not need a deep
        // ladder to do that. `crates/runner/tests/join_answer_is_unchanged.rs`
        // bounds its own fixture at 50,000 and finishes in 0.04s; this does the
        // same, and the sections below still have to render or it fails.
        let text = audit_run_within(12, 300, 50_000);
        assert!(text.starts_with(PROVENANCE), "provenance leads it too");
        assert!(text.contains("BARS"), "the sweep report is still there");
        for section in [
            "TRADES",
            "EXIT GRID",
            "WALK-FORWARD",
            "OVERFITTING",
            "BOOTSTRAP",
        ] {
            assert!(
                text.contains(section),
                "{section} must be rendered, not skipped:\n{text}"
            );
        }
        assert!(
            !text.contains("NOT SUPPLIED"),
            "every stage is driven now, so nothing may be named absent:\n{text}"
        );
    }

    /// A threshold nothing can meet reaches the audit and says so plainly.
    #[test]
    fn an_audit_with_no_closed_combination_says_that_is_extinction() {
        let text = audit_run(1, u64::MAX);
        assert!(
            text.contains("nothing to trade"),
            "an empty answer must be distinguishable from an unmeasured one:\n{text}"
        );
        assert!(
            text.contains("extinction, not a failure"),
            "and must say which it is"
        );
    }

    /// A sweep whose threshold nothing can meet still renders, and says so.
    ///
    /// The failure this guards is the one `runner::Outcome` documents: an empty
    /// answer must be distinguishable from an unmeasured one.
    #[test]
    fn an_impossible_threshold_still_renders_a_report_rather_than_nothing() {
        let text = sweep(1, u64::MAX);
        assert!(text.starts_with(PROVENANCE));
        assert!(text.contains("BARS"), "the census is still shown:\n{text}");
    }

    /// AN EVALUATOR THAT CANNOT BE BUILT IS REFUSED BY NAME, NOT RENDERED EMPTY.
    ///
    /// `evaluator()` cannot fail in this build, so this arm is unreachable
    /// through the public entry points and is driven here directly. It exists
    /// because a later widening of the tolerance table would reach it, and a
    /// refusal that printed an empty report would be indistinguishable from a
    /// sweep that found nothing — the confusion `runner::Outcome` documents.
    #[test]
    fn a_sweep_without_an_evaluator_refuses_by_name_rather_than_rendering() {
        for text in [
            sweep_with(Err("the pinned tolerances are not valid"), 1, 1),
            auto_with(Err("the pinned tolerances are not valid"), 1),
        ] {
            assert!(text.starts_with("refused: "), "named a refusal: {text}");
            assert!(text.contains("pinned tolerances"), "and why: {text}");
            assert!(
                !text.contains("BARS"),
                "a refusal must NOT render a census that would read as a sweep \
                 finding nothing: {text}"
            );
        }
    }

    /// EVERY FEED WORD THE STORE CAN WRITE IS A WORD THIS COMMAND ACCEPTS.
    ///
    /// Derived from `Vendor::ALL` rather than listed, so a vendor appended to the
    /// enum is addressable here the same day. A hand-written list is exactly how a
    /// feed becomes unpullable from the command line while the store happily holds
    /// its bars — the shape `CLAUDE.md` §4 calls a fallback that hides a failure,
    /// because the refusal would name the word rather than the missing arm.
    #[test]
    fn every_feed_the_store_can_write_is_a_feed_this_command_accepts() {
        for v in Vendor::ALL {
            assert_eq!(
                parse_vendor(v.as_str()),
                Ok(v),
                "{} is a store prefix and must be addressable",
                v.as_str()
            );
        }
        // And an unknown word is refused by NAME, listing what would have worked.
        let why = parse_vendor("bogus").expect_err("`bogus` is not a feed");
        assert!(why.contains("bogus"), "the refusal names the word: {why}");
        for v in Vendor::ALL {
            assert!(
                why.contains(v.as_str()),
                "and lists {} as an alternative: {why}",
                v.as_str()
            );
        }
    }

    /// THE STORE ROOT HAS THREE OUTCOMES AND THE THIRD IS A REFUSAL, NOT A GUESS.
    ///
    /// A default of `./store` or of `/` would be a fallback that hides a failure:
    /// the sweep would open nothing, report zero bars, and read as an instrument
    /// with no history rather than as a machine with no `HOME`.
    #[test]
    fn the_store_root_prefers_the_override_and_refuses_when_it_has_neither() {
        assert_eq!(
            root_from(Some("/tmp/elsewhere".into()), Some("/Users/x".into())),
            Ok(std::path::PathBuf::from("/tmp/elsewhere")),
            "the explicit override wins over HOME"
        );
        assert_eq!(
            root_from(None, Some("/Users/x".into())),
            Ok(std::path::PathBuf::from("/Users/x/.brutex/store")),
            "and HOME is the fallback, at the workspace's own path"
        );
        let why = root_from(None, None).expect_err("neither is set");
        assert!(
            why.contains("BRUTEX_STORE"),
            "the refusal names both: {why}"
        );
        assert!(why.contains("HOME"), "the refusal names both: {why}");
    }

    /// A MONTH THE STORE DOES NOT HOLD IS A REFUSAL THAT SAYS WHAT TO DO NEXT.
    ///
    /// The operator's next action is a pull, not a filesystem check, so the
    /// refusal says so. It also must not render a census: a sweep of a month that
    /// was never pulled and a sweep that found nothing are different statements,
    /// and only one of them is about the market.
    #[test]
    fn a_month_the_store_does_not_hold_is_refused_and_renders_no_census() {
        let text = sweep_stored("groww", "NIFTY", "1min", 1970, 1, 1);
        assert!(text.starts_with("refused: "), "named a refusal: {text}");
        assert!(
            !text.contains("BARS"),
            "a refusal must not render a census that would read as an empty \
             market: {text}"
        );
    }

    /// THE SIDE FOLLOWS THE EVIDENCE, AND A FALL IS SOLD RATHER THAN BOUGHT.
    ///
    /// # The defect, and why the better ranker made it worse
    ///
    /// `crate::rank` orders by |t| on purpose — a combination that reliably
    /// precedes a FALL is as tradeable as one that precedes a rise, and ranking
    /// on a signed `t` would discard every short setup. The sign lives in
    /// `Edge::mean_paisa`, which `rank`'s header says is carried *"for a reader
    /// to see"*.
    ///
    /// Nothing read it. The audit walked `Direction::Long` unconditionally, so
    /// the strongest DOWNWARD signal was bought, and the P&L, the 125-cell grid,
    /// the walk-forward, the PBO and all three bootstrap p-values described the
    /// wrong side of it.
    ///
    /// And selecting for the largest |t| — which `6d849ee` introduced as an
    /// improvement over an arbitrary pick — selects precisely the strongest
    /// signals of EITHER sign. The better the ranker got, the more often the
    /// side was wrong.
    #[test]
    fn the_traded_side_follows_the_sign_of_the_evidence() {
        use runner::outcome::Edge;
        use runner::rank::Scored;

        let scored = |mean: f64| Scored {
            // `Default::default()` and not the named path, for the reason every
            // other mask literal in this crate gives: spelling `ConditionMask`
            // needs a `vocab` arrow §5 does not draw for `cli`.
            #[expect(
                clippy::default_trait_access,
                reason = "the named path would add a dependency arrow §5 does not draw"
            )]
            mask: Default::default(),
            hits: 100,
            edge: Edge {
                n: 100,
                mean_paisa: mean,
                mismatched: 0,
                refused: 0,
                t: mean,
                // The payoff split is not what this fixture is about, and
                // spreading the default keeps a later field from breaking it.
                ..Edge::default()
            },
        };

        assert_eq!(
            side_of_evidence(&scored(250.0)),
            Side::Long,
            "a combination that precedes a RISE is bought"
        );
        assert_eq!(
            side_of_evidence(&scored(-250.0)),
            Side::Short,
            "a combination that precedes a FALL is SOLD — buying it is the \
             defect this test exists for, and it is the case a Long-only audit \
             got wrong on exactly half its strongest candidates"
        );

        // EXACTLY ZERO IS LONG, and it is a stated convention rather than an
        // accident: a zero mean gives t = 0, which cannot clear any bar, so
        // which way it is taken changes no verdict.
        assert_eq!(side_of_evidence(&scored(0.0)), Side::Long);

        // AND THE TWO ENUMS MAP CONSISTENTLY. Two types name one concept —
        // `Side` for excursions, `Direction` for fills — and a call site that
        // mapped them the wrong way round would price a long as a short with
        // nothing else in the report disagreeing.
        assert!(matches!(direction_of(Side::Long), Direction::Long));
        assert!(matches!(direction_of(Side::Short), Direction::Short));
    }

    /// "EXTINCTION" IS CLAIMED ONLY WHEN THE SWEEP ACTUALLY FOUND NOTHING.
    ///
    /// # Two facts wore one sentence
    ///
    /// The audit's no-trade branch printed *"This is extinction, not a
    /// failure"* unconditionally. Its input is the strongest [`audit_keep()`] by
    /// |t| intersected with the closed set, which is empty either because the
    /// sweep found nothing — genuine extinction, and §6's expected answer — or
    /// because none of the top 250 happened to be closed on a sweep that found
    /// millions.
    ///
    /// The second is not extinction, and claiming it is a statement about the
    /// market the code cannot support. [`audit_keep()`]'s own doc predicted this
    /// and the constant was raised to make it unlikely while the message was
    /// left covering both cases.
    #[test]
    fn extinction_is_claimed_only_when_nothing_was_found() {
        let extinct = nothing_to_trade(0);
        assert!(
            extinct.contains("extinction, not a failure"),
            "a genuinely empty sweep IS extinction and must say so: {extinct}"
        );

        let plenty = nothing_to_trade(3_689);
        assert!(
            plenty.contains("NOT extinction"),
            "a sweep that kept 3,689 combinations did not go extinct: {plenty}"
        );
        assert!(
            plenty.contains("3689") || plenty.contains("3,689"),
            "and the refusal names how many it did keep: {plenty}"
        );
        assert!(
            plenty.contains("Raise"),
            "§4 asks a refusal to say what to change, not merely to decline: \
             {plenty}"
        );
        assert!(
            !plenty.contains("extinction, not a failure"),
            "the two messages must not both fire — that is the defect: {plenty}"
        );
    }

    /// A THIN SAMPLE IS SAID SO, AND A SUFFICIENT ONE IS NOT NAGGED ABOUT.
    ///
    /// # The defect this closes, which is about an impression rather than a number
    ///
    /// `audit-stored` is scoped to one instrument-month — about twenty trading
    /// days. On that series the audit still runs a five-fold walk-forward, a PBO
    /// over the resulting placements, and three stationary-block bootstraps at a
    /// block length of ten. A draw is then one or two blocks and a fold tests on
    /// a handful of days.
    ///
    /// None of those figures was *wrong*. What was wrong is that they rendered
    /// in **exactly the same format** as figures computed over 3,650 generated
    /// sessions, with nothing on the page to tell a reader which they were
    /// holding. `CLAUDE.md` §3 rule 6 asks for an unmeetable bound to be said out
    /// loud; nothing said it.
    ///
    /// # Why the boundary is asserted from both sides
    ///
    /// A warning that always fires is noise a reader learns to skip, which makes
    /// it worse than none. So the sufficient case must be silent, and the empty
    /// string is asserted rather than assumed.
    #[test]
    fn a_thin_sample_is_named_and_a_sufficient_one_says_nothing() {
        // SUFFICIENT: silent, exactly at the boundary and above it.
        assert!(
            sample_warning(MIN_AUDIT_SESSIONS).is_empty(),
            "the boundary itself is sufficient; a warning here would fire on \
             every adequate run and teach the reader to ignore it"
        );
        assert!(sample_warning(MIN_AUDIT_SESSIONS + 1_000).is_empty());

        // THIN: named, with the two numbers that decide it.
        let thin = sample_warning(20);
        assert!(thin.contains("THIN"), "the verdict is stated: {thin}");
        assert!(
            thin.contains("sessions 20"),
            "and the sample it is a verdict about: {thin}"
        );
        assert!(
            thin.contains("p-values") && thin.contains("PBO"),
            "naming WHICH figures are affected is the whole point — a blanket \
             warning would also discredit the trades, which are fine: {thin}"
        );
        assert!(
            thin.contains("unaffected"),
            "and which figures are not affected, so the reader does not discard \
             the honest half: {thin}"
        );

        // ZERO SESSIONS MUST NOT PANIC. The divisors are constants here, but a
        // future change to either could make one zero, and this is the arm that
        // would catch a division by it.
        assert!(sample_warning(0).contains("THIN"));
    }

    /// THE LOG DIRECTORY IS DECIDED WITHOUT TOUCHING THE ENVIRONMENT.
    ///
    /// # Why the decision is split out from `install_log`
    ///
    /// `telemetry::install` writes a process-wide `OnceLock` and refuses a
    /// second call, so `install_log` can succeed at most once per test binary —
    /// a test that drove it would poison every later test in the same process,
    /// and which test won would depend on thread scheduling. `root_from` is
    /// split from `store_root` for the identical reason and says so.
    ///
    /// So the environment reading stays in `install_log`, which
    /// `tests/binary.rs` exercises by running the real binary, and the DECISION
    /// is tested here where it can be driven directly.
    #[test]
    fn the_log_directory_follows_the_store_unless_it_is_named_outright() {
        use std::ffi::OsString;
        use std::path::PathBuf;

        // AN EXPLICIT DIRECTORY WINS, and it wins even when a store exists —
        // otherwise an operator who set the variable would silently get the
        // store's `logs` instead of the path they named.
        assert_eq!(
            log_dir_from(
                Some(OsString::from("/tmp/brutex-events")),
                Some(PathBuf::from("/srv/store")),
            ),
            Some(PathBuf::from("/tmp/brutex-events")),
        );

        // WITH NO VARIABLE, THE LOG SITS UNDER THE STORE, IN ITS OWN
        // SUBDIRECTORY. Beneath the store and not the working directory,
        // because a sweep started from `/` or from a read-only checkout must
        // still write somewhere and the store root is already required to be
        // writable.
        //
        // `logs/cli` and not `logs`, and the child is the whole point:
        // `telemetry::sink::BASENAME` is a constant, so every writer pointed at
        // one directory appends to one `events.ndjson`. `api` resolves
        // `<store>/logs` when it is configured that way, and a long `range-all`
        // running beside a live server was two processes appending to one file.
        // A directory each removes the race with no lock and no coordination,
        // which is the only shape that stays O(1) per event with two writers.
        assert_eq!(
            log_dir_from(None, Some(PathBuf::from("/srv/store"))),
            Some(PathBuf::from("/srv/store/logs/cli")),
            "the CLI owns a child directory so it cannot interleave with the \
             server's file"
        );

        // WITH NEITHER, THERE IS NOWHERE TO WRITE AND THAT IS `None`, not a
        // guess at the working directory. `install_log` turns this into the
        // printed "events are NOT being recorded" line rather than a silent
        // absence — degrade loudly, per §4.
        assert_eq!(log_dir_from(None, None), None);

        // AND AN EXPLICIT DIRECTORY STILL WINS WITH NO STORE AT ALL, which is
        // the case for an operator who has moved the store away entirely.
        assert_eq!(
            log_dir_from(Some(OsString::from("/var/log/brutex")), None),
            Some(PathBuf::from("/var/log/brutex")),
        );
    }

    /// THE REAL-DATA AUDIT REFUSES FOR THE SAME CAUSE AS THE SWEEP, AND NAMES ITSELF.
    ///
    /// # What this can reach, stated because it decides every assertion below
    ///
    /// `cargo test` builds without `BRUTEX_COMMIT`, so `commit_stamp()` is `None`
    /// and **both commands refuse at the commit gate before the store is touched
    /// at all**. That is correct — §3 rule 3 forbids a computation whose identity
    /// cannot be recorded, so the gate must come first — but it means the
    /// fixtures below do NOT exercise the vendor parse, the store root or the
    /// month lookup.
    ///
    /// Saying so matters. The audit that prompted `audit-stored` found exactly
    /// this illusion in `sweep-stored`'s own test: it "passes because the commit
    /// gate short-circuits before any of the code under test runs". A test that
    /// looks like it covers the store path and does not is worse than an absent
    /// one, so this asserts only what an unstamped build genuinely reaches.
    ///
    /// # Why identical wording is the WRONG property to assert
    ///
    /// The two refusals differ by one word — "the sweep will not run" against
    /// "the audit will not run" — and that difference is deliberate. §4 requires
    /// a refusal to say what was refused, and an operator who ran `audit-stored`
    /// should not be told about a sweep. An earlier version of this test asserted
    /// string equality and failed on precisely that improvement, which is a test
    /// pinning a defect. So the assertions are on the CAUSE and on the
    /// self-naming, both of which stay true as the wording changes.
    #[test]
    fn the_stored_audit_refuses_for_the_same_cause_and_names_itself() {
        // EVERY GATE THESE TWO COMMANDS CAN STOP AT, NAMED. A refusal matching
        // none of them is a gate this test has not learned, and it panics below
        // rather than passing quietly — which is the failure mode this whole
        // test was in.
        // MATCHED ON THE CAUSE, NOT ON A SENTENCE, and one of these has already
        // drifted once.
        //
        // `"is not in the store"` was the third entry, and it stopped matching
        // when `stored::load`'s refusal was rewritten to name the LOCK case
        // separately -- the old wording said "is not in the store … Pull that
        // instrument-month first" for all seven `open_existing` failure modes
        // including `Locked`, which told an operator to pull a month they
        // already had while a writer held it. The replacement opens with
        // "could not be read from the store" and then names which cause it was.
        //
        // The rewrite was right and this list was not updated with it, so the
        // test went red and STAYED red: it lives in the `cli` suite, which
        // contains an end-to-end sweep slow enough that the suite is rarely run
        // to completion, and nothing else covers this. A gate list that names
        // exact prose is a second copy of a message, and CLAUDE.md S5 is about
        // exactly that shape -- two copies of one fact, correct the day they are
        // written. Kept as prose because the refusals carry no error type to
        // match on, but the entries are now the SHORTEST stable substring of
        // each cause rather than a fragment of one phrasing of it.
        const GATES: [&str; 3] = [
            "BRUTEX_COMMIT",
            "is not a feed this build knows",
            "could not be read from the store",
        ];
        for (vendor, underlying, rung, year, month) in [
            // A month no store holds.
            ("groww", "NIFTY", "1min", 1970_u16, 1_u8),
            // A feed word no build knows.
            ("nosuchfeed", "NIFTY", "1min", 2026, 8),
        ] {
            let swept = sweep_stored(vendor, underlying, rung, year, month, 500);
            let audited = audit_stored(vendor, underlying, rung, year, month, 500);

            for (label, text) in [("sweep-stored", &swept), ("audit-stored", &audited)] {
                assert!(
                    text.starts_with("refused: "),
                    "{label} must refuse ({vendor}, {underlying}, {rung}, \
                     {year}-{month}), or this test proves nothing: {text}"
                );
                assert!(
                    !text.contains("BARS"),
                    "{label}: a refusal must not render a census that would read \
                     as an empty market: {text}"
                );
            }

            // THE SAME CAUSE, AND WHICH CAUSE DEPENDS ON HOW THE BINARY WAS
            // BUILT.
            //
            // # This test used to pass only by accident
            //
            // It asserted that both refusals cite `BRUTEX_COMMIT`, under a
            // comment reading *"on this unstamped build that cause is the commit
            // gate"*. That is true of a build with no stamp — and CI produces
            // one, so the suite was green. It is FALSE of the build the usage
            // text tells an operator to make:
            //
            //     BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release
            //
            // With the stamp present the commit gate passes, both commands reach
            // the store, and both refuse for the missing month instead. So the
            // test asserted a real property in exactly the configuration where
            // the feature it guards is switched OFF, and failed in the one an
            // operator actually uses. MEASURED both ways.
            //
            // What is asserted now is the property itself: whatever gate fires,
            // BOTH commands must stop at the SAME one. An `audit-stored` that
            // reached further than `sweep-stored` would compute before it could
            // record an identity, and that is what this exists to refuse.
            let gate = GATES
                .into_iter()
                .find(|g| swept.contains(g))
                .unwrap_or_else(|| {
                    panic!(
                        "the sweep refused for a cause this test does not know. \
                         Add it to GATES and say why it is a gate:\n  {swept}"
                    )
                });
            assert!(
                audited.contains(gate),
                "both stored commands must stop at the SAME gate. The sweep \
                 stopped at `{gate}` and the audit did not:\n  sweep: {swept}\n  \
                 audit: {audited}"
            );

            // AND EACH NAMES ITSELF — ON THE GATE THAT CARRIES A NAME.
            //
            // The commit gate's refusal differs by one word, "the sweep will not
            // run" against "the audit will not run", and that difference is the
            // half that would silently rot if the two were ever merged into one
            // shared constant.
            //
            // **The store's refusal names NEITHER**, and that is a §4 gap rather
            // than a property this test may quietly drop: an operator who ran
            // `audit-stored` is told only *"NIFTY 1min 1970-01 is not in the
            // store"*, with nothing saying which command refused. Closing it
            // means threading the command's name into `load_span`'s refusal,
            // which is a change to production text and belongs in its own commit
            // with its own entry. It is named here so the next reader finds it
            // stated rather than absent.
            if gate == "BRUTEX_COMMIT" {
                assert!(
                    swept.contains("the sweep will not run"),
                    "the sweep's refusal must name the sweep: {swept}"
                );
                assert!(
                    audited.contains("the audit will not run"),
                    "the audit's refusal must name the audit, not the sweep: \
                     {audited}"
                );
            }
        }
    }

    /// AND IT IS REACHABLE FROM THE COMMAND LINE, not merely defined.
    ///
    /// `cli` dispatches on a hand-written slice match rather than a derive, so a
    /// function can exist, compile, be tested directly, and still be
    /// unreachable because no arm names it — which is the shape of the
    /// unreachability D-0169 was written to close. This drives `run` by its
    /// argv, so the arm itself is what is under test.
    #[test]
    fn the_threshold_search_is_reachable_on_stored_bars_and_discoverable() {
        // THE GAP THIS CLOSES. `auto` bisects for the deepest threshold a
        // machine can finish and is the only dynamic thing in the whole surface.
        // It took a count of SYNTHETIC sessions. Every command that touched the
        // store took the threshold as an ARGUMENT, so on real data an operator
        // guessed a number, watched it refuse or grind, and guessed again. The
        // self-tuning capability existed, was tested, and worked only on data
        // that does not exist.
        //
        // Asserted: the arm is REACHED and parses, and it appears in the usage.
        // NOT the threshold it chooses -- that is a property of which months are
        // on the machine running the test.
        let mut out = String::new();
        let code = run(
            &argv(&[
                "auto-stored",
                "zerodha",
                "NIFTY",
                "60min",
                "2024",
                "1",
                "2024",
                "1",
            ]),
            &mut out,
        );
        assert!(
            !out.contains("not a command this build knows"),
            "the arm must be reached, not fall through to the usage refusal: {out}"
        );
        assert!(
            code == OK || code == MISUSED,
            "it either searched or refused for a stated reason: {code}"
        );

        // A non-numeric date is a refusal, not a parse panic.
        let mut bad = String::new();
        assert_eq!(
            run(
                &argv(&[
                    "auto-stored",
                    "zerodha",
                    "NIFTY",
                    "60min",
                    "not-a-year",
                    "1",
                    "2024",
                    "1"
                ]),
                &mut bad
            ),
            MISUSED
        );
        assert!(bad.contains("whole numbers"), "{bad}");

        assert!(
            USAGE.contains("auto-stored"),
            "a command an operator cannot discover does not exist for them -- \
             and this one exists to stop them typing a threshold they should \
             never have had to choose"
        );
    }

    #[test]
    fn the_stored_audit_is_reachable_from_argv_and_is_in_the_usage() {
        let mut out = String::new();
        let code = run(
            &argv(&["audit-stored", "groww", "NIFTY", "1min", "1970", "1", "500"]),
            &mut out,
        );
        assert_eq!(
            code, MISUSED,
            "a month the store does not hold is a misuse, not a success: {out}"
        );
        assert!(
            out.starts_with("refused: "),
            "the arm reached the command rather than falling through to \
             `not a command this build knows`: {out}"
        );
        assert!(
            USAGE.contains("audit-stored"),
            "a command an operator cannot discover is a command that does not \
             exist for them"
        );
        // THE FEED THE PARSER ACCEPTS AND THE USAGE OMITTED. `parse_vendor`
        // takes `zerodha`; the usage listed four feeds and not that one, so an
        // operator with zerodha bars on disk would read it and conclude the
        // store could not be swept for them.
        assert!(
            USAGE.contains("zerodha"),
            "every feed the parser accepts must appear in the usage"
        );
    }

    /// THE THREE NUMBERS ARE PARSED SEPARATELY AND EACH NAMES ITSELF WHEN WRONG.
    ///
    /// One shared "bad arguments" message would leave an operator comparing six
    /// words against six meanings to find which one this build objected to.
    #[test]
    fn each_numeric_argument_of_the_stored_sweep_refuses_in_its_own_words() {
        let cases = [
            (
                ["sweep-stored", "groww", "NIFTY", "1min", "YEAR", "8", "500"],
                "YEAR",
            ),
            (
                [
                    "sweep-stored",
                    "groww",
                    "NIFTY",
                    "1min",
                    "2026",
                    "MONTH",
                    "500",
                ],
                "MONTH",
            ),
            (
                [
                    "sweep-stored",
                    "groww",
                    "NIFTY",
                    "1min",
                    "2026",
                    "8",
                    "HITS",
                ],
                "MIN_HITS",
            ),
        ];
        for (words, wanted) in cases {
            let args: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
            let mut out = String::new();
            let code = run(&args, &mut out);
            assert_eq!(code, MISUSED, "a misparsed argument is a misuse: {out}");
            assert!(
                out.contains(wanted),
                "the refusal must name {wanted}, not the other two: {out}"
            );
            assert!(out.contains("usage:"), "and print the usage: {out}");
        }
    }

    /// THE USAGE NAMES THE COMMAND, OR AN OPERATOR CANNOT FIND IT.
    ///
    /// `cli` has no `--help`; the usage IS the help, printed on every refusal. A
    /// command absent from it is a command nobody discovers.
    #[test]
    fn the_stored_sweep_appears_in_the_usage_with_its_arguments() {
        assert!(USAGE.contains("sweep-stored"), "the command is listed");
        for word in ["VENDOR", "UNDERLYING", "RUNG", "YEAR", "MONTH"] {
            assert!(USAGE.contains(word), "{word} is explained in the usage");
        }
    }

    /// THE TWO PROVENANCE BANNERS CANNOT BE MISTAKEN FOR ONE ANOTHER.
    ///
    /// A sweep over generated bars and one over real bars are byte-identical in
    /// shape, so the banner is the only thing separating them. If both said the
    /// same words, a generated run could be read as evidence about a market --
    /// the failure wearing a success's clothes that `CLAUDE.md` §5 bans.
    #[test]
    fn the_generated_and_stored_banners_make_opposite_claims() {
        assert!(PROVENANCE.contains("GENERATED"), "one says generated");
        assert!(
            STORED_PROVENANCE.contains("REAL MARKET DATA"),
            "one says real"
        );
        assert_ne!(PROVENANCE, STORED_PROVENANCE);
        assert!(
            !STORED_PROVENANCE.contains("not a backtest"),
            "the real banner must not carry the generated one's disclaimer"
        );
    }

    /// EVERY KNOB THAT MOVES THE ANSWER MOVES THE IDENTITY.
    ///
    /// # Two runs with different answers shared one name
    ///
    /// `Params::of(ladder)` records the threshold and the two budgets — what the
    /// SWEEP did — and nothing about what was done with its output.
    /// `MAX_POINTS`, the ranking lens, the grid width and the screen cap each
    /// change which combination is TRADED and what P&L is recorded, and none of
    /// them reaches a `Ladder`. So two runs that computed different answers
    /// hashed to the same `RunId`, and the results ledger — which refuses a
    /// duplicate identity — handed back whichever landed first.
    ///
    /// This asserts on [`policy_of`], the list `screen_range_inner` folds in, so
    /// a knob added to the run and forgotten here is a knob this test cannot see.
    /// The ORDER is the contract: `with_policy` hashes the slice in order, so a
    /// new choice is APPENDED and never inserted.
    #[test]
    fn the_ceiling_is_derived_from_this_machine_and_not_from_an_assumed_one() {
        /// `ceiling_from_env`'s own figure for one retained candidate.
        const BYTES_PER_CANDIDATE: usize = 146;
        /// What one core can be assumed to back, on any machine.
        const MAX_BYTES_PER_CORE: usize = 2 * 1024 * 1024 * 1024;

        // `engine::DEFAULT_CEILING` is sized in that crate's own table against
        // "a 48 GB machine" -- a static fact about somebody else's hardware
        // deciding how far this ladder may walk before it halts. The derivation
        // scales it by the parallelism of the machine actually running.
        let derived = crate::derived_ceiling();

        assert!(
            derived > 0,
            "a ceiling of zero halts before the first candidate"
        );

        // NEVER BELOW WHAT ONE CORE EARNS. A machine that will not answer keeps
        // the reference rather than guessing downward -- an unknown machine is
        // not a small one, and halving the ladder on a failed query would
        // narrow the search silently.
        // THE FLOOR IS PER SWEEP, AND SWEEPS SHARE THE MACHINE.
        //
        // `derived_ceiling` divides the machine budget by
        // `SWEEPS_SHARING_THIS_MACHINE`, which `range_over` raises while its
        // eight rungs are in flight -- so this assertion is only about a
        // SINGLE sweep and must say which one it is asking about. It asserted
        // the whole-machine floor and passed alone while failing in the suite,
        // because another test in the same binary had a `SharedBy` alive.
        //
        // That is not a test defect, it is the test finding a real property:
        // the divisor is process-global, so any concurrent sweep in one process
        // lowers every other sweep's ceiling. The server serialises sweeps
        // behind one busy slot today, which is the only reason production does
        // not see it.
        let sharing = crate::SWEEPS_SHARING_THIS_MACHINE.load(std::sync::atomic::Ordering::Relaxed);
        let per_core = engine::DEFAULT_CEILING / 14 / sharing.max(1);
        assert!(
            derived >= per_core,
            "derived {derived} is below the single-core floor {per_core} at a \
             share of {sharing}"
        );

        // THE BYTES IT IMPLIES MUST STAY INSIDE WHAT A MACHINE CAN HOLD.
        //
        // This is the assertion that would have caught the unit error, and
        // nothing else here would have: dividing by 10 while multiplying by
        // `available_parallelism` passed every structural check above and
        // produced 187,904,808 candidates on the reference machine -- 27 GB of
        // its 48, against a calibration of 19.6 GB and a documented swap at
        // 39.2 GB.
        //
        // 146 bytes per candidate is `ceiling_from_env`'s own figure. The bound
        // is stated per CORE so it holds on any machine: a candidate allowance
        // that needs more than 2 GB per core to hold is one no ordinary machine
        // has the memory to back, whatever its core count.
        let bytes_per_core = per_core.saturating_mul(BYTES_PER_CANDIDATE);
        assert!(
            bytes_per_core <= MAX_BYTES_PER_CORE,
            "the per-core allowance implies {bytes_per_core} bytes, over the \
             {MAX_BYTES_PER_CORE} a core can be assumed to back. REFERENCE_CORES \
             must be in the unit `available_parallelism` answers in"
        );

        // AND IT IS A FUNCTION OF THE MACHINE, not a constant wearing a
        // function's clothes. On the ten-core machine the constant was
        // calibrated against the two are equal by construction; anywhere else
        // they differ, and asserting equality either way would pin the test to
        // one machine. What holds everywhere is that the derivation is the
        // per-core allowance times the cores this machine reports.
        let cores = std::thread::available_parallelism().map_or(14, std::num::NonZero::get);
        assert_eq!(
            derived,
            per_core.saturating_mul(cores).max(per_core),
            "the ceiling must be the per-core allowance scaled by THIS machine's \
             {cores} core(s)"
        );
    }

    #[test]
    fn every_knob_that_moves_the_answer_moves_the_identity() {
        let bars = runner::synthetic::sessions(2);
        let base = crate::Rules::elite(points_to_ppm(10), 25);
        let lens = runner::rank::Lens::Detectability;
        let start = policy_of(&bars, base, lens, true);

        // ONE KNOB AT A TIME, each against the same baseline.
        let mut wider = base;
        wider.max_mae_ppm = base.max_mae_ppm.saturating_add(1);
        assert_ne!(
            policy_of(&bars, wider, lens, true),
            start,
            "MAX_POINTS decides which variants the operator's stop admits"
        );
        assert_ne!(
            policy_of(&bars, base, runner::rank::Lens::Payoff, true),
            start,
            "the lens decides which combination is TRADED"
        );
        assert_ne!(
            policy_of(&bars, base, lens, false),
            start,
            "an unvalidated screen is a different computation from a validated one"
        );

        // AND THE SLICE IS THE WHOLE CONTRACT. A knob appended without being
        // hashed would leave this length unchanged, and `with_policy` folds the
        // length first precisely so adding one re-keys.
        assert_eq!(
            start.len(),
            14,
            "fourteen choices are folded in. If this moved, `policy_of`'s doc \
             table and the append-never-insert rule both need reading before the \
             number is changed"
        );

        // EIGHT WERE APPENDED WHEN `Rules::operator()` MADE THEM VARIABLE, and
        // this tripwire is what forced the doc to be read first.
        //
        // While this path built its rules from `Rules::BASELINE` -- a `const` --
        // `max_mae_ppm` alone was a sufficient summary of them: nothing else
        // could differ between two runs, so nothing else could split them.
        // `Rules::operator()` reads seven environment variables, every one of
        // which changes which cell `best_within(|c| rules.admits(c))` selects
        // and therefore the recorded `pessimistic`, `trades` and exit rungs.
        // `BRUTEX_GRID_RESOLUTION` is the eighth: it moves `grid_step_ppm`, and
        // the `grid_rungs` term carries it only until `ppm_to_points_at` rounds
        // to zero and `.max(1)` pins the count while the step keeps shrinking.
        //
        // Asserted per-field rather than by length alone, because a length
        // check passes just as well if a field is folded TWICE and another not
        // at all.
        for (label, moved) in [
            (
                "min_rr_bp",
                crate::Rules {
                    min_rr_bp: base.min_rr_bp + 1,
                    ..base
                },
            ),
            (
                "min_win_rate_bp",
                crate::Rules {
                    min_win_rate_bp: base.min_win_rate_bp + 1,
                    ..base
                },
            ),
            (
                "min_assurance_bp",
                crate::Rules {
                    min_assurance_bp: base.min_assurance_bp + 1,
                    ..base
                },
            ),
            (
                "min_weakest_bp",
                crate::Rules {
                    min_weakest_bp: base.min_weakest_bp + 1,
                    ..base
                },
            ),
            (
                "min_ret_over_dd_bp",
                crate::Rules {
                    min_ret_over_dd_bp: base.min_ret_over_dd_bp + 1,
                    ..base
                },
            ),
            (
                "min_trades",
                crate::Rules {
                    min_trades: base.min_trades + 1,
                    ..base
                },
            ),
            (
                "top",
                crate::Rules {
                    top: base.top + 1,
                    ..base
                },
            ),
        ] {
            assert_ne!(
                policy_of(&bars, moved, lens, true),
                start,
                "{label} changes which cell is admitted, so two runs that differ \
                 in it are two answers and must not share a RunId"
            );
        }

        // THE SIXTH IS THE ONE D-0294 MISSED, and it is the knob that decides
        // whether the ladder explored the space at all.
        //
        // `BRUTEX_CEILING` is process-wide, so a test cannot set it without
        // changing it under every other test in this binary running in
        // parallel -- the same reason `root_from` exists beside `store_root`.
        // What IS assertable without touching the environment is that the
        // ceiling reaches the slice at all: fold the same inputs and require
        // the last term to be the ceiling the ladder would actually be built
        // with. A knob hashed as a constant would pass every `assert_ne` above
        // and fail here.
        let expected = crate::ceiling_from_env().map_or(u64::MAX, |ceiling| ceiling as u64);
        assert_eq!(
            start[5], expected,
            "the ceiling must reach the identity as the value the ladder uses, \
             not as a constant standing in for it"
        );
        assert_ne!(
            start[5], 0,
            "a ceiling folded in as zero is a knob hashed as an absence"
        );
    }

    /// AN UNVALIDATED PAGE SAYS SO, AND THE SWITCH THAT MAKES ONE IS EXPLICIT.
    ///
    /// The same argument as the two provenance banners, one layer in. A screen
    /// that skipped the walk-forward, the PBO and the bootstrap is byte-identical
    /// in shape to one that ran all three, so the banner is the only thing
    /// separating a CANDIDATE from a FINDING.
    ///
    /// The switch defaults ON and only a literal `0` turns it off: a typo must
    /// not silently buy a weaker answer, which is why a malformed value is not
    /// read as off.
    #[test]
    fn an_unvalidated_screen_cannot_be_mistaken_for_a_validated_one() {
        assert!(
            UNVALIDATED.contains("NOT VALIDATED"),
            "the banner must say so in words a reader cannot skim past"
        );
        assert!(
            UNVALIDATED.contains("CANDIDATES, not findings"),
            "and it must say what the rows ARE, not only what did not run"
        );
        assert!(
            UNVALIDATED.contains("BRUTEX_VALIDATE"),
            "and name the switch that undoes it, or the reader is stuck"
        );
        for absent in ["REAL MARKET DATA", "GENERATED"] {
            assert!(
                !UNVALIDATED.contains(absent),
                "the validation banner must not restate a PROVENANCE claim: it is \
                 a different question about the same page"
            );
        }

        // THE DEFAULT IS ON, AND ONLY `0` TURNS IT OFF.
        //
        // Asserted on `validates`, the same decision `validate_from_env` calls,
        // rather than by re-deriving the rule here — so a change to one is a
        // change to both. The lookup is not tested because it cannot be: this
        // crate is `#![forbid(unsafe_code)]` and `std::env::set_var` is unsafe.
        // Splitting the decision out is what made the rule provable at all.
        for (raw, want, why) in [
            (None, true, "absent means the full stack runs"),
            (Some("0"), false, "a literal zero is the only way off"),
            (Some(" 0 "), false, "and whitespace around it still counts"),
            (Some("1"), true, "any other value leaves it on"),
            (
                Some("no"),
                true,
                "including one that LOOKS like an off switch",
            ),
            (Some("false"), true, "and one that reads like one"),
            (Some(""), true, "and empty is not off"),
            (Some("00"), true, "and a near-miss is not off either"),
        ] {
            assert_eq!(validates(raw), want, "{why}");
        }
    }

    /// AND THE SEARCH ACTUALLY FINISHES, ON A COLUMN WITH BARS TO SWEEP.
    ///
    /// The const assertion beside `SEARCH_CEILING` is necessary and not
    /// sufficient: a ceiling low enough to probe cheaply is worthless if the rung
    /// it settles on cannot then be walked. This asserts the whole command returns
    /// a COMPLETE sweep, which `runner::Auto` refuses to do when every probe it
    /// tried was itself refused.
    #[test]
    fn the_threshold_search_returns_a_complete_sweep_rather_than_a_refusal() {
        let text = auto(6);
        assert!(
            !text.starts_with("refused: "),
            "the search refused a column it should tune: {text}"
        );
        assert!(
            text.contains("complete"),
            "a search that settles on a rung it cannot walk has not searched: {text}"
        );
    }

    /// THE RANGE COMMAND REFUSES A BAD ARGUMENT BEFORE IT REACHES THE STORE.
    ///
    /// # What an unstamped build can reach here, and what it cannot
    ///
    /// The four number parses in `audit_range_arm` run BEFORE `audit_range` is
    /// called, so they are reachable under `cargo test` even though the commit
    /// gate inside refuses immediately after — the same split
    /// `the_stored_audit_refuses_for_the_same_cause_and_names_itself` documents
    /// for `audit-stored`. These assertions therefore cover the parse arms and
    /// claim nothing about the store walk.
    #[test]
    fn the_range_command_names_which_argument_it_refused() {
        // 300 AND NOT 13, AND THE DIFFERENCE IS THE POINT. `MONTH` parses as
        // `u8`, so 13 succeeds here and the call reaches `audit_range`, which
        // refuses at the COMMIT GATE first -- the documented limit an unstamped
        // `cargo test` build always hits. Only a value that cannot be a `u8` at
        // all exercises the parse arm from this side. The 1..=12 guard itself is
        // covered directly by
        // `stored::a_month_outside_one_to_twelve_is_refused_and_never_reported_as_missing`.
        for (args, want) in [
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "x",
                    "1",
                    "2026",
                    "8",
                    "500",
                ],
                "YEAR must be a number",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "12",
                    "y",
                    "8",
                    "500",
                ],
                "YEAR must be a number",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "300",
                    "2026",
                    "8",
                    "500",
                ],
                "MONTH must be 1..=12",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "12",
                    "2026",
                    "300",
                    "500",
                ],
                "MONTH must be 1..=12",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "12",
                    "2026",
                    "8",
                    "0",
                ],
                "MIN_HITS must be 1 or more",
            ),
        ] {
            let mut out = String::new();
            let code = run(&argv(&args), &mut out);
            assert_eq!(code, MISUSED, "a bad argument exits MISUSED: {out}");
            assert!(
                out.contains(want),
                "the refusal must name the argument it rejected.\nwanted: \
                 {want}\ngot: {out}"
            );
        }
    }

    /// A well-formed range still refuses, and for the identity reason.
    ///
    /// This is the pair to the test above: the arguments are all valid, so the
    /// parse arms pass and the call reaches `audit_range`, which refuses at the
    /// commit gate before touching the store. That gate is §3 rule 3 and it must
    /// come FIRST — a computation whose identity cannot be recorded may not run,
    /// so the refusal here is the correct behaviour and not a gap.
    #[test]
    fn a_well_formed_range_refuses_at_the_identity_gate_and_says_so() {
        let mut out = String::new();
        let code = run(
            &argv(&[
                "audit-range",
                "zerodha",
                "NIFTY",
                "1min",
                "2019",
                "12",
                "2026",
                "8",
                "500",
            ]),
            &mut out,
        );
        assert_eq!(code, MISUSED);
        assert!(
            out.contains("commit stamp"),
            "an unstamped build must refuse by naming the stamp, not by \
             failing to find a file: {out}"
        );
        assert!(
            out.contains("the audit will not run"),
            "and it must say which command it refused: {out}"
        );
    }

    /// The usage text lists the range command, so an operator can find it.
    #[test]
    fn the_range_command_is_listed_in_usage() {
        assert!(USAGE.contains("audit-range"), "the command is listed");
        assert!(
            USAGE.contains("CONTIGUOUS SPAN"),
            "and usage says what makes it different from `audit-stored`"
        );
    }

    /// A COMPLETED RANGE RUN LEAVES A ROW, and a rerun does not leave a second.
    ///
    /// # What this can reach under `cargo test`, stated because it bounds the
    /// # assertion
    ///
    /// `commit_stamp()` is `option_env!`, so an unstamped test build refuses at
    /// the identity gate before the store is touched -- the limit
    /// `the_stored_audit_refuses_for_the_same_cause_and_names_itself` documents.
    /// So this cannot drive `audit-range` end to end.
    ///
    /// What it CAN do is drive the results store through the same calls
    /// `record_run` makes, which is where every defect in that path would live:
    /// the codec, the addressing, the duplicate rule and the reopen. The wiring
    /// itself is one call and is verified by measurement on a stamped build,
    /// recorded in `docs/05-decisions.md`.
    #[test]
    fn a_recorded_run_is_addressable_and_a_rerun_adds_nothing() {
        let root = std::env::temp_dir().join("brutex-wire-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        let record = crate::results::Record {
            identity: [42; 32],
            finished_micros: 1_785_727_500_000_000,
            feed: crate::results::field("zerodha"),
            underlying: crate::results::field("NIFTY"),
            timeframe: crate::results::field("15min"),
            from_year: 2019,
            from_month: 12,
            to_year: 2026,
            to_month: 8,
            months_asked: 81,
            months_found: 81,
            bars: 43_875,
            min_hits: 200,
            combinations: 54_895_691,
            depth: 11,
            halted: 1,
            trades: 412,
            pessimistic: -987_654_321,
            optimistic: 123_456_789,
            worst_trade: -87_654_321,
            max_drawdown: 76_543_210,
            winner_mae: 2_291,
            winner_mfe: 8_876,
            all_mae: 2_295,
            exit_rungs: [2, 3, -1, 1, 0],
            mask_words: [1, 0, 0, 0, 0, 0],
        };

        let mut store = crate::results::Results::open(&root).expect("the store opens");
        let row = store.append(&record).expect("the run is recorded");
        assert_eq!(row, 0, "the first run is row zero");

        // ADDRESSABLE, which is the whole reason for a fixed stride.
        let back = store.read(row).expect("the row reads back");
        assert_eq!(back, record, "every field survived the write and the read");
        assert_eq!(crate::results::read_field(&back.underlying), "NIFTY");
        assert_eq!(back.halted, 1, "a truncated search stays flagged as one");

        // A RERUN ADDS NOTHING. §3 rule 5 makes it byte-identical.
        assert!(
            store.append(&record).is_err(),
            "the same identity must not be recorded twice"
        );
        assert_eq!(store.len().expect("measurable"), 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    /// `range-all` refuses a bad argument exactly as `audit-range` does.
    #[test]
    fn the_all_rungs_command_refuses_the_same_way_its_sibling_does() {
        // TWO COMMANDS, ONE ARGUMENT SHAPE, ONE SET OF RULES. An operator who
        // learned `audit-range`'s refusals must not have to learn a second set.
        for (args, want) in [
            (
                [
                    "range-all",
                    "zerodha",
                    "NIFTY",
                    "x",
                    "1",
                    "2026",
                    "8",
                    "500",
                ],
                "YEAR must be a number",
            ),
            (
                [
                    "range-all",
                    "zerodha",
                    "NIFTY",
                    "2019",
                    "300",
                    "2026",
                    "8",
                    "500",
                ],
                "MONTH must be 1..=12",
            ),
            (
                [
                    "range-all",
                    "zerodha",
                    "NIFTY",
                    "2019",
                    "12",
                    "2026",
                    "8",
                    "0",
                ],
                "SUPPORT_PPM must be 1 or more",
            ),
        ] {
            let mut out = String::new();
            assert_eq!(run(&argv(&args), &mut out), MISUSED, "{out}");
            assert!(out.contains(want), "wanted {want}, got: {out}");
        }
    }

    /// It is listed, and it sweeps every rung an operator pulls.
    #[test]
    fn the_all_rungs_command_covers_the_eight_intraday_rungs_and_says_so() {
        assert!(USAGE.contains("range-all"), "the command is listed");
        assert!(
            USAGE.contains("ALL EIGHT INTRADAY RUNGS"),
            "and usage says what makes it different from `audit-range`"
        );
        // The nine an operator actually pulls. `1s` is deliberately absent --
        // nothing has measured what a seven-year second-resolution span costs,
        // and §3 rule 6 says an unmeasured bound is not a bound.
        assert_eq!(super::EVERY_RUNG.len(), 8);
        for rung in [
            "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min",
        ] {
            assert!(
                super::EVERY_RUNG.contains(&rung),
                "{rung} is a rung the operator pulls and must be swept"
            );
        }
        assert!(
            !super::EVERY_RUNG.contains(&"1s"),
            "1s is excluded until its cost is measured"
        );
        // FINEST FIRST. The rungs run in PARALLEL now, so ordering no longer
        // decides which reports early -- it decides which STARTS first, and the
        // one-minute rung is fifteen times the bars of any other. Starting the
        // longest job first is the standard scheduling answer: the makespan of a
        // parallel batch is bounded by its longest task, so that task must not
        // be the last one picked up.
        assert_eq!(super::EVERY_RUNG.first().copied(), Some("1min"));
        assert_eq!(super::EVERY_RUNG.last().copied(), Some("60min"));
        // 1day IS NOT A RUNG. It is one whole session, so sweeping it produces
        // at most one trade per day -- 365 in 6.75 years, measured -- on an
        // engine that squares off at 15:10. Daily data is an INPUT: the previous
        // session's HLC feeds `indicators::daily`, whose PDH, PDL, CPR and pivot
        // conditions are already in the vocabulary and already fire on every
        // intraday rung.
        assert!(
            !super::EVERY_RUNG.contains(&"1day"),
            "a daily bar is one session; it is an input to the conditions, not \
             a rung to sweep"
        );
    }

    /// A ledger that can only be appended to is a write-only file.
    #[test]
    fn the_listing_ranks_on_the_worst_case_and_excludes_halted_rows() {
        // THE ASSERTION THAT MATTERS. A halted run's total is not comparable
        // with a complete one's: it covers less of the ladder while its
        // combination count looks LARGER. So the largest number in the table
        // must not automatically win.
        let halted_but_huge = crate::results::Record {
            identity: [1; 32],
            finished_micros: 1,
            feed: crate::results::field("zerodha"),
            underlying: crate::results::field("NIFTY"),
            timeframe: crate::results::field("1min"),
            from_year: 2019,
            from_month: 12,
            to_year: 2026,
            to_month: 8,
            months_asked: 81,
            months_found: 81,
            bars: 1,
            min_hits: 120,
            combinations: 55_000_000,
            depth: 9,
            halted: 1,
            trades: 900,
            pessimistic: 9_000_000,
            optimistic: 9_000_000,
            worst_trade: 0,
            max_drawdown: 1,
            winner_mae: 0,
            winner_mfe: 0,
            all_mae: 0,
            exit_rungs: [-1; 5],
            mask_words: [0; 6],
        };
        let complete_but_smaller = crate::results::Record {
            identity: [2; 32],
            min_hits: 500,
            halted: 0,
            pessimistic: 1_000,
            trades: 12,
            ..halted_but_huge
        };

        let line = super::best_complete_line(&[halted_but_huge, complete_but_smaller]);
        assert!(
            line.contains("min_hits 500"),
            "the COMPLETE run must win even though the halted one shows a total \
             nine thousand times larger:\n{line}"
        );
        assert!(
            // ₹10.00 IS 1000 paisa. The renderer moved to rupees in the
            // `rupees, and what the run RISKED` commit and this assertion did
            // not, so it was checking a spelling the surface no longer uses
            // while the figure it exists to pin was unchanged.
            line.contains("₹10.00"),
            "and it must quote that run's own figure:\n{line}"
        );

        // AND WHEN NOTHING COMPLETED, IT SAYS SO rather than crowning the least
        // truncated row.
        let none = super::best_complete_line(&[halted_but_huge]);
        assert!(
            none.contains("NO COMPLETE RUN"),
            "a table of only halted rows has no comparable winner:\n{none}"
        );
        assert!(
            !none.contains("BEST COMPLETE RUN"),
            "and must not name one anyway:\n{none}"
        );
    }

    /// The command is listed, and both of its shapes dispatch.
    #[test]
    fn the_results_command_takes_no_filter_or_exactly_two() {
        assert!(USAGE.contains("cli results"), "the command is listed");
        // Neither shape may be MISUSED: an empty ledger is an ordinary state,
        // not an operator error.
        for args in [vec!["results"], vec!["results", "zerodha", "NIFTY"]] {
            let mut out = String::new();
            let code = run(&argv(&args), &mut out);
            assert_ne!(code, MISUSED, "`{args:?}` is a valid invocation: {out}");
        }
        // One filter word is NOT a shape: `results zerodha` would silently list
        // everything, which reads as a filter that did nothing.
        let mut out = String::new();
        assert_eq!(
            run(&argv(&["results", "zerodha"]), &mut out),
            MISUSED,
            "a half-given filter must refuse rather than ignore itself: {out}"
        );
    }

    /// A request no rung can serve is a REFUSAL, never a report.
    ///
    /// # Found by attacking the command, not by reading it
    ///
    /// `range-all nosuchfeed NIFTY ...`, a backwards range, month 0, month 13,
    /// an empty symbol, a unicode symbol and `../../etc` ALL printed the
    /// `STORED_PROVENANCE` banner -- *"THESE BARS ARE REAL MARKET DATA, READ
    /// FROM THE STORE"* -- and exited ZERO. Nine rows said REFUSED underneath,
    /// which is honest; the banner above them claimed real bars had been read
    /// and the exit code told a script the run succeeded.
    ///
    /// The provenance banner is the one line in this workspace that must never
    /// print over nothing: `cli`'s own header calls the two banners *"the only
    /// thing separating"* a real sweep from a generated one.
    #[test]
    fn a_request_no_rung_can_serve_refuses_and_prints_no_provenance() {
        for args in [
            [
                "range-all",
                "nosuchfeed",
                "NIFTY",
                "2019",
                "12",
                "2026",
                "8",
                "200000",
            ],
            [
                "range-all",
                "zerodha",
                "NIFTY",
                "2026",
                "8",
                "2019",
                "12",
                "200000",
            ],
            [
                "range-all",
                "zerodha",
                "NIFTY",
                "2019",
                "0",
                "2026",
                "8",
                "200000",
            ],
            [
                "range-all",
                "zerodha",
                "NIFTY",
                "2019",
                "13",
                "2026",
                "8",
                "200000",
            ],
            [
                "range-all",
                "zerodha",
                "../../etc",
                "2019",
                "12",
                "2026",
                "8",
                "200000",
            ],
            [
                "range-all",
                "zerodha",
                "",
                "2019",
                "12",
                "2026",
                "8",
                "200000",
            ],
        ] {
            let mut out = String::new();
            let code = run(&argv(&args), &mut out);
            assert_eq!(code, MISUSED, "`{args:?}` must exit MISUSED:\n{out}");
            assert!(
                out.starts_with("refused: "),
                "and lead with the refusal:\n{out}"
            );
            assert!(
                !out.contains("REAL MARKET DATA"),
                "the provenance banner must NEVER print over a run that read \
                 nothing -- it is the only line separating a real sweep from a \
                 generated one:\n{out}"
            );
            assert!(
                out.contains("first reason:"),
                "and it must name WHY, not merely that it refused:\n{out}"
            );
        }
    }

    /// `BRUTEX_CEILING` is read, bounded, and refuses rather than falling back.
    ///
    /// # Why a malformed value must not use the default
    ///
    /// An operator who sets the variable meant to change the ceiling. Quietly
    /// using the compiled one is the `CLAUDE.md` §4 fallback that hides a
    /// failure: the run would complete, report a depth, and be a different
    /// search from the one that was asked for, with nothing on the page saying
    /// so.
    ///
    /// The environment is process-wide and `cargo test` runs threads in
    /// parallel, so this drives the PARSE rather than mutating `std::env` —
    /// setting it here would change the ceiling under every other test in the
    /// binary, which is the same reason `root_from` takes its environment as an
    /// argument.
    #[test]
    fn a_malformed_ceiling_refuses_and_names_the_default() {
        // The shipped default is what an unset variable resolves to, and it is
        // the figure the refusal quotes, so the two cannot drift apart.
        assert_eq!(::engine::DEFAULT_CEILING, 1 << 27);
        // A parse that succeeds is used verbatim: the operator's machine is not
        // this one and the number is theirs.
        assert_eq!("41943040".parse::<usize>().ok(), Some(41_943_040));
        // Zero and nonsense are the two refusals, and both are refusals rather
        // than clamps: `Ladder::with_ceiling` raises a zero to one, which would
        // turn "I set it wrong" into a sweep that halts at k=1 and looks like
        // extinction.
        assert_eq!("0".parse::<usize>().ok(), Some(0));
        assert!("many".parse::<usize>().is_err());
        assert!("-1".parse::<usize>().is_err());
    }

    /// The stored winner is printed BY NAME, not only by its P&L.
    ///
    /// # The gap this closes
    ///
    /// Version 2 of the ledger held twenty-four fields and every one of them
    /// was a number. A row read back said a sweep made ₹13,504 over 19,759
    /// trades and had **no way at all** to say which conditions did it: the
    /// combination existed only in the live run's stdout, which scrolls. So the
    /// one fact an operator would act on was the one fact the durable surface
    /// dropped, and §3 rule 3's whole promise -- that a run is identified and
    /// reproducible -- covered the identity and not the content.
    ///
    /// Version 3 carries the six mask words. This asserts the round trip is
    /// end to end: words written, words read, words resolved to names, names
    /// rendered. Asserting the SHAPE and not a literal string is deliberate --
    /// pinning `"bar_bullish"` here would make an unrelated vocabulary edit
    /// fail a rendering test, and §3 rule 8 lets positions be tombstoned.
    #[test]
    fn the_stored_winner_is_named_and_not_only_priced() {
        let mut record = record_for_naming();
        // Three bits, deliberately spread across words so a renderer that only
        // ever reads word zero fails here. Bit 0, bit 64 (word 1) and bit 320
        // (word 5) -- the first and last words plus one in between.
        record.mask_words = [1, 1, 0, 0, 0, 1];

        let block = crate::quality_block(&record);

        assert!(
            block.contains("the conditions it required"),
            "the block must name the row, got:\n{block}"
        );
        assert!(
            block.contains("3 of them"),
            "all three bits must be counted, including the ones outside word \
             zero. got:\n{block}"
        );
        for ordinal in ["1.", "2.", "3."] {
            assert!(
                block.contains(ordinal),
                "each condition is listed on its own numbered line; {ordinal} \
                 is missing from:\n{block}"
            );
        }
    }

    /// An empty mask says NONE rather than printing nothing.
    ///
    /// # Why this is a separate test and not an edge case
    ///
    /// A halted run, or one whose frequent frontier emptied at k=1, records no
    /// combination and stores six zero words. A renderer that simply loops over
    /// an empty list emits a header and no rows, which is indistinguishable
    /// from a rendering bug -- and §4 bans a failure wearing a success's
    /// clothes. The absence has to be STATED.
    #[test]
    fn a_row_with_no_combination_says_so_rather_than_rendering_blank() {
        let mut record = record_for_naming();
        record.mask_words = [0; 6];

        let block = crate::quality_block(&record);

        assert!(
            block.contains("NONE"),
            "an absent combination is stated, not implied by silence. got:\n{block}"
        );
        assert!(
            block.contains("no combination was recorded"),
            "and the reason is given rather than left to be inferred. got:\n{block}"
        );
        assert!(
            !block.contains("of them"),
            "and it must NOT also print a count. got:\n{block}"
        );
    }

    /// Six mask words with a couple of low bits set, for a name to render from.
    fn vocab_words_for_test() -> [u64; 6] {
        let mut words = [0_u64; 6];
        // Bits 0 and 1 -- two real, live positions, so `names_from_words`
        // resolves them instead of returning an empty list.
        words[0] = 0b11;
        words
    }

    /// A record with the money fields filled and the mask left to the caller.
    fn record_for_naming() -> crate::results::Record {
        crate::results::Record {
            identity: [7; 32],
            finished_micros: 1_785_727_500_000_000,
            feed: crate::results::field("zerodha"),
            from_year: 2019,
            from_month: 1,
            to_year: 2025,
            to_month: 9,
            underlying: crate::results::field("NIFTY"),
            timeframe: crate::results::field("15min"),
            months_asked: 81,
            months_found: 81,
            bars: 96_280,
            combinations: 1_024_058,
            min_hits: 600,
            depth: 3,
            halted: 0,
            trades: 19_759,
            pessimistic: 1_350_424,
            optimistic: 1_450_424,
            worst_trade: -18_400,
            max_drawdown: 76_543,
            winner_mae: 2_291,
            winner_mfe: 8_876,
            all_mae: 2_295,
            exit_rungs: [2, 3, -1, 1, 0],
            mask_words: [0; 6],
        }
    }

    /// A tier refuses a PERFECT record that is too small to support its claim.
    ///
    /// # The hole this closes
    ///
    /// A tier that reads "95% win rate" was satisfied by twenty winners out of
    /// twenty. That is a raw rate of 100% and a 95% lower bound of 83.88% -- the
    /// sample does not support a 95% claim, and the tier stamped it `S++++++`
    /// anyway. An operator reading that label would believe the engine had found
    /// a 95% system when what it had found was twenty coin flips landing heads.
    ///
    /// The fix is not a `min_trades` floor. A floor set high enough to make 95%
    /// safe would also throw away the rare high-conviction setup the operator is
    /// actually hunting. The bound refuses only what is genuinely unsupported,
    /// and admits the same combination the moment enough trades confirm it.
    #[test]
    fn a_tier_refuses_a_perfect_record_too_small_to_support_its_claim() {
        let strict = crate::Tier {
            name: "S++++++",
            max_points: 25,
            min_rr_bp: 300,
            min_win_rate_bp: 9_500,
            min_trades: 0,
        };
        let rules = strict.rules(25);

        // The money shape is identical in both cells. ONLY the sample differs,
        // so anything that separates them is separating on evidence alone.
        let tiny = perfect_cell(20);
        let ample = perfect_cell(200);

        assert_eq!(
            tiny.win_rate_bp(),
            ample.win_rate_bp(),
            "both are perfect records -- the raw rate cannot separate them"
        );
        assert!(
            !rules.admits(&tiny),
            "twenty perfect trades bound to {}bp, below the {}bp this tier \
             claims, and must be refused",
            tiny.assurance_bp(),
            rules.min_assurance_bp
        );
        assert!(
            rules.admits(&ample),
            "two hundred perfect trades bound to {}bp and must be admitted -- \
             the rule refuses INSUFFICIENCY, not rarity",
            ample.assurance_bp()
        );
    }

    /// Turning the rule off restores the old behaviour exactly.
    ///
    /// The support floor lands where it does, and a 50% rule cannot be tested.
    ///
    /// # The number that sizes every browser sweep
    ///
    /// `sweeprun::conduct` passes `None`, so each rung derives its floor through
    /// [`statistical_support_floor`] → `Rules::elite(1, 1)` →
    /// `trades_needed_for(8_000, assurance_floor_bp(8_000), TRADES_SEARCH_CEILING)`.
    /// That is **29 round trips**, pinned below. Measured against the operator's
    /// own run of 2026-08-28: all eight rungs were handed `min_hits: 28` — 29
    /// less the one trade the ppm round trip truncates away — which on the
    /// 1-minute rung's 617,921 bars is 0.0045% support. `docs/05-decisions.md`
    /// D-0258 records 4.7% as the deepest support ever measured to COMPLETE, so
    /// this floor is three orders of magnitude below anything observed to
    /// finish. Whoever changes `Rules::elite` moves this number, and it decides
    /// whether a run ends.
    ///
    /// # Why the operator's 50% cannot simply be substituted
    ///
    /// The second row is the trap. [`assurance_floor_bp`] returns the caller's
    /// own figure when it is at or below chance — deliberately, so a rate under
    /// a coin flip is not quietly raised — which at exactly 5,000 makes the pair
    /// `(5_000, 5_000)`: demand that the 95% LOWER BOUND on the rate reach the
    /// rate itself. The Wilson bound approaches the observed rate FROM BELOW and
    /// never arrives, so no sample size satisfies it and `trades_needed_for`
    /// returns its ceiling — indistinguishable, to its caller, from an honest
    /// answer of "five thousand trades".
    ///
    /// That is not a bug in the arithmetic. It is the arithmetic reporting that
    /// **"at least 50% of trades win" is exactly the null hypothesis of a coin
    /// flip**, and carries no evidence on its own. The discriminating half of
    /// the operator's rule is the other one — smallest win at least 1.25x the
    /// largest loss — which `grid::Cell::reward_to_risk_bp` already computes as
    /// a true min/max. Any future wiring of that rule must size the search from
    /// something satisfiable; the rows between show what a bound set below the
    /// stated rate actually costs.
    #[test]
    fn the_support_floor_is_twenty_nine_trades_and_a_fifty_percent_rule_is_untestable() {
        let cases = [
            // (win rate bp, assurance bp, trades needed)
            (8_000_i64, crate::assurance_floor_bp(8_000), 29_u64),
            (
                5_000,
                crate::assurance_floor_bp(5_000),
                crate::TRADES_SEARCH_CEILING,
            ),
            (5_000, 4_000, 83),
            (5_000, 4_500, 361),
            (6_000, 5_500, 352),
        ];
        for (rate, assurance, want) in cases {
            let got =
                runner::grid::trades_needed_for(rate, assurance, crate::TRADES_SEARCH_CEILING);
            assert_eq!(
                got, want,
                "rate {rate}bp against a bound of {assurance}bp needs {want} \
                 round trips, not {got}"
            );
        }

        assert_eq!(
            crate::assurance_floor_bp(5_000),
            5_000,
            "at chance the bound equals the rate, which is what makes it \
             unsatisfiable -- see this test's doc"
        );
        assert_eq!(
            runner::grid::trades_needed_for(5_000, 5_000, crate::TRADES_SEARCH_CEILING),
            crate::TRADES_SEARCH_CEILING,
            "an unsatisfiable pair returns the CEILING, and its caller cannot \
             tell that from a real answer of five thousand"
        );
    }

    /// Asserted because every other rule in [`Rules`] documents "zero drops the
    /// rule", and a rule that quietly kept filtering at zero would disqualify
    /// rows an operator had explicitly stopped asking about.
    #[test]
    fn an_assurance_of_zero_drops_the_rule() {
        let rules = crate::Rules {
            max_mae_ppm: i64::MAX,
            min_rr_bp: 0,
            min_win_rate_bp: 0,
            min_assurance_bp: 0,
            min_weakest_bp: 0,
            min_trades: 0,
            min_ret_over_dd_bp: 0,
            top: 25,
        };
        assert!(
            rules.admits(&perfect_cell(1)),
            "a single trade must pass once the rule is switched off"
        );
    }

    /// A cell with `n` trades, all winners, and a shape that clears every rule
    /// except the one under test.
    fn perfect_cell(n: u64) -> runner::grid::Cell {
        runner::grid::Cell {
            trades: n,
            wins: n,
            gross_win: 100_000 * i64::try_from(n).unwrap_or(1),
            gross_loss: 0,
            worst_mae: 0,
            ..runner::grid::Cell::default()
        }
    }

    /// The support a real run was launched with PRUNES the cadence it wanted.
    ///
    /// # The measurement this pins, and it is the reason the descent exists
    ///
    /// A real `range-all` was launched on the 81-month NIFTY span at
    /// `SUPPORT_PPM 20000`. The operator's requirement was "one intraday trade a
    /// week, every one a winner". Those two numbers are incompatible and nothing
    /// in the engine said so: the run banner reported its support, the sweep
    /// reported its combinations, and the one combination shape that was asked
    /// for had been pruned before the first exit grid was built.
    ///
    /// 15-minute bars, 81 months, 96,280 bars. One trade a week is 352 trades,
    /// which is 3,655 ppm. The floor was 20,000. **Five times too strict**, and
    /// the only combinations that could survive it fire on 2% of all bars --
    /// better than five trades a week, which is a grinder by any reading.
    ///
    /// If this assertion ever fails, either the arithmetic moved or the cadence
    /// did, and the descent's floor is no longer the operator's requirement.
    /// `top` renders a stored frontier, best rank first, with its names.
    ///
    /// The whole point of [`crate::frontier`] is that this list survives the
    /// process that produced it, so the test writes rows, closes nothing, and
    /// reads them back through the same surface an operator uses.
    #[test]
    fn the_top_list_renders_a_stored_frontier_in_rank_order() {
        let root = std::env::temp_dir().join(format!("brutex-top-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        // A complete ledger row, so `newest_complete` has something to choose.
        let mut ledger = crate::results::Results::open(&root).expect("a fresh ledger");
        let mut record = record_for_naming();
        record.identity = [11; 32];
        record.halted = 0;
        record.pessimistic = 5_000;
        ledger.append(&record).expect("the row appends");

        // Two frontier rows for that run, written out of rank order.
        let mut store = crate::frontier::Frontier::open(&root).expect("a fresh frontier");
        let mask = vocab_words_for_test();
        store
            .append_all(&[
                crate::frontier::Row {
                    identity: [11; 32],
                    rank: 2,
                    mask_words: mask,
                    hits: 400,
                    n: 380,
                    mean_milli_paisa: 2_500,
                    t_milli: 2_100,
                    payoff_bp: 150,
                    wins: 200,
                    trades: 0,
                    cell_wins: 0,
                    pessimistic: 0,
                    worst_trade: 0,
                    max_drawdown: 0,
                    min_win: 0,
                },
                crate::frontier::Row {
                    identity: [11; 32],
                    rank: 1,
                    mask_words: mask,
                    hits: 900,
                    n: 880,
                    mean_milli_paisa: 7_000,
                    t_milli: 4_007,
                    payoff_bp: i64::MAX,
                    wins: 880,
                    trades: 0,
                    cell_wins: 0,
                    pessimistic: 0,
                    worst_trade: 0,
                    max_drawdown: 0,
                    min_win: 0,
                },
            ])
            .expect("both rows append");

        let page = top_at(&root, None, None);
        assert!(page.starts_with(STORED_PROVENANCE), "provenance leads it");
        assert!(page.contains("TOP COMBINATIONS"), "the section is named");

        // RANK ORDER, not write order.
        let first = page.find(" 1 ").or_else(|| page.find("  1  "));
        let second = page.find(" 2 ").or_else(|| page.find("  2  "));
        assert!(
            matches!((first, second), (Some(a), Some(b)) if a < b),
            "rank 1 must be printed before rank 2, whatever order they were \
             written in:\n{page}"
        );

        // The unbounded payoff is NAMED rather than printed as a huge number.
        assert!(
            page.contains("inf"),
            "a combination that never lost reads `inf`, not 92233720368547758.07:\n{page}"
        );
        assert!(
            !page.contains("92233720368547758"),
            "i64::MAX must never reach the page as a figure:\n{page}"
        );

        // The money column goes through `rupees`, like every other in this
        // binary -- 7,000 thousandths of a paisa is 7 paisa is Rs 0.07.
        assert!(
            page.contains("0.07"),
            "the mean is rendered as rupees:\n{page}"
        );

        // And the caveats an operator needs beside the numbers.
        assert!(
            page.contains("ONE \nunit") || page.contains("ONE unit"),
            "the page states the denomination:\n{page}"
        );
        assert!(
            page.contains("gross of the statutory charge stack"),
            "the page states that charges are not deducted:\n{page}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A run that recorded no frontier says so, rather than printing nothing.
    ///
    /// Every run made before `crate::frontier` existed is in this state, so the
    /// message has to distinguish "this run kept no list" from "this run found
    /// nothing" — they are different facts and only one is about the market.
    #[test]
    fn a_run_with_no_frontier_says_so_rather_than_looking_empty() {
        let root = std::env::temp_dir().join(format!("brutex-top-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        let mut ledger = crate::results::Results::open(&root).expect("a fresh ledger");
        let mut record = record_for_naming();
        record.identity = [12; 32];
        record.halted = 0;
        ledger.append(&record).expect("the row appends");

        let page = top_at(&root, None, None);
        assert!(
            page.contains("recorded NO frontier"),
            "the gap in the RECORD is named, not reported as an empty result:\n{page}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A halted run is never the one `top` reports on.
    ///
    /// A halted run's total covers less of the ladder than its combination count
    /// suggests, so ranking it against a complete one compares two different
    /// searches — the same reason `range-all` prints a `complete` column.
    #[test]
    fn a_halted_run_is_not_the_run_top_reports_on() {
        let root = std::env::temp_dir().join(format!("brutex-top-halt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        let mut ledger = crate::results::Results::open(&root).expect("a fresh ledger");
        let mut halted = record_for_naming();
        halted.identity = [13; 32];
        halted.halted = 1;
        // Enormous, so only the halted flag can keep it out.
        halted.pessimistic = 9_000_000;
        ledger.append(&halted).expect("the halted row appends");

        let page = top_at(&root, None, None);
        assert!(
            page.contains("NO COMPLETE RUN"),
            "a halted run is not a candidate however large its total:\n{page}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// [`COMMANDS`], the dispatch and [`USAGE`] name the same set of commands.
    ///
    /// The list exists so a wrong-arity refusal can tell a real command from a
    /// typo, and a list that drifts from the dispatch would make that refusal
    /// lie in the other direction. Every entry must appear in the usage as its
    /// own `cli <word>` line, and every such line must be in the list.
    #[test]
    fn every_command_is_listed_in_both_places() {
        for word in COMMANDS {
            assert!(
                USAGE.contains(&format!("cli {word} ")) || USAGE.contains(&format!("cli {word}\n")),
                "`{word}` is in COMMANDS and has no usage line"
            );
        }

        // And the other direction: every `cli <word>` the usage advertises must
        // be a word the dispatch answers to.
        for line in USAGE.lines() {
            let mut words = line.split_whitespace();
            if words.next() != Some("cli") {
                continue;
            }
            if let Some(word) = words.next() {
                assert!(
                    COMMANDS.contains(&word),
                    "the usage advertises `{word}` and COMMANDS does not list it"
                );
            }
        }

        assert!(
            COMMANDS.windows(2).all(|w| w.first() < w.last()),
            "kept sorted so a new command is added in one obvious place"
        );
    }

    /// A known command with the wrong arity says so, instead of denying itself.
    ///
    /// Every dispatch arm is a fixed-length slice pattern, so `cli screen` with
    /// no arguments fell through to the unknown-word arm and was told `screen`
    /// is not a command this build knows. It is, and the reader was sent looking
    /// for a typo or a stale binary.
    #[test]
    fn a_known_command_with_the_wrong_arity_is_not_called_unknown() {
        for word in COMMANDS {
            let mut out = String::new();
            let code = run(&argv(&[word]), &mut out);

            // TWO COMMANDS LEGITIMATELY TAKE ZERO ARGUMENTS, and both READ the
            // store rather than sweeping it: `results` lists every recorded run
            // and `top` reports the best complete one's frontier. For both, no
            // argument means NO FILTER, which is a request rather than a
            // mistake.
            //
            // Listed rather than inferred: "takes zero arguments" is not
            // something the dispatch can be asked, and a command that grows a
            // required argument must fail HERE rather than silently leaving the
            // exemption behind.
            if matches!(word, "results" | "top") {
                continue;
            }

            assert_eq!(code, MISUSED, "`{word}` alone is a misuse");
            assert!(
                out.contains("is a command, but not with 0 arguments"),
                "`{word}` alone must be named as a command with the wrong arity, got: {}",
                out.lines().next().unwrap_or_default()
            );
            assert!(
                !out.contains("is not a command this build knows"),
                "`{word}` must not deny being a command"
            );
        }

        // A genuine typo still gets the honest answer.
        let mut out = String::new();
        assert_eq!(run(&argv(&["scrreen"]), &mut out), MISUSED);
        assert!(
            out.contains("`scrreen` is not a command this build knows"),
            "an unknown word is still unknown, got: {}",
            out.lines().next().unwrap_or_default()
        );

        // Singular for one argument, so the message reads.
        let mut one = String::new();
        let _ = run(&argv(&["screen", "zerodha"]), &mut one);
        assert!(
            one.contains("not with 1 argument."),
            "one argument is singular, got: {}",
            one.lines().next().unwrap_or_default()
        );
    }

    /// `elite` refuses every shape it exists to refuse, and admits the sniper.
    ///
    /// Each case below is a combination that passes some of the rules and fails
    /// one. That is the whole design: five properties of the trade list plus one
    /// of the equity path, and a row must clear all six.
    #[test]
    fn the_elite_profile_refuses_each_shape_it_exists_to_refuse() {
        // 10 points at NIFTY 25,000 is 400 ppm.
        let rules = crate::Rules::elite(points_to_ppm(10), 25);

        // A cell that satisfies everything: 40 trades, all winners, tight
        // excursion, big average win, and a fall it made back many times over.
        let good = grid::Cell {
            trades: 40,
            wins: 40,
            worst_mae: 300,
            min_win: 1_500,
            worst_trade: -500,
            pessimistic: 1_000_000,
            max_drawdown: 100_000,
            ..grid::Cell::default()
        };
        assert!(
            rules.admits(&good),
            "a 40-of-40 record with a 10x return over drawdown is what this profile is FOR: \
             win {} bp, assurance {} bp, rr {} bp, ret/dd {}",
            good.win_rate_bp(),
            good.assurance_bp(),
            good.reward_to_risk_bp(),
            good.return_over_drawdown()
        );

        // THE OPERATOR'S OWN COUNTEREXAMPLE, from `min_rr_bp`'s doc: 81% won,
        // average win 7.57 against average loss 18.65. A superb win rate
        // wearing a losing shape.
        let high_rate_bad_rr = grid::Cell {
            min_win: 757,
            worst_trade: -1_865,
            ..good
        };
        assert!(
            !rules.admits(&high_rate_bad_rr),
            "an 81%-win-rate strategy with a reward-to-risk of 0.41 must not pass"
        );

        // A LUCKY TINY SAMPLE. 12 of 12 is a 100% rate and a 75.75% lower
        // bound, so the raw rate passes and the assurance bound refuses it.
        let lucky_twelve = grid::Cell {
            trades: 12,
            wins: 12,
            ..good
        };
        assert_eq!(
            lucky_twelve.win_rate_bp(),
            10_000,
            "the raw rate is perfect"
        );
        // TWELVE PERFECT TRADES NOW PASSES, and the change is deliberate.
        //
        // This asserted a refusal, under a bound fixed at 8,000. That bound also
        // refused **32 of 40 — the operator's own stated 80% over forty
        // trades** — because the 95% lower bound of an exactly-80% record never
        // reaches 80% at any sample size. The rule enforced was not the rule
        // stated.
        //
        // The floor is now the midpoint between chance and the stated rate, and
        // twelve consecutive wins clears it (7,575 bp against 6,500). That is
        // the honest reading: twelve wins in a row is genuinely unlikely by
        // chance. What the floor still refuses is a four-trade perfect record
        // at 5,101 bp, which is the case below.
        assert!(
            rules.admits(&lucky_twelve),
            "twelve consecutive wins clears a midpoint floor: assurance {} bp",
            lucky_twelve.assurance_bp()
        );

        // A FOUR-TRADE PERFECT RECORD IS STILL NOISE, and is still refused.
        let lucky_four = grid::Cell {
            trades: 4,
            wins: 4,
            ..good
        };
        assert!(
            !rules.admits(&lucky_four),
            "four perfect trades is noise wearing a certificate: assurance {} bp",
            lucky_four.assurance_bp()
        );

        // AND THE CASE THE OPERATOR ACTUALLY ASKED FOR: 80% over forty trades.
        // The old fixed bound refused this outright.
        let stated = grid::Cell {
            trades: 40,
            wins: 32,
            ..good
        };
        assert_eq!(stated.win_rate_bp(), 8_000, "exactly the stated rate");
        assert!(
            rules.admits(&stated),
            "32 of 40 IS the operator's 80%, and must not be refused: assurance {} bp",
            stated.assurance_bp()
        );

        // THE ROW THIS WHOLE FIELD WAS ADDED FOR. Everything above is
        // identical; it simply gave back most of what it made along the way.
        let gave_it_back = grid::Cell {
            max_drawdown: 900_000,
            ..good
        };
        assert!(
            !rules.admits(&gave_it_back),
            "made 1,000,000 after a 900,000 fall -- a ret/DD of {} -- must not pass a 5x rule",
            gave_it_back.return_over_drawdown()
        );

        // A stop that one trade ran through. Broken once is disqualification.
        let stop_broke = grid::Cell {
            worst_mae: 1_200,
            ..good
        };
        assert!(
            !rules.admits(&stop_broke),
            "a trade that ran 1,200 ppm against a 400 ppm stop must not pass"
        );

        // NO TRADE FLOOR, and that is deliberate -- it is what admits a rare
        // setup. The assurance bound is doing the work a floor would do badly.
        assert_eq!(
            rules.min_trades, 0,
            "a floor would reject the rare high-conviction setup this profile hunts"
        );
    }

    /// A drawdown rule of zero drops the rule, like every other rule here.
    ///
    /// The five older rules all treat zero as "not asked for", and a sixth that
    /// silently applied itself would be the invented policy `BASELINE`'s comment
    /// refuses.
    #[test]
    fn a_drawdown_rule_of_zero_admits_what_it_would_otherwise_refuse() {
        let ruinous = grid::Cell {
            trades: 40,
            wins: 40,
            worst_mae: 300,
            min_win: 1_500,
            worst_trade: -500,
            pessimistic: 1_000_000,
            // Gave back 99% of everything it made.
            max_drawdown: 990_000,
            ..grid::Cell::default()
        };

        let off = crate::Rules {
            min_ret_over_dd_bp: 0,
            ..crate::Rules::elite(points_to_ppm(10), 25)
        };
        assert!(off.admits(&ruinous), "zero drops the rule");

        let on = crate::Rules::elite(points_to_ppm(10), 25);
        assert!(!on.admits(&ruinous), "the profile turns it on");
    }

    /// A variant that never gave anything back clears any drawdown floor.
    ///
    /// `return_over_drawdown` answers `i64::MAX` there rather than dividing by
    /// zero, and the gate must read that as "passes" and not as an overflow.
    #[test]
    fn a_variant_that_never_gave_anything_back_clears_the_drawdown_rule() {
        let flawless = grid::Cell {
            trades: 40,
            wins: 40,
            worst_mae: 300,
            min_win: 1_500,
            worst_trade: -500,
            pessimistic: 1_000_000,
            max_drawdown: 0,
            ..grid::Cell::default()
        };
        assert_eq!(flawless.return_over_drawdown(), i64::MAX);
        assert!(
            crate::Rules::elite(points_to_ppm(10), 25).admits(&flawless),
            "a variant with no drawdown at all must clear a drawdown floor"
        );
    }

    /// The `ret/DD` column produces a VALUE on the sign the engine writes.
    ///
    /// The defect this pins printed `-` on every row of every sweep because its
    /// guard tested `max_drawdown < 0` while `grid::Cell` only ever writes
    /// `>= 0`. Four fixtures carrying the wrong sign kept the suite green over
    /// it, so this asserts against the ENGINE's convention and names a figure.
    #[test]
    fn the_ledger_ratio_is_the_cell_ratio() {
        // The row the operator pasted: ₹24,591.60 made against a ₹291.63 fall.
        assert_eq!(
            return_over_drawdown_cell(2_459_160, 29_163),
            "84.32",
            "a real row renders a number, not a dash -- and as a decimal, because \
             the underlying figure is hundredths and 8432 reads as eight thousand"
        );

        // Never gave anything back -- unbounded, and named rather than divided.
        assert_eq!(return_over_drawdown_cell(2_459_160, 0), "inf");
        // No profit to divide.
        assert_eq!(return_over_drawdown_cell(0, 29_163), "-");
        assert_eq!(return_over_drawdown_cell(-5_000, 29_163), "-");

        // THE REGRESSION ITSELF. A negative drawdown is not a value the engine
        // can produce; if one ever reaches here the answer must not be a
        // plausible-looking ratio computed from a sign nobody writes.
        assert_eq!(
            return_over_drawdown_cell(2_459_160, -29_163),
            "inf",
            "the impossible sign is handled by the one implementation, not a second one"
        );

        // ONE IMPLEMENTATION. This function must agree with the method it
        // delegates to across the whole grid of sign combinations -- the hand
        // copy is how the two came to disagree.
        for &pess in &[-1_i64, 0, 1, 2_459_160] {
            for &dd in &[-1_i64, 0, 1, 29_163] {
                let cell = grid::Cell {
                    pessimistic: pess,
                    max_drawdown: dd,
                    ..grid::Cell::default()
                };
                let want = match cell.return_over_drawdown() {
                    i64::MAX => "inf".to_owned(),
                    0 => "-".to_owned(),
                    r => hundredths_of(r),
                };
                assert_eq!(
                    return_over_drawdown_cell(pess, dd),
                    want,
                    "the ledger renderer and grid::Cell disagree at ({pess}, {dd})"
                );
            }
        }
    }

    /// The rate `reference_price`'s own doc states in writing, pinned.
    ///
    /// # What this caught, and why a doc sentence is the oracle
    ///
    /// `reference_price` returns PAISA — `b.low` and `b.high` are paisa, per
    /// `CLAUDE.md` §7 and `docs/02-store-format.md`. Its fallback returned
    /// `NIFTY_REFERENCE` unscaled, an index LEVEL, so one function had two
    /// units and `points_to_ppm_at` divided by whichever it got. Against the
    /// paisa branch, twenty points came out as **8 ppm** where the doc two
    /// screens up says 800.
    ///
    /// The shipped stop ladder was therefore 2..10 ppm — 0.05 to 0.25 index
    /// points, one to five NIFTY ticks — rather than 200..1000. Every stop the
    /// exit grid priced sat inside the entry bar's own range, which is the
    /// failure [`STOP_FLOOR_POINTS`] was introduced to prevent, and the
    /// operator's own `--max-mae` came through the CONSTANT path correctly at
    /// 800 ppm and was merged into a ladder eighty times tighter than itself.
    ///
    /// Nothing failed, because no test named a figure. This one does, and it
    /// takes the figure from the prose rather than from the code, so a
    /// reintroduced unit error cannot agree with it.
    #[test]
    fn a_rule_in_points_converts_at_the_documented_rate() {
        // NIFTY at 25,000 index is 2,500,000 paisa. `reference_price` returns
        // that; `NIFTY_REFERENCE` names the index level it is scaled from.
        let nifty_paisa = NIFTY_REFERENCE * PAISA_PER_POINT;
        assert_eq!(nifty_paisa, 2_500_000, "the paisa value of the doc's level");

        assert_eq!(
            points_to_ppm_at(20, nifty_paisa),
            800,
            "reference_price's doc: a rule of twenty points against 25,000 is 800 ppm"
        );
        assert_eq!(
            points_to_ppm_at(20, nifty_paisa),
            points_to_ppm(20),
            "the measured path and the constant path must agree at the constant's own level"
        );

        // BANKNIFTY at 52,000. The same doc: 800 ppm there is "FORTY-ONE
        // points", so the operator's twenty points must NOT come out as 800.
        //
        // The exact quantity is 4,160 paisa = 41.6 points. The doc's
        // "forty-one" is prose truncating 41.6, not a claim that the converter
        // floors -- so the paisa is asserted exactly and the printed figure is
        // allowed to be either side of the boundary. Asserting `== 41` here
        // would pin the ROUNDING MODE to a sentence that was never about it.
        let banknifty_paisa = 52_000 * PAISA_PER_POINT;
        assert_eq!(
            800 * banknifty_paisa / 1_000_000,
            4_160,
            "800 ppm on a 52,000 index is 4,160 paisa, i.e. 41.6 points"
        );
        assert_eq!(
            ppm_to_points_at(800, banknifty_paisa),
            42,
            "41.6 points rounds to nearest, and the doc's 'forty-one' is 41.6 truncated"
        );
        assert!(
            ppm_to_points_at(800, banknifty_paisa) > 2 * 20,
            "the doc's actual point: a NIFTY-derived 800 ppm DOUBLES a twenty-point \
             rule on BANKNIFTY"
        );
        assert!(
            points_to_ppm_at(20, banknifty_paisa) < 800,
            "twenty points on a higher index is FEWER ppm, not the NIFTY figure"
        );

        // The pair must be an exact inverse on the ladder's own rungs, or a
        // ladder built in ppm prints as a different ladder in points.
        for points in [STOP_FLOOR_POINTS, 10, 15, 20, MAX_STOP_POINTS] {
            assert_eq!(
                ppm_to_points_at(points_to_ppm_at(points, nifty_paisa), nifty_paisa),
                points,
                "round trip must be the identity at {points} points"
            );
        }

        // A reference that is not a price refuses rather than dividing by it.
        assert_eq!(points_to_ppm_at(20, 0), 0, "no price, no conversion");
        assert_eq!(ppm_to_points_at(800, -1), 0, "no price, no conversion");
    }

    /// A ladder has more than one rung, and for eight months it did not.
    ///
    /// # The defect this pins
    ///
    /// `stop_floor_points` and `max_stop_points` were BYTE-IDENTICAL — same
    /// filter, same sort, same median index, same divisor, same clamp, differing
    /// only in the empty-slice fallback. So `floor == cap` on every real series,
    /// `stop_ladder_ppm`'s `while halves <= cap_halves` ran exactly once, and
    /// a fixed 2.5-point step never advanced anything. Every sweep this
    /// repository has run tested ONE stop value, against a doc claiming a span
    /// of "5, 7.5, 10 … 25", while the operator's standing requirement is that
    /// the stop VARY.
    ///
    /// It survived because the test named for the span asserted the span only
    /// through `rungs.windows(2).all(..)`, which yields nothing on a one-element
    /// slice and is therefore vacuously true. A vacuous assertion is the
    /// "test that asserts nothing" `CLAUDE.md` §4 bans, wearing a name that says
    /// otherwise.
    ///
    /// The fixture is built here rather than taken from `synthetic`, because
    /// `synthetic::bar` gives every bar a range of 120..=195 paisa — ONE point
    /// for all of them once divided by [`PAISA_PER_POINT`] — so it cannot
    /// express a ladder no matter how the bounds are derived. These ranges span
    /// 2..=40 points, which is the shape a real index minute actually has.
    #[test]
    fn the_stop_ladder_has_more_than_one_rung_when_the_ranges_vary() {
        let bars: Vec<indicators::Candle> = (0..400_i64)
            .map(|i| {
                // 2..=40 points of range, cycling, so the 25th and 90th
                // percentiles land in genuinely different places.
                let half = (2 + i % 39).saturating_mul(PAISA_PER_POINT) / 2;
                let mid = synthetic::BASE.saturating_add(i.saturating_mul(10));
                indicators::Candle::new(
                    i.saturating_mul(60_000_000)
                        .saturating_add(synthetic::IST_OPEN_UTC_MICROS),
                    mid,
                    mid.saturating_add(half),
                    mid.saturating_sub(half),
                    mid,
                    1_000,
                    indicators::OI_NULL,
                )
            })
            .collect();

        let floor = stop_floor_points(&bars);
        let cap = crate::max_stop_points(&bars);
        assert!(
            floor < cap,
            "the floor and the cap must come from DIFFERENT places in the range \
             distribution, or there is no ladder to walk: floor {floor} cap {cap}"
        );

        let rungs = stop_ladder_ppm(&bars);
        assert!(
            rungs.len() > 1,
            "the stop must VARY: a one-rung ladder is the collapse where the \
             floor and the cap were the same expression: {rungs:?}"
        );
        assert!(
            rungs.windows(2).all(|w| w.first() < w.last()),
            "rungs ascend and are distinct: {rungs:?}"
        );
    }

    /// The shipped ladder lands where [`STOP_FLOOR_POINTS`] says it does.
    ///
    /// The units defect above was invisible at the ladder's own boundary
    /// because `stop_ladder_ppm` returns ppm and nothing converted it back to
    /// the points an operator speaks in. This asserts the FIRST rung is the
    /// floor and the LAST is no wider than the cap, in points, through the same
    /// converter the report prints with.
    #[test]
    fn the_stop_ladder_spans_the_points_its_constants_name() {
        let bars = synthetic::sessions(8);
        let reference = reference_price(&bars);
        // `synthetic::BASE` is 2,500,000 — NIFTY at 25,000 index, in paisa. The
        // fixture is itself evidence for the unit this whole block is about.
        assert!(
            reference > synthetic::BASE / 2,
            "the reference is paisa off the bars, not an index level: {reference}"
        );

        let rungs = stop_ladder_ppm(&bars);
        let tightest = *rungs.first().expect("a ladder is never empty");
        let widest = *rungs.last().expect("a ladder is never empty");
        assert!(
            rungs.windows(2).all(|w| w.first() < w.last()),
            "rungs ascend and are distinct: {rungs:?}"
        );

        // AND IT IS A LADDER HERE TOO, WHICH IT COULD NOT BE UNTIL TODAY.
        //
        // This block used to say no span could be asserted on this fixture:
        // `synthetic::bar` gives every bar a range of 120..=195 paisa, and the
        // ladder derived its ends from percentiles ROUNDED TO WHOLE POINTS, so
        // all of them collapsed to one point and floor equalled cap. The
        // fixture was blamed; the rounding was the cause. `stop_ladder_ppm` now
        // reads the percentiles in paisa, and 120 and 195 are two different
        // halves.
        //
        // Asserting it here matters because the check below is
        // `rungs.windows(2)`, which yields NOTHING on a one-element slice and is
        // therefore vacuously true — the "test that asserts nothing" S4 bans,
        // and the exact reason the original one-rung defect survived unseen.
        // Leaving it unasserted on the grounds that this fixture cannot express
        // a ladder made that vacuity permanent.
        assert!(
            rungs.len() > 1,
            "the stop must VARY on the shared fixture too, or the ascending \
             check above is vacuous and this test is guarding nothing: {rungs:?}"
        );

        // COMPARED IN PPM, which is the unit the ladder is built in. Going the
        // other way and comparing points would fold the inverse's rounding into
        // the assertion and test two things at once.
        assert_eq!(
            tightest,
            points_to_ppm_at(stop_floor_points(&bars), reference),
            "the tightest rung IS the DERIVED floor -- `STOP_FLOOR_POINTS` is a \
             NIFTY figure and BANKNIFTY travels 25 points in one minute: {rungs:?}"
        );

        let first = ppm_to_points_at(tightest, reference);
        let last = ppm_to_points_at(widest, reference);
        assert_eq!(
            first,
            stop_floor_points(&bars),
            "the floor rung prints as the derived floor: {rungs:?}"
        );
        assert!(
            last <= MAX_STOP_POINTS,
            "the widest rung never exceeds the cap: {last} > {MAX_STOP_POINTS}"
        );

        // THE DEFECT THIS PINS. Before the units fix every rung was 2..10 ppm,
        // which is under a tenth of a point and inside the entry bar's range.
        // A rung is now at least the floor, on any instrument, at any level.
        for &rung in &rungs {
            assert!(
                ppm_to_points_at(rung, reference) >= stop_floor_points(&bars),
                "rung {rung} ppm is {} points, tighter than the derived floor -- \
                 this is the hundredfold units defect returning",
                ppm_to_points_at(rung, reference)
            );
        }
    }

    #[test]
    fn the_support_a_real_run_used_prunes_the_cadence_it_was_hunting() {
        let floor = cadence_floor_ppm(96_280, 81, 1).expect("a real span has bars");

        assert!(
            floor < 20_000,
            "one trade a week needs {floor} ppm; the run was launched at 20,000. \
             If this no longer holds, the descent is solving a problem that went \
             away"
        );
        // Pinned as a range rather than a literal so a rounding change does not
        // fail the build, but a FIVEFOLD error does.
        assert!(
            (3_000..4_500).contains(&floor),
            "one trade a week over 81 months of 15-minute bars is about 3,655 \
             ppm; got {floor}"
        );
    }

    /// The same cadence is a DIFFERENT support on every rung.
    ///
    /// # Why the floor cannot be a constant
    ///
    /// 81 months holds 96,280 fifteen-minute bars and 1,444,200 one-minute ones.
    /// One trade a week is the same 352 trades on both -- and 3,655 ppm on one,
    /// 243 ppm on the other. A single `SUPPORT_PPM` applied across rungs asks a
    /// fifteen-fold different question at each, which is the same defect
    /// `range_all`'s own banner already warns about for `min_hits`.
    #[test]
    fn one_cadence_is_a_different_support_on_every_rung() {
        let fifteen = cadence_floor_ppm(96_280, 81, 1).expect("bars");
        let one = cadence_floor_ppm(1_444_200, 81, 1).expect("bars");

        assert!(
            fifteen > one * 10,
            "a fifteen-fold bar count must move the floor by roughly fifteen: \
             15min {fifteen} ppm against 1min {one} ppm"
        );
        assert!(
            one >= 1,
            "and the finer rung must not round away to nothing"
        );
    }

    /// Multiplying before dividing is what keeps a fine rung from reading zero.
    ///
    /// # The bug this would be
    ///
    /// `trades / bars` is integer zero for every cadence worth hunting -- 352
    /// over 1,444,200 is 0 -- and scaling a zero gives 0 ppm, which reads as "no
    /// floor at all" and sweeps the entire space. The order of operations is the
    /// whole defence and nothing else in the function would catch it.
    #[test]
    fn a_fine_rung_never_rounds_its_floor_away_to_nothing() {
        for bars in [500_000_u64, 1_444_200, 10_000_000, 100_000_000] {
            let floor = cadence_floor_ppm(bars, 81, 1).expect("bars");
            assert!(
                floor >= 1,
                "a floor of zero prunes nothing and sweeps everything: \
                 {bars} bars gave {floor} ppm"
            );
        }
    }

    /// A span with no bars has no floor, and says so rather than dividing.
    #[test]
    fn an_empty_span_has_no_floor_rather_than_a_division() {
        assert_eq!(cadence_floor_ppm(0, 81, 1), None);
        assert_eq!(cadence_floor_ppm(96_280, 0, 1), None);
        assert_eq!(cadence_floor_ppm(96_280, 81, 0), None);
    }

    /// The ladder ends ON the floor, never merely near it.
    ///
    /// # Why the last rung is appended and not divided into
    ///
    /// Halving from an arbitrary ceiling lands somewhere above the floor and
    /// stops. At 20,000 halving toward 3,655 the last computed rung is 5,000 --
    /// still 1.4x too strict, still pruning exactly the cadence the walk exists
    /// to reach. The descent would report itself complete having never asked the
    /// question. §4 bans that shape by name.
    #[test]
    fn the_ladder_lands_exactly_on_the_floor() {
        let ladder = support_ladder(20_000, 3_655);

        assert_eq!(
            ladder.last().copied(),
            Some(3_655),
            "the walk must END on the operator's cadence: {ladder:?}"
        );
        assert_eq!(
            ladder.first().copied(),
            Some(20_000),
            "and start where it was told to: {ladder:?}"
        );
        for pair in ladder.windows(2) {
            // `.expect` and not `unreachable!`: the latter is a project region
            // that can never execute, so it can never be covered, and the
            // coverage floor in `CLAUDE.md` §9 is 100%. `.expect` panics inside
            // std, which is not instrumented, and costs nothing.
            let a = pair.first().copied().expect("windows(2) yields two");
            let b = pair.get(1).copied().expect("windows(2) yields two");
            assert!(a > b, "the ladder must descend strictly: {a} then {b}");
        }
    }

    /// A ceiling already at or below the floor is one rung, not an empty walk.
    ///
    /// An empty ladder would sweep nothing and report a finished descent, which
    /// is indistinguishable from a descent that found nothing.
    #[test]
    fn a_ceiling_below_the_floor_still_walks_once() {
        assert_eq!(support_ladder(1_000, 3_655), vec![3_655]);
        assert_eq!(support_ladder(3_655, 3_655), vec![3_655]);
        assert_eq!(support_ladder(500, 0), vec![1]);
    }

    /// Months are counted inclusively, because a span holds both its ends.
    #[test]
    fn a_span_holds_both_of_its_ends() {
        assert_eq!(months_between((2019, 12), (2026, 8)), 81);
        assert_eq!(months_between((2024, 6), (2024, 6)), 1);
        assert_eq!(months_between((2024, 1), (2024, 12)), 12);
        // Backwards is not negative months; it saturates to the single month it
        // was given, because a negative span cannot be swept and the caller's
        // own range check is what refuses it.
        assert_eq!(months_between((2026, 8), (2019, 12)), 1);
    }

    /// Consistency is MEASURED, end to end, from bars to per-grain shares.
    ///
    /// # The gap this closes, and why a compile is not enough
    ///
    /// `crate::stability` was written with six tests and had ZERO callers, and
    /// `runner::grid::per_trade` had none either. Both halves of the operator's
    /// "same every month, every week, every quarter, every year" requirement
    /// existed and neither was reachable from anything that runs.
    ///
    /// Wiring them compiles whether or not the chain actually produces buckets:
    /// `per_trade` returning `None`, or returning rows whose timestamps all land
    /// in one bucket, would both build cleanly and render a table of zeroes that
    /// looked like a measurement. This walks the real chain -- sweep, rank,
    /// grid, chosen variant, per-trade rows, six grains -- and asserts the
    /// output could only come from actual bucketing.
    #[test]
    fn consistency_is_measured_from_real_bars_and_not_merely_wired() {
        let bars = runner::synthetic::sessions(6);
        let mut ev = evaluator().expect("the synthetic evaluator builds");
        let ladder = ladder_for(20).expect("a ladder at twenty hits");
        let horizon = Horizon::DEFAULT;
        let run = runner::Sweeper::new(ladder).run_ranked(&bars, &mut ev, horizon, 3);

        let Some(scored) = run.ranked.top.first() else {
            panic!("this fixture must rank at least one combination");
        };
        let side = side_of_evidence(scored);
        let stop_rungs = stop_ladder_ppm(&bars);
        let exits = grid::evaluate(
            &bars,
            &run.column,
            &scored.mask,
            horizon,
            side,
            grid::Levels {
                // TWO RUNGS, not the derived count. The chain under test is
                // bars -> rows -> buckets, and grid WIDTH changes only how
                // many variants are searched before one is chosen. The
                // derived count made this test take over a minute and proved
                // nothing the two-rung grid does not.
                rungs: 2,
                step_ppm: Some(grid_step_ppm(&bars)),
                forced: None,
                ratios: false,
                stops_ppm: &stop_rungs,
            },
        );
        let cell = *exits.best().expect("the grid must hold a best variant");

        let measured = consistency_of(&bars, &run.column, scored, horizon, side, &exits, &cell)
            .expect("a variant with trades must re-walk into per-trade rows");

        // THE ASSERTION THAT PROVES BUCKETING HAPPENED.
        //
        // Six synthetic sessions span six days, so the DAILY grain must
        // hold more periods than the yearly one. A chain that returned rows but
        // failed to bucket them -- every trade landing in one bucket, or the
        // timestamp arithmetic collapsing -- makes every grain identical, and
        // that is exactly the failure a compile cannot catch.
        assert!(
            measured.years >= 1,
            "six sessions must fall in at least one year, got {}",
            measured.years
        );
        for slot in 0..6 {
            let share = measured.share_bp(slot);
            assert!(
                (0..=10_000).contains(&share),
                "grain {slot} reported {share}bp, outside the basis-point range \
                 -- a share is a proportion and cannot exceed its whole"
            );
        }
        assert!(
            measured.weakest_bp() <= measured.share_bp(0),
            "the weakest grain cannot exceed the yearly one: {} against {}",
            measured.weakest_bp(),
            measured.share_bp(0)
        );
        assert_eq!(
            measured.weakest_bp(),
            (0..6).map(|g| measured.share_bp(g)).min().unwrap_or(0),
            "WEAKEST must be the minimum across grains and not one of them"
        );
    }

    /// The weakest grain is the minimum, never the mean.
    ///
    /// # Why the distinction decides whether the rule works
    ///
    /// A strategy positive in every year and negative in half its months is not
    /// consistent, and it is the exact shape an operator asking for "every month
    /// AND every year the same" is trying to exclude. Averaging 10,000 and 5,000
    /// reports 7,500 and admits it; taking the minimum reports 5,000 and refuses.
    #[test]
    fn the_weakest_grain_is_the_minimum_and_never_the_mean() {
        let uneven = Consistency {
            shares_bp: [10_000, 10_000, 5_000, 10_000, 10_000, 10_000],
            worst_day: -50_000,
            years: 7,
        };
        assert_eq!(
            uneven.weakest_bp(),
            5_000,
            "one bad grain is the answer, not one sixth of it"
        );
        // The mean would be 9,166 and would clear a 9,000 rule this must fail.
        assert!(
            uneven.weakest_bp() < 9_000,
            "a strategy negative in half its months must not clear a 90% rule"
        );
    }

    /// A grain past the list reads zero rather than panicking on an index.
    #[test]
    fn a_grain_past_the_list_reads_zero_rather_than_panicking() {
        let c = Consistency {
            shares_bp: [10_000; 6],
            worst_day: 0,
            years: 1,
        };
        assert_eq!(c.share_bp(6), 0);
        assert_eq!(c.share_bp(usize::MAX), 0);
    }

    /// Every rule that can refuse a row is NAMED when it does.
    ///
    /// # The single character this fixes
    ///
    /// `why_refused` knew two rules of six. A row refused for its win rate, its
    /// trade count, its confidence or its consistency printed `-` in the rule
    /// column -- the identical glyph a PASSING row prints. `admitted` was false
    /// and the reason said nothing was wrong.
    #[test]
    fn every_rule_that_refuses_a_row_says_which_one_it_was() {
        let strict = crate::Rules {
            max_mae_ppm: 1_000,
            min_rr_bp: 200,
            min_win_rate_bp: 9_000,
            min_assurance_bp: 8_000,
            min_weakest_bp: 9_000,
            min_trades: 50,
            min_ret_over_dd_bp: 0,
            top: 25,
        };
        // Each cell breaks exactly one rule, so the label is unambiguous.
        let mut wide = perfect_cell(100);
        wide.worst_mae = 5_000;
        assert_eq!(crate::why_refused(&wide, strict, true), "MAE");

        // `reward_to_risk_bp` is `min_win / worst_trade`, and returns i64::MAX
        // when `worst_trade` is zero -- so a cell with no losing trade can never
        // fail this rule, which is why both fields are set here.
        let mut thin = perfect_cell(100);
        thin.worst_trade = -1_000;
        thin.min_win = 100;
        thin.wins = 10;
        assert_eq!(
            crate::why_refused(&thin, strict, true),
            "R:R",
            "a poor reward-to-risk is named before the win rate it also fails"
        );

        let mut losing = perfect_cell(100);
        losing.wins = 50;
        assert_eq!(crate::why_refused(&losing, strict, true), "win%");

        assert_eq!(
            crate::why_refused(&perfect_cell(10), strict, true),
            "few",
            "ten trades is below the fifty this rule set demands"
        );

        // 60 perfect trades: rate 100% clears `win%`, count clears `few`, and
        // the bound is 93.98% which clears 80% -- so only consistency is left.
        assert_eq!(
            crate::why_refused(&perfect_cell(60), strict, false),
            "steady",
            "an inconsistent row must say so rather than printing a dash"
        );
        assert_eq!(
            crate::why_refused(&perfect_cell(60), strict, true),
            "-",
            "and a row that breaks nothing prints the dash"
        );
    }

    /// Confidence and win rate are named SEPARATELY, because they are opposite
    /// findings.
    ///
    /// One says the strategy did not win often enough. The other says it did
    /// not win often enough TIMES for the rate to mean anything. Collapsing
    /// them into one label would tell an operator to improve the wrong thing --
    /// a 100%-winning combination refused for confidence needs MORE TRADES, and
    /// no amount of tuning the exit will produce them.
    #[test]
    fn confidence_and_win_rate_are_named_separately() {
        let rules = crate::Rules {
            max_mae_ppm: i64::MAX,
            min_rr_bp: 0,
            min_win_rate_bp: 9_000,
            min_assurance_bp: 9_000,
            min_weakest_bp: 0,
            min_trades: 0,
            min_ret_over_dd_bp: 0,
            top: 25,
        };
        // 20 of 20: a rate of 100% clears the win-rate rule outright, and a
        // bound of 83.88% fails the confidence one.
        let tiny = perfect_cell(20);
        assert!(tiny.win_rate_bp() >= rules.min_win_rate_bp);
        assert!(tiny.assurance_bp() < rules.min_assurance_bp);
        assert_eq!(
            crate::why_refused(&tiny, rules, true),
            "conf",
            "a perfect but small record is refused for CONFIDENCE, not for \
             winning too little"
        );
    }

    /// A rule switched off reads `off`, never `0.00%`.
    ///
    /// Every rule in [`Rules`] documents zero as DROPPING the rule. A banner
    /// reading "at least 0.00% of trades must win" states a rule that is not
    /// being applied, which is the same lie as omitting it.
    #[test]
    fn a_rule_that_is_off_says_off_and_not_zero_percent() {
        assert_eq!(crate::bp_as_percent(0), "off");
        assert_eq!(crate::bp_as_percent(9_000), "90.00%");
        assert_eq!(crate::bp_as_percent(9_550), "95.50%");
        assert_eq!(crate::bp_as_percent(10_000), "100.00%");
    }

    /// A descent rung whose row did not land refuses, instead of printing an
    /// older run's numbers under this run's banner.
    ///
    /// # The substitution this closes
    ///
    /// `one_rung` discards the long report and reads the row back out of the
    /// ledger by KEY -- feed, underlying, rung, span, `min_hits` -- and the key
    /// carries no identity. So an append that failed did not make the read fail
    /// with it: `latest_for` returned the newest row matching that key, which is
    /// an EARLIER run at a different commit. Nine plausible rows, each possibly
    /// from a different binary.
    ///
    /// It was reachable rather than theoretical: `Results::append` refuses a
    /// ledger version it does not write while `read_at` reads older rows and
    /// widens them, so a store holding a v2 ledger under a v3 build failed every
    /// append and satisfied every read.
    ///
    /// Both `record_run` failure paths open with [`crate::NOT_RECORDED`], and
    /// both sides name that constant rather than repeating the sentence -- a
    /// caller matching a literal the producer may reword is a check that lapses
    /// silently the first time somebody edits the message.
    #[test]
    fn a_row_that_did_not_reach_the_ledger_is_read_as_a_refusal_and_not_a_lookup() {
        // The two shapes `record_run` actually returns, built from the constant
        // both sides share so a reword cannot pass this test by accident.
        for why in ["the store is unwritable", "this ledger is version 2"] {
            let report = format!(
                "{}: {why}\n  The figures below are correct; only the row is missing.\n\n",
                crate::NOT_RECORDED
            );
            let seen = crate::not_recorded_reason(&report).expect("the sentence is present");
            assert_eq!(seen, why, "the reason is carried, not just the fact");
            assert!(
                !seen.contains("figures below"),
                "one line only -- there are no figures below a descent row: {seen}"
            );
        }

        // The marker mid-report is still a failure. `record_run`'s output is
        // appended to a longer document, so it is not at position zero.
        let embedded = format!("RUN\n  bars 1000\n{}: disk full\n", crate::NOT_RECORDED);
        assert_eq!(
            crate::not_recorded_reason(&embedded).as_deref(),
            Some("disk full")
        );

        // AND THE ORDINARY CASE STAYS A LOOKUP. A recorded run must not be
        // turned into a refusal -- that would trade a silent wrong answer for a
        // loud wrong one.
        assert_eq!(
            crate::not_recorded_reason("RESULT RECORDED\n  row 7\n"),
            None
        );
        assert_eq!(crate::not_recorded_reason(""), None);
    }
}

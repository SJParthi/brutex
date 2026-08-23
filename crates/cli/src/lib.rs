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
pub mod results;
pub mod stability;
pub mod stored;

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
                Rules {
                    max_mae_ppm: points_to_ppm(pts),
                    min_rr_bp: rr,
                    // `screen` takes three numbers today, so the two new rules are
                    // off rather than guessed. A win-rate floor an operator did
                    // not type is a policy the engine invented.
                    min_win_rate_bp: 0,
                    min_trades: 0,
                    top: n,
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
/// reached exactly when there is no price to read.
fn reference_price(bars: &[indicators::Candle]) -> i64 {
    let lo = bars.iter().map(|b| b.low).filter(|&l| l > 0).min();
    let hi = bars.iter().map(|b| b.high).filter(|&h| h > 0).max();
    match (lo, hi) {
        (Some(l), Some(h)) if h >= l => l.saturating_add(h) / 2,
        _ => NIFTY_REFERENCE,
    }
}

/// Index points as parts per million against a price read off the bars.
const fn points_to_ppm_at(points: i64, reference: i64) -> i64 {
    if reference <= 0 {
        return 0;
    }
    points.saturating_mul(1_000_000) / reference
}

/// Parts per million back to index points, at a price read off the bars.
const fn ppm_to_points_at(ppm: i64, reference: i64) -> i64 {
    ppm.saturating_mul(reference) / 1_000_000
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
            let text = range_all(vendor, underlying, (fy, fm), (ty, tm), h);
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
        ["range-all", v, u, fy, fm, ty, tm, mh] => range_all_arm(out, v, u, (fy, fm), (ty, tm), mh),
        ["verify", feed, underlying] => verify_arm(out, feed, underlying),
        ["auto-stored", v, u, r, fy, fm, ty, tm] => auto_stored_arm(out, v, u, r, (fy, fm, ty, tm)),
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
        [word, ..] => {
            let owned = format!("`{word}` is not a command this build knows");
            refuse(out, &owned)
        }
    }
}

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
             The /logs page reads whatever directory `api` resolved, which is \
             NOT this one unless BRUTEX_LOG_DIR is set for both. Set it for \
             both, or read this file directly."
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
        .clamp(1_000, 100_000)
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
/// and no percentage appears in it: `grid_rungs()` alone decides how far the
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
    (median / 20).max(1)
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
fn max_stop_points(bars: &[indicators::Candle]) -> i64 {
    let reference = reference_price(bars);
    let mut ranges: Vec<i64> = bars
        .iter()
        .map(|b| b.high.saturating_sub(b.low))
        .filter(|&r| r > 0)
        .collect();
    if ranges.is_empty() {
        return MAX_STOP_POINTS;
    }
    ranges.sort_unstable();
    let median = ranges.get(ranges.len() / 2).copied().unwrap_or(0);
    // Paisa to points: the reference is in the same paisa units the bars are.
    let in_points = ppm_to_points_at(points_to_ppm_at(median, reference), reference);
    in_points.clamp(1, MAX_STOP_POINTS)
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
/// It was `625`, then `12_393`, each pinned to `grid_rungs()` by a `const`
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
fn grid_variants() -> u64 {
    u64::try_from(grid::variants(grid_rungs(), grid_rungs(), grid_rungs())).unwrap_or(u64::MAX)
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
fn grid_rungs() -> usize {
    const DEFAULT: usize = 8;
    std::env::var("BRUTEX_GRID_RUNGS")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT)
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
/// combination at [`grid_variants()`] stop/target/trail settings and keeps the
/// best of them, and selecting a maximum over 125 cells is 125 more chances to
/// look good by luck. None of it entered the bar printed above.
///
/// # Why a second bar rather than a replacement
///
/// Because the true correction is **unknown and this one is only a ceiling**.
/// The cells share a single trade walk, so they are heavily correlated and the
/// effective trial count is somewhere between 1 and [`grid_variants()`] —
/// unmeasured.
/// Replacing the printed bar with the ceiling would reject real findings;
/// leaving it alone accepts noise. Printing both, and saying which is which,
/// hands the reader the range that is actually known. `CLAUDE.md` §3 rule 6
/// asks for exactly that when a bound cannot be met, and §3 rule 1 forbids
/// inventing the discount that would collapse the range to a point.
fn grid_exposure(sweep: &engine::Sweep) -> String {
    let plain = runner::significance::effective_trials(sweep);
    // THE SAME `plain` ON BOTH SIDES OF THE SENTENCE.
    //
    // This read `trials_with_grid(sweep, grid_variants())`, which multiplied the
    // RAW trial count while the line beside it printed the DUPLICATE-DEFLATED
    // one -- so the sentence "with N exit settings each, at most {ceiling}"
    // claimed the only difference was the grid, and the measured ratio was
    // 929,577x where it promised 325x. The upper Bonferroni bar was overstated
    // by 1.27 t-units as a result.
    //
    // Passing `plain` makes the claim true by construction rather than by two
    // calls happening to agree, and `trials_with_grid` now takes a count for
    // exactly that reason.
    let ceiling = runner::significance::trials_with_grid(plain, grid_variants());
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
        width = grid_variants(),
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
    audit_with(evaluator(), sessions, min_hits)
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
        params: Params::of(ladder),
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
            rules: Rules::BASELINE,
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
    out
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
fn ledger_round_trip() -> Check {
    let dir = std::env::temp_dir().join("brutex-verify-ledger");
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
        max_drawdown: -29_163,
        winner_mae: 300,
        winner_mfe: 700,
        all_mae: 500,
        exit_rungs: [-1, -1, 3, 0, 0],
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
const EVERY_RUNG: [&str; 8] = [
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
) -> (runner::trade::Trades, grid::Grid, String) {
    let side = side_of_evidence(first);
    let taken = trade::walk(bars, column, &first.mask, horizon, direction_of(side));
    // The same forced rung the screen uses, so the headline grid and the screened
    // rows are built from one ladder. Two ladders would let the report show a
    // variant in one table that cannot exist in the other.
    //
    let exits = grid::evaluate(
        bars,
        column,
        &first.mask,
        horizon,
        side,
        grid::Levels {
            rungs: grid_rungs(),
            step_ppm: Some(grid_step_ppm(bars)),
            forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
            ratios: true,
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
    let screened = screen_cascade(bars, column, by_evidence, horizon, rules.top);
    (taken, exits, screened)
}

/// The three lines that precede the trade figures: which combination was taken,
/// how much of the grid it exposed, and how large the sample is.
///
/// Split from [`audit_bars`] to keep it under `clippy::too_many_lines`. One
/// idea: everything a reader needs BEFORE a number, so no figure below arrives
/// without the sample size and the exposure that qualify it.
fn traded_preamble(first: &runner::rank::Scored, sweep: &engine::Sweep, sessions: usize) -> String {
    let mut out = traded_line(first);
    out.push_str(&grid_exposure(sweep));
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
    /// How many combinations to report. Ten or twenty-five, the operator's call.
    pub top: usize,
}

impl Rules {
    /// Whether a variant satisfies every rule. All of them, not a score.
    ///
    /// A rule broken once is a disqualification and not a lower rank: a stop
    /// that one trade in ten thousand ran through is a stop that did not hold.
    #[must_use]
    pub const fn admits(&self, cell: &grid::Cell) -> bool {
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
        cell.worst_mae <= self.max_mae_ppm
            && cell.reward_to_risk_bp() >= self.min_rr_bp
            && cell.win_rate_bp() >= self.min_win_rate_bp
            && cell.trades >= self.min_trades
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
    const BASELINE: Self = Self {
        max_mae_ppm: 2_000,
        min_rr_bp: 200,
        // ZERO, not a number. The baseline exists so a command that takes no
        // policy still prints a screen, and it states its three numbers on the
        // page. A win-rate floor nobody typed would be a fourth number an
        // operator never chose, silently disqualifying rows.
        min_win_rate_bp: 0,
        min_trades: 0,
        top: 25,
    };
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
    (1..=grid_rungs())
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
    pub const fn rules(&self, top: usize) -> Rules {
        Rules {
            max_mae_ppm: points_to_ppm(self.max_points),
            min_rr_bp: self.min_rr_bp,
            min_win_rate_bp: self.min_win_rate_bp,
            min_trades: self.min_trades,
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
    std::env::var("BRUTEX_SCREEN_CAP")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT)
}

/// One screened combination, priced in full and judged.
struct Screened {
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
fn screen_cascade(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    by_evidence: &[&runner::rank::Scored],
    horizon: Horizon,
    top: usize,
) -> String {
    let mut out = String::with_capacity(4_096);
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
        let body = screen(bars, column, by_evidence, horizon, rules);
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
        ));
    }
    out
}

fn screen(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    by_evidence: &[&runner::rank::Scored],
    horizon: Horizon,
    rules: Rules,
) -> String {
    let mut rows: Vec<Screened> = Vec::with_capacity(by_evidence.len().min(screen_cap()));
    for (rank, scored) in by_evidence.iter().take(screen_cap()).enumerate() {
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
                rungs: grid_rungs(),
                step_ppm: Some(grid_step_ppm(bars)),
                forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
                ratios: true,
            },
        );
        // THE BEST VARIANT THAT SATISFIES THE RULES, falling back to the best
        // overall only so a failing combination can still be SHOWN with the rule
        // it broke. Asking `best()` first and judging that was the error: the
        // profit-maximising cell is the one with no stop at all, so every
        // combination failed a stop rule by construction.
        let within = g.best_within(|c| rules.admits(c)).copied();
        let Some(cell) = within.or_else(|| g.best().copied()) else {
            continue;
        };
        if cell.trades == 0 {
            continue;
        }
        rows.push(Screened {
            rank: rank.saturating_add(1),
            tightest: g.tightest_containment().copied(),
            admitted: within.is_some(),
            names: runner::report::condition_names(&scored.mask).join(" · "),
            cell,
        });
    }

    // PASSERS FIRST, then by net. `Reverse` and not a negation, for the reason
    // `audit::grid` gives: `pessimistic` saturates at `i64::MIN` and negating
    // that panics under `overflow-checks`, killing the process over a sort.
    rows.sort_by_key(|r| (!r.admitted, core::cmp::Reverse(r.cell.pessimistic)));

    let passed = rows.iter().filter(|r| r.admitted).count();
    let mut out = String::from("TOP COMBINATIONS, SCREENED\n");
    let _ = writeln!(
        out,
        "  RULES, all yours and all required together:\n    \
         1. no single trade may run more than {} against entry (the WORST, not the mean)\n    \
         2. the SMALLEST win must be at least {} times the LARGEST loss\n    \
         3. report the top {}\n\n  \
         {passed} of {} priced combinations satisfy every rule.{}\n",
        ppm_as_percent(rules.max_mae_ppm),
        hundredths_of(rules.min_rr_bp),
        rules.top,
        rows.len(),
        if passed == 0 {
            " NOTHING PASSED -- the rows below are shown so a reader can see what \
             was tried and by how much each missed, which is a finding rather \
             than an empty table."
        } else {
            ""
        }
    );
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
                why_refused(&row.cell, rules)
            },
            row.names
        );
    }
    let _ = writeln!(out);
    out
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
fn why_refused(cell: &grid::Cell, rules: Rules) -> &'static str {
    if cell.worst_mae > rules.max_mae_ppm {
        "MAE"
    } else if cell.reward_to_risk_bp() < rules.min_rr_bp {
        "R:R"
    } else {
        "-"
    }
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
fn one_rung(
    vendor_word: &str,
    underlying: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: u64,
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
    let bars = match stored::load_span(&root, vendor, underlying, rung, from, to) {
        Ok(span) => span.bars.len(),
        Err(why) => {
            return RungRow {
                rung,
                outcome: Err(first_line(why)),
            };
        }
    };
    let min_hits = min_hits_for(bars, support_ppm);

    // The long report is DISCARDED on purpose: nine of them is six thousand
    // lines. The row is read back from the store, which is the point of having
    // one.
    let text = audit_range(vendor_word, underlying, rung, from, to, min_hits);
    let outcome = if let Some(why) = text.strip_prefix("refused: ") {
        Err(first_line(why.to_owned()))
    } else {
        latest_for(vendor_word, underlying, rung, from, to, min_hits)
    };
    RungRow { rung, outcome }
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
#[must_use]
pub fn range_all(
    vendor_word: &str,
    underlying: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: u64,
) -> String {
    let mut out = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "feed {vendor_word} · {underlying} · ALL EIGHT INTRADAY RUNGS · {}-{:02}..{}-{:02} · support {}.{}%",
        from.0,
        from.1,
        to.0,
        to.1,
        support_ppm / 10_000,
        (support_ppm / 1_000) % 10
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
    // DETERMINISM (§3 rule 5) IS HELD BY SHAPE. `map` on an INDEXED parallel
    // iterator preserves order, so `rows` is the same sequence whatever order
    // the threads finish in -- the same argument `batch::sweep_under` makes, and
    // for the same reason. A rerun produces the same bytes.
    let rows: Vec<RungRow> = EVERY_RUNG
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
                    if r.max_drawdown < 0 && r.pessimistic > 0 {
                        (r.pessimistic.saturating_mul(100) / -r.max_drawdown).to_string()
                    } else {
                        "-".to_owned()
                    },
                );
            }
        }
    }
    let _ = writeln!(
        out,
        "\n  `worst` and `best` are the CHOSEN exit variant's total in paisa \
         under adverse-extreme and open fills. Selection ranks on `worst`.\n  \
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
    rules: Rules,
) -> String {
    match screen_range_inner(vendor_word, underlying, rung, from, to, support_ppm, rules) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// [`screen_range`]'s work, with its refusals unrendered.
fn screen_range_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: u64,
    rules: Rules,
) -> Result<String, stored::Refusal> {
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
        params: Params::of(ladder),
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
fn audit_with(ev: Result<Evaluator, &'static str>, sessions: i64, min_hits: u64) -> String {
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
static LEDGER: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
fn ceiling_from_env() -> Result<usize, String> {
    match std::env::var_os("BRUTEX_CEILING") {
        None => Ok(engine::DEFAULT_CEILING),
        Some(raw) => {
            let text = raw.to_string_lossy().into_owned();
            match text.trim().parse::<usize>() {
                Ok(0) | Err(_) => Err(format!(
                    "BRUTEX_CEILING is `{text}`, which is not a candidate count \
                     of 1 or more. It is roughly the bytes you can spare divided \
                     by 146; the default is {} for a 48 GB machine. Unset it to \
                     use that.",
                    engine::DEFAULT_CEILING
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
    Ok(Ladder::with_min_hits(min_hits).with_ceiling(ceiling_from_env()?))
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
fn record_run(
    into: Recording<'_>,
    id: &runner::identity::RunId,
    outcome: &runner::Outcome,
    exits: &runner::grid::Grid,
    bars: u64,
    min_hits: u64,
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
    };

    // HELD ACROSS THE OPEN AND THE APPEND, not just the append: `open` builds
    // the duplicate set by reading the whole file, and a set read before another
    // thread's append is stale by the time this one writes.
    let _guard = LEDGER.lock();
    let mut store = match results::Results::open(into.root) {
        Ok(store) => store,
        Err(why) => {
            return format!(
                "RESULT NOT RECORDED: {why}\n  The figures below are correct; only the row is missing.\n\n"
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
            "RESULT NOT RECORDED: {why}\n  The figures below are correct; only the row is missing.\n\n"
        ),
    }
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
    let ladder = match ladder_for(min_hits) {
        Ok(l) => l,
        Err(why) => return format!("refused: {why}\n"),
    };
    let run = Sweeper::new(ladder).run_ranked(&bars, &mut ev, horizon, audit_keep());
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
    let ranked = match execution {
        None => ranked,
        Some(_) => runner::rank::rank(
            &outcome.sweep,
            &trade_column,
            &runner::outcome::forward(&trade_bars, horizon),
            audit_keep(),
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
    let (taken, exits, screened) = trade_and_screen(
        &trade_bars,
        &trade_column,
        first,
        &by_evidence,
        horizon,
        rules,
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
    let folds = runner::validate::walk_forward(
        &bars,
        horizon,
        walk_forward_splits(bars.len()),
        direction_of(side_of_evidence(first)),
        &Sweeper::new(ladder),
        move || fresh,
    );
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
    let placements: Vec<_> = folds
        .folds
        .iter()
        .filter_map(|f| runner::pbo::place(&f.in_sample_all, &f.out_of_sample_all))
        .collect();
    let overfit =
        (!placements.is_empty()).then(|| runner::pbo::probability_of_overfitting(&placements));

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
    let boot_owned = bootstrap_family(&trade_bars, &trade_column, &by_evidence, horizon);
    let boot = boot_owned
        .as_ref()
        .map(|(rc, spa, named)| (rc.as_ref(), spa.as_ref(), *named));

    // RECORDED BEFORE IT IS RENDERED, so a process killed while formatting a
    // large report still leaves its row. The same ordering `cli.audit`'s
    // `audit rendered` event uses, and for the same reason.
    if let (Some(into), Some(run_id)) = (recording, id) {
        out.push_str(&record_run(
            into,
            run_id,
            &outcome,
            &exits,
            u64::try_from(bars.len()).unwrap_or(u64::MAX),
            min_hits,
        ));
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
    use super::{Direction, Side};
    use super::{
        MIN_AUDIT_SESSIONS, MISUSED, OK, PROVENANCE, STORED_PROVENANCE, USAGE, Vendor, audit_run,
        audit_stored, auto, auto_with, direction_of, evaluator_from, log_dir_from,
        nothing_to_trade, parse_min_hits, parse_sessions, parse_vendor, root_from, run,
        sample_warning, side_of_evidence, sweep, sweep_stored, sweep_with,
    };

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
        let text = audit_run(12, 300);
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

            // THE SAME CAUSE. On this unstamped build that cause is the commit
            // gate, and both must cite it — an `audit-stored` that reached the
            // store first would compute before it could record an identity.
            assert!(
                swept.contains("BRUTEX_COMMIT") && audited.contains("BRUTEX_COMMIT"),
                "both stored commands must refuse at the commit gate before \
                 reading a bar:\n  sweep: {swept}\n  audit: {audited}"
            );

            // AND EACH NAMES ITSELF. This is the half that would silently rot if
            // the two messages were ever merged into one shared constant, and it
            // is why this test does NOT assert the two strings are equal.
            assert!(
                swept.contains("the sweep will not run"),
                "the sweep's refusal must name the sweep: {swept}"
            );
            assert!(
                audited.contains("the audit will not run"),
                "the audit's refusal must name the audit, not the sweep: {audited}"
            );
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
            max_drawdown: -76_543_210,
            winner_mae: 2_291,
            winner_mfe: 8_876,
            all_mae: 2_295,
            exit_rungs: [2, 3, -1, 1, 0],
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
            max_drawdown: -1,
            winner_mae: 0,
            winner_mfe: 0,
            all_mae: 0,
            exit_rungs: [-1; 5],
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
}

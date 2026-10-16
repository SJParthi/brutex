//! `cli tix-rebuild` — the `.tix` time index for every named month of one
//! spot series, built through the store's writer-open index step. D-3340,
//! WP-15; DB-064, DB-G81, DB-G88.
//!
//! # Usage
//!
//! ```text
//! cli tix-rebuild VENDOR EXCHANGE/SEGMENT/SYMBOL RUNG FROM TO [--dry-run]
//! cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2024-07
//! ```
//!
//! `FROM` and `TO` are `YYYY-MM`, inclusive. The instrument is the store's own
//! three path segments, spot only — the series an option's spot and greeks
//! read (`/spot.json`, D-2807) — so nothing about it is inferred.
//!
//! # Why it exists
//!
//! The index one-second months from 2018-09 to 2024-07 — 71 NIFTY and 71
//! BANKNIFTY, 142 files — were written before D-2329 and carry no `.tix`, so
//! every spot lookup an option page makes there bisects: about 35 µs an instant
//! against 0.85 µs, 14 ms a 400-instant chunk against 1 ms, and one `store.tix`
//! warning per request (DB-064, DB-G81). The writer door builds a missing
//! index on open, but nothing reopens a month for writing once its import is
//! done. This verb does exactly that step and nothing else, through
//! `store::file::BarFile::index_existing`:
//!
//! * a month that is not held is reported and NOTHING is created for it;
//! * the bar file and its checksums are opened read-only and never change;
//! * a month another process holds — a writer, or a reader mid-request — is
//!   refused by the store's own exclusive lock, named, and left as it was;
//! * a month already indexed is kept, after three reads, so a rerun is safe;
//! * each month built is then reopened through the READ door and asked how it
//!   looks a timestamp up, so "indexed" in the receipt is what a reader sees,
//!   not what the writer believes.
//!
//! `--dry-run` opens every month read-only, says which are indexed and which
//! would be built, and writes nothing.
//!
//! # The small-month rule (D-4933)
//!
//! A month that holds fewer than 1,024 bars (`store::time_index::INDEX_MIN_ROWS`)
//! and has no `.tix` is given none, by the verb as by the writer: it is
//! reported `small`, nothing is written for it, and it counts as clean,
//! because its lookups bisect in at most ten bar reads and that is not a
//! finding. The commit that brings such a month to the threshold writes its
//! index whole. A `.tix` already beside a small month is kept or rebuilt as
//! any other.
//!
//! # Cost
//!
//! Building one month reads every committed bar once through the verified
//! read path — O(`n_valid`), and a one-second session month holds at most
//! 22,500 bars a session — and writes its entries once; keeping one is three
//! reads. The verb walks the
//! months one at a time and holds one month's lock at a time.
//!
//! # Exit
//!
//! [`OK`] only when every month in the span is held and a reader finds it
//! indexed (or, with `--dry-run`, when every month is held and readable);
//! [`FAILED`] for any month not held, refused, unindexable or not indexed after
//! the step, so a script cannot read a partial rebuild as a clean one;
//! [`MISUSED`] for arguments that do not name a series and a span, before
//! anything is opened.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::{BarFile, Reindexed, StoreError, TimeLookup};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::time_index::{INDEX_MIN_ROWS, Why};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli tix-rebuild VENDOR EXCHANGE/SEGMENT/SYMBOL RUNG FROM TO \
     [--dry-run]  (FROM and TO are YYYY-MM, inclusive; e.g. gdfl NSE/INDEX/NIFTY 1s 2018-09 \
     2024-07)";

/// What the arguments name: one spot series and an inclusive span of months.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    vendor: Vendor,
    exchange: String,
    segment: String,
    symbol: String,
    rung: Timeframe,
    months: Vec<YearMonth>,
    dry_run: bool,
}

impl Ask {
    /// The store path of one month of the series.
    fn path(&self, month: YearMonth) -> Result<StorePath<'_>, String> {
        StorePath::new(PathParts {
            vendor: self.vendor,
            exchange: &self.exchange,
            segment: &self.segment,
            symbol: &self.symbol,
            contract: None,
            timeframe: self.rung,
            month,
            file: FileKind::Bars,
        })
        .map_err(|why| {
            format!(
                "{}/{}/{} {} {month} is not a store path: {why}",
                crate::stored::clipped(&self.exchange),
                crate::stored::clipped(&self.segment),
                crate::stored::clipped(&self.symbol),
                self.rung.as_str()
            )
        })
    }

    /// The header's symbol id, derived exactly as every writer derives it:
    /// the low 32 bits of the symbol's FNV-1a (`pull::ingest`'s `identify`).
    fn symbol_id(&self) -> u32 {
        let [a, b, c, d, ..] = brutex_core::universe::fnv1a(&self.symbol).to_le_bytes();
        u32::from_le_bytes([a, b, c, d])
    }
}

/// A feed word, as the store names vendors.
fn vendor(word: &str) -> Result<Vendor, String> {
    Vendor::ALL
        .into_iter()
        .find(|vendor| vendor.as_str() == word)
        .ok_or_else(|| {
            let known: Vec<&str> = Vendor::ALL.iter().map(|vendor| vendor.as_str()).collect();
            format!(
                "`{}` is not a feed this build knows: {}",
                crate::stored::clipped(word),
                known.join(", ")
            )
        })
}

/// A `YYYY-MM` month, digits and widths exact.
fn month(label: &str, word: &str) -> Result<(u16, u8), String> {
    let digits = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u16>().ok())
            .flatten()
    };
    let parsed = word
        .split_once('-')
        .and_then(|(y, m)| digits(y, 4).zip(digits(m, 2)));
    let Some((year, number)) = parsed else {
        return Err(format!(
            "{label} `{}` is not a YYYY-MM month",
            crate::stored::clipped(word)
        ));
    };
    let number = u8::try_from(number).unwrap_or(u8::MAX);
    YearMonth::new(year, number)
        .map(|_| (year, number))
        .map_err(|why| format!("{label} `{}`: {why}", crate::stored::clipped(word)))
}

/// The arguments, refused by name before anything is opened.
fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let (positional, dry_run) = match arguments {
        [head @ .., "--dry-run"] => (head, true),
        all => (all, false),
    };
    let [feed, instrument, rung, from, to] = positional else {
        return Err(format!(
            "tix-rebuild takes five arguments and an optional --dry-run; it was given {}",
            arguments.len()
        ));
    };
    let vendor = vendor(feed)?;
    let parts: Vec<&str> = instrument.split('/').collect();
    let [exchange, segment, symbol] = parts.as_slice() else {
        return Err(format!(
            "`{}` is not EXCHANGE/SEGMENT/SYMBOL, the store's three path segments of a \
             spot series (e.g. NSE/INDEX/NIFTY)",
            crate::stored::clipped(instrument)
        ));
    };
    if [exchange, segment, symbol]
        .iter()
        .any(|part| part.is_empty())
    {
        return Err(format!(
            "`{}` has an empty segment",
            crate::stored::clipped(instrument)
        ));
    }
    let rung = crate::stored::rung(rung)?;
    let (from, to) = (month("FROM", from)?, month("TO", to)?);
    let months = crate::stored::months_between(from, to)?
        .into_iter()
        .filter_map(|(year, number)| YearMonth::new(year, number).ok())
        .collect();
    let ask = Ask {
        vendor,
        exchange: (*exchange).to_owned(),
        segment: (*segment).to_owned(),
        symbol: (*symbol).to_owned(),
        rung,
        months,
        dry_run,
    };
    // EVERY PATH IS PROVEN BEFORE THE FIRST MONTH IS OPENED, so a name the
    // store cannot hold refuses the whole run rather than half of it.
    for &month in &ask.months {
        ask.path(month)?;
    }
    Ok(ask)
}

/// What happened to one month.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    /// Already indexed: nothing was written.
    Kept { n_valid: u64 },
    /// Written from the bars, for the reason the store named.
    Built { n_valid: u64, because: String },
    /// No index can describe the bars; lookups bisect, for this reason.
    Unindexable { n_valid: u64, why: String },
    /// The step said it left an index and a reader does not find one.
    NotIndexed { n_valid: u64, reader: String },
    /// The month is not in the store; nothing was created.
    NotHeld,
    /// The store refused the month, in its own words.
    Refused { why: String },
    /// `--dry-run`: a reader goes through the index.
    Indexed { n_valid: u64 },
    /// `--dry-run`: a reader bisects, for this reason.
    WouldBuild { n_valid: u64, why: String },
    /// Under the small-month threshold with no `.tix`: none is made, nothing
    /// was written, and a reader bisects by rule (D-4933).
    Small { n_valid: u64 },
}

impl Line {
    /// Whether this month ends the run indexed (or, dry, held and readable).
    const fn clean(&self) -> bool {
        matches!(
            self,
            Self::Kept { .. }
                | Self::Built { .. }
                | Self::Indexed { .. }
                | Self::WouldBuild { .. }
                | Self::Small { .. }
        )
    }
}

/// How a reader opened now looks a timestamp up in `path`.
fn reader_view(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
) -> Result<(u64, TimeLookup), StoreError> {
    let file = BarFile::open_existing(root, path, symbol_id)?;
    Ok((file.header().n_valid, file.time_lookup()))
}

/// One month, built or confirmed, then checked through the read door.
fn rebuild(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Line {
    let done = match BarFile::index_existing(root, path, symbol_id) {
        Ok(done) => done,
        Err(StoreError::Missing { .. }) => return Line::NotHeld,
        // The store's sentence says "another writer"; the exclusive lock is
        // also refused while a READER holds the month, which is the likelier
        // holder while the API serves it. Said as both.
        Err(StoreError::Locked { path }) => {
            return Line::Refused {
                why: format!(
                    "{} is held by another process (a writer, or a reader mid-request) and is \
                     not waited for",
                    path.display()
                ),
            };
        }
        Err(why) => {
            return Line::Refused {
                why: why.to_string(),
            };
        }
    };
    let n_valid = done.n_valid;
    let line = match done.outcome {
        Reindexed::Kept => Line::Kept { n_valid },
        Reindexed::Built(because) => Line::Built {
            n_valid,
            because: because.to_string(),
        },
        Reindexed::Unindexable(why) => {
            return Line::Unindexable {
                n_valid,
                why: why.to_string(),
            };
        }
        Reindexed::BelowThreshold => Line::Small { n_valid },
        // `Reindexed` is non-exhaustive: an outcome this build does not know
        // is not a month it can call indexed.
        other => {
            return Line::Refused {
                why: format!("the store reported an outcome this build does not know: {other:?}"),
            };
        }
    };
    match reader_view(root, path, symbol_id) {
        // A SMALL MONTH IS READ BACK TOO: a reader must give the rule as its
        // reason, so the receipt's `small` is what a reader sees.
        Ok((_, TimeLookup::Bisection(Why::BelowThreshold)))
            if matches!(line, Line::Small { .. }) =>
        {
            line
        }
        Ok((_, TimeLookup::Indexed)) if !matches!(line, Line::Small { .. }) => line,
        Ok((_, reader)) => Line::NotIndexed {
            n_valid,
            reader: format!("{reader:?}"),
        },
        Err(why) => Line::Refused {
            why: format!("the index step finished and the read-back was refused: {why}"),
        },
    }
}

/// One month as a reader sees it, writing nothing.
fn inspect(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Line {
    match reader_view(root, path, symbol_id) {
        Ok((n_valid, TimeLookup::Indexed)) => Line::Indexed { n_valid },
        Ok((n_valid, TimeLookup::Bisection(Why::BelowThreshold))) => Line::Small { n_valid },
        Ok((n_valid, TimeLookup::Bisection(why))) => Line::WouldBuild {
            n_valid,
            why: why.to_string(),
        },
        Ok((n_valid, other)) => Line::WouldBuild {
            n_valid,
            why: format!("{other:?}"),
        },
        Err(StoreError::Missing { .. }) => Line::NotHeld,
        Err(why) => Line::Refused {
            why: why.to_string(),
        },
    }
}

/// The receipt line for one month.
fn render_line(out: &mut String, month: YearMonth, line: &Line) {
    let _ = match line {
        Line::Kept { n_valid } => writeln!(
            out,
            "  {month}  kept          {n_valid:>9} bars  already indexed; nothing written"
        ),
        Line::Built { n_valid, because } => writeln!(
            out,
            "  {month}  built         {n_valid:>9} bars  because {because}; a reader now goes through the index"
        ),
        Line::Unindexable { n_valid, why } => writeln!(
            out,
            "  {month}  UNINDEXABLE   {n_valid:>9} bars  no .tix is kept and lookups bisect: {why}"
        ),
        Line::NotIndexed { n_valid, reader } => writeln!(
            out,
            "  {month}  NOT INDEXED   {n_valid:>9} bars  the step finished and a reader still answers {reader}"
        ),
        Line::NotHeld => writeln!(
            out,
            "  {month}  NOT HELD      no such month; nothing was created"
        ),
        Line::Refused { why } => writeln!(
            out,
            "  {month}  REFUSED       {why}; nothing was written to it"
        ),
        Line::Indexed { n_valid } => writeln!(out, "  {month}  indexed       {n_valid:>9} bars"),
        Line::WouldBuild { n_valid, why } => writeln!(
            out,
            "  {month}  would build   {n_valid:>9} bars  a reader bisects: {why}"
        ),
        Line::Small { n_valid } => writeln!(
            out,
            "  {month}  small         {n_valid:>9} bars  under {INDEX_MIN_ROWS} bars: no .tix by the small-month rule (D-4933); a reader bisects in at most ten reads; nothing written"
        ),
    };
}

/// Runs the verb against the store root `store` names, so every arm is driven
/// by a test without the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let ask = match ask(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let root = match store() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(
                out,
                "TIX REBUILD REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    let symbol_id = ask.symbol_id();
    let (first, last) = (
        ask.months
            .first()
            .map(ToString::to_string)
            .unwrap_or_default(),
        ask.months
            .last()
            .map(ToString::to_string)
            .unwrap_or_default(),
    );
    let _ = writeln!(
        out,
        "TIX REBUILD{} — {} {}/{}/{} {}, {first}..{last}: {} month(s) under {}",
        if ask.dry_run {
            " (dry run: nothing is written)"
        } else {
            ""
        },
        ask.vendor.as_str(),
        ask.exchange,
        ask.segment,
        ask.symbol,
        ask.rung.as_str(),
        ask.months.len(),
        root.display()
    );
    let mut lines = Vec::with_capacity(ask.months.len());
    for &month in &ask.months {
        // Proven in `ask`, so this arm is the refusal it would have been.
        let line = match ask.path(month) {
            Ok(path) if ask.dry_run => inspect(&root, path, symbol_id),
            Ok(path) => rebuild(&root, path, symbol_id),
            Err(why) => Line::Refused { why },
        };
        render_line(out, month, &line);
        lines.push(line);
    }
    let count = |want: fn(&Line) -> bool| lines.iter().filter(|line| want(line)).count();
    let clean = lines.iter().all(Line::clean);
    if ask.dry_run {
        let _ = writeln!(
            out,
            "Indexed {}, would build {}, small {}, not held {}, refused {}. Nothing was written.",
            count(|line| matches!(line, Line::Indexed { .. })),
            count(|line| matches!(line, Line::WouldBuild { .. })),
            count(|line| matches!(line, Line::Small { .. })),
            count(|line| matches!(line, Line::NotHeld)),
            count(|line| matches!(line, Line::Refused { .. })),
        );
    } else {
        let _ = writeln!(
            out,
            "Built {}, kept {}, small {}, unindexable {}, not indexed {}, not held {}, refused {}: {}",
            count(|line| matches!(line, Line::Built { .. })),
            count(|line| matches!(line, Line::Kept { .. })),
            count(|line| matches!(line, Line::Small { .. })),
            count(|line| matches!(line, Line::Unindexable { .. })),
            count(|line| matches!(line, Line::NotIndexed { .. })),
            count(|line| matches!(line, Line::NotHeld)),
            count(|line| matches!(line, Line::Refused { .. })),
            if clean && count(|line| matches!(line, Line::Small { .. })) > 0 {
                "every month in the span is indexed, or holds too few bars to need an index (D-4933)"
            } else if clean {
                "every month in the span is indexed"
            } else {
                "NOT every month in the span is indexed; rerun once the cause is gone (a rerun keeps what is built)"
            }
        );
    }
    if clean { OK } else { FAILED }
}

/// `cli tix-rebuild`, against the CLI's store root (`BRUTEX_STORE`).
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // AN UNSTAMPED BUILD BUILDS NO INDEX (DB-B71, WP-81); `--dry-run` writes
    // nothing and is not refused.
    let store = crate::store_write::root_for("tix-rebuild", arguments);
    command_with(arguments, &store, out)
}

#[cfg(test)]
#[path = "tix_rebuilder_tests.rs"]
mod tests;

/// THE CLOUD MUTATION PASS (`reports/cloud-mut-cli-20261016.md`): each test
/// pins the behaviour a mutant of this file broke while every other test
/// stayed green, or runs a line no test reached. Invented stores only.
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test that cannot panic cannot fail"
)]
mod cloud_mutant_tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use store::format::{Bar, OI_NULL};

    use super::*;

    /// The dispatched verb refuses a misuse with its usage, before the store
    /// root is asked for.
    #[test]
    fn the_dispatched_verb_refuses_a_misuse_with_its_usage() {
        let mut out = String::new();
        assert_eq!(command(&["gdfl"], &mut out), MISUSED);
        assert!(out.starts_with("refused: ") && out.contains(USAGE), "{out}");
    }

    /// A month filed under another symbol's header id is refused by the
    /// store, dry or not, and no index is written for it.
    #[test]
    fn a_month_under_another_symbols_id_is_refused_dry_or_not() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-tix-cloud-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let month = YearMonth::new(2019, 1).unwrap();
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::SECOND_1,
            month,
            file: FileKind::Bars,
        })
        .unwrap();
        let other =
            u32::try_from(brutex_core::universe::fnv1a("BANKNIFTY") & u64::from(u32::MAX)).unwrap();
        let open = month.ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
        let bars: Vec<Bar> = (0..1_100_i64)
            .map(|k| Bar {
                ts_micros: open + k * 1_000_000,
                open: 424_200,
                high: 424_300,
                low: 424_100,
                close: 424_250,
                volume: 0,
                open_interest: OI_NULL,
            })
            .collect();
        let mut writer = BarFile::open_or_create(&root, path, other).unwrap();
        writer.append(&bars).unwrap();
        drop(writer);
        let tix = path.with_file(FileKind::TimeIndex).to_path_buf(&root);
        fs::remove_file(&tix).unwrap();
        for dry in [true, false] {
            let mut arguments = vec!["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-01", "2019-01"];
            if dry {
                arguments.push("--dry-run");
            }
            let mut out = String::new();
            let at = root.clone();
            let code = command_with(&arguments, &move || Ok(at.clone()), &mut out);
            assert_eq!(code, FAILED, "{out}");
            assert!(out.contains("  2019-01  REFUSED       "), "{out}");
            assert!(out.contains("refused 1"), "{out}");
        }
        assert!(!tix.exists(), "no index is written for a refused month");
        fs::remove_dir_all(&root).unwrap();
    }

    /// The receipt line for a month a reader does not find indexed after the
    /// step, which no store state in a test reaches, word for word.
    #[test]
    fn the_receipt_line_for_a_month_a_reader_does_not_find_indexed() {
        let mut out = String::new();
        render_line(
            &mut out,
            YearMonth::new(2019, 7).unwrap(),
            &Line::NotIndexed {
                n_valid: 1_100,
                reader: "Bisection(Stale)".to_owned(),
            },
        );
        assert_eq!(
            out,
            "  2019-07  NOT INDEXED        1100 bars  the step finished and a reader still \
             answers Bisection(Stale)\n"
        );
        assert!(
            !Line::NotIndexed {
                n_valid: 0,
                reader: String::new()
            }
            .clean()
        );
    }
}

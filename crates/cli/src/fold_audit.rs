//! Does each stored coarse rung equal the fold of the stored one-minute bars?
//!
//! # The hole this fills, and it was invisible by construction
//!
//! `2min`, `3min`, `5min`, `10min`, `15min`, `30min` and `60min` are **never
//! asked of a vendor**. `pull::ingest::derive_all` folds them from the
//! one-minute file at ingest time — `crates/pull/src/ingest.rs:2206` calls
//! `pull::fold::fold` and writes the result straight to the rung's own
//! directory. `pull::fold`'s own doc names the failure that follows:
//!
//! > *"Folding a month whose one-minute pull was dirty produces coarse bars
//! > built out of gaps, and nothing downstream can tell them from complete
//! > ones — a month that looks finished and is not, which is the whole failure
//! > this ladder exists to prevent."*
//!
//! That is exactly what happened, and **nothing in this workspace could see
//! it.** `cli verify` checks span monotonicity, determinism, suffix
//! independence and the ledger round trip; not one of its checks looks across
//! rungs. No test folds a stored coarse file against the stored minutes.
//!
//! MEASURED, zerodha NIFTY, the 12:40 five-minute bucket of 2023-06-14, where
//! the minute series is missing 12:41 through 12:47 so the bucket was folded
//! from ONE minute instead of five:
//!
//! | | high | low | close | range |
//! |---|---|---|---|---|
//! | stored `5min` | 1 874 730 | 1 874 500 | 1 874 530 | 2.30 pts |
//! | folded from a complete minute series | 1 874 885 | 1 873 820 | 1 873 820 | 10.65 pts |
//!
//! The high is understated, the low overstated, the close taken from an
//! entirely different minute, and the printed range is **4.6x too narrow**. The
//! record is not marked, not refused, and byte-indistinguishable from a correct
//! one. Anything reading a range, a breakout or an extreme on a coarse rung
//! reads a smaller number than the market printed.
//!
//! # Why a whole-bucket comparison and not a count
//!
//! A count cannot find this. A bucket disappears only when EVERY minute inside
//! it is absent, so at `5min` and coarser the record count stays exactly right
//! while the bars inside it are wrong — measured: 2023-06 is short 8 minutes and
//! its `5min`, `10min`, `15min`, `30min` and `60min` counts are all exact.
//! Every field of every record is compared here for that reason.
//!
//! # What a mismatch means, and what it does not
//!
//! A mismatch says the coarse file does not equal the fold of the minutes NOW
//! ON DISK. That is the defect when the minutes are complete. It is also what a
//! legitimately repaired minute series would produce against a stale coarse
//! file, so the report names the fields rather than only counting them.
//!
//! This audit does not say the minute series itself is complete — that is the
//! calendar's question and `CalendarReceiptV2` answers it. The two together are
//! the whole check: the calendar proves the minutes are all there, and this
//! proves the coarse rungs were folded from them.
//!
//! # Cost
//!
//! One `open_existing` and one `read_record` per bar of the minute file, plus
//! one fold and one file walk per coarse rung — so O(minutes) per rung and
//! O(8 x minutes) per month. It is a whole-store audit and is not on any sweep
//! path. **UNVERIFIED as a measured bound**; read off the source per
//! `CLAUDE.md` section 3 rule 6.

use brutex_core::instrument::InstrumentKey;
use brutex_core::vendor::Vendor;
use std::path::Path;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

/// Every rung folded from the minute file, in the order `pull` derives them.
///
/// `1day` is absent deliberately: `pull::ingest::derived_from` excludes it and
/// `docs/05-decisions.md` D-0077 records why — *"the day is served, never
/// derived"*. Auditing it here would report every daily bar as a mismatch
/// against a fold nobody performed.
pub const DERIVED_RUNGS: [Timeframe; 7] = [
    Timeframe::MINUTE_2,
    Timeframe::MINUTE_3,
    Timeframe::MINUTE_5,
    Timeframe::MINUTE_10,
    Timeframe::MINUTE_15,
    Timeframe::MINUTE_30,
    Timeframe::MINUTE_60,
];

/// One disagreement between a stored coarse bar and the fold of the minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disagreement {
    /// Index of the record in the coarse file.
    pub at: u64,
    /// The bar's opening timestamp, from whichever side has one.
    pub ts_micros: i64,
    /// Which field disagreed first, in record order.
    pub field: &'static str,
    /// What the stored coarse file holds.
    pub stored: i64,
    /// What folding the stored minutes produces.
    pub folded: i64,
}

/// One rung's verdict for one instrument-month.
#[derive(Clone, Debug)]
pub struct RungVerdict {
    /// The rung, as its canonical directory word.
    pub rung: &'static str,
    /// Records the coarse file holds.
    pub stored_bars: u64,
    /// Records folding the minutes produces.
    pub folded_bars: u64,
    /// Every disagreement found, capped by [`MAX_REPORTED`].
    pub disagreements: Vec<Disagreement>,
    /// Disagreements beyond the cap, counted but not named.
    pub elided: u64,
}

impl RungVerdict {
    /// Did this rung's file equal the fold of the minutes?
    #[must_use]
    pub fn agrees(&self) -> bool {
        self.stored_bars == self.folded_bars && self.disagreements.is_empty() && self.elided == 0
    }
}

/// Disagreements named individually before the rest are only counted.
///
/// A month whose minute file is wholly absent would otherwise name one
/// disagreement per bar, and a report nobody can read is a report nobody reads.
pub const MAX_REPORTED: usize = 8;

/// Compare one coarse rung against the fold of the supplied minutes.
///
/// Pure, so the comparison itself is testable without a store on disk.
#[must_use]
pub fn compare(rung: Timeframe, minutes: &[Bar], stored: &[Bar]) -> RungVerdict {
    let folded = store_bucket(rung).map_or_else(Vec::new, |bucket| {
        pull::fold::fold(minutes, bucket).unwrap_or_default()
    });
    let mut disagreements = Vec::new();
    let mut elided = 0_u64;
    let push = |d: Disagreement, found: &mut Vec<Disagreement>, elided: &mut u64| {
        if found.len() < MAX_REPORTED {
            found.push(d);
        } else {
            *elided = elided.saturating_add(1);
        }
    };

    for index in 0..stored.len().max(folded.len()) {
        let at = u64::try_from(index).unwrap_or(u64::MAX);
        match (stored.get(index), folded.get(index)) {
            (Some(s), Some(f)) => {
                // EVERY FIELD, IN RECORD ORDER, AND THE FIRST ONE ONLY.
                //
                // Naming all seven for one bar would bury the next bar's
                // disagreement under six restatements of the same fault: a
                // bucket folded over a hole typically disagrees on high, low,
                // close AND volume at once.
                for (field, sv, fv) in [
                    ("ts_micros", s.ts_micros, f.ts_micros),
                    ("open", s.open, f.open),
                    ("high", s.high, f.high),
                    ("low", s.low, f.low),
                    ("close", s.close, f.close),
                    ("volume", s.volume, f.volume),
                    ("open_interest", s.open_interest, f.open_interest),
                ] {
                    if sv != fv {
                        push(
                            Disagreement {
                                at,
                                ts_micros: s.ts_micros,
                                field,
                                stored: sv,
                                folded: fv,
                            },
                            &mut disagreements,
                            &mut elided,
                        );
                        break;
                    }
                }
            }
            (Some(s), None) => push(
                Disagreement {
                    at,
                    ts_micros: s.ts_micros,
                    field: "present in the store, absent from the fold",
                    stored: s.ts_micros,
                    folded: 0,
                },
                &mut disagreements,
                &mut elided,
            ),
            (None, Some(f)) => push(
                Disagreement {
                    at,
                    ts_micros: f.ts_micros,
                    field: "absent from the store, present in the fold",
                    stored: 0,
                    folded: f.ts_micros,
                },
                &mut disagreements,
                &mut elided,
            ),
            (None, None) => break,
        }
    }

    RungVerdict {
        rung: rung.as_str(),
        stored_bars: u64::try_from(stored.len()).unwrap_or(u64::MAX),
        folded_bars: u64::try_from(folded.len()).unwrap_or(u64::MAX),
        disagreements,
        elided,
    }
}

/// The fold bucket for a rung, or `None` for a width of zero seconds.
fn store_bucket(rung: Timeframe) -> Option<pull::fold::Bucket> {
    pull::fold::Bucket::of_secs(rung.secs())
}

/// Read every committed record of one instrument-month, or say why not.
///
/// # Errors
///
/// The store's own refusal, in its own words. An absent file is an error here
/// and not an empty vector: this audit exists to compare two files, and calling
/// a missing one "empty" would report every bar of the other as a disagreement.
pub fn read_month(
    root: &Path,
    vendor: Vendor,
    key: &InstrumentKey,
    rung: Timeframe,
    ym: YearMonth,
) -> Result<Vec<Bar>, String> {
    let path = StorePath::for_key(vendor, key, rung, ym, FileKind::Bars)
        .map_err(|why| format!("no store path for {} {}: {why}", rung.as_str(), key.underlying))?;
    // THE STORE'S OWN SYMBOL ID, hashed from the NORMALISED underlying exactly
    // as `stored::load_classified_with_ceiling` does. Hashing the caller's raw
    // string instead opens the right file and is then refused by it, naming two
    // numbers that mean nothing to a reader.
    let symbol_id = brutex_core::universe::fnv1a(key.underlying.as_str()) as u32;
    let file = BarFile::open_existing(root, path, symbol_id)
        .map_err(|why| format!("{} {} could not be read: {why}", rung.as_str(), ym))?;
    let n = file.records();
    let mut bars = Vec::new();
    bars.try_reserve(usize::try_from(n).unwrap_or(0))
        .map_err(|why| format!("cannot reserve {n} records for {}: {why}", rung.as_str()))?;
    for index in 0..n {
        bars.push(
            file.read_record(index)
                .map_err(|why| format!("record {index} of {n} could not be read: {why}"))?,
        );
    }
    Ok(bars)
}

/// Audit every derived rung of one instrument-month against its minute file.
///
/// # Errors
///
/// Only the minute file's own refusal. A coarse rung that cannot be read is
/// reported as a verdict carrying that reason rather than failing the month:
/// an absent `30min` file is a finding about the store, not a reason to stop
/// auditing `60min`.
pub fn audit_month(
    root: &Path,
    vendor: Vendor,
    key: &InstrumentKey,
    ym: YearMonth,
) -> Result<Vec<Result<RungVerdict, String>>, String> {
    let minutes = read_month(root, vendor, key, Timeframe::MINUTE_1, ym)
        .map_err(|why| format!("the one-minute file is the authority here and {why}"))?;
    Ok(DERIVED_RUNGS
        .iter()
        .map(|rung| {
            read_month(root, vendor, key, *rung, ym).map(|stored| compare(*rung, &minutes, &stored))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-minute bar on the NSE open grid, `minute` minutes past 09:15 IST.
    fn minute_bar(minute: i64, open: i64, high: i64, low: i64, close: i64) -> Bar {
        // 2024-06-03 09:15 IST as micros, plus the offset. The exact day does
        // not matter to the fold; the grid position does.
        const BASE: i64 = 1_717_384_500_000_000;
        Bar {
            ts_micros: BASE + minute * 60_000_000,
            open,
            high,
            low,
            close,
            volume: 10,
            open_interest: i64::MIN,
        }
    }

    /// A complete minute series folds to a coarse file that agrees exactly.
    #[test]
    fn a_complete_minute_series_agrees_with_its_own_fold() {
        let minutes: Vec<Bar> = (0..30)
            .map(|m| minute_bar(m, 100 + m, 110 + m, 90 + m, 105 + m))
            .collect();
        for rung in DERIVED_RUNGS {
            let bucket = store_bucket(rung).expect("every derived rung has a bucket");
            let folded = pull::fold::fold(&minutes, bucket).expect("a clean series folds");
            let verdict = compare(rung, &minutes, &folded);
            assert!(
                verdict.agrees(),
                "{} disagreed with its own fold: {:?}",
                rung.as_str(),
                verdict.disagreements
            );
            assert_eq!(verdict.stored_bars, verdict.folded_bars);
        }
    }

    /// The measured defect: a bucket folded over a hole understates its range,
    /// keeps the right record COUNT, and is caught only by comparing fields.
    ///
    /// # Why this test exists rather than a comment
    ///
    /// This is the 2023-06-14 12:40 case in miniature. The store holds a
    /// five-minute bar folded from ONE minute because four are missing; the
    /// complete series folds to a bar with a wider high, a lower low and a
    /// different close. A count check passes both — which is exactly why the
    /// defect survived in the real store until a field comparison found it.
    #[test]
    fn a_bucket_folded_over_a_hole_keeps_its_count_and_loses_its_range() {
        let complete: Vec<Bar> = (0..5)
            .map(|m| minute_bar(m, 1000, 1000 + m * 10, 1000 - m * 10, 1000 + m))
            .collect();
        // The store's version: only the first minute of the bucket survived.
        let holed = vec![complete[0]];

        let bucket = store_bucket(Timeframe::MINUTE_5).expect("5min has a bucket");
        let stored = pull::fold::fold(&holed, bucket).expect("one minute still folds");
        let truth = pull::fold::fold(&complete, bucket).expect("five minutes fold");

        assert_eq!(stored.len(), 1, "the bucket still exists");
        assert_eq!(truth.len(), 1, "and so does the correct one");

        let verdict = compare(Timeframe::MINUTE_5, &complete, &stored);
        assert_eq!(
            verdict.stored_bars, verdict.folded_bars,
            "THE COUNT AGREES, which is the whole reason a count check cannot \
             find this"
        );
        assert!(
            !verdict.agrees(),
            "the field comparison must refuse where the count could not"
        );
        let first = verdict
            .disagreements
            .first()
            .expect("a disagreement is named");
        assert_eq!(first.field, "high", "the high is the first field to differ");
        assert!(
            first.stored < first.folded,
            "a hole UNDERSTATES the high: stored {} against {}",
            first.stored,
            first.folded
        );
    }

    /// A rung whose file holds more or fewer records than the fold is named as
    /// that, rather than reported as a field disagreement on a bar that has no
    /// counterpart.
    #[test]
    fn a_missing_or_extra_record_is_named_as_absence_not_as_a_field() {
        let minutes: Vec<Bar> = (0..10)
            .map(|m| minute_bar(m, 100, 110, 90, 105))
            .collect();
        let bucket = store_bucket(Timeframe::MINUTE_5).expect("5min has a bucket");
        let full = pull::fold::fold(&minutes, bucket).expect("ten minutes fold");
        assert_eq!(full.len(), 2, "ten minutes make two five-minute buckets");

        let short = vec![full[0]];
        let verdict = compare(Timeframe::MINUTE_5, &minutes, &short);
        assert!(!verdict.agrees());
        assert_eq!(verdict.stored_bars, 1);
        assert_eq!(verdict.folded_bars, 2);
        assert_eq!(
            verdict
                .disagreements
                .first()
                .expect("one disagreement")
                .field,
            "absent from the store, present in the fold"
        );

        let mut long = full.clone();
        long.push(minute_bar(99, 1, 1, 1, 1));
        let other = compare(Timeframe::MINUTE_5, &minutes, &long);
        assert!(!other.agrees());
        assert_eq!(
            other.disagreements.first().expect("one disagreement").field,
            "present in the store, absent from the fold"
        );
    }

    /// Beyond the cap, disagreements are counted rather than named, and the
    /// verdict still refuses.
    #[test]
    fn disagreements_past_the_cap_are_counted_and_still_refuse() {
        let minutes: Vec<Bar> = (0..120)
            .map(|m| minute_bar(m, 100 + m, 110 + m, 90 + m, 105 + m))
            .collect();
        let bucket = store_bucket(Timeframe::MINUTE_2).expect("2min has a bucket");
        let truth = pull::fold::fold(&minutes, bucket).expect("the series folds");
        // Every stored bar carries a wrong high.
        let wrong: Vec<Bar> = truth
            .iter()
            .map(|bar| Bar {
                high: bar.high + 1,
                ..*bar
            })
            .collect();
        let verdict = compare(Timeframe::MINUTE_2, &minutes, &wrong);
        assert!(!verdict.agrees());
        assert_eq!(verdict.disagreements.len(), MAX_REPORTED);
        assert!(
            verdict.elided > 0,
            "the rest are counted rather than dropped"
        );
    }

    /// `1day` is not in the audited set, and that is deliberate.
    #[test]
    fn the_daily_rung_is_never_audited_because_it_is_never_folded() {
        assert!(
            !DERIVED_RUNGS.contains(&Timeframe::DAY_1),
            "the day is served, never derived -- D-0077"
        );
        assert!(
            !DERIVED_RUNGS.contains(&Timeframe::MINUTE_1),
            "the minute is the source, not a derived rung"
        );
        assert_eq!(DERIVED_RUNGS.len(), 7);
    }
}

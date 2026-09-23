//! Read-only preparation for the cash-stock discovery workflow.
//!
//! Resolves 2020-01-01 through yesterday in IST once, before inspecting the
//! store. A month file is NOT proof of complete daily/minute coverage. This
//! module deliberately reports inventory, never a readiness certificate or a
//! backtest. Existing month-addressed sweep commands are unchanged.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use brutex_core::instrument::Kind;
use brutex_core::vendor::Vendor;
use indicators::Candle;
use pull::session::{Day, IST_OFFSET_SECS, IstMoment, SECS_PER_DAY};
use store::catalog::{Held, Holdings};

/// Version of the exact-date research-window policy, not a store-file version.
pub const WINDOW_POLICY: u64 = 1;

/// Immutable requested signal/execution bounds. Reference warm-up is separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResearchWindow {
    from: Day,
    through: Day,
    end_exclusive: Day,
}

impl ResearchWindow {
    /// Resolve yesterday in IST from one supplied UTC epoch-second timestamp.
    ///
    /// The supplied clock makes midnight, leap-day and replay tests deterministic.
    /// No worker should independently resolve a new window after this call.
    ///
    /// # Errors
    /// Refuses an invalid clock or a window ending before 2020-01-01.
    pub fn at(now_secs: i64) -> Result<Self, String> {
        let today = IstMoment::from_epoch_secs(now_secs)
            .map_err(|why| format!("cannot resolve the IST research date: {why}"))?
            .day();
        let from = Day::new(2020, 1, 1).map_err(|why| why.to_string())?;
        if today <= from {
            return Err("yesterday in IST is before the required start 2020-01-01".into());
        }
        let through = Day::from_days(today.days_from_epoch() - 1).map_err(|why| why.to_string())?;
        Ok(Self {
            from,
            through,
            end_exclusive: today,
        })
    }

    /// Resolve the clock exactly once for a new research request.
    ///
    /// # Errors
    /// Refuses a pre-epoch/unrepresentable clock or an invalid research window.
    pub fn now() -> Result<Self, String> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|why| format!("research clock is before the Unix epoch: {why}"))?;
        let secs = i64::try_from(elapsed.as_secs())
            .map_err(|why| format!("research clock does not fit epoch seconds: {why}"))?;
        Self::at(secs)
    }

    /// Inclusive start of requested trading history, always 2020-01-01.
    #[must_use]
    pub const fn from(self) -> Day {
        self.from
    }

    /// Inclusive final calendar date, even when it is a non-trading day.
    #[must_use]
    pub const fn through(self) -> Day {
        self.through
    }

    /// UTC-microsecond interval [start, end), ending at today's IST midnight.
    #[must_use]
    pub fn bounds_micros(self) -> (i64, i64) {
        (
            midnight_micros(self.from),
            midnight_micros(self.end_exclusive),
        )
    }

    /// Whether a bar's opening timestamp falls in the frozen requested window.
    #[must_use]
    pub fn contains(self, ts_micros: i64) -> bool {
        let (start, end) = self.bounds_micros();
        ts_micros >= start && ts_micros < end
    }

    /// UNVERIFIED performance: no named cost test or measured latency bound is established here.
    /// Remove observations outside the requested signal/execution window.
    ///
    /// Returns the number removed, preserving order. O(bars) setup work, NOT an
    /// O(1) whole-slice claim. Apply before deriving levels or hashing trade data;
    /// any earlier warm-up references must remain explicitly separate.
    pub fn retain(self, bars: &mut Vec<Candle>) -> usize {
        let before = bars.len();
        bars.retain(|bar| self.contains(bar.ts_micros));
        before - bars.len()
    }

    /// Stable fields that a future research-run identity must include.
    ///
    /// Both requested dates remain material even if the last day is a holiday
    /// and two windows happen to contain identical bars. This is NOT itself a
    /// full run identity: feed/data/commit/strategy/exit policy are still needed.
    #[must_use]
    pub fn identity_words(self) -> [u64; 3] {
        [
            WINDOW_POLICY,
            u64::from(self.from.days_from_epoch()),
            u64::from(self.through.days_from_epoch()),
        ]
    }

    /// Number of month files intersecting the requested interval.
    #[must_use]
    pub fn months(self) -> usize {
        usize::from(self.through.year() - self.from.year()) * 12 + usize::from(self.through.month())
    }

    fn admits_month(self, held: &Held) -> bool {
        let month = (held.month.year(), held.month.month());
        month >= (self.from.year(), self.from.month())
            && month <= (self.through.year(), self.through.month())
    }
}

fn midnight_micros(day: Day) -> i64 {
    (i64::from(day.days_from_epoch()) * SECS_PER_DAY - IST_OFFSET_SECS) * 1_000_000
}

/// Produce a read-only inventory report; never pulls, sweeps or writes results.
///
/// # Errors
/// A store walk refusal. Missing data is listed, not treated as full coverage.
pub fn inspect(root: &Path, vendor: Vendor, window: ResearchWindow) -> Result<String, String> {
    let holdings = store::catalog::walk(root).map_err(|why| why.to_string())?;
    Ok(render(&holdings, vendor, window))
}

fn cash_symbols() -> Vec<&'static str> {
    brutex_core::universe::FNO_UNDERLYINGS
        .iter()
        .copied()
        // F&O membership includes index underlyings. Positively identify cash
        // equities through the existing total-market index before resolving a
        // sweep key; swept_index's fallback alone can construct a cash key for
        // an index outside the two swept spot indices.
        .filter(|symbol| brutex_core::universe::NTM_INDEX.contains(symbol))
        .filter(|symbol| {
            crate::stored::swept_index(symbol).is_ok_and(|key| key.kind == Kind::Equity)
        })
        .collect()
}

fn render(holdings: &Holdings, vendor: Vendor, window: ResearchWindow) -> String {
    let indexed: BTreeSet<_> = holdings
        .held
        .iter()
        .filter(|held| {
            held.vendor == vendor
                && held.exchange == "NSE"
                && held.segment == "CASH"
                && window.admits_month(held)
        })
        .map(|held| (held.symbol.as_str(), held.timeframe.as_str(), held.month))
        .collect();
    let symbols = cash_symbols();
    let mut counts = BTreeMap::new();
    for (symbol, rung, _) in indexed {
        *counts.entry((symbol, rung)).or_insert(0_usize) += 1;
    }
    let mut out = format!(
        "RESEARCH INVENTORY -- NOT A SWEEP OR A COMPLETENESS CERTIFICATE\n\
         requested: {} through {} inclusive, IST\n\
         cutoff: today and later are excluded; frozen for this request\n\
         window policy: {:?}\n\
         feed: {vendor:?}; cash-stock members in the compiled universe: {}\n\
         expected month files per series: {} (not a claim each stock existed throughout)\n\
         values below count file paths, not verified complete sessions\n\n\
         symbol | 60min | 30min | 15min | 10min | 5min | 3min | 2min | 1min | 1day\n",
        window.from(),
        window.through(),
        window.identity_words(),
        symbols.len(),
        window.months()
    );
    for symbol in symbols {
        let _ = write!(out, "{symbol}");
        for rung in [
            "60min", "30min", "15min", "10min", "5min", "3min", "2min", "1min", "1day",
        ] {
            let found = counts.get(&(symbol, rung)).copied().unwrap_or(0);
            let _ = write!(out, " | {found}/{}", window.months());
        }
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "\nNOT VERIFIED: exact-day and minute coverage, OHLCV validity, VWAP volume, \
         derived-rung equality, listing/corporate-action history and point-in-time F&O membership.\n\
         A current member list is NOT the historical F&O universe from 2020.\n\
         A file for the final month does NOT prove data reaches {}.\n\
         Discovery policy requested: no charges; no statistical validation. \
         Execution correctness checks remain required.\n\
         No stock search was started; no winner or runtime guarantee is made.\n\
         Catalog: {} paths seen; {} spot files; {} contract files excluded.",
        window.through(),
        holdings.census.seen,
        holdings.census.spot,
        holdings.census.with_contract
    );
    out
}

/// CLI boundary for `research-plan VENDOR` with no operator-typed end month.
pub(crate) fn command(vendor: &str, out: &mut String) -> u8 {
    let result = (|| {
        let feed = crate::parse_vendor(vendor)?;
        let window = ResearchWindow::now()?;
        let root = crate::preflight_store_root()?;
        inspect(&root, feed, window)
    })();
    match result {
        Ok(report) => {
            out.push_str(&report);
            crate::OK
        }
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            crate::MISUSED
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn date(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).unwrap()
    }
    fn clock(y: u16, m: u8, d: u8) -> i64 {
        midnight_micros(date(y, m, d)) / 1_000_000
    }

    #[test]
    fn yesterday_uses_ist_midnight_not_the_utc_date() {
        let boundary = clock(2026, 9, 5);
        assert_eq!(
            ResearchWindow::at(boundary - 1).unwrap().through(),
            date(2026, 9, 3)
        );
        let window = ResearchWindow::at(boundary).unwrap();
        assert_eq!(window.from(), date(2020, 1, 1));
        assert_eq!(window.through(), date(2026, 9, 4));
        assert_eq!(window, ResearchWindow::at(boundary + 86_399).unwrap());
        assert_eq!(window.months(), 81);
    }

    #[test]
    fn year_and_leap_month_rollovers_are_exact() {
        for (today, yesterday) in [
            ((2026, 1, 1), (2025, 12, 31)),
            ((2024, 3, 1), (2024, 2, 29)),
            ((2023, 3, 1), (2023, 2, 28)),
        ] {
            let w = ResearchWindow::at(clock(today.0, today.1, today.2)).unwrap();
            assert_eq!(w.through(), date(yesterday.0, yesterday.1, yesterday.2));
        }
    }

    #[test]
    fn clock_and_empty_history_fail_loudly() {
        for secs in [i64::MIN, i64::MAX, clock(2020, 1, 1)] {
            assert!(ResearchWindow::at(secs).is_err());
        }
        assert_eq!(ResearchWindow::at(clock(2020, 1, 2)).unwrap().months(), 1);
    }

    #[test]
    fn microsecond_boundaries_exclude_today_and_pre_2020_data() {
        let window = ResearchWindow::at(clock(2026, 9, 5)).unwrap();
        let (start, end) = window.bounds_micros();
        assert!(!window.contains(start - 1));
        assert!(window.contains(start));
        assert!(window.contains(end - 1));
        assert!(!window.contains(end));
        assert!(!window.contains(i64::MAX));
        assert!(!window.contains(i64::MIN));
    }

    #[test]
    fn clipping_preserves_order_and_is_idempotent() {
        let window = ResearchWindow::at(clock(2026, 9, 5)).unwrap();
        let (start, end) = window.bounds_micros();
        let mut bars: Vec<_> = [start - 1, start, start + 1, end - 1, end]
            .into_iter()
            .map(|ts_micros| Candle {
                ts_micros,
                ..Candle::default()
            })
            .collect();
        assert_eq!(window.retain(&mut bars), 2);
        assert_eq!(
            bars.iter().map(|bar| bar.ts_micros).collect::<Vec<_>>(),
            vec![start, start + 1, end - 1]
        );
        assert_eq!(window.retain(&mut bars), 0);
        assert_eq!(window.retain(&mut Vec::new()), 0);
    }

    #[test]
    fn a_new_day_changes_identity_words_even_without_new_bars() {
        let original = ResearchWindow::at(clock(2026, 9, 5)).unwrap();
        let next = ResearchWindow::at(clock(2026, 9, 6)).unwrap();
        assert_ne!(original.identity_words(), next.identity_words());
        assert_eq!(original.through(), date(2026, 9, 4));
    }

    #[test]
    fn empty_inventory_is_not_reported_as_ready_or_as_a_search() {
        let window = ResearchWindow::at(clock(2026, 9, 5)).unwrap();
        let text = render(&Holdings::default(), Vendor::Zerodha, window);
        assert!(text.contains("2020-01-01 through 2026-09-04"));
        assert!(text.contains("| 3min | 2min | 1min | 1day"));
        assert!(text.contains("0/81"));
        assert!(text.contains("NOT A SWEEP OR A COMPLETENESS CERTIFICATE"));
        assert!(text.contains("No stock search was started"));
    }

    #[test]
    fn cash_inventory_never_relabels_an_index_as_a_stock() {
        let symbols = cash_symbols();
        assert!(symbols.contains(&"RELIANCE"));
        assert!(symbols.contains(&"HINDALCO"));
        for index in ["NIFTY", "BANKNIFTY", "FINNIFTY", "MIDCPNIFTY", "NIFTYNXT50"] {
            assert!(!symbols.contains(&index), "{index} is not a cash equity");
        }
        assert!(
            symbols
                .iter()
                .all(|symbol| brutex_core::universe::NTM_INDEX.contains(symbol)
                    && brutex_core::universe::FNO_INDEX.contains(symbol))
        );
    }

    #[test]
    fn inventory_deduplicates_paths_and_separates_feed_segment_and_month() {
        use store::path::{Timeframe, YearMonth};
        let window = ResearchWindow::at(clock(2026, 9, 5)).unwrap();
        let held = Held {
            vendor: Vendor::Zerodha,
            exchange: "NSE".into(),
            segment: "CASH".into(),
            symbol: "RELIANCE".into(),
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2026, 9).unwrap(),
        };
        let mut rows = vec![held.clone(), held.clone()];
        rows.push(Held {
            vendor: Vendor::Dhan,
            ..held.clone()
        });
        rows.push(Held {
            exchange: "BSE".into(),
            ..held.clone()
        });
        rows.push(Held {
            segment: "FNO".into(),
            ..held.clone()
        });
        rows.push(Held {
            month: YearMonth::new(2019, 12).unwrap(),
            ..held.clone()
        });
        rows.push(Held {
            month: YearMonth::new(2026, 10).unwrap(),
            ..held
        });
        let report = render(
            &Holdings {
                held: rows,
                ..Holdings::default()
            },
            Vendor::Zerodha,
            window,
        );
        let row = report
            .lines()
            .find(|line| line.starts_with("RELIANCE | "))
            .unwrap();
        assert!(row.ends_with("| 1/81 | 0/81"), "{row}");
        assert!(!row.contains("2/81"));
    }
}

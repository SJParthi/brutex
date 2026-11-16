//! The store's bars through the EXISTING reader: `store::file::BarFile::open_existing` (generation-aware: a month a
//! repair promoted is served from `bar-revisions-v1/<n>/`, checked against its pointer), with the quarantine probe,
//! exactly the door round 3's `store_io::open_month` uses. Read only.

use std::path::Path;

use brutex_core::vendor::Vendor;
use common_feat::fold::Candle;
use store::file::{BarFile, StoreError};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::Probe;

/// A timeframe by its store name (`1s`, `1min` ... `60min`, `1day`).
#[must_use]
pub fn tf_of(name: &str) -> Option<Timeframe> {
    Some(match name {
        "1s" => Timeframe::SECOND_1,
        "1min" => Timeframe::MINUTE_1,
        "2min" => Timeframe::MINUTE_2,
        "3min" => Timeframe::MINUTE_3,
        "5min" => Timeframe::MINUTE_5,
        "10min" => Timeframe::MINUTE_10,
        "15min" => Timeframe::MINUTE_15,
        "30min" => Timeframe::MINUTE_30,
        "60min" => Timeframe::MINUTE_60,
        "1day" => Timeframe::DAY_1,
        _ => return None,
    })
}

/// The symbol id the store's headers carry (round 3's `sym_id`: FNV-1a, low 32 bits).
#[must_use]
pub fn sym_id(series: &str) -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(series) & 0xFFFF_FFFF).unwrap_or(0)
}

/// One month's outcome.
pub enum Month {
    /// No such month file.
    Missing,
    /// Refused (quarantined, malformed ...), with the reason.
    Refused(String),
    /// The month's bars, oldest first.
    Bars(Vec<Candle>),
}

/// Reads one stored month of `vendor / NSE / segment / symbol / tf` read-only. A bar's `end` is `start + tf`; the host
/// clamps it to the segment end.
#[must_use]
pub fn read_month(root: &Path, vendor: Vendor, segment: &str, symbol: &str, tf: Timeframe, y: u16, m: u8) -> Month {
    let month = match YearMonth::new(y, m) {
        Ok(v) => v,
        Err(e) => return Month::Refused(e.to_string()),
    };
    let path = match StorePath::new(PathParts {
        vendor,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: tf,
        month,
        file: FileKind::Bars,
    }) {
        Ok(p) => p,
        Err(e) => return Month::Refused(e.to_string()),
    };
    match store::quarantine::probe(root, path) {
        Ok(Probe::Clear) => {}
        Ok(Probe::Quarantined(_)) => return Month::Refused(format!("{symbol} {y}-{m:02} is quarantined")),
        Err(e) => return Month::Refused(format!("{symbol} {y}-{m:02} quarantine unknown: {e}")),
    }
    let f = match BarFile::open_existing(root, path, sym_id(symbol)) {
        Ok(f) => f,
        Err(StoreError::Missing { .. }) => return Month::Missing,
        Err(e) => return Month::Refused(format!("{symbol} {y}-{m:02}: {e}")),
    };
    let n = f.records();
    let secs = i64::from(tf.secs());
    let mut v = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    for i in 0..n {
        match f.read_record(i) {
            Ok(b) => {
                let start = b.ts_micros.div_euclid(1_000_000);
                v.push(Candle {
                    start,
                    end: start + secs,
                    open: b.open,
                    high: b.high,
                    low: b.low,
                    close: b.close,
                    volume: u64::try_from(b.volume).unwrap_or(0),
                });
            }
            Err(e) => return Month::Refused(format!("{symbol} {y}-{m:02} record {i} of {n}: {e}")),
        }
    }
    Month::Bars(v)
}

/// Reads every month from `from` to `to` inclusive (`(year, month)`), concatenated oldest first, with one note per
/// missing or refused month.
#[must_use]
pub fn read_range(
    root: &Path,
    vendor: Vendor,
    segment: &str,
    symbol: &str,
    tf: Timeframe,
    from: (u16, u8),
    to: (u16, u8),
) -> (Vec<Candle>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    let (mut y, mut m) = from;
    while (y, m) <= to {
        match read_month(root, vendor, segment, symbol, tf, y, m) {
            Month::Bars(mut v) => out.append(&mut v),
            Month::Missing => notes.push(format!("{symbol} {} {y}-{m:02}: missing", tf.as_str())),
            Month::Refused(why) => notes.push(format!("{symbol} {} {y}-{m:02}: refused: {why}", tf.as_str())),
        }
        if m == 12 {
            y += 1;
            m = 1;
        } else {
            m += 1;
        }
    }
    (out, notes)
}

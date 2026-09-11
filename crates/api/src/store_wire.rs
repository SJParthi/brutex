//! Census transfer formats cached for one immutable manifest snapshot.
//! Cold encoding and retained bytes grow with the census; an unchanged poll
//! reuses the exact body and validator. This is not a total O(1) inventory.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use axum::body::Bytes;
use brutex_core::vendor::Vendor;
use serde::Serialize;

use super::{census, held_row, month_before, month_change};

/// An explicit version keeps old array readers compatible.
pub(super) const ENCODING: &str = "census-tuples-v1";

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) enum Format {
    Expanded,
    Compact,
}

/// Byte ownership can be shared with a response without cloning the census.
#[derive(Clone, Debug)]
pub(super) struct Encoded {
    pub(super) body: Bytes,
    pub(super) etag: String,
}

#[derive(Debug)]
struct Snapshot {
    source: Weak<Vec<census::VendorCensus>>,
    bodies: HashMap<(Vendor, Format), Encoded>,
}

/// At most two formats per fixed vendor, for one immutable source generation.
/// The mutex coalesces cold encoding; callers run it on a blocking worker.
#[derive(Default, Debug)]
pub(super) struct Cache(Mutex<Option<Snapshot>>);

impl Cache {
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.0.lock().is_ok_and(|held| held.is_none())
    }

    pub(super) fn get(
        &self,
        source: &Arc<Vec<census::VendorCensus>>,
        vendor: Vendor,
        format: Format,
        build: impl FnOnce() -> Result<String, String>,
    ) -> Result<Encoded, String> {
        let mut held = self.0.lock().map_err(|_| "census wire cache is poisoned")?;
        let same = held
            .as_ref()
            .is_some_and(|current| current.source.ptr_eq(&Arc::downgrade(source)));
        if !same {
            *held = Some(Snapshot {
                source: Arc::downgrade(source),
                bodies: HashMap::new(),
            });
        }
        let current = held
            .as_mut()
            .ok_or("census wire snapshot was unavailable")?;
        if let Some(encoded) = current.bodies.get(&(vendor, format)) {
            return Ok(encoded.clone());
        }
        let body = build()?;
        let encoded = Encoded {
            etag: super::census_etag(&body),
            body: Bytes::from(body),
        };
        current.bodies.insert((vendor, format), encoded.clone());
        Ok(encoded)
    }
}

/// The tuple positions are versioned on the wire, never inferred by a reader.
type Row = (
    usize,
    usize,
    usize,
    u64,
    i64,
    i64,
    Option<i64>,
    Option<usize>,
    Option<i64>,
    Option<usize>,
);

#[derive(Serialize)]
struct Compact {
    schema: u8,
    encoding: &'static str,
    feed: &'static str,
    instruments: Vec<String>,
    months: Vec<String>,
    timeframes: Vec<String>,
    reasons: Vec<String>,
    rows: Vec<Row>,
}

#[derive(Default)]
struct Dictionary {
    values: Vec<String>,
    index: HashMap<String, usize>,
}

impl Dictionary {
    fn intern(&mut self, value: String) -> usize {
        if let Some(index) = self.index.get(&value) {
            return *index;
        }
        let index = self.values.len();
        self.values.push(value.clone());
        self.index.insert(value, index);
        index
    }
}

pub(super) fn compact(
    censuses: &[census::VendorCensus],
    entries: &[(census::Series, store::path::YearMonth)],
    feed: Vendor,
) -> Result<String, String> {
    let mut instruments = Dictionary::default();
    let mut months = Dictionary::default();
    let mut timeframes = Dictionary::default();
    let mut reasons = Dictionary::default();
    let mut rows = Vec::new();
    if let Some(census) = censuses.iter().find(|c| c.vendor == feed) {
        for (series, month) in entries {
            let Some(row) = held_row(census, &series.at(*month)) else {
                continue;
            };
            let mut change = |value: Result<i64, super::Unknown>| match value {
                Ok(bps) => (Some(bps), None),
                Err(why) => (None, Some(reasons.intern(why.code().to_owned()))),
            };
            let current = change(month_change(series.segment, Some(row.closes)));
            let before = month_before(*month).and_then(|at| held_row(census, &series.at(at)));
            let previous = change(month_change(series.segment, before.map(|held| held.closes)));
            rows.push((
                instruments.intern(series.to_string()),
                months.intern(month.to_string()),
                timeframes.intern(series.timeframe.as_str().to_owned()),
                row.entry.rows,
                row.entry.first_ts_micros,
                row.entry.last_ts_micros,
                current.0,
                current.1,
                previous.0,
                previous.1,
            ));
        }
    }
    serde_json::to_string(&Compact {
        schema: 1,
        encoding: ENCODING,
        feed: feed.as_str(),
        instruments: instruments.values,
        months: months.values,
        timeframes: timeframes.values,
        reasons: reasons.values,
        rows,
    })
    .map_err(|why| format!("census encoding failed: {why}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn unchanged_sources_share_bytes_and_never_rebuild_or_rehash() {
        let source = Arc::new(Vec::new());
        let cache = Cache::default();
        let first = cache
            .get(&source, Vendor::Zerodha, Format::Expanded, || {
                Ok("[]".into())
            })
            .unwrap();
        let second = cache
            .get(&source, Vendor::Zerodha, Format::Expanded, || {
                panic!("warm read rebuilt")
            })
            .unwrap();
        assert_eq!(first.body.as_ptr(), second.body.as_ptr());
        assert_eq!(first.etag, second.etag);
        let changed = cache
            .get(
                &Arc::new(Vec::new()),
                Vendor::Zerodha,
                Format::Expanded,
                || Ok("[1]".into()),
            )
            .unwrap();
        assert_ne!(first.etag, changed.etag);
        assert_eq!(changed.body.as_ref(), b"[1]");
    }

    #[test]
    fn vendor_and_format_do_not_share_validators_or_bodies() {
        let source = Arc::new(Vec::new());
        let cache = Cache::default();
        for (vendor, format, text) in [
            (Vendor::Zerodha, Format::Expanded, "[]"),
            (Vendor::Groww, Format::Expanded, "[2]"),
            (Vendor::Zerodha, Format::Compact, "[3]"),
        ] {
            let value = cache
                .get(&source, vendor, format, || Ok(text.into()))
                .unwrap();
            assert_eq!(value.body.as_ref(), text.as_bytes());
            assert_eq!(value.etag, super::super::census_etag(text));
        }
    }

    #[test]
    fn concurrent_cold_requests_encode_once() {
        let source = Arc::new(Vec::new());
        let cache = Cache::default();
        let count = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            let readers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        cache
                            .get(&source, Vendor::Zerodha, Format::Compact, || {
                                count.fetch_add(1, Ordering::Relaxed);
                                Ok("[]".into())
                            })
                            .unwrap()
                    })
                })
                .collect();
            for reader in readers {
                assert_eq!(reader.join().unwrap().body.as_ref(), b"[]");
            }
        });
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn poisoned_cache_refuses_instead_of_serving_stale_bytes() {
        let cache = Cache::default();
        let _ = std::panic::catch_unwind(|| {
            let _guard = cache.0.lock().unwrap();
            panic!("interrupt the cache update");
        });
        assert!(
            cache
                .get(
                    &Arc::new(Vec::new()),
                    Vendor::Zerodha,
                    Format::Expanded,
                    || Ok("[]".into())
                )
                .is_err()
        );
    }

    #[test]
    fn failed_encoding_is_not_cached_as_an_empty_success() {
        let cache = Cache::default();
        let source = Arc::new(Vec::new());
        assert!(
            cache
                .get(&source, Vendor::Zerodha, Format::Compact, || Err(
                    "refused".into()
                ))
                .is_err()
        );
        assert_eq!(
            cache
                .get(
                    &source,
                    Vendor::Zerodha,
                    Format::Compact,
                    || Ok("[]".into())
                )
                .unwrap()
                .body
                .as_ref(),
            b"[]"
        );
    }

    #[test]
    fn empty_compact_census_retains_its_feed_and_version() {
        let value: serde_json::Value =
            serde_json::from_str(&compact(&[], &[], Vendor::Zerodha).unwrap()).unwrap();
        assert_eq!(value["schema"], 1);
        assert_eq!(value["encoding"], ENCODING);
        assert_eq!(value["feed"], "zerodha");
        for field in ["rows", "instruments", "months", "timeframes", "reasons"] {
            assert_eq!(value[field], serde_json::json!([]));
        }
    }
}

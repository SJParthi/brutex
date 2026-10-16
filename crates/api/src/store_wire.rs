//! Census transfer formats, kept per (vendor, format) for one kept census
//! read of that vendor, and the other answers derived once per such read.
//!
//! # What a request costs (D-3327 item 1, DB-G79)
//!
//! The encoded body and its validator are kept in a cell per (vendor,
//! format), keyed on the vendor's own census slot
//! (`census_cache::Slot`), which only that vendor's manifest moves. So the
//! 21 MB `/store.json?feed=gdfl` body survives every Zerodha commit, and an
//! unchanged poll is a lock, a pointer comparison and a reference count.
//!
//! A cold cell is encoded and hashed under ITS OWN lock and no other: the
//! map of cells is held only to find the cell, so a cold GDFL encode never
//! stalls a Zerodha reader (it did when one mutex guarded every body), and
//! concurrent cold readers of the same cell wait for one encode
//! (single-flight). The encode itself is O(rows of that vendor), once per
//! census change; it is what the full tuples body costs, which is why `/db`
//! is served facets (`facets.rs`) and the full body only on a cold load.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use axum::body::Bytes;
use brutex_core::vendor::Vendor;
use serde::Serialize;

use super::census_cache::Slot;
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

/// One (vendor, format)'s body, and the slot it was encoded from.
#[derive(Debug, Default)]
struct WireCell(Mutex<Option<(Weak<Slot>, Encoded)>>);

/// Every derived answer kept against one census read: the encoded bodies,
/// and (through [`Self::kept`]) whatever else a route derives once per read.
#[derive(Default, Debug)]
pub(super) struct Cache {
    bodies: Mutex<HashMap<(Vendor, Format), Arc<WireCell>>>,
    /// Answers other routes keep per (vendor, name), each against the slot it
    /// was derived from. See [`Self::kept`].
    kept: Mutex<HashMap<(Vendor, &'static str), Arc<KeptCell>>>,
    /// The import journal's verdict on the census, kept against the journal
    /// and log reads it came from (`store_route::behind`).
    pub(super) behind: super::store_route::BehindCache,
    /// `/census/facets.json`'s index per (vendor, segment) (`facets.rs`).
    pub(super) facets: super::facets::Cache,
}

/// One derived answer, the slot and the extra key it came from, typed at the
/// call site.
#[derive(Default)]
#[allow(clippy::type_complexity, reason = "one private cell, named by its doc")]
struct KeptCell(Mutex<Option<(Weak<Slot>, u64, Arc<dyn std::any::Any + Send + Sync>)>>);

impl std::fmt::Debug for KeptCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KeptCell")
    }
}

impl Cache {
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.bodies.lock().is_ok_and(|held| {
            held.values()
                .all(|cell| cell.0.lock().is_ok_and(|one| one.is_none()))
        })
    }

    /// The body of `vendor`'s census in `format`, encoded once per kept slot.
    ///
    /// # Errors
    ///
    /// A poisoned cell, which refuses rather than serving bytes a panicked
    /// encode may have left half-built, or `build`'s own refusal, which is
    /// not kept: the next request encodes again.
    pub(super) fn get(
        &self,
        source: &Arc<Slot>,
        vendor: Vendor,
        format: Format,
        build: impl FnOnce() -> Result<String, String>,
    ) -> Result<Encoded, String> {
        let cell = {
            let mut cells = self
                .bodies
                .lock()
                .map_err(|_| "census wire cache is poisoned")?;
            Arc::clone(cells.entry((vendor, format)).or_default())
        };
        // THIS CELL'S LOCK, NOT THE MAP'S: held across the encode so a second
        // cold reader of the same body waits for this one, and nobody else
        // waits at all.
        let mut held = cell.0.lock().map_err(|_| "census wire cache is poisoned")?;
        if let Some((from, encoded)) = held.as_ref()
            && from.ptr_eq(&Arc::downgrade(source))
        {
            return Ok(encoded.clone());
        }
        let body = build()?;
        let encoded = Encoded {
            etag: super::census_etag(&body),
            body: Bytes::from(body),
        };
        *held = Some((Arc::downgrade(source), encoded.clone()));
        Ok(encoded)
    }

    /// An answer `name` derives from `vendor`'s census and whatever `extra`
    /// numbers (a universe's parse generation), derived once per kept slot and
    /// `extra` by one reader while concurrent readers of the same cell wait,
    /// and shared after that. The same shape as [`Self::get`], for any type.
    ///
    /// # Errors
    ///
    /// A poisoned cell, or `build`'s refusal, which is not kept.
    pub(super) fn kept<T: Send + Sync + 'static>(
        &self,
        source: &Arc<Slot>,
        vendor: Vendor,
        name: &'static str,
        extra: u64,
        build: impl FnOnce() -> Result<T, String>,
    ) -> Result<Arc<T>, String> {
        let cell = {
            let mut cells = self
                .kept
                .lock()
                .map_err(|_| "derived census cache is poisoned")?;
            Arc::clone(cells.entry((vendor, name)).or_default())
        };
        let mut held = cell
            .0
            .lock()
            .map_err(|_| "derived census cache is poisoned")?;
        if let Some((from, kept_extra, value)) = held.as_ref()
            && from.ptr_eq(&Arc::downgrade(source))
            && *kept_extra == extra
            && let Ok(value) = Arc::clone(value).downcast::<T>()
        {
            return Ok(value);
        }
        let value = Arc::new(build()?);
        let shared: Arc<dyn std::any::Any + Send + Sync> = Arc::clone(&value) as _;
        *held = Some((Arc::downgrade(source), extra, shared));
        Ok(value)
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
    serde_json::to_string(&rows_of(censuses, entries, feed))
        .map_err(|why| format!("census encoding failed: {why}"))
}

/// What the census holds NOW, beside a delta's rows: the two counters a
/// reader checks its merged census against before it believes the merge.
#[derive(Serialize)]
struct HeldNow {
    /// Distinct keys held: the rows a whole body carries.
    months: u64,
    /// Bars held, summed over them.
    rows: u64,
}

/// A whole body's fields, and the three a delta adds.
#[derive(Serialize)]
struct Delta<'a> {
    #[serde(flatten)]
    body: Compact,
    /// The `since` this answers, exactly as it was asked: how a reader tells
    /// a delta from the whole body an API older than this one answers.
    since: &'a str,
    held: HeldNow,
}

/// `/store.json?...&since=`: the SAME wire as [`compact`], holding only
/// `entries` (the rows changed after `since`), with `since` echoed and the
/// census's present counters beside them (WP-56; D-3327 item 3; DB-060).
///
/// One wire and one decoder for a whole read and a refresh, so the two
/// cannot spell a row differently. The counters are what makes a merge
/// checkable: a reader that merged these rows into the census it held must
/// hold exactly `months` rows summing to `rows`, or it reads the census
/// whole. O(`entries`): two probes per row, as [`compact`] costs.
pub(super) fn compact_delta(
    censuses: &[census::VendorCensus],
    entries: &[(census::Series, store::path::YearMonth)],
    feed: Vendor,
    since: &str,
    (months, rows): (u64, u64),
) -> Result<String, String> {
    serde_json::to_string(&Delta {
        body: rows_of(censuses, entries, feed),
        since,
        held: HeldNow { months, rows },
    })
    .map_err(|why| format!("census encoding failed: {why}"))
}

/// The tuples of `entries`, in the order given, with their dictionaries.
fn rows_of(
    censuses: &[census::VendorCensus],
    entries: &[(census::Series, store::path::YearMonth)],
    feed: Vendor,
) -> Compact {
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
            let current = change(month_change(
                series.segment,
                series.symbol.as_str(),
                Some(row.closes),
            ));
            let before = month_before(*month).and_then(|at| held_row(census, &series.at(at)));
            let previous = change(month_change(
                series.segment,
                series.symbol.as_str(),
                before.map(|held| held.closes),
            ));
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
    Compact {
        schema: 1,
        encoding: ENCODING,
        feed: feed.as_str(),
        instruments: instruments.values,
        months: months.values,
        timeframes: timeframes.values,
        reasons: reasons.values,
        rows,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A slot over an absent census, standing for one kept read.
    fn slot() -> Arc<Slot> {
        Slot::detached(census::VendorCensus {
            vendor: Vendor::Zerodha,
            path: std::path::PathBuf::from("/nonexistent/store-wire/zerodha.man"),
            state: census::Census::Absent,
        })
    }

    #[test]
    fn unchanged_sources_share_bytes_and_never_rebuild_or_rehash() {
        let source = slot();
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
            .get(&slot(), Vendor::Zerodha, Format::Expanded, || {
                Ok("[1]".into())
            })
            .unwrap();
        assert_ne!(first.etag, changed.etag);
        assert_eq!(changed.body.as_ref(), b"[1]");
    }

    #[test]
    fn vendor_and_format_do_not_share_validators_or_bodies() {
        let source = slot();
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

    /// **ANOTHER VENDOR'S NEW READ LEAVES THIS VENDOR'S BODY KEPT** (DB-G79).
    /// The old cache held one snapshot for every vendor, so a Zerodha commit
    /// dropped GDFL's encoded body; each body is now kept against its own
    /// vendor's slot.
    #[test]
    fn another_vendors_new_read_leaves_this_vendors_body_kept() {
        let cache = Cache::default();
        let gdfl = slot();
        let first = cache
            .get(&gdfl, Vendor::Gdfl, Format::Compact, || {
                Ok("[\"gdfl\"]".into())
            })
            .unwrap();
        for _ in 0..3 {
            let _zerodha = cache
                .get(
                    &slot(),
                    Vendor::Zerodha,
                    Format::Compact,
                    || Ok("[]".into()),
                )
                .unwrap();
        }
        let again = cache
            .get(&gdfl, Vendor::Gdfl, Format::Compact, || {
                panic!("GDFL's body was dropped by another vendor's read")
            })
            .unwrap();
        assert_eq!(first.body.as_ptr(), again.body.as_ptr());
    }

    #[test]
    fn concurrent_cold_requests_encode_once() {
        let source = slot();
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

    /// **A COLD ENCODE HOLDS ONLY ITS OWN CELL.** One vendor's encode is
    /// parked inside its build; another vendor's body is answered meanwhile.
    /// Under one mutex for every body this deadlocked.
    #[test]
    fn a_cold_encode_holds_only_its_own_cell() {
        let cache = Cache::default();
        let cache = &cache;
        let (entered, parked) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel::<()>();
        std::thread::scope(|scope| {
            let slow = scope.spawn(move || {
                cache
                    .get(&slot(), Vendor::Gdfl, Format::Compact, || {
                        entered.send(()).unwrap();
                        wait.recv().unwrap();
                        Ok("[]".into())
                    })
                    .unwrap()
            });
            parked.recv().unwrap();
            let other = cache
                .get(&slot(), Vendor::Zerodha, Format::Compact, || {
                    Ok("[0]".into())
                })
                .unwrap();
            assert_eq!(other.body.as_ref(), b"[0]");
            release.send(()).unwrap();
            assert_eq!(slow.join().unwrap().body.as_ref(), b"[]");
        });
    }

    #[test]
    fn poisoned_cache_refuses_instead_of_serving_stale_bytes() {
        let cache = Cache::default();
        let source = slot();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = cache.get(&source, Vendor::Zerodha, Format::Expanded, || {
                panic!("interrupt the cache update")
            });
        }));
        assert!(
            cache
                .get(&source, Vendor::Zerodha, Format::Expanded, || Ok(
                    "[]".into()
                ))
                .is_err()
        );
    }

    #[test]
    fn failed_encoding_is_not_cached_as_an_empty_success() {
        let cache = Cache::default();
        let source = slot();
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
    fn a_kept_answer_is_derived_once_per_slot_and_again_for_a_new_one() {
        let cache = Cache::default();
        let source = slot();
        let first = cache
            .kept(&source, Vendor::Gdfl, "count", 0, || Ok(7_u64))
            .unwrap();
        let again = cache
            .kept(
                &source,
                Vendor::Gdfl,
                "count",
                0,
                || -> Result<u64, String> { panic!("derived twice") },
            )
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        let fresh = cache
            .kept(&slot(), Vendor::Gdfl, "count", 0, || Ok(8_u64))
            .unwrap();
        assert_eq!(*fresh, 8);
        let reparsed = cache
            .kept(&source, Vendor::Gdfl, "count", 1, || Ok(9_u64))
            .unwrap();
        assert_eq!(*reparsed, 9, "another extra key is derived again");
        assert!(
            cache
                .kept(
                    &source,
                    Vendor::Gdfl,
                    "other",
                    0,
                    || -> Result<u64, String> { Err("refused".into()) }
                )
                .is_err(),
            "a refusal is answered, and not kept"
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

//! Read-only original VIX annotations beside immutable single-stop trades.
//! Cold admission authenticates the bounded catalog and reference companion.
//! Warm pages use exact saved trade extents under their compound owner lease.
use axum::http::{StatusCode, Uri};
use cli::index_stop_vix::{Bounds, Reader, Stamp, View};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);
const PAGE_ROWS: u64 = 256;

#[derive(Debug, PartialEq, Eq)]
struct Asked {
    identity: [u8; 32],
    pin: [u8; 32],
    setting: usize,
    offset: u64,
    limit: u64,
}
impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').ok_or("VIX query requires key=value")?;
            if value.is_empty()
                || !matches!(key, "identity" | "pin" | "setting" | "offset" | "limit")
                || !seen.insert(key)
            {
                return Err("empty, duplicate or unknown saved-VIX query field".into());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .filter(|value| *value != [0; 32])
            .ok_or("exact original catalog identity required for VIX reference")?;
        let pin = crate::candidatejson::hex_param(query, "pin")?
            .filter(|value| *value != [0; 32])
            .ok_or("exact original catalog completion required for VIX reference")?;
        let setting = crate::candidatejson::integer(query, "setting")?
            .ok_or("exact saved setting required for VIX reference")?;
        let offset = crate::candidatejson::integer(query, "offset")?.unwrap_or(0);
        let limit = crate::candidatejson::integer(query, "limit")?.unwrap_or(16);
        if !(1..=PAGE_ROWS).contains(&limit) || offset.checked_add(limit).is_none() {
            return Err("saved-VIX page requires a checked offset and limit1..256".into());
        }
        Ok(Self {
            identity,
            pin,
            setting: usize::try_from(setting).map_err(|why| why.to_string())?,
            offset,
            limit,
        })
    }
}

fn response(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}
fn refused(status: StatusCode, why: &str) -> Response {
    response(
        status,
        &json!({"schema_version":1,"status":"refused","rows":[],"refusal":why}),
    )
}

/// Read the original entry/exit-minute VIX stamps for a pinned native trade page.
pub async fn index_stop_vix_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(value) => value,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| render(&root, &asked)) {
        Ok(body) => {
            let result = response(StatusCode::OK, &body);
            if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
                result
            } else {
                refused(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "saved-VIX response exceeds byte admission; no prefix returned",
                )
            }
        }
        Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    })
    .await
    {
        Ok(value) => value,
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "saved-VIX inspection capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}

struct Cached {
    root: PathBuf,
    identity: [u8; 32],
    pin: [u8; 32],
    bounds: Bounds,
    reader: Reader,
}
fn render(root: &Path, asked: &Asked) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    // The producer and reader share this resolver; no smaller reader default.
    let bytes = cli::index_stop::source_context::observation_budget()?;
    let bounds = Bounds {
        bytes,
        records: bytes / 96,
        memory_bytes: bytes,
        page_records: PAGE_ROWS,
    };
    let mut held = CACHE
        .get_or_init(|| Mutex::new(None))
        .try_lock()
        .map_err(|_| "saved-VIX reader busy; nothing queued")?;
    if !held.as_ref().is_some_and(|cached| {
        cached.root == root
            && cached.identity == asked.identity
            && cached.pin == asked.pin
            && cached.bounds == bounds
    }) {
        *held = None;
        let reader = Reader::open(root, asked.identity, asked.pin, bounds)?;
        *held = Some(Cached {
            root: root.to_owned(),
            identity: asked.identity,
            pin: asked.pin,
            bounds,
            reader,
        });
    }
    project_cached(
        &mut held,
        |cached| cached.reader.with_current(|view| project(view, asked)),
        |cached| cached.reader.require_current(),
    )
}

fn project_cached<T, V>(
    cached: &mut Option<T>,
    project: impl FnOnce(&T) -> Result<V, String>,
    require_current: impl FnOnce(&T) -> Result<(), String>,
) -> Result<V, String> {
    let reader = cached.as_ref().ok_or("saved-VIX reader admission absent")?;
    let result = project(reader);
    if result.is_err() && require_current(reader).is_err() {
        // Invalid page coordinates do not discard an authenticated catalog.
        // Changed/busy authorities still refuse and evict, so an explicit retry
        // can cold-admit byte-identical restored files under the original pin.
        *cached = None;
    }
    result
}

fn project(view: &View<'_>, asked: &Asked) -> Result<Value, String> {
    let meta = view.metadata();
    let selected = view.native_record(asked.setting)?;
    let page = view.page(asked.setting, asked.offset, asked.limit)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(page.rows.len())
        .map_err(|why| format!("saved-VIX page allocation refused: {why}"))?;
    for row in page.rows {
        let month = view.month(row.month_index)?;
        let trade_index = usize::try_from(row.trade_index).map_err(|why| why.to_string())?;
        let original = view.original_trade(asked.setting, row.trade_index)?;
        rows.push(json!({
            "trade_index":row.trade_index.to_string(),"run_id":hex(row.run_id),
            "original_trade_digest":hex(row.trade_digest),
            "entry_micros":row.entry_micros.to_string(),"exit_bar_micros":row.exit_bar_micros.to_string(),
            "exit_from_micros":row.exit_from_micros.to_string(),"exit_until_micros":row.exit_until_micros.to_string(),
            "month":{"index":row.month_index.to_string(),"year":month.year,"month":month.month,
                "records":month.records.map(|n|n.to_string()),"snapshot_digest":month.snapshot_digest.map(hex),
                "unavailable_code":month.unavailable_code(),"unavailable_reason":month.unavailable_reason},
            "entry":stamp(row.entry),"exit":stamp(row.exit),
            "original_trade":crate::indexstopjson::trade(trade_index,original)
        }));
    }
    Ok(json!({
        "schema_version":1,"status":"saved","model":"index-stop-vix","provenance_status":"saved_reference_snapshot",
        "catalog_identity":hex(meta.catalog_identity),"catalog_completion":hex(meta.catalog_completion),
        "reference":{"identity":hex(meta.lookup_identity),"publication_id":hex(meta.publication_id),"completion":hex(meta.completion_digest)},
        "selected":crate::indexstopjson::setting(asked.setting,selected),"feed":meta.feed,
        "reference_symbol":"NSE-INDIAVIX","reference_timeframe":"1min","reference_only":true,
        "entry_basis":"entry_minute","exit_basis":"exit_minute_candle","policy":cli::index_stop_vix::POLICY,
        "summary":{"settings":meta.settings.to_string(),"trades":meta.trades.to_string(),
            "exact_stamps":meta.exact_stamps.to_string(),"absent_stamps":meta.absent_stamps.to_string(),"unavailable_stamps":meta.unavailable_stamps.to_string()},
        "total":page.total.to_string(),"offset":page.offset.to_string(),"limit":asked.limit,
        "next":page.next_offset.map(|n|n.to_string()),"rows":rows,
        "admitted_bytes":view.admitted_bytes().to_string(),"observation_byte_limit":view.bounds().bytes.to_string()
    }))
}

fn stamp(value: Stamp) -> Value {
    match value {
        Stamp::Exact(candle) => json!({"state":"exact","candle":{
            "micros":candle.ts_micros.to_string(),"open_paisa":candle.open.to_string(),
            "high_paisa":candle.high.to_string(),"low_paisa":candle.low.to_string(),
            "close_paisa":candle.close.to_string(),"volume":candle.volume.to_string(),
            "open_interest":candle.open_interest.to_string()
        }}),
        Stamp::Absent => json!({"state":"absent","candle":null}),
        Stamp::Unavailable => json!({"state":"unavailable","candle":null}),
    }
}
fn hex(value: [u8; 32]) -> String {
    crate::server::hex32(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const PIN: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    fn query() -> String {
        format!("identity={ID}&pin={PIN}&setting=0")
    }
    #[test]
    fn every_reference_page_requires_exact_catalog_completion_and_canonical_bounds()
    -> Result<(), String> {
        let asked = Asked::parse(&query())?;
        assert_eq!((asked.setting, asked.offset, asked.limit), (0, 0, 16));
        let upper = Asked::parse(&format!("{}&offset=123&limit=256", query()))?;
        assert_eq!((upper.offset, upper.limit), (123, 256));
        for bad in [
            String::new(),
            format!("identity={ID}&setting=0"),
            format!("identity={ID}&pin={PIN}"),
            format!("identity={ID}&pin={}&setting=0", "0".repeat(64)),
            format!("identity={}&pin={PIN}&setting=0", "0".repeat(64)),
            format!("identity={ID}&pin={PIN}&setting=01"),
        ] {
            assert!(Asked::parse(&bad).is_err(), "accepted {bad}");
        }
        for suffix in [
            "&setting=0",
            "&limit=0",
            "&limit=257",
            "&limit=01",
            "&limit=",
            "&offset=-1",
            "&offset=18446744073709551615",
            "&offset=18446744073709551616",
            "&feed=other",
            "&kind=current",
            "&before=1",
            "&pin=bad",
            "&garbage",
            "&",
        ] {
            let bad = format!("{}{suffix}", query());
            assert!(Asked::parse(&bad).is_err(), "accepted {bad}");
        }
        Ok(())
    }
    #[test]
    #[expect(
        clippy::default_trait_access,
        reason = "API receives the native candle through CLI's stamp type without an indicators dependency"
    )]
    fn absence_and_unavailable_remain_different_reference_states() {
        // Native stamp variants remain explicit; a missing reference has no
        // numeric field that a browser could mistake for a measured zero.
        assert_eq!(
            stamp(Stamp::Absent),
            json!({"state":"absent","candle":null})
        );
        assert_eq!(
            stamp(Stamp::Unavailable),
            json!({"state":"unavailable","candle":null})
        );
        assert_ne!(stamp(Stamp::Absent), stamp(Stamp::Unavailable));
        assert_eq!(
            stamp(Stamp::Exact(Default::default())),
            json!({"state":"exact","candle":{"micros":"0","open_paisa":"0",
                "high_paisa":"0","low_paisa":"0","close_paisa":"0","volume":"0","open_interest":"0"}})
        );
    }
    #[test]
    #[expect(
        clippy::default_trait_access,
        reason = "API receives the native candle through CLI's stamp type without an indicators dependency"
    )]
    fn complete_reference_candle_keeps_exact_signed_integer_values_and_null_open_interest()
    -> Result<(), String> {
        let mut original = Stamp::Exact(Default::default());
        let Stamp::Exact(candle) = &mut original else {
            return Err("generated exact reference fixture must contain a candle".into());
        };
        candle.ts_micros = 1_700_000_040_000_000;
        candle.open = 1_500;
        candle.high = 1_550;
        candle.low = 1_490;
        candle.close = 1_530;
        candle.volume = i64::MAX;
        candle.open_interest = i64::MIN;
        assert_eq!(
            stamp(original),
            json!({"state":"exact","candle":{"micros":"1700000040000000","open_paisa":"1500",
                "high_paisa":"1550","low_paisa":"1490","close_paisa":"1530",
                "volume":"9223372036854775807","open_interest":"-9223372036854775808"}})
        );
        Ok(())
    }
    #[test]
    fn invalid_vix_pages_keep_current_admission_but_changed_or_busy_owners_evict() {
        let mut cached = Some(41_u64);
        for reason in [
            "setting outside saved extent",
            "offset outside saved extent",
        ] {
            let failed: Result<(), String> = project_cached(
                &mut cached,
                |reader| {
                    assert_eq!(*reader, 41);
                    Err(reason.into())
                },
                |reader| {
                    assert_eq!(*reader, 41);
                    Ok(())
                },
            );
            assert_eq!(failed, Err(reason.into()));
            assert_eq!(cached, Some(41));
        }
        let mut rechecked = false;
        assert_eq!(
            project_cached(
                &mut cached,
                |reader| Ok(*reader),
                |_| {
                    rechecked = true;
                    Ok(())
                }
            ),
            Ok(41)
        );
        assert!(!rechecked);
        for failure in ["owner busy", "completion generation changed"] {
            let failed: Result<(), String> = project_cached(
                &mut cached,
                |_| Err("original guarded projection refused".into()),
                |_| Err(failure.into()),
            );
            assert_eq!(failed, Err("original guarded projection refused".into()));
            assert!(cached.is_none());
            // Models an explicit later cold admission, not an automatic retry.
            cached = Some(41);
            assert_eq!(
                project_cached(&mut cached, |reader| Ok(*reader), |_| Ok(())),
                Ok(41)
            );
        }
    }
}

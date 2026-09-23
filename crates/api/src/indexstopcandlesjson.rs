//! Read-only original-source names and real candle windows for saved index trades.
//! Cold admission authenticates and reconstructs the bounded archived source.
//! Warm pages retain that reader and revalidate all immutable generations.
use axum::http::{StatusCode, Uri};
use cli::index_stop::source_context::{ReadBounds, Reader, View};
use cli::index_stop_store::Direction;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

#[derive(Debug, PartialEq, Eq)]
struct Asked {
    identity: [u8; 32],
    pin: [u8; 32],
    setting: usize,
    trade: Option<usize>,
    before: Option<u64>,
    after: Option<u64>,
}
impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or("source query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity" | "pin" | "setting" | "kind" | "trade" | "before" | "after"
                )
                || !seen.insert(key)
            {
                return Err("empty, duplicate or unknown original-source query field".into());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .filter(|value| *value != [0; 32])
            .ok_or("exact original catalog identity required")?;
        let pin = crate::candidatejson::hex_param(query, "pin")?
            .filter(|value| *value != [0; 32])
            .ok_or("exact original catalog completion required")?;
        let setting = crate::candidatejson::integer(query, "setting")?
            .ok_or("exact saved setting required")?;
        let trade = crate::candidatejson::integer(query, "trade")?;
        let before = crate::candidatejson::integer(query, "before")?;
        let after = crate::candidatejson::integer(query, "after")?;
        match crate::server::param(query, "kind").as_str() {
            "source" if trade.is_none() && before.is_none() && after.is_none() => {}
            "candles" if trade.is_some() && before.is_some() && after.is_some() => {
                if before
                    .zip(after)
                    .and_then(|(a, b)| a.checked_add(b))
                    .is_none_or(|count| count >= crate::detail::MAX_RESULT_ROWS)
                {
                    return Err("original candle context exceeds its page admission".into());
                }
            }
            _ => {
                return Err(
                    "choose source metadata or one exact saved trade and candle context".into(),
                );
            }
        }
        Ok(Self {
            identity,
            pin,
            setting: usize::try_from(setting).map_err(|why| why.to_string())?,
            trade: trade
                .map(usize::try_from)
                .transpose()
                .map_err(|why| why.to_string())?,
            before,
            after,
        })
    }
    const fn kind(&self) -> &'static str {
        if self.trade.is_some() {
            "candles"
        } else {
            "source"
        }
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

/// Inspect one pinned original source or one real saved trade's candle window.
pub async fn index_stop_candles_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(value) => value,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| render(&root, &asked)) {
        Ok(body) => {
            let result = response(StatusCode::OK, &body);
            if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
                refused(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "original-source response exceeds byte admission; no prefix returned",
                )
            } else {
                result
            }
        }
        Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    })
    .await
    {
        Ok(value) => value,
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "original-source inspection capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}

struct Cached {
    root: PathBuf,
    identity: [u8; 32],
    pin: [u8; 32],
    bounds: ReadBounds,
    reader: Reader,
}
fn render(root: &Path, asked: &Asked) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let bytes = cli::index_stop::source_context::observation_budget()?;
    let bounds = ReadBounds {
        bytes,
        records: bytes / 96,
        // The archive decoder independently checks its actual strides and memory.
        source_records: bytes,
        memory_bytes: bytes,
        page_records: crate::detail::MAX_RESULT_ROWS,
    };
    let mut held = CACHE
        .get_or_init(|| Mutex::new(None))
        .try_lock()
        .map_err(|_| "original-source reader busy; nothing queued")?;
    if !held.as_ref().is_some_and(|cached| {
        cached.root == root
            && cached.identity == asked.identity
            && cached.pin == asked.pin
            && cached.bounds == bounds
    }) {
        *held = None;
        let reader = Reader::open(root, asked.identity, asked.pin, asked.setting, bounds)
            .map_err(|why| why.to_string())?;
        *held = Some(Cached {
            root: root.to_owned(),
            identity: asked.identity,
            pin: asked.pin,
            bounds,
            reader,
        });
    }
    let reader = &mut held
        .as_mut()
        .ok_or("original-source reader admission absent")?
        .reader;
    if let Err(why) = reader.require_current() {
        *held = None;
        return Err(why.to_string());
    }
    let body = reader
        .select_setting(asked.setting)
        .and_then(|()| reader.with_current(|view| project(view, asked)));
    if body.is_err() && reader.require_current().is_err() {
        // The current request still refuses. An explicit retry may re-admit
        // byte-identical restored files under the same original completion.
        *held = None;
    }
    body.map_err(|why| why.to_string())
}

fn project(reader: &View<'_>, asked: &Asked) -> Result<Value, String> {
    let meta = reader.metadata();
    let direction = match reader.direction() {
        Direction::Long => "long",
        Direction::Short => "short",
        Direction::Undirected => return Err("saved candle source has no trading direction".into()),
    };
    let window = asked
        .trade
        .map(|trade| {
            reader
                .window(
                    trade,
                    asked.before.ok_or("original candle context is missing")?,
                    asked.after.ok_or("original candle context is missing")?,
                )
                .map_err(|why| why.to_string())
        })
        .transpose()?;
    let candles: Vec<Value> = window
        .as_ref()
        .map(|window| {
            window
                .candles
                .iter()
                .map(|bar| {
                    json!({"micros":bar.ts_micros.to_string(),"open_paisa":bar.open.to_string(),
            "high_paisa":bar.high.to_string(),"low_paisa":bar.low.to_string(),
            "close_paisa":bar.close.to_string(),"volume":bar.volume.to_string(),
            "open_interest":bar.open_interest.to_string()})
                })
                .collect()
        })
        .unwrap_or_default();
    let names: Vec<Value> = reader
        .condition_names()
        .iter()
        .map(|row| json!({"bit":row.bit,"name":row.name}))
        .collect();
    Ok(
        json!({"schema_version":1,"model":"index-stop-candles","provenance_status":"verified_original_source",
        "kind":asked.kind(),"catalog_identity":hex(meta.catalog_identity),"catalog_completion":hex(meta.catalog_completion),
        "setting":meta.setting.to_string(),"source_id":hex(meta.source_id),"run_id":hex(meta.run_id),
        "source_context_identity":hex(meta.source_context_identity),"source_context_completion":hex(meta.source_context_completion),
        "feed":meta.feed,"instrument":meta.instrument,"timeframe":meta.timeframe,"expression":meta.expression,
        "original_build_commit":meta.original_build_commit,"vocabulary_version":meta.vocabulary_version,
        "condition_names":names,"direction":direction,"source_first_day":meta.source_first_day.to_string(),
        "source_last_day":meta.source_last_day.to_string(),"measurement_first_day":meta.measurement_first_day.to_string(),
        "measurement_last_day":meta.measurement_last_day.to_string(),"trade_index":asked.trade.map(|n|n.to_string()),
        "before":asked.before.map(|n|n.to_string()),"after":asked.after.map(|n|n.to_string()),
        "first_bar":window.as_ref().map(|window|window.first_bar.to_string()),"total_bars":reader.total_bars().to_string(),
        "candles":candles,"trade":window.as_ref().zip(asked.trade).map(|(window,index)|crate::indexstopjson::trade(index,&window.trade)),
        "admitted_bytes":reader.admitted_bytes().to_string(),"observation_byte_limit":reader.observation_byte_limit().to_string()}),
    )
}
fn hex(value: [u8; 32]) -> String {
    crate::server::hex32(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    #[test]
    fn original_source_requests_are_pinned_exact_and_bounded_before_file_reads() {
        let base = format!("identity={ID}&pin={ID}&setting=0");
        assert!(Asked::parse(&format!("{base}&kind=source")).is_ok());
        assert!(Asked::parse(&format!("{base}&kind=candles&trade=0&before=60&after=0")).is_ok());
        for extra in [
            "",
            "&kind=source&trade=0",
            "&kind=source&before=0",
            "&kind=candles",
            "&kind=candles&trade=0&before=0",
            "&kind=source&feed=fixture",
            "&kind=source&setting=1",
            "&kind=candles&trade=01&before=0&after=0",
            "&kind=candles&trade=0&before=4096&after=0",
            "&kind=candles&trade=0&before=18446744073709551615&after=1",
        ] {
            assert!(Asked::parse(&format!("{base}{extra}")).is_err(), "{extra}");
        }
        assert!(Asked::parse(&format!("identity={ID}&setting=0&kind=source")).is_err());
        assert!(
            Asked::parse(&format!(
                "identity={}&pin={ID}&setting=0&kind=source",
                "0".repeat(64)
            ))
            .is_err()
        );
    }
    #[test]
    fn source_refusal_keeps_an_empty_evidence_schema_and_exact_reason() {
        let result = refused(
            StatusCode::SERVICE_UNAVAILABLE,
            "original_context_unavailable",
        );
        assert_eq!(result.0, StatusCode::SERVICE_UNAVAILABLE);
        assert!(result.2.contains("original_context_unavailable"));
        assert!(result.2.contains("\"rows\":[]"));
        assert!(!result.2.contains("verified_original_source"));
    }
}

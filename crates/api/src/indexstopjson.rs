//! Bounded, read-only views of immutable single-stop candidate observations.
use axum::http::{StatusCode, Uri};
use cli::index_stop_store::{Direction, Event, Metrics, Period, Policy, Reader, Record, Trade};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);
struct Asked {
    identity: [u8; 32],
    completion: Option<[u8; 32]>,
    kind: String,
    setting: Option<usize>,
    offset: usize,
    limit: usize,
}
impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or("single-stop query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity" | "completion" | "kind" | "setting" | "offset" | "limit"
                )
                || !seen.insert(key)
            {
                return Err("empty, duplicate or unknown single-stop query field".into());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("exact single-stop identity required")?;
        let completion = crate::candidatejson::hex_param(query, "completion")?;
        let kind = match crate::server::param(query, "kind").as_str() {
            "" => "settings".into(),
            s @ ("settings" | "trades" | "events" | "days") => s.into(),
            _ => return Err("unknown single-stop page kind".into()),
        };
        let number = |key, default| {
            usize::try_from(crate::candidatejson::integer(query, key)?.unwrap_or(default))
                .map_err(|why| why.to_string())
        };
        let (offset, limit) = (number("offset", 0)?, number("limit", 16)?);
        let setting = crate::candidatejson::integer(query, "setting")?
            .map(usize::try_from)
            .transpose()
            .map_err(|why| why.to_string())?;
        if identity == [0; 32]
            || completion == Some([0; 32])
            || !(1..=256).contains(&limit)
            || (kind != "settings") != setting.is_some()
            || (kind != "settings" || offset != 0) && completion.is_none()
        {
            return Err(
                "single-stop continuation requires exact completion, setting and limit1..256"
                    .into(),
            );
        }
        Ok(Self {
            identity,
            completion,
            kind,
            setting,
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
        &json!({"schema_version":1,"status":"refused","refusal":why,"rows":[]}),
    )
}
/// Read only a bounded settings/trades/events/day page from an exact receipt.
pub async fn index_stop_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(value) => value,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| render(&root, &asked)) {
        Ok(body) => {
            let reply = response(StatusCode::OK, &body);
            if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
                reply
            } else {
                refused(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "single-stop response exceeds byte limit; no prefix returned",
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
            "single-stop detail capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
struct Cached {
    root: PathBuf,
    identity: [u8; 32],
    budget: crate::detail::BooleanObservationBudget,
    reader: Reader,
}
fn render(root: &Path, asked: &Asked) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let budget = crate::detail::BooleanObservationBudget::load()?;
    let mut held = CACHE
        .get_or_init(|| Mutex::new(None))
        .try_lock()
        .map_err(|_| "single-stop evidence cache is busy; no request queued")?;
    if asked.completion.is_none()
        || !held.as_ref().is_some_and(|value| {
            value.root == root && value.identity == asked.identity && value.budget == budget
        })
    {
        *held = None;
        let reader = Reader::open(root, asked.identity, budget.bytes(), budget.bytes() / 96)
            .map_err(|why| {
                budget.context(&format!(
                    "Single-stop candidate {} unavailable under {}: {why}",
                    crate::server::hex32(asked.identity),
                    root.display()
                ))
            })?;
        *held = Some(Cached {
            root: root.to_owned(),
            identity: asked.identity,
            budget,
            reader,
        });
    }
    let cached = held.as_ref().ok_or("single-stop cache admission absent")?;
    cached
        .reader
        .with_current(|| project(&cached.reader, asked, budget.bytes()))
}
fn project(reader: &Reader, asked: &Asked, budget: u64) -> Result<Value, String> {
    let pin = reader.completion_digest();
    if asked.completion.is_some_and(|expected| expected != pin) {
        return Err("single-stop completion changed; no replacement page returned".into());
    }
    let selected = asked
        .setting
        // render() already holds the publication lease through this projection.
        // A nested record()->require_current() would acquire then release the
        // same file lock before this complete projection has finished.
        .map(|index| {
            reader
                .records()
                .get(index)
                .ok_or("single-stop setting is outside saved extent")
        })
        .transpose()?;
    let (total, rows) = match asked.kind.as_str() {
        "settings" => (
            reader.records().len(),
            window(reader.records(), asked)?
                .iter()
                .enumerate()
                .map(|(n, row)| setting(asked.offset + n, row))
                .collect::<Vec<_>>(),
        ),
        "trades" => {
            let row = selected.ok_or("single-stop setting missing")?;
            (
                row.trades().len(),
                window(row.trades(), asked)?
                    .iter()
                    .enumerate()
                    .map(|(n, row)| trade(asked.offset + n, row))
                    .collect(),
            )
        }
        "events" => {
            let row = selected.ok_or("single-stop setting missing")?;
            (
                row.events().len(),
                window(row.events(), asked)?
                    .iter()
                    .enumerate()
                    .map(|(n, row)| event(asked.offset + n, row))
                    .collect(),
            )
        }
        "days" => {
            let row = selected.ok_or("single-stop setting missing")?;
            (
                row.periods().len(),
                window(row.periods(), asked)?
                    .iter()
                    .enumerate()
                    .map(|(n, row)| day(asked.offset + n, row))
                    .collect(),
            )
        }
        _ => return Err("single-stop unsupported page".into()),
    };
    let next = crate::booleanjson::next_offset(total, asked.offset, asked.limit, rows.len())?;
    Ok(
        json!({"schema_version":1,"status":"saved","model":"index-stop","authority":"authenticated-saved-observations","identity":hex(reader.identity()),"completion":hex(pin),"kind":asked.kind,"setting":asked.setting.map(|n|n.to_string()),"selected":selected.zip(asked.setting).map(|(row,n)|setting(n,row)),"offset":asked.offset.to_string(),"limit":asked.limit,"total":total.to_string(),"next":next.map(|n|n.to_string()),"rows":rows,"refusal":null,"admitted_bytes":reader.admitted_bytes().to_string(),"observation_byte_limit":budget.to_string(),"policy":policy()}),
    )
}
fn window<'a, T>(rows: &'a [T], asked: &Asked) -> Result<&'a [T], String> {
    let end = asked
        .offset
        .checked_add(asked.limit)
        .ok_or("single-stop page overflow")?
        .min(rows.len());
    rows.get(asked.offset..end)
        .ok_or_else(|| "single-stop page outside saved extent".into())
}
fn policy() -> Value {
    json!({"digest":hex(Policy::V1.digest()),"model":"completed_signal_candle_stop","entry":"immediate_next_1min_open","long_stop":"completed_signal_candle_low","short_stop":"completed_signal_candle_high","forced_exit_ist":"15:10","target":false,"trail":false,"horizon":false,"costs_included":false,"institutional_admission":"not_assessed_in_this_view"})
}
pub(crate) fn setting(index: usize, row: &Record) -> Value {
    let truth = row.truth();
    json!({"index":index.to_string(),"program_index":(index/2).to_string(),"run_id":hex(row.run_id()),"source_id":hex(row.source_id()),"evaluation_digest":hex(row.evaluation_digest()),"instrument":row.family().instrument().to_string(),"direction":match row.direction(){Direction::Long=>"long",Direction::Short=>"short",Direction::Undirected=>"unreachable"},"timeframe":row.timeframe(),"first_day":row.first_day().to_string(),"last_day":row.last_day().to_string(),"expression":row.program().to_string(),"truth":{"evaluated":truth.evaluated.to_string(),"hits":truth.hits.to_string(),"misses":truth.misses.to_string(),"unknown":truth.unknown.to_string()},"metrics":metrics(row.metrics()),"events_count":row.events().len().to_string(),"trades_count":row.trades().len().to_string(),"days_count":row.periods().len().to_string(),"feed":null})
}
fn metrics(m: Metrics) -> Value {
    json!({"trades":m.trades.to_string(),"wins":m.wins.to_string(),"optimistic_paisa":m.optimistic_paisa.to_string(),"pessimistic_paisa":m.pessimistic_paisa.to_string(),"drawdown_paisa":m.drawdown_paisa.to_string(),"worst_trade_paisa":m.worst_trade_paisa.to_string(),"adverse_paisa":m.adverse_paisa.to_string(),"favourable_paisa":m.favourable_paisa.to_string(),"holding_minutes":m.holding_minutes.to_string(),"unreachable":m.unreachable.to_string(),"too_late":m.too_late.to_string(),"gap_invalid":m.gap_invalid.to_string(),"while_open":m.while_open.to_string(),"entry_refused":m.entry_refused.to_string(),"path_refused":m.path_refused.to_string(),"closing_refused":m.closing_refused.to_string(),"stopped":m.stopped.to_string(),"forced":m.forced.to_string(),"stop_gaps":m.stop_gaps.to_string()})
}
pub(crate) fn trade(index: usize, t: &Trade) -> Value {
    json!({"index":index.to_string(),"signal_bar":t.signal_bar.to_string(),"entry_bar":t.entry_bar.to_string(),"exit_bar":t.exit_bar.to_string(),"signal_micros":t.signal_micros.to_string(),"signal_close_micros":t.signal_close_micros.to_string(),"entry_micros":t.entry_micros.to_string(),"exit_bar_micros":t.exit_bar_micros.to_string(),"exit_from_micros":t.exit_from_micros.to_string(),"exit_until_micros":t.exit_until_micros.to_string(),"stop_paisa":t.stop_paisa.to_string(),"entry_paisa":t.entry_paisa.to_string(),"optimistic_exit_paisa":t.optimistic_exit_paisa.to_string(),"pessimistic_exit_paisa":t.pessimistic_exit_paisa.to_string(),"optimistic_paisa":t.optimistic_paisa.to_string(),"pessimistic_paisa":t.pessimistic_paisa.to_string(),"adverse_paisa":t.adverse_paisa.to_string(),"favourable_paisa":t.favourable_paisa.to_string(),"holding_minutes":t.holding_minutes.to_string(),"exit_reason":t.exit_reason.as_str(),"gapped":t.gapped})
}
fn event(index: usize, e: &Event) -> Value {
    json!({"index":index.to_string(),"signal_bar":e.signal_bar.to_string(),"signal_micros":e.signal_micros.to_string(),"signal_close_micros":e.signal_close_micros.to_string(),"stop_paisa":e.stop_paisa.to_string(),"reason":e.reason.as_str(),"entry_bar":e.entry_bar.map(|v|v.to_string()),"entry_micros":e.entry_micros.map(|v|v.to_string()),"occupied_through_micros":e.occupied_through_micros.map(|v|v.to_string()),"trade_index":e.trade_index.map(|v|v.to_string())})
}
fn day(index: usize, p: &Period) -> Value {
    json!({"index":index.to_string(),"day":p.day.to_string(),"offered_bars":p.offered_bars.to_string(),"accepted_bars":p.accepted_bars.to_string(),"unavailable_minutes":p.unavailable_minutes.to_string(),"close_verified":p.close_verified,"trades":p.trades.to_string(),"wins":p.wins.to_string(),"optimistic_paisa":p.optimistic_paisa.to_string(),"pessimistic_paisa":p.pessimistic_paisa.to_string(),"refused":p.refused.to_string(),"signals":p.signals.to_string()})
}
fn hex(value: [u8; 32]) -> String {
    crate::server::hex32(value)
}
#[cfg(test)]
#[path = "indexstop_projection_tests.rs"]
mod projection_tests;

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    #[test]
    fn strict_query_requires_pins_for_every_continuation_and_cannot_change_model() {
        assert!(Asked::parse(&format!("identity={ID}")).is_ok());
        for extra in [
            "&offset=1",
            "&kind=trades&setting=0",
            "&kind=grid",
            "&kind=days",
            "&setting=0",
            "&limit=257",
            "&limit=0",
            "&limit=01",
            "&identity=22",
            "&kind=settings&kind=settings",
        ] {
            assert!(
                Asked::parse(&format!("identity={ID}{extra}")).is_err(),
                "{extra}"
            );
        }
        assert!(
            Asked::parse(&format!(
                "identity={ID}&completion={ID}&kind=trades&setting=0&offset=32&limit=256"
            ))
            .is_ok()
        );
    }
    #[test]
    fn zero_and_extreme_metrics_remain_exact_without_inferred_approval() {
        let value = metrics(Metrics {
            trades: u64::MAX,
            pessimistic_paisa: i64::MIN,
            optimistic_paisa: i64::MAX,
            ..Metrics::default()
        });
        assert_eq!(value.get("trades"), Some(&json!(u64::MAX.to_string())));
        assert_eq!(
            value.get("pessimistic_paisa"),
            Some(&json!(i64::MIN.to_string()))
        );
        let policy = policy();
        assert_eq!(
            policy
                .get("institutional_admission")
                .and_then(Value::as_str),
            Some("not_assessed_in_this_view")
        );
        assert_eq!(policy.get("target"), Some(&Value::Bool(false)));
        assert_eq!(policy.get("trail"), Some(&Value::Bool(false)));
        assert_eq!(policy.get("horizon"), Some(&Value::Bool(false)));
    }
    #[test]
    fn page_window_is_bounded_and_zero_extent_differs_from_outside_extent() -> Result<(), String> {
        let mut asked = Asked::parse(&format!("identity={ID}&completion={ID}&limit=2"))?;
        assert_eq!(window(&[1, 2, 3], &asked), Ok(&[1, 2][..]));
        asked.offset = 2;
        assert_eq!(window(&[1, 2, 3], &asked), Ok(&[3][..]));
        asked.offset = 3;
        assert_eq!(window(&[1, 2, 3], &asked), Ok(&[][..]));
        asked.offset = 4;
        assert!(window(&[1, 2, 3], &asked).is_err());
        asked.offset = usize::MAX;
        assert!(window(&[1, 2, 3], &asked).is_err());
        asked.offset = 0;
        assert_eq!(window::<u8>(&[], &asked), Ok(&[][..]));
        let zero = "0".repeat(64);
        assert!(Asked::parse(&format!("identity={zero}")).is_err());
        assert!(Asked::parse(&format!("identity={ID}&completion={zero}")).is_err());
        Ok(())
    }
}

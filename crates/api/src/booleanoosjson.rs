//! Pinned original/later coordinate comparisons. These readers grant no authority.
use axum::http::{StatusCode, Uri};
use cli::boolean_evidence::LaterPeriod;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
#[path = "booleanoosjson_tests.rs"]
mod tests;
type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);
struct Asked {
    identity: [u8; 32],
    completion: Option<[u8; 32]>,
    kind: String,
    candidate: Option<usize>,
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
                .ok_or("later comparison query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity" | "completion" | "kind" | "candidate" | "offset" | "limit"
                )
                || !seen.insert(key)
            {
                return Err("empty, repeated or unknown later comparison selector".into());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("exact later comparison identity required")?;
        let completion = crate::candidatejson::hex_param(query, "completion")?;
        let kind = match crate::server::param(query, "kind").as_str() {
            "" | "coordinates" => "coordinates",
            "trades" => "trades",
            "sessions" => "sessions",
            _ => return Err("unsupported later comparison page".into()),
        }
        .to_owned();
        let integer = |key, default| {
            usize::try_from(crate::candidatejson::integer(query, key)?.unwrap_or(default))
                .map_err(|why| why.to_string())
        };
        let offset = integer("offset", 0)?;
        let limit = integer("limit", 32)?;
        let candidate = crate::candidatejson::integer(query, "candidate")?
            .map(usize::try_from)
            .transpose()
            .map_err(|why| why.to_string())?;
        if !(1..=256).contains(&limit)
            || ((kind != "coordinates") != candidate.is_some())
            || ((offset != 0 || kind != "coordinates") && completion.is_none())
        {
            return Err(
                "later detail requires exact completion, coordinate and limit 1..=256".into(),
            );
        }
        Ok(Self {
            identity,
            completion,
            kind,
            candidate,
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
/// Reads one bounded later-period page and its pinned original settings.
pub async fn later_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(asked) => asked,
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
                    "later comparison response exceeds byte ceiling; no prefix returned",
                )
            }
        }
        Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    })
    .await
    {
        Ok(reply) => reply,
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "later comparison capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
struct Cached {
    root: PathBuf,
    identity: [u8; 32],
    budget: crate::detail::BooleanObservationBudget,
    reader: LaterPeriod,
}
fn render(root: &Path, asked: &Asked) -> Result<Value, String> {
    render_with_budget(
        root,
        asked,
        crate::detail::BooleanObservationBudget::load()?,
    )
}
fn render_with_budget(
    root: &Path,
    asked: &Asked,
    budget: crate::detail::BooleanObservationBudget,
) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "later comparison cache poisoned")?;
    if asked.completion.is_none()
        || !cache.as_ref().is_some_and(|held| {
            held.root == root && held.identity == asked.identity && held.budget == budget
        })
    {
        *cache = None;
        let reader=LaterPeriod::open(root,asked.identity,budget.bytes()).map_err(|why|budget.context(&format!("Later comparison {} unavailable under configured root {}: {why}. Dashboard BRUTEX_STORE must match command OUTPUT_ROOT; no other folder searched.",crate::server::hex32(asked.identity),root.display())))?;
        *cache = Some(Cached {
            root: root.to_path_buf(),
            identity: asked.identity,
            budget,
            reader,
        });
    }
    let mut body = project(
        &cache
            .as_ref()
            .ok_or("later comparison cache disappeared")?
            .reader,
        asked,
    )?;
    body.as_object_mut()
        .ok_or("later projection object absent")?
        .insert(
            "observation_byte_limit".to_owned(),
            json!(budget.bytes().to_string()),
        );
    Ok(body)
}
fn project(reader: &LaterPeriod, asked: &Asked) -> Result<Value, String> {
    reader.require_current()?;
    let pin = reader.completion_digest();
    if asked.completion.is_some_and(|expected| expected != pin) {
        return Err("later comparison completion changed; no replacement page returned".into());
    }
    let selected = asked
        .candidate
        .map(|index| pair(reader, pin, index))
        .transpose()?
        .unwrap_or(Value::Null);
    let (total, rows) = page(reader, asked, pin)?;
    let next = crate::booleanjson::next_offset(total, asked.offset, asked.limit, rows.len())?;
    let parent = reader.parent();
    let summary = reader.summary();
    let family = reader.family();
    let (fy, fm) = summary.from;
    let (ty, tm) = summary.to;
    let admitted_bytes = reader
        .body_bytes()
        .checked_add(parent.body_bytes())
        .and_then(|n| n.checked_add(224))
        .ok_or("later comparison byte count overflow")?;
    let body = json!({"schema_version":1,"status":"saved","authority":"authenticated-later-comparison-observation","identity":crate::server::hex32(reader.identity()),"completion":crate::server::hex32(pin),
        "parent":{"identity":crate::server::hex32(summary.parent),"completion":crate::server::hex32(summary.parent_completion)},"cohort":crate::server::hex32(parent.cohort_digest()),"instrument":family.instrument().to_string(),"cash":family.is_cash(),"membership_digest":crate::server::hex32(family.membership_digest()),
        "program_count":reader.programs().len().to_string(),"coordinate_count":reader.coordinate_count().to_string(),"training_session_count":parent.sessions().len().to_string(),"session_count":reader.sessions().len().to_string(),"grids":parent.grids().iter().map(crate::booleanjson::grid_summary).collect::<Vec<_>>(),
        "later":{"source":crate::server::hex32(summary.source),"execution":crate::server::hex32(summary.execution),"first_micros":summary.first_micros.to_string(),"last_micros":summary.last_micros.to_string(),"bars":summary.bars.to_string(),"from":format!("{fy:04}-{fm:02}"),"to":format!("{ty:04}-{tm:02}")},
        "kind":asked.kind,"candidate":asked.candidate.map(|n|n.to_string()),"offset":asked.offset.to_string(),"limit":asked.limit,"total":total.to_string(),"next":next.map(|n|n.to_string()),"page_complete":true,"selected":selected,"rows":rows,"admitted_bytes":admitted_bytes.to_string(),"refusal":null,
        "scope":"Complete frozen training-coordinate population compared on the explicit later period. Saved bodies and original receipt are authenticated; current raw OHLCV is not reread. Cost-excluded research only, with no selection, admission, full-campaign or future-profitability approval."});
    reader.require_current()?;
    Ok(body)
}
fn pair(reader: &LaterPeriod, pin: [u8; 32], index: usize) -> Result<Value, String> {
    let later = reader
        .coordinates(pin, index, 1)?
        .first()
        .copied()
        .ok_or("later coordinate absent")?;
    let parent = reader.parent();
    let original = parent
        .coordinates(parent.completion_digest(), index, 1)?
        .first()
        .copied()
        .ok_or("original coordinate absent")?;
    Ok(
        json!({"index":index.to_string(),"training":crate::booleanjson::coordinate(parent,index,&original)?,"later":crate::booleanjson::coordinate(parent,index,&later)?}),
    )
}
fn page(reader: &LaterPeriod, asked: &Asked, pin: [u8; 32]) -> Result<(usize, Vec<Value>), String> {
    let start = asked.offset;
    let limit = asked.limit;
    match asked.kind.as_str(){
        "coordinates"=>{
            let later=reader.coordinates(pin,start,limit)?;
            let parent=reader.parent();let original=parent.coordinates(parent.completion_digest(),start,limit)?;
            if later.len()!=original.len(){return Err("later/original page cardinality differs".into());}
            let rows=later.iter().zip(&original).enumerate().map(|(n,(later,original))|Ok(json!({"index":(start+n).to_string(),"training":crate::booleanjson::coordinate(parent,start+n,original)?,"later":crate::booleanjson::coordinate(parent,start+n,later)?}))).collect::<Result<Vec<_>,String>>()?;
            Ok((reader.coordinate_count(),rows))
        }
        "trades"=>{
            let index=asked.candidate.ok_or("later trade coordinate absent")?;
            let row=reader.coordinates(pin,index,1)?.first().copied().ok_or("later trade coordinate absent")?;
            Ok((usize::try_from(row.cell.trades).map_err(|why|why.to_string())?,reader.trades(pin,index,start,limit)?.into_iter().enumerate().map(|(n,row)|crate::booleanjson::trade(start+n,row)).collect()))
        }
        "sessions"=>Ok((reader.sessions().len(),reader.periods(pin,asked.candidate.ok_or("later session coordinate absent")?,start,limit)?.into_iter().enumerate().map(|(n,row)|json!({"index":(start+n).to_string(),"day":row.day.to_string(),"return_paisa":row.return_paisa.to_string(),"trades":row.trades.to_string(),"wins":row.wins.to_string()})).collect())),
        _=>Err("unsupported later comparison page".into()),
    }
}

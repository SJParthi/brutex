//! Bounded search-history observation and separately authenticated exact details.
use axum::http::{StatusCode, Uri};
use cli::boolean_evidence::{QualifiedSearch, SearchAllocation, SearchFraction, SearchSummary};
use serde_json::{Value, json};
use std::path::Path;

const RUNGS: [&str; 8] = cli::EVERY_RUNG;
#[path = "boolean_search_budget.rs"]
pub(crate) mod budget;
type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

struct Asked {
    identity: [u8; 32],
    pin: Option<[u8; 32]>,
    detail: Option<(u64, usize)>,
    completion: Option<[u8; 32]>,
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
                .ok_or("search selectors require key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity" | "pin" | "batch" | "rung" | "completion" | "offset" | "limit"
                )
                || !seen.insert(key)
            {
                return Err("empty, repeated or unknown search selector".into());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("exact search identity is required")?;
        let pin = crate::candidatejson::hex_param(query, "pin")?;
        let completion = crate::candidatejson::hex_param(query, "completion")?;
        let batch = crate::candidatejson::integer(query, "batch")?;
        let rung = crate::candidatejson::integer(query, "rung")?;
        let offset = usize::try_from(crate::candidatejson::integer(query, "offset")?.unwrap_or(0))
            .map_err(|_| "offset is not addressable")?;
        let limit = usize::try_from(crate::candidatejson::integer(query, "limit")?.unwrap_or(16))
            .map_err(|_| "limit is not addressable")?;
        let detail = match (batch, rung) {
            (None, None)
                if completion.is_none() && !seen.contains("offset") && !seen.contains("limit") =>
            {
                None
            }
            (Some(batch), Some(rung))
                if pin.is_some() && rung < 8 && (offset == 0 || completion.is_some()) =>
            {
                Some((
                    batch,
                    usize::try_from(rung).map_err(|_| "rung is not addressable")?,
                ))
            }
            _ => return Err(
                "detail needs exact parent pin, batch and rung; later pages need child completion"
                    .into(),
            ),
        };
        if !(1..=256).contains(&limit) {
            return Err("search page limit must be 1..=256".into());
        }
        Ok(Self {
            identity,
            pin,
            detail,
            completion,
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

/// GET-only exact search history and full common-policy projection pages.
pub async fn search_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(value) => value,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || {
        let result = root.and_then(|root| {
            let budget = crate::detail::BooleanObservationBudget::load()?;
            let replay = budget::ReplayBudget::load()?;
            render(&root, &asked, budget.bytes(), replay.nodes()).map_err(|why| {
                format!(
                    "{}; independent replay node allowance is {}",
                    budget.context(&why),
                    replay.nodes()
                )
            })
        });
        match result {
            Ok(body) => {
                let result = response(StatusCode::OK, &body);
                if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
                    refused(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "search page exceeds response byte limit; no prefix returned",
                    )
                } else {
                    result
                }
            }
            Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
        }
    })
    .await
    {
        Ok(result) => result,
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "search observer capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
fn render(root: &Path, asked: &Asked, bytes: u64, nodes: u64) -> Result<Value, String> {
    let reader=QualifiedSearch::open(root,asked.identity,bytes,nodes).map_err(|why|format!("Qualified search unavailable under configured root {}: {why}. BRUTEX_STORE must match command OUTPUT_ROOT; no other folder searched.",root.display()))?;
    if asked.pin.is_some_and(|pin| pin != reader.pin()) {
        return Err("search checkpoint pin changed; explicitly refresh overview, no replacement page returned".into());
    }
    reader.require_current()?;
    let mut body = match asked.detail {
        Some((batch, rung)) => detail(&reader, asked, batch, rung, bytes)?,
        None => overview(&reader, bytes)?,
    };
    let object = body
        .as_object_mut()
        .ok_or("search projection is not an object")?;
    object.insert(
        "projection_version".into(),
        json!(reader.projection_version()),
    );
    object.insert("timeframes".into(), json!(reader.rungs().labels()));
    object.insert("replay_node_limit".into(), json!(nodes.to_string()));
    if asked.detail.is_none() {
        object.insert(
            "planned_campaign".into(),
            json!(reader.planned_campaign()?.map(crate::server::hex32)),
        );
        object.insert(
            "planned_campaign_authority".into(),
            json!("derived-plan-identity"),
        );
    }
    object
        .entry("replay_nodes_charged")
        .or_insert_with(|| json!(reader.replay_nodes().to_string()));
    reader.require_current()?;
    Ok(body)
}
fn phase(value: u64, owner: bool, exhausted: bool) -> Result<(&'static str, &'static str), String> {
    match value {
        0 if !exhausted => Ok(("reserved", if owner { "running" } else { "paused" })),
        1 => Ok((
            "complete",
            if exhausted {
                "completed"
            } else if owner {
                "running"
            } else {
                "paused"
            },
        )),
        2 if !exhausted => Ok(("refused", "refused")),
        _ => Err("search phase or exhaustion differs".into()),
    }
}
fn summary(rung: usize, s: &SearchSummary) -> Result<Value, String> {
    let name = RUNGS.get(rung).ok_or("rung outside declared eight")?;
    let complete = s.child.identity != [0; 32] && s.child.pin != [0; 32];
    let sum = s
        .counts
        .iter()
        .try_fold(0_u64, |sum, n| sum.checked_add(*n))
        .ok_or("search verdict count overflow")?;
    if sum != s.count
        || (complete && (s.projection == [0; 32] || s.allocation == [0; 32]))
        || (!complete
            && (s.child.identity != [0; 32]
                || s.child.pin != [0; 32]
                || s.count != 0
                || s.projection != [0; 32]
                || s.allocation != [0; 32]))
    {
        return Err("search summary links or complete verdict counts do not reconcile".into());
    }
    let [admitted, rejected, unmeasured, refused] = s.counts;
    Ok(
        json!({"rung":name,"rung_index":rung.to_string(),"qualification":complete.then(||json!({"identity":crate::server::hex32(s.child.identity),"completion":crate::server::hex32(s.child.pin)})),"count":s.count.to_string(),"counts":{"admitted":admitted.to_string(),"rejected":rejected.to_string(),"unmeasured":unmeasured.to_string(),"refused":refused.to_string()},"projection_digest":complete.then(||crate::server::hex32(s.projection)),"allocation_digest":complete.then(||crate::server::hex32(s.allocation))}),
    )
}
fn overview(reader: &QualifiedSearch, bytes: u64) -> Result<Value, String> {
    let (phase, state) = phase(reader.phase(), reader.owner_observed(), reader.exhausted())?;
    let rows = selected_summaries(reader.summaries(), reader.rungs())?;
    let node_only = phase == "complete" && rows.iter().all(|row| row["qualification"].is_null());
    Ok(
        json!({"schema_version":1,"status":"saved","model":"qualified-search","kind":"overview","authority":"acknowledged-search-history","identity":crate::server::hex32(reader.identity()),"pin":crate::server::hex32(reader.pin()),"sequence":reader.sequence().to_string(),"batch":reader.batch().to_string(),"phase":phase,"state":state,"reason":(!reader.reason().is_empty()).then_some(reader.reason()),"owner_active":reader.owner_observed(),"completed_batches":reader.completed_batches().to_string(),"exhausted":reader.exhausted(),"grammar_work":reader.work().to_string(),"programs":reader.programs()?.to_string(),"node_only":node_only,"alpha_ppm":reader.alpha_ppm().to_string(),"admitted_bytes":reader.admitted_bytes().to_string(),"observation_byte_limit":bytes.to_string(),"history_checked":true,"child_bodies_checked":false,"rows":rows,"refusal":null,"scope":"Recorded grammar reservations and complete batch projections. Counts include every declared rung; no estimated total or invented completion percentage. Overview does not authenticate child bodies or reread current raw-market files."}),
    )
}
fn selected_summaries(
    summaries: &[SearchSummary; 8],
    rungs: cli::boolean_campaign::RungScope,
) -> Result<Vec<Value>, String> {
    summaries
        .iter()
        .enumerate()
        .filter(|(rung, _)| rungs.contains(*rung))
        .map(|(rung, s)| summary(rung, s))
        .collect()
}
fn fraction(value: SearchFraction) -> Value {
    json!({"numerator":value.numerator().to_string(),"denominator":value.denominator().to_string()})
}
fn allocation(value: SearchAllocation, draws: u64) -> Value {
    let minimum = value.minimum_draws();
    json!({"digest":crate::server::hex32(value.digest()),"batch":value.batch().to_string(),"rung":value.rung().to_string(),"rungs":value.rungs().to_string(),"alpha_ppm":value.alpha_ppm().to_string(),"threshold":fraction(value.threshold()),"minimum_draws":minimum.map(|n|n.to_string()),"draw_resolution_met":minimum.is_some_and(|n|u128::from(draws)>=n),"rule":"alpha / (8 * (batch + 1) * (batch + 2)); reserved/refused slots are not recycled"})
}
fn detail(
    reader: &QualifiedSearch,
    asked: &Asked,
    batch: u64,
    rung: usize,
    bytes: u64,
) -> Result<Value, String> {
    let selected = reader.rung(reader.pin(), batch, rung)?;
    selected.require_current()?;
    let source = selected.source();
    let completion = source.completion_digest();
    if asked.completion.is_some_and(|pin| pin != completion) {
        return Err("qualification child pin changed; no replacement page returned".into());
    }
    source.require_current()?;
    let rows = selected.rows(completion, asked.offset, asked.limit)?;
    let search_policy = selected.policy();
    let search_policy_digest = search_policy.digest();
    if rows
        .iter()
        .any(|row| row.projection.policy_digest() != search_policy_digest)
    {
        return Err("search comparison policy differs from its shared probability ceiling".into());
    }
    let original = crate::booleanevidencejson::qualification_observation(
        source,
        completion,
        asked.offset,
        asked.limit,
    )?;
    let rows=rows.iter().enumerate().map(|(n,row)|{
        let [romano,white,spa]=row.probabilities;
        Ok(json!({"comparison":crate::booleanevidencejson::search_comparison(source,asked.offset+n,row)?,"probabilities":{"romano":fraction(romano),"white":fraction(white),"spa":fraction(spa)}}))
    }).collect::<Result<Vec<_>,String>>()?;
    let [[white_n, white_d], [spa_n, spa_d]] = source.family_probabilities();
    let [draws, _, _] = source.procedure();
    let summary = summary(rung, &selected.summary())?;
    let admitted = selected.admitted_bytes()?;
    let replay_nodes = selected.replay_nodes()?;
    let body = json!({"schema_version":1,"status":"saved","model":"qualified-search","kind":"rung","authority":"authenticated-search-projection","identity":crate::server::hex32(reader.identity()),"pin":crate::server::hex32(reader.pin()),"batch":batch.to_string(),"rung":rung.to_string(),"completion":crate::server::hex32(completion),"source":original,"summary":summary,"allocation":allocation(selected.allocation(),draws),"original_family_probabilities":{"white":{"numerator":white_n.to_string(),"denominator":white_d.to_string()},"spa":{"numerator":spa_n.to_string(),"denominator":spa_d.to_string()}},"admitted_bytes":admitted.to_string(),"observation_byte_limit":bytes.to_string(),"child_bodies_checked":true,"rows":rows,"refusal":null,"scope":"Additional search-wide comparison of fixed-training later evidence. The effective policy retains every original setting and caps four family probability ceilings at the shared search allowance. Original qualification and policy remain separate. Cost-excluded research; no Selection V6, live trading, calibrated adaptive search or future-return guarantee."});
    let mut body = body;
    if selected.projection_version() == 1 {
        body.as_object_mut()
            .ok_or("search detail is not an object")?
            .insert("scope".into(), json!("Historical V1 comparison retains its original probability ceilings and exact original arithmetic. White and SPA were not capped at the shared search allowance. This is an observation of the old result, not a V2 admission. Original qualification remains separate. Cost-excluded research; no live trading or future-return guarantee."));
    }
    body.as_object_mut()
        .ok_or("search detail is not an object")?
        .insert(
            "search_policy".into(),
            crate::booleanevidencejson::search_policy(&search_policy),
        );
    body.as_object_mut()
        .ok_or("search detail is not an object")?
        .insert(
            "replay_nodes_charged".into(),
            json!(replay_nodes.to_string()),
        );
    source.require_current()?;
    selected.require_current()?;
    Ok(body)
}

#[cfg(test)]
#[path = "booleansearchjson_tests.rs"]
mod tests;

//! Bounded, read-only campaign snapshots; child bodies open only on detail routes.
use axum::http::{StatusCode, Uri};
use cli::boolean_campaign::{Link, Reader, Rung};
use serde_json::{Value, json};
use std::path::Path;

#[cfg(test)]
#[path = "booleancampaignjson_tests.rs"]
mod tests;

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);
struct Asked {
    identity: [u8; 32],
    pin: Option<[u8; 32]>,
}
impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or("campaign query requires key=value")?;
            if value.is_empty() || !matches!(key, "identity" | "pin") || !seen.insert(key) {
                return Err("empty, repeated or unknown campaign selector".to_owned());
            }
        }
        Ok(Self {
            identity: crate::candidatejson::hex_param(query, "identity")?
                .ok_or("exact campaign identity is required")?,
            pin: crate::candidatejson::hex_param(query, "pin")?,
        })
    }
}
fn response(status: StatusCode, value: &Value) -> Response {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        value.to_string(),
    )
}
fn refused(status: StatusCode, reason: &str) -> Response {
    response(
        status,
        &json!({"schema_version":1,"status":"refused","refusal":reason,"rows":[]}),
    )
}

/// Reads one current acknowledged campaign snapshot under server-owned limits.
/// A supplied pin must match; no history or alternate output folder is searched.
pub async fn campaign_json(uri: Uri) -> Response {
    serve(uri, false).await
}
/// Observes selected predeclared qualification slots and full checkpoint history.
pub async fn qualified_campaign_json(uri: Uri) -> Response {
    serve(uri, true).await
}
async fn serve(uri: Uri, qualified: bool) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(asked) => asked,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| {
        if qualified { render_qualified(&root, &asked) } else { render(&root, &asked) }
    }) {
        Ok(body) => {
            let reply = response(StatusCode::OK, &body);
            if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
                reply
            } else {
                refused(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "campaign comparison exceeds response byte ceiling; no partial rows returned",
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
            "campaign observer capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
fn render_qualified(root: &Path, asked: &Asked) -> Result<Value, String> {
    let reader=cli::boolean_evidence::QualifiedCampaign::open(root,asked.identity,crate::detail::MAX_SCAN_BYTES)
        .map_err(|why|format!("Qualified campaign {} unavailable under configured root {}: {why}. Dashboard BRUTEX_STORE must match command OUTPUT_ROOT; no other folder searched.",crate::server::hex32(asked.identity),root.display()))?;
    require_pin(asked.pin, reader.pin())?;
    let selected: Vec<_> = reader
        .slots()
        .iter()
        .zip(cli::EVERY_RUNG)
        .enumerate()
        .filter(|(index, _)| reader.rungs().contains(*index))
        .collect();
    let rows:Vec<_>=selected.iter().map(|(index,(slot,rung))|json!({"rung":rung,"rung_index":index.to_string(),"unit":crate::server::hex32(slot.unit),"state":if slot.complete.is_some(){"completed"}else if !slot.reason.is_empty(){"refused"}else if slot.started{"running"}else{"waiting"},"reason":(!slot.reason.is_empty()).then_some(&slot.reason),"qualification":slot.complete.map(|link|json!({"identity":crate::server::hex32(link.identity),"completion":crate::server::hex32(link.pin)}))})).collect();
    let state = if selected.iter().all(|(_, (s, _))| s.complete.is_some()) {
        "completed"
    } else if selected.iter().any(|(_, (s, _))| !s.reason.is_empty()) {
        "refused"
    } else if selected.iter().any(|(_, (s, _))| s.started) {
        "running"
    } else {
        "waiting"
    };
    let body = json!({"schema_version":1,"status":"saved","authority":"acknowledged-qualified-campaign-history","identity":crate::server::hex32(reader.identity()),"pin":crate::server::hex32(reader.pin()),"sequence":reader.sequence().to_string(),"descriptor_digest":crate::server::hex32(reader.descriptor()),"timeframes":reader.rungs().labels(),"state":state,"owner_active":reader.owner_observed(),"history_records":reader.history_records().to_string(),"admitted_bytes":reader.admitted_bytes().to_string(),"history_checked":true,"child_completion_receipts_checked":false,"child_bodies_checked":false,"rows":rows,"refusal":null,"scope":"Complete bounded acknowledged qualification history only. Child pins are recorded links; their separate detail reader authenticates receipt bodies and ancestors. An owner observation is not liveness. No current raw-source, Selection V6, live-trading or future-profitability approval."});
    reader.require_current()?;
    Ok(body)
}
fn render(root: &Path, asked: &Asked) -> Result<Value, String> {
    // The mutable latest-snapshot selector is freshly observed. Immutable
    // children are not decoded here; their separate detail caches own that work.
    let reader = Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)
        .map_err(|why|format!("Campaign {} unavailable under configured root {}: {why}. Dashboard BRUTEX_STORE must match command OUTPUT_ROOT; no other folder searched.",crate::server::hex32(asked.identity),root.display()))?;
    require_pin(asked.pin, reader.pin())?;
    let (from_year, from_month) = reader.from();
    let (to_year, to_month) = reader.to();
    let value = json!({"schema_version":1,"status":"saved","authority":"acknowledged-campaign-snapshot","identity":crate::server::hex32(reader.identity()),"pin":crate::server::hex32(reader.pin()),"sequence":reader.sequence().to_string(),"state":reader.status().as_str(),"owner_active":reader.owner_active(),"from":format!("{from_year:04}-{from_month:02}"),"to":format!("{to_year:04}-{to_month:02}"),"horizon_bars":reader.horizon().to_string(),"program_count":reader.program_count().to_string(),"program_digest":crate::server::hex32(reader.program_digest()),"descriptor_digest":crate::server::hex32(reader.descriptor_digest()),"child_completion_receipts_checked":true,"child_bodies_checked":false,"rows":rows(reader.rows()),"refusal":null,"scope":"Acknowledged finite-catalog campaign snapshot and fixed child completion receipts only. Child bodies authenticate on their detail routes. No current raw-market re-attestation, exhaustive Boolean grammar, Selection V6, later-period acceptance or profitability approval."});
    reader.require_current()?;
    Ok(value)
}
fn require_pin(expected: Option<[u8; 32]>, actual: [u8; 32]) -> Result<(), String> {
    if expected.is_some_and(|pin| pin != actual) {
        return Err(
            "campaign snapshot changed; explicitly refresh to read the new snapshot".to_owned(),
        );
    }
    Ok(())
}
fn link(value: Link) -> Value {
    json!({"identity":crate::server::hex32(value.identity),"completion":crate::server::hex32(value.completion)})
}
fn rows(rows: &[Rung]) -> Vec<Value> {
    rows.iter().map(|row| {
        let expected:Vec<_>=row.catalogs.iter().map(|catalog|json!({"instrument":catalog.family.instrument().to_string(),"identity":crate::server::hex32(catalog.expected),"completion":catalog.completion.map(crate::server::hex32),"cash":catalog.family.is_cash(),"membership_digest":crate::server::hex32(catalog.family.membership_digest())})).collect();
        let catalogs:Vec<_>=row.catalogs.iter().filter_map(|catalog|catalog.completion.map(|pin|json!({"instrument":catalog.family.instrument().to_string(),"identity":crate::server::hex32(catalog.expected),"completion":crate::server::hex32(pin)}))).collect();
        json!({"rung":row.rung,"state":row.status.as_str(),"reason":(!row.reason.is_empty()).then_some(&row.reason),"expected_catalogs":expected,"catalogs":catalogs,"statistics":row.statistics.map(link),"admission":row.admission.map(link)})
    }).collect()
}

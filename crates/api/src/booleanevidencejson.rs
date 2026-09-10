//! Bounded observation-only views of saved Boolean statistics and admission.
use axum::http::{StatusCode, Uri};
use cli::boolean_evidence::{
    Admission, Qualification, Statistics, StatisticsRow, StatisticsSource, StatisticsSummary,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[path = "booleanadmission_projection.rs"]
pub(crate) mod admission_projection;
#[path = "index_consistency_projection.rs"]
pub(crate) mod index_consistency_projection;
#[path = "booleanqualification_projection.rs"]
mod qualification_projection;
pub(crate) use index_consistency_projection::policy as index_consistency_policy;
#[cfg(test)]
#[path = "booleanevidencejson_tests.rs"]
mod tests;
type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Model {
    Statistics,
    Admission,
    Qualification,
}
impl Model {
    const fn name(self) -> &'static str {
        match self {
            Self::Statistics => "statistics",
            Self::Admission => "admission",
            Self::Qualification => "qualification",
        }
    }
}
struct Asked {
    model: Model,
    identity: [u8; 32],
    completion: Option<[u8; 32]>,
    kind: String,
    offset: usize,
    limit: usize,
    setting: Option<usize>,
    period: String,
}
impl Asked {
    fn parse(query: &str, model: Model) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or("evidence query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity" | "completion" | "kind" | "offset" | "limit" | "setting" | "period"
                )
                || !seen.insert(key)
            {
                return Err("empty, repeated or unknown Boolean evidence selector".to_owned());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("exact evidence identity is required")?;
        let completion = crate::candidatejson::hex_param(query, "completion")?;
        let kind = match crate::server::param(query, "kind").as_str() {
            "" | "candidates" => "candidates".to_owned(),
            value @ ("sources" | "splits") if model == Model::Statistics => value.to_owned(),
            value @ ("index-weeks" | "index-days") if model == Model::Qualification => {
                value.to_owned()
            }
            _ => return Err("unsupported page kind for this evidence model".to_owned()),
        };
        let offset = usize::try_from(crate::candidatejson::integer(query, "offset")?.unwrap_or(0))
            .map_err(|why| why.to_string())?;
        let limit = usize::try_from(crate::candidatejson::integer(query, "limit")?.unwrap_or(16))
            .map_err(|why| why.to_string())?;
        let setting = crate::candidatejson::integer(query, "setting")?
            .map(usize::try_from)
            .transpose()
            .map_err(|why| why.to_string())?;
        let period = match crate::server::param(query, "period").as_str() {
            "" | "full" => "full",
            "training" => "training",
            "later" => "later",
            _ => return Err("index consistency period must be full, training or later".into()),
        }
        .to_owned();
        let index_detail = matches!(kind.as_str(), "index-weeks" | "index-days");
        if index_detail != setting.is_some() || (!index_detail && seen.contains("period")) {
            return Err("index detail requires one exact setting; other evidence pages reject setting/period".into());
        }
        if !(1..=256).contains(&limit)
            || ((offset != 0 || kind != "candidates") && completion.is_none())
        {
            return Err("later/detail pages require exact completion and limit 1..=256".to_owned());
        }
        Ok(Self {
            model,
            identity,
            completion,
            kind,
            offset,
            limit,
            setting,
            period,
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
/// Read authenticated saved statistics, including explicit unavailable tests.
pub async fn statistics_json(uri: Uri) -> Response {
    serve(uri, Model::Statistics).await
}
/// Read every saved admission reason and value; observation grants no authority.
pub async fn admission_json(uri: Uri) -> Response {
    serve(uri, Model::Admission).await
}
/// Observe complete fixed-training later qualification, without promotion authority.
pub async fn qualification_json(uri: Uri) -> Response {
    serve(uri, Model::Qualification).await
}

/// Reuse the same authenticated qualification projection in a search-wide view.
pub(crate) fn qualification_observation(
    reader: &Qualification,
    completion: [u8; 32],
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    qualification_projection::project(
        reader,
        &Asked {
            model: Model::Qualification,
            identity: reader.identity(),
            completion: Some(completion),
            kind: "candidates".into(),
            offset,
            limit,
            setting: None,
            period: "full".into(),
        },
    )
}

/// Project the common policy result without replacing its original qualification.
pub(crate) fn search_comparison(
    reader: &Qualification,
    index: usize,
    row: &cli::boolean_evidence::SearchRow,
) -> Result<Value, String> {
    let mut result = admission_projection::row(
        index,
        &cli::boolean_evidence::AdmissionRow {
            identity: row.source.original,
            source_index: index,
            values: row.projection.values(),
            verdict: row.projection.verdict(),
        },
    )?;
    put(
        &mut result,
        "policy_digest",
        json!(crate::server::hex32(row.projection.policy_digest())),
    )?;
    put(
        &mut result,
        "index_consistency",
        index_consistency_projection::setting(reader, index, row.projection.verdict().status())?,
    )?;
    Ok(result)
}

/// Exact original or derived common-policy values, emitted once per detail page.
pub(crate) fn search_policy(policy: &cli::boolean_evidence::AdmissionPolicyV1) -> Value {
    admission_projection::policy(policy)
}
async fn serve(uri: Uri, model: Model) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default(), model) {
        Ok(asked) => asked,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| render(&root, &asked)) {
        Ok(body) => {
            let response = response(StatusCode::OK, &body);
            if response.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
                response
            } else {
                refused(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Boolean evidence response exceeds its byte ceiling; no prefix returned",
                )
            }
        }
        Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    })
    .await
    {
        Ok(response) => response,
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "Boolean evidence detail capacity full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
enum Reader {
    Statistics(Box<Statistics>),
    Admission(Box<Admission>),
    Qualification(Box<Qualification>),
}
struct Cached {
    root: PathBuf,
    model: Model,
    identity: [u8; 32],
    budget: crate::detail::BooleanObservationBudget,
    reader: Reader,
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
    // One aggregate evidence tree in this cache. Catalog detail has its own
    // bounded slot. Cold budgets count all linked serialized evidence, not RAM.
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Boolean evidence cache poisoned")?;
    if asked.completion.is_none()
        || !cache.as_ref().is_some_and(|held| {
            held.root == root
                && held.identity == asked.identity
                && held.model == asked.model
                && held.budget == budget
        })
    {
        *cache = None;
        let reader=match asked.model {
            Model::Statistics=>Statistics::open(root,asked.identity,budget.bytes()).map(Box::new).map(Reader::Statistics),
            Model::Admission=>Admission::open(root,asked.identity,budget.bytes()).map(Box::new).map(Reader::Admission),
            Model::Qualification=>Qualification::open(root,asked.identity,budget.bytes()).map(Box::new).map(Reader::Qualification),
        }.map_err(|why|budget.context(&format!("Boolean {} evidence {} unavailable under configured root {}: {why}. Dashboard BRUTEX_STORE must match command OUTPUT_ROOT; no other folder searched.",asked.model.name(),crate::server::hex32(asked.identity),root.display())))?;
        *cache = Some(Cached {
            root: root.to_path_buf(),
            model: asked.model,
            identity: asked.identity,
            budget,
            reader,
        });
    }
    let mut body = match &cache
        .as_ref()
        .ok_or("Boolean evidence cache disappeared")?
        .reader
    {
        Reader::Statistics(reader) => statistics(reader, asked),
        Reader::Admission(reader) => admission(reader, asked),
        Reader::Qualification(reader) => qualification_projection::project(reader, asked),
    }?;
    put(
        &mut body,
        "observation_byte_limit",
        json!(budget.bytes().to_string()),
    )?;
    Ok(body)
}
fn put(body: &mut Value, key: &str, value: Value) -> Result<(), String> {
    body.as_object_mut()
        .ok_or("Boolean projection object absent")?
        .insert(key.to_owned(), value);
    Ok(())
}
fn base(asked: &Asked, pin: [u8; 32], total: usize, rows: Vec<Value>) -> Result<Value, String> {
    if asked.completion.is_some_and(|expected| expected != pin) {
        return Err("Boolean evidence completion differs; no replacement page returned".to_owned());
    }
    let expected = total
        .checked_sub(asked.offset)
        .ok_or("Boolean evidence offset outside extent")?
        .min(asked.limit);
    if expected != rows.len() {
        return Err("Boolean evidence page omits or adds saved rows".to_owned());
    }
    let end = asked
        .offset
        .checked_add(expected)
        .ok_or("Boolean evidence cursor overflow")?;
    let mut body = json!({"schema_version":1,"status":"saved","authority":"authenticated-saved-evidence-observation","model":asked.model.name(),"identity":crate::server::hex32(asked.identity),"completion":crate::server::hex32(pin),"kind":asked.kind,"offset":asked.offset.to_string(),"limit":asked.limit,"total":total.to_string(),"next":(end<total).then(||end.to_string()),"page_complete":true,"refusal":null,
        "scope":"Saved research evidence and exact linked catalog receipts. No current raw-market re-attestation, successor authoring capability, Selection V6 or future-profitability approval is granted by this view."});
    put(&mut body, "rows", Value::Array(rows))?;
    Ok(body)
}
fn statistics(reader: &Statistics, asked: &Asked) -> Result<Value, String> {
    reader.require_current()?;
    let pin = reader.completion_digest();
    if asked.completion.is_some_and(|expected| expected != pin) {
        return Err("statistics completion pin differs".to_owned());
    }
    let (total,rows)=match asked.kind.as_str() {
        "candidates"=>(reader.summary().candidates,reader.rows(pin,asked.offset,asked.limit)?.iter().enumerate().map(|(n,row)|statistics_row(reader,asked.offset+n,row)).collect::<Result<Vec<_>,_>>()?),
        "splits"=>(reader.summary().splits,reader.splits(pin,asked.offset,asked.limit)?.iter().enumerate().map(|(n,row)|json!({"index":(asked.offset+n).to_string(),"train_mask":row.train_mask.to_string(),"test_mask":row.test_mask.to_string(),"bottom_half":row.bottom_half,"rankable":row.rankable,"scores_digest":crate::server::hex32(row.scores_digest)})).collect()),
        "sources"=>{let sources=reader.sources();let end=asked.offset.checked_add(asked.limit).ok_or("source page overflow")?.min(sources.len());let page=sources.get(asked.offset..end).ok_or("source offset outside extent")?;(sources.len(),page.iter().enumerate().map(|(n,row)|source(asked.offset+n,row)).collect())},
        _=>return Err("unknown statistics page kind".to_owned()),
    };
    let mut body = base(asked, pin, total, rows)?;
    put(&mut body, "summary", summary(reader.summary())?)?;
    put(
        &mut body,
        "admitted_bytes",
        json!(reader.admitted_bytes().to_string()),
    )?;
    reader.require_current()?;
    Ok(body)
}
fn admission(reader: &Admission, asked: &Asked) -> Result<Value, String> {
    reader.require_current()?;
    let pin = reader.completion_digest();
    if asked.completion.is_some_and(|expected| expected != pin) {
        return Err("admission completion pin differs".to_owned());
    }
    let rows = reader.rows(pin, asked.offset, asked.limit)?;
    let stats = reader.statistics();
    let observations = stats.rows(stats.completion_digest(), asked.offset, asked.limit)?;
    if observations.len() != rows.len() {
        return Err("admission page differs from linked statistics extent".to_owned());
    }
    let mut rendered = Vec::new();
    for (n, (row, observation)) in rows.iter().zip(&observations).enumerate() {
        let index = asked.offset + n;
        if row.source_index != index || row.identity != observation.identity {
            return Err("admission row differs from linked candidate statistics".to_owned());
        }
        let mut value = admission_projection::row(index, row)?;
        put(
            &mut value,
            "statistics",
            statistics_row(stats, index, observation)?,
        )?;
        rendered.push(value);
    }
    let mut body = base(asked, pin, reader.row_count(), rendered)?;
    put(
        &mut body,
        "statistics_identity",
        json!(crate::server::hex32(stats.identity())),
    )?;
    put(
        &mut body,
        "statistics_completion",
        json!(crate::server::hex32(stats.completion_digest())),
    )?;
    put(&mut body, "summary", summary(stats.summary())?)?;
    put(
        &mut body,
        "policy",
        admission_projection::policy(&reader.policy()),
    )?;
    put(
        &mut body,
        "admitted_bytes",
        json!(
            reader
                .body_bytes()
                .checked_add(112)
                .and_then(|bytes| bytes.checked_add(stats.admitted_bytes()))
                .ok_or("admission observed-byte count overflow")?
                .to_string()
        ),
    )?;
    reader.require_current()?;
    Ok(body)
}
fn source(index: usize, row: &StatisticsSource) -> Value {
    json!({"index":index.to_string(),"instrument":row.family.instrument().to_string(),"cash":row.family.is_cash(),"membership_digest":crate::server::hex32(row.family.membership_digest()),"identity":crate::server::hex32(row.identity),"completion":crate::server::hex32(row.completion),"coordinates":row.coordinates.to_string()})
}
fn statistics_row(reader: &Statistics, index: usize, row: &StatisticsRow) -> Result<Value, String> {
    let linked = reader
        .sources()
        .get(row.source)
        .ok_or("statistics source link missing")?;
    let romano=row.romano.map(|[strategy,rank,statistic,exceedances,initial_n,initial_d,adjusted_n,adjusted_d]|json!({"strategy":strategy.to_string(),"stepdown_rank":rank.to_string(),"statistic":float(statistic),"strict_exceedances":exceedances.to_string(),"initial":{"numerator":initial_n.to_string(),"denominator":initial_d.to_string()},"adjusted":{"numerator":adjusted_n.to_string(),"denominator":adjusted_d.to_string()}}));
    Ok(
        json!({"index":index.to_string(),"identity":crate::server::hex32(row.identity),"run":crate::server::hex32(row.run),"source":source(row.source,linked),"coordinate":row.coordinate.to_string(),"program_index":row.program_index.to_string(),"side":row.side,"ordinal":row.ordinal.to_string(),"execution_refusal_bits":row.execution_refusal_bits.to_string(),"trades":row.trades.to_string(),"wins":row.wins.to_string(),"return_paisa":row.return_paisa.to_string(),"wilson_lower":float(row.wilson_lower_bits),"period_digest":crate::server::hex32(row.period_digest),"romano_availability":availability(row.romano_availability)?,"romano":romano}),
    )
}
fn availability(code: u64) -> Result<&'static str, String> {
    match code {
        1 => Ok("measured"),
        2 => Ok("unavailable-constant-return-candidate"),
        3 => Ok("unavailable-numerical-refusal"),
        _ => Err("unknown saved Romano-Wolf availability".to_owned()),
    }
}
fn float(bits: u64) -> Value {
    let value = f64::from_bits(bits);
    json!({"bits":bits.to_string(),"decimal":value.is_finite().then(||value.to_string())})
}
fn family_test(
    [
        statistic,
        probability,
        numerator,
        denominator,
        exceedances,
        draws,
        strategies,
        periods,
    ]: [u64; 8],
) -> Value {
    json!({"statistic":float(statistic),"probability":float(probability),"exact_probability":{"numerator":numerator.to_string(),"denominator":denominator.to_string()},"matched_or_exceeded":exceedances.to_string(),"draws":draws.to_string(),"strategies":strategies.to_string(),"periods":periods.to_string()})
}
fn summary(summary: &StatisticsSummary) -> Result<Value, String> {
    let [draws, seed, block_length] = summary.procedure;
    let [
        families,
        candidates,
        observations,
        bootstrap_work,
        split_work,
        memory_bytes,
        bytes,
    ] = summary.bounds;
    let [white, spa, romano] = summary.test_families;
    Ok(
        json!({"scope":crate::server::hex32(summary.scope),"cohort":crate::server::hex32(summary.cohort),"layout":crate::server::hex32(summary.layout),"families":summary.families.to_string(),"candidates":summary.candidates.to_string(),"periods":summary.periods.to_string(),"segments":summary.segments.to_string(),"periods_per_segment":summary.periods_per_segment.to_string(),"splits":summary.splits.to_string(),"procedure":{"draws":draws.to_string(),"seed":seed.to_string(),"block_length":block_length.to_string()},"bounds":{"families":families.to_string(),"candidates":candidates.to_string(),"observations":observations.to_string(),"bootstrap_work":bootstrap_work.to_string(),"split_work":split_work.to_string(),"additional_working_bytes":memory_bytes.to_string(),"saved_body_bytes":bytes.to_string()},"test_families":{"white":crate::server::hex32(white),"spa":crate::server::hex32(spa),"romano":crate::server::hex32(romano)},"white":family_test(summary.white),"spa":family_test(summary.spa),"romano_availability":availability(summary.romano_availability)?,"contributing_splits":summary.contributing_splits.to_string(),"bottom_half_splits":summary.bottom_half_splits.to_string()}),
    )
}

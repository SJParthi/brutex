//! Exact, pinned observation pages for the finite supplied Boolean catalog.
//! Cold admission hashes and decodes the bounded body; warm reads retain its
//! owner/receipt/body guards. No decoded page mints a successor authority.
use axum::http::{StatusCode, Uri};
use cli::boolean_observation::{
    Cell, Coordinate, ExecutionRefusalBitsV1 as Refusal, ExitGridSelectorV1, ForcedStopV1,
    GridContext, RangeResolutionV1, Reader, Side, TradeRow,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

#[derive(Debug)]
struct Asked {
    identity: [u8; 32],
    completion: Option<[u8; 32]>,
    kind: String,
    candidate: Option<usize>,
    side: Option<Side>,
    axis: Option<String>,
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
                .ok_or("catalog query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity"
                        | "completion"
                        | "kind"
                        | "candidate"
                        | "side"
                        | "axis"
                        | "offset"
                        | "limit"
                )
                || !seen.insert(key)
            {
                return Err("empty, repeated or unknown catalog query field".to_owned());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("full catalog identity is required")?;
        let completion = crate::candidatejson::hex_param(query, "completion")?;
        let text = |key| crate::server::param(query, key);
        let kind = match text("kind").as_str() {
            "" => "coordinates".to_owned(),
            value @ ("coordinates" | "programs" | "trades" | "sessions" | "grid") => {
                value.to_owned()
            }
            _ => return Err("unknown catalog page kind".to_owned()),
        };
        let integer = |key, default| -> Result<usize, String> {
            usize::try_from(crate::candidatejson::integer(query, key)?.unwrap_or(default))
                .map_err(|why| why.to_string())
        };
        let offset = integer("offset", 0)?;
        let limit = integer("limit", 32)?;
        let candidate = crate::candidatejson::integer(query, "candidate")?
            .map(usize::try_from)
            .transpose()
            .map_err(|why| why.to_string())?;
        let side = match text("side").as_str() {
            "" => None,
            "long" => Some(Side::Long),
            "short" => Some(Side::Short),
            _ => return Err("side must be long or short".to_owned()),
        };
        let axis = match text("axis").as_str() {
            "" => None,
            value @ ("stop" | "target" | "trail" | "requested-stop" | "requested-target"
            | "requested-trail") => Some(value.to_owned()),
            _ => return Err("unknown saved grid axis".to_owned()),
        };
        if !(1..=256).contains(&limit)
            || (matches!(kind.as_str(), "trades" | "sessions") != candidate.is_some())
            || (kind == "grid" && (side.is_none() || axis.is_none()))
            || (kind != "grid" && (side.is_some() || axis.is_some()))
            || ((kind != "coordinates" || offset != 0) && completion.is_none())
        {
            return Err(
                "catalog page requires exact completion, selectors and limit 1..=256".to_owned(),
            );
        }
        Ok(Self {
            identity,
            completion,
            kind,
            candidate,
            side,
            axis,
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
/// Read a bounded saved catalog page on the common blocking-work budget.
pub async fn boolean_json(uri: Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(asked) => asked,
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
                    "catalog response exceeds byte admission; no prefix returned",
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
            "catalog detail capacity full; nothing queued",
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
    // One bounded catalog, including all decoded child observations. No scan of
    // other identities and no accumulation of a process-wide unbounded history.
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Boolean catalog cache poisoned")?;
    if asked.completion.is_none()
        || !cache.as_ref().is_some_and(|held| {
            held.root == root && held.identity == asked.identity && held.budget == budget
        })
    {
        *cache = None;
        let reader = Reader::open(root, asked.identity, budget.bytes()).map_err(|why| budget.context(&format!(
            "Catalog {} unavailable under configured evidence root {}: {why}. The dashboard BRUTEX_STORE must match the catalog command OUTPUT_ROOT; no other folder was searched.",
            crate::server::hex32(asked.identity), root.display()
        )))?;
        *cache = Some(Cached {
            root: root.to_path_buf(),
            identity: asked.identity,
            budget,
            reader,
        });
    }
    let held = cache
        .as_ref()
        .ok_or("catalog cache admission disappeared")?;
    let mut body = project(&held.reader, asked)?;
    body.as_object_mut()
        .ok_or("catalog projection object absent")?
        .insert(
            "observation_byte_limit".to_owned(),
            json!(budget.bytes().to_string()),
        );
    Ok(body)
}

fn project(reader: &Reader, asked: &Asked) -> Result<Value, String> {
    reader.require_current()?;
    let completion = reader.completion_digest();
    if asked.completion.is_some_and(|pin| pin != completion) {
        return Err("catalog completion changed; no replacement page returned".to_owned());
    }
    let (total, selected, rows) = page(reader, asked, completion)?;
    let next = next_offset(total, asked.offset, asked.limit, rows.len())?;
    let family = reader.family();
    let body = json!({
        "schema_version":1,"status":"saved","authority":"authenticated-catalog-observation",
        "identity":crate::server::hex32(reader.identity()),"completion":crate::server::hex32(completion),
        "cohort":crate::server::hex32(reader.cohort_digest()),
        "kind":asked.kind,"candidate":asked.candidate.map(|n|n.to_string()),"side":asked.side.map(side_name),"axis":asked.axis,
        "offset":asked.offset.to_string(),"limit":asked.limit,"total":total.to_string(),"next":next.map(|n|n.to_string()),
        "page_complete":true,"program_count":reader.programs().len().to_string(),"coordinate_count":reader.coordinate_count().to_string(),
        "session_count":reader.sessions().len().to_string(),"instrument":family.instrument().to_string(),"cash":family.is_cash(),
        "membership_digest":crate::server::hex32(family.membership_digest()),
        "grids":reader.grids().iter().map(grid_summary).collect::<Vec<_>>(),"selected":selected,"rows":rows,"refusal":null,
        "scope":"Complete supplied program catalog only. Saved training observations do not grant statistical admission, Selection V6, OOS performance or live trading approval."
    });
    reader.require_current()?;
    Ok(body)
}

fn page(
    reader: &Reader,
    asked: &Asked,
    pin: [u8; 32],
) -> Result<(usize, Value, Vec<Value>), String> {
    let candidate = asked
        .candidate
        .map(|index| -> Result<Coordinate, String> {
            reader
                .coordinates(pin, index, 1)?
                .first()
                .copied()
                .ok_or("selected coordinate absent".to_owned())
        })
        .transpose()?;
    let selected = candidate
        .zip(asked.candidate)
        .map(|(row, index)| coordinate(reader, index, &row))
        .transpose()?
        .unwrap_or(Value::Null);
    let start = asked.offset;
    let limit = asked.limit;
    let (total, rows) = match asked.kind.as_str() {
        "coordinates" => (reader.coordinate_count(), reader.coordinates(pin,start,limit)?.iter().enumerate().map(|(n,row)|coordinate(reader,start+n,row)).collect::<Result<Vec<_>,_>>()?),
        "programs" => (reader.programs().len(), window(reader.programs(),start,limit)?.iter().enumerate().map(|(n,row)|json!({"index":(start+n).to_string(),"expression":row.to_string()})).collect()),
        "trades" => {
            let row = candidate.ok_or("trade coordinate missing")?;
            (usize::try_from(row.cell.trades).map_err(|why|why.to_string())?, reader.trades(pin,asked.candidate.ok_or("trade coordinate missing")?,start,limit)?.into_iter().enumerate().map(|(n,row)|trade(start+n,row)).collect())
        }
        "sessions" => (reader.sessions().len(),reader.periods(pin,asked.candidate.ok_or("session coordinate missing")?,start,limit)?.into_iter().enumerate().map(|(n,row)|json!({"index":(start+n).to_string(),"day":row.day.to_string(),"return_paisa":row.return_paisa.to_string(),"trades":row.trades.to_string(),"wins":row.wins.to_string()})).collect()),
        "grid" => grid_page(reader,asked)?,
        _ => return Err("unsupported catalog page".to_owned()),
    };
    Ok((total, selected, rows))
}
fn window<T>(rows: &[T], start: usize, limit: usize) -> Result<&[T], String> {
    let end = start
        .checked_add(limit)
        .ok_or("catalog page overflow")?
        .min(rows.len());
    rows.get(start..end)
        .ok_or("catalog page outside saved extent".to_owned())
}
pub(crate) fn next_offset(
    total: usize,
    start: usize,
    limit: usize,
    actual: usize,
) -> Result<Option<usize>, String> {
    let expected = total
        .checked_sub(start)
        .ok_or("catalog offset beyond extent")?
        .min(limit);
    if actual != expected {
        return Err("catalog page omitted or added saved rows".to_owned());
    }
    let end = start
        .checked_add(actual)
        .ok_or("catalog continuation overflow")?;
    Ok((end < total).then_some(end))
}
fn side_name(side: Side) -> &'static str {
    match side {
        Side::Long => "long",
        Side::Short => "short",
    }
}
pub(crate) fn coordinate(reader: &Reader, index: usize, row: &Coordinate) -> Result<Value, String> {
    let program = reader
        .programs()
        .get(row.program_index)
        .ok_or("coordinate program missing")?;
    let grid = reader
        .grids()
        .iter()
        .find(|grid| grid.policy.side() == row.side)
        .ok_or("coordinate grid missing")?;
    Ok(
        json!({"index":index.to_string(),"identity":crate::server::hex32(row.identity),"run":crate::server::hex32(row.run),
        "program_index":row.program_index.to_string(),"expression":program.to_string(),"side":side_name(row.side),"ordinal":row.ordinal.to_string(),
        "truth":{"evaluated":row.truth.evaluated.to_string(),"hits":row.truth.hits.to_string(),"misses":row.truth.misses.to_string(),"unknown":row.truth.unknown.to_string()},
        "support_sessions":row.support_sessions.to_string(),"execution_refusal_bits":row.execution_refusal_bits.to_string(),"execution_refusals":refusal_names(row.execution_refusal_bits)?,
        "cell":cell(&row.cell),"levels":selected_levels(&row.cell,grid)?}),
    )
}
fn refusal_names(bits: u64) -> Result<Vec<&'static str>, String> {
    let bits = Refusal::from_bits(bits).ok_or("unknown execution refusal bit")?;
    Ok([
        (Refusal::MISSING_STOP, "Missing stop"),
        (Refusal::MISSING_TARGET, "Missing target"),
        (Refusal::ZERO_TRADES, "No completed trades"),
        (Refusal::AMBIGUITY_LIMIT, "Ambiguous-bar ceiling exceeded"),
        (Refusal::GAP_LIMIT, "Gap-fill ceiling exceeded"),
        (Refusal::FORCED_STOP_MISMATCH, "Required exact stop differs"),
    ]
    .into_iter()
    .filter_map(|(reason, name)| bits.contains(reason).then_some(name))
    .collect())
}
fn cell(row: &Cell) -> Value {
    json!({"stop":row.stop.map(|n|n.to_string()),"target":row.target.map(|n|n.to_string()),"tsl":row.tsl.map(|n|n.to_string()),
        "ttp":row.ttp.map(|value|json!({"arm":value.arm.to_string(),"trail":value.trail.to_string()})),
        "trades":row.trades.to_string(),"wins":row.wins.to_string(),"pessimistic":row.pessimistic.to_string(),"optimistic":row.optimistic.to_string(),"fill_cost":row.fill_cost.to_string(),
        "stopped":row.stopped.to_string(),"targeted":row.targeted.to_string(),"trailed_stop":row.trailed_stop.to_string(),"trailed_profit":row.trailed_profit.to_string(),"timed_out":row.timed_out.to_string(),"ambiguous_bars":row.ambiguous_bars.to_string(),"gapped":row.gapped.to_string()})
}
fn selected_levels(row: &Cell, grid: &GridContext) -> Result<Value, String> {
    let level = |values: &[i64], index: Option<usize>| {
        index
            .map(|n| {
                values
                    .get(n)
                    .map(ToString::to_string)
                    .ok_or("coordinate ladder index missing".to_owned())
            })
            .transpose()
    };
    Ok(
        json!({"stop_ppm":level(&grid.stops,row.stop)?,"target_ppm":level(&grid.targets,row.target)?,"tsl_ppm":level(&grid.trails,row.tsl)?,
        "ttp_arm_ppm":level(&grid.targets,row.ttp.map(|value|value.arm))?,"ttp_trail_ppm":level(&grid.trails,row.ttp.map(|value|value.trail))?}),
    )
}
pub(crate) fn grid_summary(grid: &GridContext) -> Value {
    let policy = &grid.policy;
    let forced = match policy.forced_stop() {
        ForcedStopV1::Disabled => json!({"kind":"disabled","ppm":null}),
        ForcedStopV1::IncludeExactObserved(ppm) => {
            json!({"kind":"include-exact-observed","ppm":ppm.to_string()})
        }
        ForcedStopV1::RequireExactObserved(ppm) => {
            json!({"kind":"require-exact-observed","ppm":ppm.to_string()})
        }
    };
    json!({"side":side_name(policy.side()),"resolution":crate::server::hex32(grid.resolution),"execution":crate::server::hex32(grid.execution),"feed":crate::server::hex32(grid.feed),"commit":crate::server::hex32(grid.commit),"calendar":crate::server::hex32(grid.calendar),
        "bars":grid.bars.to_string(),"first_micros":grid.first_micros.to_string(),"last_micros":grid.last_micros.to_string(),"cells":grid.cells.to_string(),
        "stop_count":grid.stops.len().to_string(),"target_count":grid.targets.len().to_string(),"trail_count":grid.trails.len().to_string(),
        "requested_stop_count":policy.rungs().stop().len().to_string(),"requested_target_count":policy.rungs().target().len().to_string(),"requested_trail_count":policy.rungs().trail().len().to_string(),
        "execution_seconds":"60","horizon_bars":grid.horizon_bars.to_string(),"range_rounding":match policy.range_resolution(){RangeResolutionV1::PpmFloor=>"floor",RangeResolutionV1::PpmCeiling=>"ceiling"},
        "selector":match policy.selector(){ExitGridSelectorV1::PessimisticTotal=>"pessimistic-total",ExitGridSelectorV1::EdgeThenPessimistic=>"edge-then-pessimistic",ExitGridSelectorV1::GuaranteedFloor=>"guaranteed-floor"},
        "ratio_min_hundredths":policy.ratios().min_hundredths().to_string(),"ratio_max_hundredths":policy.ratios().max_hundredths().to_string(),"max_pairs":policy.ratios().max_pairs().to_string(),"max_cells":policy.max_cells().to_string(),"max_levels_per_axis":policy.rungs().max_levels_per_axis().to_string(),
        "cost_model":crate::server::hex32(policy.cost_model_id()),"forced_stop":forced,"max_ambiguous_bars":policy.max_ambiguous_bars().to_string(),"max_gap_fills":policy.max_gap_fills().to_string()})
}
fn grid_page(reader: &Reader, asked: &Asked) -> Result<(usize, Vec<Value>), String> {
    let grid = reader
        .grids()
        .iter()
        .find(|grid| Some(grid.policy.side()) == asked.side)
        .ok_or("saved grid side missing")?;
    let axis = asked.axis.as_deref().ok_or("saved grid axis missing")?;
    if let Some(name) = axis.strip_prefix("requested-") {
        let values = match name {
            "stop" => grid.policy.rungs().stop(),
            "target" => grid.policy.rungs().target(),
            "trail" => grid.policy.rungs().trail(),
            _ => return Err("saved requested axis missing".to_owned()),
        };
        Ok((values.len(),window(values,asked.offset,asked.limit)?.iter().enumerate().map(|(n,value)|json!({"index":(asked.offset+n).to_string(),"numerator":value.numerator().to_string(),"denominator":value.denominator().to_string()})).collect()))
    } else {
        let values = match axis {
            "stop" => &grid.stops,
            "target" => &grid.targets,
            "trail" => &grid.trails,
            _ => return Err("saved resolved axis missing".to_owned()),
        };
        Ok((values.len(),window(values,asked.offset,asked.limit)?.iter().enumerate().map(|(n,value)|json!({"index":(asked.offset+n).to_string(),"ppm":value.to_string()})).collect()))
    }
}
pub(crate) fn trade(index: usize, row: TradeRow) -> Value {
    json!({"index":index.to_string(),"signal_bar":row.signal_bar.to_string(),"entry_bar":row.entry_bar.to_string(),"exit_bar":row.exit_bar.to_string(),"best":row.best.to_string(),"worst":row.worst.to_string(),"entry_micros":row.entry_micros.to_string(),"exit_micros":row.exit_micros.to_string(),"adverse_ppm":row.adverse.to_string(),"adverse_paisa":row.adverse_paisa.to_string(),"favourable_ppm":row.favourable.to_string(),"favourable_paisa":row.favourable_paisa.to_string()})
}

#[cfg(test)]
#[path = "booleanjson_tests.rs"]
pub(crate) mod tests;

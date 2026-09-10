//! Exact index-only signal-candle-stop launch through the shared audited worker.
use axum::http::{StatusCode, Uri};
use cli::index_stop_search::{Launch, LaunchInput};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

#[path = "indexstoplaunch_metadata.rs"]
mod configuration;

/// The additive native command; old grid searches retain their own contract.
pub const COMMAND: &str = cli::index_stop_search::COMMAND;
/// Entire request cap; an oversized prefix is never executed.
pub const REQUEST_BYTES: usize = 16_384;
const MAX_LOSS_POINTS: u64 = u64::MAX / 100;

/// One exact index, selected intraday timeframes and fixed observation periods.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Asked {
    command: String,
    pub(crate) feed: String,
    pub(crate) index: String,
    timeframes: Vec<String>,
    from_year: u16,
    from_month: u8,
    to_year: u16,
    to_month: u8,
    later_from_year: u16,
    later_from_month: u8,
    later_to_year: u16,
    later_to_month: u8,
    bits: String,
    batch_programs: String,
    node_allowance: String,
    batch_allowance: String,
    max_loss_points: String,
    expected_policy_digest: String,
}
impl Asked {
    pub(crate) const fn window(&self) -> ((u16, u8), (u16, u8)) {
        (
            (self.from_year, self.from_month),
            (self.to_year, self.to_month),
        )
    }
    fn input(&self) -> Result<LaunchInput, String> {
        let number = |name: &str, raw: &str| {
            positive(raw)
                .ok_or_else(|| format!("{name} requires a canonical positive decimal string"))
        };
        let input = LaunchInput {
            feed: self.feed.clone(),
            index: self.index.clone(),
            timeframes: self.timeframes.clone(),
            training: self.window(),
            later: (
                (self.later_from_year, self.later_from_month),
                (self.later_to_year, self.later_to_month),
            ),
            bits: self.bits.clone(),
            batch_programs: number("batch_programs", &self.batch_programs)?,
            node_allowance: number("node_allowance", &self.node_allowance)?,
            batch_allowance: number("batch_allowance", &self.batch_allowance)?,
            max_loss_points: number("max_loss_points", &self.max_loss_points)?,
            expected_policy_digest: digest(&self.expected_policy_digest)?,
        };
        input.validate()?;
        Ok(input)
    }
}

pub(crate) fn parse(body: &str) -> Result<Asked, String> {
    if body.len() > REQUEST_BYTES {
        return Err(format!(
            "Single-stop launch exceeds {REQUEST_BYTES} bytes; no prefix accepted"
        ));
    }
    let asked: Asked = serde_json::from_str(body)
        .map_err(|why| format!("Single-stop launch schema refused: {why}"))?;
    if asked.command != COMMAND {
        return Err("single-stop launch requires its exact native command".into());
    }
    asked.input()?;
    Ok(asked)
}

/// Exact invocation and acknowledged journal evidence, independent of activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    request: Asked,
    observed: Option<cli::index_stop_search::Progress>,
    preparing: [Option<cli::index_stop_search::RungStage>; 8],
    saved: Option<SavedBatch>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct SavedBatch {
    ordinal: u64,
    qualifications: [Option<([u8; 32], [u8; 32])>; 8],
}
impl Status {
    pub(crate) fn new(request: &Asked) -> Self {
        Self {
            request: request.clone(),
            observed: None,
            preparing: [None; 8],
            saved: None,
        }
    }
    pub(crate) fn fields(&self) -> Value {
        let qualifications: Vec<_> = cli::EVERY_RUNG.iter().enumerate()
            .filter(|(_, label)| self.request.timeframes.iter().any(|selected| selected == **label))
            .map(|(rung, timeframe)| {
                let link = self.observed.as_ref().and_then(|value| value.qualifications.get(rung)).copied().flatten();
                let stage = self.observed.as_ref().map_or_else(|| self.preparing.get(rung), |value|value.rung_stages.get(rung))
                    .copied().flatten().map(cli::index_stop_search::RungStage::as_str);
                json!({"timeframe":timeframe, "rung":rung,
                    "identity":link.map(|(id,_)| hex(&id)), "completion":link.map(|(_,pin)|hex(&pin)),"stage":stage})
            }).collect();
        let saved = self.saved.as_ref().map(|saved| {
            let qualifications:Vec<_>=cli::EVERY_RUNG.iter().enumerate().filter_map(|(rung,timeframe)| {
                saved.qualifications.get(rung).copied().flatten().map(|(id,pin)|
                    json!({"timeframe":timeframe,"rung":rung,"identity":hex(&id),"completion":hex(&pin)}))
            }).collect();
            json!({"batch":saved.ordinal.to_string(),"qualifications":qualifications})
        });
        json!({"schema_version":1,"command":COMMAND,"execution_policy":"signal_candle_stop_v1",
            "request":self.request,"search_identity":self.observed.as_ref().map(|value|hex(&value.identity)),
            "completed_batches":self.observed.as_ref().map(|value|value.completed_batches.to_string()),
            "completed_programs":self.observed.as_ref().map(|value|value.completed_programs.to_string()),
            "completed_work":self.observed.as_ref().map(|value|value.completed_work.to_string()),
            "current_batch":self.observed.as_ref().and_then(|value|value.current_batch.map(|batch|batch.to_string())),
            "exhausted":self.observed.as_ref().map(|value|value.exhausted),"qualifications":qualifications,
            "latest_saved_batch":saved})
    }
    fn observe_event(&mut self, next: cli::index_stop_search::Observation) -> Result<(), String> {
        use cli::index_stop_search::{Observation, RungStage};
        match next {
            Observation::Search(progress) => self.observe(progress),
            Observation::Preparing { rung, stage } => {
                if self.observed.is_some()
                    || !matches!(stage, RungStage::Preparing | RungStage::Refused)
                    || !self.selected(rung)
                {
                    return Err("single-stop preparation reported execution or foreign scope before its declaration".into());
                }
                let slot = self
                    .preparing
                    .get_mut(rung)
                    .ok_or("single-stop preparation slot missing")?;
                if *slot == Some(RungStage::Refused) && stage != RungStage::Refused {
                    return Err("single-stop refused source cannot silently restart".into());
                }
                *slot = Some(stage);
                Ok(())
            }
        }
    }
    fn selected(&self, rung: usize) -> bool {
        cli::EVERY_RUNG
            .get(rung)
            .is_some_and(|label| self.request.timeframes.iter().any(|value| value == label))
    }
    fn observe(&mut self, next: cli::index_stop_search::Progress) -> Result<(), String> {
        if next.identity == [0; 32]
            || self.observed.as_ref().is_some_and(|prior| {
                prior.identity != next.identity
                    || next.completed_batches < prior.completed_batches
                    || next.completed_programs < prior.completed_programs
                    || next.completed_work < prior.completed_work
                    || prior.exhausted && prior != &next
                    || next.completed_batches == prior.completed_batches
                        && (next.completed_programs != prior.completed_programs
                            || next.completed_work != prior.completed_work
                            || prior.exhausted != next.exhausted
                            || prior.current_batch == next.current_batch
                                && prior.qualifications != next.qualifications)
            })
        {
            return Err(
                "single-stop identity or acknowledged progress changed or regressed".into(),
            );
        }
        let has_links = next.qualifications.iter().any(Option::is_some);
        let retained = self.saved_observation(&next)?;
        if next.completed_batches == 0
            && (has_links
                || next.exhausted
                || next.completed_programs != 0
                || next.completed_work != 0)
        {
            return Err("single-stop initial declaration cannot claim completed research".into());
        }
        if next
            .current_batch
            .is_some_and(|batch| batch != next.completed_batches || next.exhausted)
        {
            return Err(
                "single-stop current batch is not its next acknowledged reservation".into(),
            );
        }
        for (rung, link) in next.qualifications.iter().enumerate() {
            let selected = self.selected(rung);
            if link.is_some() != (has_links && selected)
                || link.is_some_and(|(id, pin)| id == [0; 32] || pin == [0; 32])
            {
                return Err("single-stop progress omitted a selected timeframe or contains a foreign/invalid qualification".into());
            }
            let stage = next.rung_stages.get(rung).copied().flatten();
            if !selected && stage.is_some()
                || next.current_batch.is_some() && selected && stage.is_none()
            {
                return Err("single-stop stage does not match its exact selected scope".into());
            }
            if next.current_batch.is_none() && stage.is_some() != link.is_some()
                || link.is_some() && stage != Some(cli::index_stop_search::RungStage::Saved)
            {
                return Err(
                    "single-stop saved boundary does not match its qualification link".into(),
                );
            }
            if let Some(prior) = self.observed.as_ref().filter(|prior| {
                prior.current_batch.is_some() && prior.current_batch == next.current_batch
            }) {
                let before = prior.rung_stages.get(rung).copied().flatten();
                if before.is_some_and(|before| {
                    stage.is_none_or(|after| stage_rank(after) < stage_rank(before))
                }) {
                    return Err(
                        "single-stop timeframe stage regressed within its reserved batch".into(),
                    );
                }
            }
        }
        if next.current_batch.is_none() && has_links {
            self.saved = Some(SavedBatch {
                ordinal: next
                    .completed_batches
                    .checked_sub(1)
                    .ok_or("single-stop completed receipt is missing its batch ordinal")?,
                qualifications: next.qualifications,
            });
        } else if let Some(saved) = retained {
            self.saved = Some(saved);
        }
        self.observed = Some(next);
        Ok(())
    }

    fn saved_observation(
        &self,
        next: &cli::index_stop_search::Progress,
    ) -> Result<Option<SavedBatch>, String> {
        let Some(saved) = next.latest_saved.as_ref() else {
            return Ok(None);
        };
        if saved.batch >= next.completed_batches {
            return Err("single-stop retained table has no acknowledged batch".into());
        }
        for (rung, link) in saved.qualifications.iter().enumerate() {
            if link.is_some() != self.selected(rung)
                || link.is_some_and(|(id, pin)| id == [0; 32] || pin == [0; 32])
            {
                return Err(
                    "single-stop retained table is incomplete or has foreign timeframe links"
                        .into(),
                );
            }
        }
        let retained = SavedBatch {
            ordinal: saved.batch,
            qualifications: saved.qualifications,
        };
        if self.saved.as_ref().is_some_and(|prior| {
            retained.ordinal < prior.ordinal
                || retained.ordinal == prior.ordinal && retained != *prior
        }) || next.qualifications.iter().any(Option::is_some)
            && (Some(retained.ordinal) != next.completed_batches.checked_sub(1)
                || retained.qualifications != next.qualifications)
        {
            return Err(
                "single-stop retained batch identity or pinned qualifications changed".into(),
            );
        }
        Ok(Some(retained))
    }
}

pub(crate) fn prepare(asked: &Asked, root: &Path) -> Result<Launch, String> {
    let input = asked.input()?;
    let config =
        cli::index_stop_search::configuration(input.max_loss_points, input.batch_programs)?;
    configuration::observation_limits(&config)?;
    cli::index_stop_search::prepare(input, root)
}

pub(crate) fn conduct(
    asked: &Asked,
    admission: Launch,
    started: i64,
    attempt: u64,
    site: &crate::server::Loaded,
) -> crate::sweeprun::Progress {
    let mut progress = crate::sweeprun::Progress::started(
        &asked.feed,
        &asked.index,
        asked.window().0,
        asked.window().1,
        None,
        started,
        attempt,
    )
    .of_kind(crate::sweeprun::Kind::Command);
    let mut status = Status::new(asked);
    let result = asked.input().and_then(|input| {
        if &input != admission.input() {
            return Err("single-stop request changed after admission; no research started".into());
        }
        admission.execute_observed(&mut |next| {
            status.observe_event(next)?;
            let mut held = site
                .sweep
                .lock()
                .map_err(|_| "single-stop status lock is poisoned")?;
            let target = held
                .as_mut()
                .filter(|run| run.attempt == attempt && run.in_flight())
                .ok_or(
                    "single-stop invocation lost its exact active status; no further batch starts",
                )?;
            if target
                .index_stop
                .as_ref()
                .is_none_or(|prior| prior.request != *asked)
            {
                return Err("single-stop active request changed; no further batch starts".into());
            }
            target.index_stop = Some(Box::new(status.clone()));
            Ok(())
        })
    });
    progress.index_stop = Some(Box::new(status));
    progress.finished_micros = Some(started);
    match result {
        Ok(report) => progress.report = Some(report),
        Err(why) => progress.refusal = Some(why),
    }
    progress
}

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

/// Preview explicit configuration only; no history, journal or worker is opened.
pub async fn metadata(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    uri: Uri,
) -> Response {
    let query = uri.query().map(str::to_owned);
    match crate::detail::run(move || configuration::value(&site.store_root, query.as_deref())).await
    {
        Ok(value) => response(StatusCode::OK, &value),
        Err(why) => response(
            StatusCode::SERVICE_UNAVAILABLE,
            &json!({"schema_version":1,"model":"index-stop-qualified-search-launch",
            "ready":false,"refusal":format!("bounded configuration read unavailable: {why:?}")}),
        ),
    }
}
fn response(status: StatusCode, value: &Value) -> Response {
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        value.to_string(),
    )
}
fn positive(raw: &str) -> Option<u64> {
    if raw.is_empty()
        || raw.len() > 20
        || raw.starts_with('0')
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    raw.parse().ok()
}
fn digest(raw: &str) -> Result<[u8; 32], String> {
    if raw.len() != 64
        || !raw
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "expected_policy_digest requires a nonzero lowercase policy fingerprint".into(),
        );
    }
    let mut digest = [0; 32];
    for (slot, pair) in digest.iter_mut().zip(raw.as_bytes().chunks_exact(2)) {
        let pair = std::str::from_utf8(pair).map_err(|why| why.to_string())?;
        *slot = u8::from_str_radix(pair, 16).map_err(|why| why.to_string())?;
    }
    if digest == [0; 32] {
        return Err("zero is not a displayed research policy fingerprint".into());
    }
    Ok(digest)
}
fn hex(digest: &[u8; 32]) -> String {
    crate::server::hex32(*digest)
}
const fn stage_rank(stage: cli::index_stop_search::RungStage) -> u8 {
    use cli::index_stop_search::RungStage;
    match stage {
        RungStage::Preparing => 0,
        RungStage::Training => 1,
        RungStage::Later => 2,
        RungStage::Institutional => 3,
        RungStage::Saved => 4,
        RungStage::Refused => 5,
    }
}

#[cfg(test)]
#[path = "indexstoplaunch_tests.rs"]
mod tests;

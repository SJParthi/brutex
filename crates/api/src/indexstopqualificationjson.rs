//! Pinned native qualification comparisons with one bounded retained reader.
//! Sorting and numerical verification run once per cold admission. A warm page
//! copies at most 256 records under four distinct immutable publication leases.
use crate::booleanevidencejson::{
    admission_projection, index_consistency_projection as consistency,
};
use axum::http::{StatusCode, Uri};
use cli::index_consistency_store::Record as Daily;
use cli::index_stop_qualification::{Fold, Reader, ReplayBounds};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const MODEL: &str = "index-stop-qualification";
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
    period: String,
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
                .ok_or("native qualification query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity" | "completion" | "kind" | "setting" | "period" | "offset" | "limit"
                )
                || !seen.insert(key)
            {
                return Err("empty, repeated or unknown native qualification query field".into());
            }
        }
        let identity = crate::candidatejson::hex_param(query, "identity")?
            .ok_or("exact native qualification identity required")?;
        let completion = crate::candidatejson::hex_param(query, "completion")?;
        let kind: String = match crate::server::param(query, "kind").as_str() {
            "" => "settings".into(),
            kind @ ("settings" | "folds" | "index-days" | "index-weeks") => kind.into(),
            _ => return Err("unknown native qualification page kind".into()),
        };
        let period = match crate::server::param(query, "period").as_str() {
            "" => "full".into(),
            p @ ("training" | "later" | "full") => p.into(),
            _ => return Err("unknown native qualification assessment period".into()),
        };
        let number = |key, default| {
            usize::try_from(crate::candidatejson::integer(query, key)?.unwrap_or(default))
                .map_err(|why| why.to_string())
        };
        let offset = number("offset", 0)?;
        let limit = number("limit", 16)?;
        let setting = crate::candidatejson::integer(query, "setting")?
            .map(usize::try_from)
            .transpose()
            .map_err(|why| why.to_string())?;
        if identity == [0; 32]
            || completion == Some([0; 32])
            || !(1..=256).contains(&limit)
            || (kind != "settings") != setting.is_some()
            || (kind != "settings" || offset > 0) && completion.is_none()
            || seen.contains("period") && !matches!(kind.as_str(), "index-days" | "index-weeks")
        {
            return Err("native qualification continuation requires exact pin, setting, period and bounded page".into());
        }
        Ok(Self {
            identity,
            completion,
            kind,
            setting,
            period,
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
        &json!({"schema_version":1,"status":"refused","model":MODEL,"rows":[],"refusal":why}),
    )
}

/// Read a bounded page from an exact native stop qualification receipt.
pub async fn index_stop_qualification_json(uri: Uri) -> Response {
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
                    "native qualification response exceeds byte limit; no prefix returned",
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
            "native qualification detail capacity is full; nothing queued",
        ),
        Err(crate::detail::RunError::Join(why)) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
struct Cached {
    root: PathBuf,
    identity: [u8; 32],
    budget: crate::detail::BooleanObservationBudget,
    replay: ReplayBounds,
    reader: Reader,
    ranked: Vec<(i64, usize)>,
    summary: Value,
}
fn render(root: &Path, asked: &Asked) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let budget = crate::detail::BooleanObservationBudget::load()?;
    let replay = replay_bounds(budget.bytes())?;
    let mut held = CACHE
        .get_or_init(|| Mutex::new(None))
        .try_lock()
        .map_err(|_| "native qualification cache is busy; no request queued")?;
    if !held.as_ref().is_some_and(|value| {
        value.root == root
            && value.identity == asked.identity
            && value.budget == budget
            && value.replay == replay
    }) {
        *held = None;
        let reader = Reader::open_with_replay_bounds(
            root,
            asked.identity,
            budget.bytes(),
            budget.bytes() / 64,
            replay,
        )
        .map_err(|why| {
                budget.context(&format!(
                    "Native qualification {} unavailable under {}: {why}; independent cold replay bootstrap work {} and split work {}, estimated numeric-memory admission {} bytes (not an RSS limit)",
                    hex(asked.identity),
                    root.display(),
                    replay.bootstrap_work,
                    replay.split_work,
                    replay.memory_bytes
                ))
            })?;
        let (ranked, summary) = reader.with_current(|| rank(&reader, budget.bytes()))?;
        *held = Some(Cached {
            root: root.to_owned(),
            identity: asked.identity,
            budget,
            replay,
            reader,
            ranked,
            summary,
        });
    }
    let cached = held
        .as_ref()
        .ok_or("native qualification cache admission absent")?;
    if let Err(why) = cached.reader.require_current() {
        // Surface the stale evidence error now; a later fresh request may admit
        // the exact immutable artifact again. Never substitute a pinned page.
        *held = None;
        return Err(why);
    }
    cached.reader.with_current(|| project(cached, asked))
}
pub(crate) fn replay_bounds(observation_bytes: u64) -> Result<ReplayBounds, String> {
    let work = crate::booleansearchjson::budget::ReplayBudget::load()?.nodes();
    Ok(ReplayBounds {
        bootstrap_work: work,
        split_work: work,
        memory_bytes: observation_bytes,
    })
}
fn rank(reader: &Reader, budget: u64) -> Result<(Vec<(i64, usize)>, Value), String> {
    let memory = u64::try_from(reader.rows().len())
        .map_err(display)?
        .checked_mul(u64::try_from(std::mem::size_of::<(i64, usize)>()).map_err(display)?)
        .ok_or("native rank extent overflow")?;
    if reader
        .admitted_bytes()
        .checked_add(memory)
        .is_none_or(|bytes| bytes > budget)
    {
        return Err("native ranking plus retained evidence exceeds read admission".into());
    }
    let mut ranked = Vec::new();
    ranked
        .try_reserve_exact(reader.rows().len())
        .map_err(display)?;
    let (mut admitted, mut rejected, mut refused, mut unmeasured, mut combined) =
        (0_u64, 0_u64, 0_u64, 0_u64, 0_u64);
    for (index, row) in reader.rows().iter().enumerate() {
        let after = reader
            .later()
            .records()
            .get(index)
            .ok_or("native ranking later coordinate absent")?;
        let daily = reader
            .consistencies()
            .get(index)
            .ok_or("native ranking day assessment absent")?;
        ranked.push((after.metrics().pessimistic_paisa, index));
        match row.projection.verdict().status() {
            cli::boolean_evidence::AdmissionStatusV1::Admitted => admitted += 1,
            cli::boolean_evidence::AdmissionStatusV1::Rejected => rejected += 1,
            cli::boolean_evidence::AdmissionStatusV1::Refused => refused += 1,
            cli::boolean_evidence::AdmissionStatusV1::Unmeasured => unmeasured += 1,
        }
        combined += u64::from(daily.combined_qualifies(row.projection.verdict().status()));
    }
    sort_ranks(&mut ranked);
    Ok((
        ranked,
        json!({"institutional_admitted":admitted.to_string(),"rejected":rejected.to_string(),"refused":refused.to_string(),"unmeasured":unmeasured.to_string(),"combined_qualified":combined.to_string()}),
    ))
}
fn sort_ranks(ranked: &mut [(i64, usize)]) {
    ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
}
fn project(cached: &Cached, asked: &Asked) -> Result<Value, String> {
    let reader = &cached.reader;
    let pin = reader.completion_digest();
    if asked.completion.is_some_and(|expected| expected != pin) {
        return Err("native qualification pin differs; no replacement page returned".into());
    }
    if matches!(asked.kind.as_str(), "index-days" | "index-weeks") {
        return days(reader, asked);
    }
    let original = json!({"identity":hex(reader.training().identity()),"completion":hex(reader.training().completion_digest())});
    let later = json!({"identity":hex(reader.later().identity()),"completion":hex(reader.later().completion_digest())});
    if asked.kind == "folds" {
        let index = asked.setting.ok_or("native fold setting missing")?;
        let row = reader
            .rows()
            .get(index)
            .ok_or("native fold setting outside saved extent")?;
        let rows = window(&row.folds, asked)?
            .iter()
            .enumerate()
            .map(|(n, row)| fold(asked.offset + n, row))
            .collect::<Vec<_>>();
        return Ok(
            json!({"schema_version":1,"status":"saved","model":MODEL,"kind":"folds","identity":hex(reader.identity()),"completion":hex(pin),"original":original,"later":later,
            "setting":index.to_string(),"total":row.folds.len().to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"rows":rows,"refusal":null}),
        );
    }
    let rows = window(&cached.ranked, asked)?
        .iter()
        .enumerate()
        .map(|(n, (_, index))| setting(reader, *index, asked.offset + n + 1))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"schema_version":1,"status":"saved","model":MODEL,"kind":"settings","identity":hex(reader.identity()),"completion":hex(pin),"search_identity":hex(reader.search_identity()),
        "batch":reader.allocation().batch().to_string(),"rung":reader.allocation().rung().to_string(),"original":original,"later":later,
        "total":reader.rows().len().to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"summary":cached.summary,"ranking":"later_pessimistic_descending",
        "policy":admission_projection::policy(&reader.policy()),"rows":rows,"refusal":null}),
    )
}
pub(crate) fn setting(reader: &Reader, index: usize, rank: usize) -> Result<Value, String> {
    let row = reader
        .rows()
        .get(index)
        .ok_or("native comparison coordinate missing")?;
    let original = reader
        .training()
        .records()
        .get(index)
        .ok_or("native original coordinate missing")?;
    let later = reader
        .later()
        .records()
        .get(index)
        .ok_or("native later coordinate missing")?;
    let record = reader
        .consistencies()
        .get(index)
        .ok_or("native daily coordinate missing")?;
    let institutional = admission_projection::row(
        index,
        &cli::boolean_evidence::AdmissionRow {
            identity: row.later_run,
            source_index: index,
            values: row.projection.values(),
            verdict: row.projection.verdict(),
        },
    )?;
    Ok(
        json!({"index":index.to_string(),"rank":rank.to_string(),"original":crate::indexstopjson::setting(index,original),"later":crate::indexstopjson::setting(index,later),"institutional":institutional,
        "index_consistency":consistency::saved_record(reader.identity(),reader.completion_digest(),index,record,reader.consistency_receipt(),row.projection.verdict().status())?}),
    )
}
fn days(reader: &Reader, asked: &Asked) -> Result<Value, String> {
    let setting = asked.setting.ok_or("native day/week setting missing")?;
    let record = reader
        .consistencies()
        .get(setting)
        .ok_or("native day/week setting outside saved extent")?;
    let (evaluation, days) = period(record, &asked.period)?;
    let (total, rows) = if asked.kind == "index-weeks" {
        (
            evaluation.weeks.len(),
            window(&evaluation.weeks, asked)?
                .iter()
                .enumerate()
                .map(|(n, row)| consistency::week(asked.offset + n, row))
                .collect::<Vec<_>>(),
        )
    } else {
        (
            days.len(),
            window(days, asked)?
                .iter()
                .enumerate()
                .map(|(n, row)| consistency::day(asked.offset + n, *row))
                .collect(),
        )
    };
    let receipt = reader.consistency_receipt();
    Ok(
        json!({"schema_version":1,"status":"saved","model":MODEL,"kind":asked.kind,"identity":hex(reader.identity()),"completion":hex(reader.completion_digest()),
        "receipt":{"identity":hex(receipt.identity),"completion":hex(receipt.completion)},"period":asked.period,"setting":setting.to_string(),"total":total.to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"rows":rows,"refusal":null}),
    )
}
fn period<'a>(
    record: &'a Daily,
    which: &str,
) -> Result<
    (
        &'a cli::index_consistency::Evaluation,
        &'a [cli::index_consistency::Session],
    ),
    String,
> {
    match which {
        "full" => Ok((&record.evaluation, &record.sessions)),
        "training" => Ok((
            &record.training,
            record
                .sessions
                .get(..record.training_session_count)
                .ok_or("native training day prefix missing")?,
        )),
        "later" => Ok((
            &record.later,
            record
                .sessions
                .get(record.training_session_count..)
                .ok_or("native later day suffix missing")?,
        )),
        _ => Err("native assessment period unknown".into()),
    }
}
fn fold(index: usize, row: &Fold) -> Value {
    json!({"index":index.to_string(),"first_day":row.first_day.to_string(),"last_day":row.last_day.to_string(),"observed_days":row.observed_days.to_string(),"trades":row.trades.to_string(),"wins":row.wins.to_string(),"pessimistic_paisa":row.pessimistic_paisa.to_string(),"refused":row.refused.to_string(),"decided":row.decided})
}
fn window<'a, T>(rows: &'a [T], asked: &Asked) -> Result<&'a [T], String> {
    if asked.offset > rows.len() {
        return Err("native qualification page outside saved extent".into());
    }
    rows.get(asked.offset..asked.offset.saturating_add(asked.limit).min(rows.len()))
        .ok_or_else(|| "native qualification page outside saved extent".into())
}
fn hex(value: [u8; 32]) -> String {
    crate::server::hex32(value)
}
fn display(value: impl std::fmt::Display) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    #[test]
    fn pins_scope_period_and_bounded_paging_are_required_before_any_read() {
        assert!(Asked::parse(&format!("identity={ID}")).is_ok());
        for extra in [
            "&offset=1",
            "&kind=folds&setting=0",
            "&setting=0",
            "&period=full",
            "&kind=settings&period=later",
            "&kind=trades",
            "&limit=0",
            "&limit=257",
            "&limit=01",
            "&offset=-1",
            "&offset=18446744073709551616",
            "&kind=settings&kind=settings",
            "&alien=1",
        ] {
            assert!(
                Asked::parse(&format!("identity={ID}{extra}")).is_err(),
                "{extra}"
            );
        }
        for kind in ["folds", "index-days", "index-weeks"] {
            assert!(
                Asked::parse(&format!(
                    "identity={ID}&completion={ID}&kind={kind}&setting=0&limit=256"
                ))
                .is_ok()
            );
        }
        for p in ["training", "later", "full"] {
            assert!(
                Asked::parse(&format!(
                    "identity={ID}&completion={ID}&kind=index-days&setting=0&period={p}"
                ))
                .is_ok()
            );
        }
    }
    #[test]
    fn pessimistic_ranking_is_total_signed_and_stable_at_ties() {
        let mut rows = [
            (i64::MIN, 3),
            (0, 9),
            (100, 7),
            (100, 2),
            (i64::MAX, 8),
            (-1, 1),
        ];
        sort_ranks(&mut rows);
        assert_eq!(
            rows,
            [
                (i64::MAX, 8),
                (100, 2),
                (100, 7),
                (0, 9),
                (-1, 1),
                (i64::MIN, 3)
            ]
        );
    }
    #[test]
    fn fold_projection_preserves_all_integer_widths_and_zero_trade_decisions() {
        let value = fold(
            usize::MAX,
            &Fold {
                first_day: i64::MIN,
                last_day: i64::MAX,
                observed_days: u64::MAX,
                trades: 0,
                wins: 0,
                pessimistic_paisa: i64::MIN,
                refused: u64::MAX,
                decided: false,
            },
        );
        assert_eq!(
            value.get("pessimistic_paisa"),
            Some(&json!(i64::MIN.to_string()))
        );
        assert_eq!(
            value.get("observed_days"),
            Some(&json!(u64::MAX.to_string()))
        );
        assert_eq!(value.get("decided"), Some(&json!(false)));
    }
    #[test]
    fn an_outside_page_is_not_a_silent_empty_success() {
        let asked = Asked {
            identity: [1; 32],
            completion: Some([2; 32]),
            kind: "settings".into(),
            setting: None,
            period: "full".into(),
            offset: 3,
            limit: 256,
        };
        assert!(window(&[1, 2], &asked).is_err());
        let asked = Asked { offset: 2, ..asked };
        assert_eq!(window(&[1, 2], &asked), Ok([].as_slice()));
    }
    #[test]
    fn joined_loss_streak_cannot_disappear_between_passing_period_pages() -> Result<(), String> {
        use cli::index_consistency::{Eligibility, Outcome, Policy, Session, evaluate};
        let first = i64::from(
            pull::session::Day::new(2025, 1, 6)
                .map_err(display)?
                .days_from_epoch(),
        );
        let returns = [
            1, 1, 1, -1, -1, 1, 1, 1, -1, -1, // independently passing training
            -1, 1, 1, -1, 1, 1, 1, 1, -1, -1, // independently passing later
        ];
        let sessions: Vec<_> = (first..first + 26)
            .filter(|day| cli::index_consistency::eligibility_of(*day) == Eligibility::Eligible)
            .zip(returns)
            .map(|(day, pessimistic_paisa)| Session {
                day,
                pessimistic_paisa,
                trades: 1,
            })
            .collect();
        assert_eq!(sessions.len(), returns.len());
        let (training_days, later_days) = sessions
            .split_at_checked(10)
            .ok_or("fixture period boundary missing")?;
        let family = cli::index_stop_store::ResearchFamilyV1::new(
            brutex_core::instrument::InstrumentKey::index(
                brutex_core::instrument::Exchange::Nse,
                "NIFTY",
            )
            .map_err(display)?,
        )
        .map_err(display)?;
        let measure = |days: &[Session]| -> Result<_, String> {
            Ok(evaluate(
                Policy::V1,
                family,
                days.first().ok_or("fixture first session absent")?.day,
                days.last().ok_or("fixture last session absent")?.day,
                days,
                true,
            ))
        };
        let record = Daily {
            binding: cli::index_consistency_store::Binding {
                original: [1; 32],
                later: [2; 32],
                later_identity: [3; 32],
                later_completion: [4; 32],
                source: [5; 32],
                family: 0,
                coordinate: 0,
            },
            evaluation: measure(&sessions)?,
            training: measure(training_days)?,
            later: measure(later_days)?,
            training_session_count: training_days.len(),
            sessions: sessions.clone(),
        };
        assert_eq!(record.training.outcome, Outcome::Passed);
        assert_eq!(record.later.outcome, Outcome::Passed);
        assert_eq!(record.evaluation.outcome, Outcome::Failed);
        assert_eq!(record.evaluation.summary.longest_losing_streak, 3);
        for (name, wanted) in [("training", training_days), ("later", later_days)] {
            let (evaluation, actual) = period(&record, name)?;
            assert_eq!(actual, wanted);
            assert_eq!(evaluation.summary.observed_days, 10);
        }
        let value = consistency::saved_record(
            [6; 32],
            [7; 32],
            0,
            &record,
            cli::index_consistency_store::Receipt {
                identity: [8; 32],
                completion: [9; 32],
            },
            cli::boolean_evidence::AdmissionStatusV1::Admitted,
        )?;
        assert_eq!(value.get("combined_qualifies"), Some(&json!(false)));
        assert_eq!(value.get("state"), Some(&json!("failed")));
        assert_eq!(
            value.pointer("/periods/training/days_count"),
            Some(&json!("10"))
        );
        assert_eq!(
            value.pointer("/periods/later/days_count"),
            Some(&json!("10"))
        );
        assert_eq!(value.get("days_count"), Some(&json!("20")));
        Ok(())
    }
}

//! Bounded exact priced-candidate pages pinned to a sealed capture catalog.

use cli::candidate_trades::{self, Candidate, Key, Model, Summary, Tier, TradeReader};
use cli::trades::Direction;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type Response = (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
);

struct Asked {
    model: Model,
    identity: [u8; 32],
    attempt: u64,
    digest: Option<[u8; 32]>,
    tier: u64,
    rank: Option<u64>,
    direction: Option<Direction>,
    offset: u64,
    limit: usize,
}
impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or("candidate query requires key=value")?;
            if value.is_empty()
                || !matches!(
                    key,
                    "identity"
                        | "model"
                        | "attempt"
                        | "digest"
                        | "tier"
                        | "rank"
                        | "direction"
                        | "offset"
                        | "limit"
                )
                || !seen.insert(key)
            {
                return Err(format!(
                    "unknown, repeated or empty candidate query field {key:?}"
                ));
            }
        }
        let identity = hex_param(query, "identity")?.ok_or("candidate identity is required")?;
        let model = match crate::server::param(query, "model").as_str() {
            "" | "and-mask" => Model::And,
            "expression" => Model::Expression,
            _ => return Err("candidate model must be and-mask or expression".to_owned()),
        };
        let attempt = integer(query, "attempt")?
            .filter(|value| *value > 0)
            .ok_or("positive candidate attempt is required")?;
        let digest = hex_param(query, "digest")?;
        let tier = integer(query, "tier")?.unwrap_or(0);
        let rank = integer(query, "rank")?;
        let direction = match crate::server::param(query, "direction").as_str() {
            "" => None,
            "long" => Some(Direction::Long),
            "short" => Some(Direction::Short),
            _ => return Err("candidate direction must be long or short".to_owned()),
        };
        let offset = integer(query, "offset")?.unwrap_or(0);
        let limit = integer(query, "limit")?.unwrap_or(crate::detail::MAX_PAGE_ROWS);
        if limit == 0 || limit > crate::detail::MAX_PAGE_ROWS {
            return Err("candidate page limit must be 1..=256".to_owned());
        }
        if rank.is_some() != direction.is_some() || rank == Some(0) {
            return Err(
                "candidate trade pages require both positive rank and direction".to_owned(),
            );
        }
        if (offset > 0 || tier > 0 || rank.is_some()) && digest.is_none() {
            return Err(
                "candidate continuation and trade pages require the initial catalog digest"
                    .to_owned(),
            );
        }
        Ok(Self {
            model,
            identity,
            attempt,
            digest,
            tier,
            rank,
            direction,
            offset,
            limit: usize::try_from(limit).map_err(|why| why.to_string())?,
        })
    }
}
pub(crate) fn integer(query: &str, name: &str) -> Result<Option<u64>, String> {
    let raw = crate::server::param(query, name);
    if raw.is_empty() {
        return Ok(None);
    }
    let value = raw
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a canonical unsigned integer"))?;
    if raw != value.to_string() {
        return Err(format!("{name} must be a canonical unsigned integer"));
    }
    Ok(Some(value))
}
pub(crate) fn hex_param(query: &str, name: &str) -> Result<Option<[u8; 32]>, String> {
    let raw = crate::server::param(query, name);
    if raw.is_empty() {
        return Ok(None);
    }
    let value = crate::trades::from_hex_public(&raw)
        .ok_or("candidate identity/digest must be 64 lowercase hex characters")?;
    if raw != crate::server::hex32(value) {
        return Err("candidate identity/digest must use canonical lowercase hex".to_owned());
    }
    Ok(Some(value))
}

/// Serves one bounded candidate manifest or exact trade page outside async workers.
pub async fn candidate_json(uri: axum::http::Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(value) => value,
        Err(why) => return refusal(axum::http::StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || match root.and_then(|root| render(&root, &asked)) {
        Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
            (axum::http::StatusCode::OK, headers(), body)
        }
        Ok(_) => refusal(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "candidate JSON exceeds its response bound; no prefix exposed",
        ),
        Err(why) => refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why),
    })
    .await
    {
        Ok(response) => response,
        Err(crate::detail::RunError::Saturated) => refusal(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "candidate detail capacity is full; no work was queued",
        ),
        Err(crate::detail::RunError::Join(why)) => {
            refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why)
        }
    }
}
fn headers() -> [(axum::http::header::HeaderName, &'static str); 1] {
    [(axum::http::header::CONTENT_TYPE, "application/json")]
}
fn refusal(status: axum::http::StatusCode, why: &str) -> Response {
    (
        status,
        headers(),
        json!({"schema_version":1,"status":"refused","refusal":why,"rows":[]}).to_string(),
    )
}
fn render(root: &Path, asked: &Asked) -> Result<String, String> {
    let Some(summary) = candidate_trades::read_model(
        root,
        asked.identity,
        asked.attempt,
        asked.model,
        crate::detail::MAX_SCAN_BYTES,
    )?
    else {
        if asked.digest.is_some() {
            return Err(
                "the pinned candidate catalog is absent; no replacement page exposed".to_owned(),
            );
        }
        return Ok(json!({"schema_version":1,"status":"missing","model":asked.model.as_str(),"identity":crate::server::hex32(asked.identity),"attempt":asked.attempt.to_string(),"rows":[],"refusal":null,"why":"No sealed priced-candidate capture is saved for this attempt. An absent or unfinished capture is not a measured zero."}).to_string());
    };
    if asked.digest.is_some_and(|digest| digest != summary.digest) {
        return Err("candidate catalog changed; restart from its first page".to_owned());
    }
    if summary.tiers == 0 {
        return empty_catalog(&summary, asked);
    }
    let tier = candidate_trades::tier(root, &summary, asked.tier, crate::detail::MAX_SCAN_BYTES)?;
    let (kind, total, rows, selected) = match (asked.rank, asked.direction) {
        (Some(rank), Some(direction)) => {
            let key = Key {
                tier: asked.tier,
                rank,
                direction,
            };
            let (candidate, rows) = trade_page(root, &summary, key, asked.offset, asked.limit)?;
            let total = candidate.cell.map_or(0, |cell| cell.trades);
            (
                "trades",
                total,
                rows.into_iter().map(trade_json).collect::<Vec<_>>(),
                Some(candidate_json_value(&candidate)),
            )
        }
        (None, None) => {
            let total = tier
                .evaluated
                .checked_mul(2)
                .ok_or("candidate side count overflow")?;
            let rows = candidate_trades::candidates_page(
                root,
                &summary,
                asked.tier,
                asked.offset,
                asked.limit,
                crate::detail::MAX_SCAN_BYTES,
            )?;
            (
                "candidates",
                total,
                rows.iter().map(candidate_json_value).collect(),
                None,
            )
        }
        _ => return Err("candidate key is incomplete".to_owned()),
    };
    validate_window(total, asked.offset, rows.len())?;
    let observed = candidate_trades::read_model(
        root,
        asked.identity,
        asked.attempt,
        asked.model,
        crate::detail::MAX_SCAN_BYTES,
    )?
    .ok_or("candidate catalog disappeared during read")?;
    if observed.digest != summary.digest {
        return Err("candidate catalog changed during read; no mixed page exposed".to_owned());
    }
    let lifecycle = cli::sweep_evidence::read_attempt(
        root,
        asked.identity,
        asked.attempt,
        crate::detail::MAX_SCAN_BYTES,
    )?;
    let next = asked
        .offset
        .checked_add(rows.len() as u64)
        .filter(|end| *end < total)
        .map(|value| value.to_string());
    Ok(json!({"schema_version":1,"status":"saved","model":summary.model.as_str(),"expression":expression_json(summary.expression()),"identity":crate::server::hex32(summary.identity),"attempt":summary.attempt.to_string(),"catalog_digest":crate::server::hex32(summary.digest),"execution_digest":crate::server::hex32(summary.execution_digest),"capture":"sealed-pricing-evidence","audit_completion":lifecycle.map(|value|value.completion.as_str()),"kind":kind,"tier_count":summary.tiers.to_string(),"candidate_side_count":summary.candidates.to_string(),"tier":tier_json(&tier),"offset":asked.offset.to_string(),"limit":asked.limit,"total_count":total.to_string(),"next_offset":next,"page_complete":true,"selected":selected,"rows":rows,"refusal":null}).to_string())
}
fn empty_catalog(summary: &Summary, asked: &Asked) -> Result<String, String> {
    if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
        return Err("empty candidate catalog has only its initial metadata page".to_owned());
    }
    Ok(json!({"schema_version":1,"status":"saved","model":summary.model.as_str(),"expression":expression_json(summary.expression()),"identity":crate::server::hex32(summary.identity),"attempt":summary.attempt.to_string(),"catalog_digest":crate::server::hex32(summary.digest),"execution_digest":crate::server::hex32(summary.execution_digest),"capture":"sealed-pricing-evidence","audit_completion":null,"kind":"candidates","tier_count":"0","candidate_side_count":"0","tier":null,"offset":"0","limit":asked.limit,"total_count":"0","next_offset":null,"page_complete":true,"selected":null,"rows":[],"refusal":null}).to_string())
}
fn validate_window(total: u64, offset: u64, count: usize) -> Result<(), String> {
    if (total == 0 && offset != 0)
        || (total > 0 && offset >= total)
        || offset
            .checked_add(count as u64)
            .is_none_or(|end| end > total)
    {
        return Err("candidate page is outside its exact recorded extent".to_owned());
    }
    Ok(())
}
struct Cached {
    model: Model,
    root: PathBuf,
    identity: [u8; 32],
    attempt: u64,
    digest: [u8; 32],
    key: Key,
    reader: TradeReader,
}
fn trade_page(
    root: &Path,
    summary: &Summary,
    key: Key,
    offset: u64,
    limit: usize,
) -> Result<(Candidate, Vec<cli::trades::Row>), String> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    let mut cached = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "candidate trade reader cache poisoned")?;
    if cached.as_ref().is_none_or(|held| {
        held.model != summary.model
            || held.root != root
            || held.identity != summary.identity
            || held.attempt != summary.attempt
            || held.digest != summary.digest
            || held.key != key
    }) {
        let reader = TradeReader::open(root, summary, key, crate::detail::MAX_SCAN_BYTES)?;
        *cached = Some(Cached {
            model: summary.model,
            root: root.to_path_buf(),
            identity: summary.identity,
            attempt: summary.attempt,
            digest: summary.digest,
            key,
            reader,
        });
    }
    let held = cached.as_mut().ok_or("candidate reader cache missing")?;
    let rows = held.reader.page(offset, limit)?;
    Ok((held.reader.candidate().clone(), rows))
}
fn tier_json(tier: &Tier) -> Value {
    let rules = tier.rules;
    json!({"index":tier.index.to_string(),"eligible":tier.eligible.to_string(),"evaluated":tier.evaluated.to_string(),"horizon":tier.horizon.to_string(),"rungs":tier.rungs.to_string(),"step_ppm":tier.step_ppm.map(|v|v.to_string()),"forced_ppm":tier.forced_ppm.map(|v|v.to_string()),"ratios":tier.ratios,"stops_ppm":tier.stops_ppm.iter().map(ToString::to_string).collect::<Vec<_>>(),"rules":{"max_mae_ppm":rules.max_mae_ppm.to_string(),"min_rr_bp":rules.min_rr_bp.to_string(),"min_win_rate_bp":rules.min_win_rate_bp.to_string(),"min_trades":rules.min_trades.to_string(),"min_assurance_bp":rules.min_assurance_bp.to_string(),"min_weakest_bp":rules.min_weakest_bp.to_string(),"min_ret_over_dd_bp":rules.min_ret_over_dd_bp.to_string(),"require_protective_exits":rules.require_protective_exits,"min_fill_headroom_bp":rules.min_fill_headroom_bp.to_string(),"min_avg_rr_bp":rules.min_avg_rr_bp.to_string(),"top":rules.top.to_string()}})
}

fn expression_json(expression: Option<&vocab::expression::Expression>) -> Value {
    use std::fmt::Write as _;
    let Some(expression) = expression else {
        return Value::Null;
    };
    let encoded = expression.encode();
    let mut hex = String::with_capacity(encoded.len().saturating_mul(2));
    for byte in encoded {
        let _ = write!(hex, "{byte:02x}");
    }
    json!({"version":vocab::expression::VERSION,"encoded_hex":hex,"source":expression.to_string(),"referenced_mask_words":expression.referenced().words().map(|word|word.to_string())})
}
fn candidate_json_value(candidate: &Candidate) -> Value {
    let cell=candidate.cell.map(|cell|json!({"stop":cell.stop.map(|v|v.to_string()),"target":cell.target.map(|v|v.to_string()),"tsl":cell.tsl.map(|v|v.to_string()),"ttp":cell.ttp.map(|v|json!({"arm":v.arm.to_string(),"trail":v.trail.to_string()})),"trades":cell.trades.to_string(),"wins":cell.wins.to_string(),"pessimistic":cell.pessimistic.to_string(),"optimistic":cell.optimistic.to_string(),"max_drawdown":cell.max_drawdown.to_string(),"worst_trade":cell.worst_trade.to_string()}));
    json!({"tier":candidate.key.tier.to_string(),"rank":candidate.key.rank.to_string(),"direction":candidate.key.direction.as_str(),"mask_words":candidate.mask_words.map(|v|v.to_string()),"trade_identity":crate::server::hex32(candidate.trade_identity),"cell_rules_pass":candidate.admitted,"cell":cell,"signals":candidate.signals.to_string(),"refused_paths":candidate.refused_paths.to_string(),"stops_ppm":candidate.stops.iter().map(ToString::to_string).collect::<Vec<_>>(),"targets_ppm":candidate.targets.iter().map(ToString::to_string).collect::<Vec<_>>(),"trails_ppm":candidate.trails.iter().map(ToString::to_string).collect::<Vec<_>>()})
}
fn trade_json(row: cli::trades::Row) -> Value {
    json!({"identity":crate::server::hex32(row.identity),"seq":row.seq.to_string(),"direction":row.direction.as_str(),"signal_bar":row.signal_bar.to_string(),"entry_bar":row.entry_bar.to_string(),"exit_bar":row.exit_bar.to_string(),"best":row.best.to_string(),"worst":row.worst.to_string(),"entry_micros":row.entry_micros.to_string(),"exit_micros":row.exit_micros.to_string(),"adverse_ppm":row.adverse_ppm.to_string(),"adverse_paisa":row.adverse_paisa.to_string(),"favourable_ppm":row.favourable_ppm.to_string(),"favourable_paisa":row.favourable_paisa.to_string()})
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "4242424242424242424242424242424242424242424242424242424242424242";
    #[test]
    fn exact_query_bounds_and_continuation_authority_are_required() {
        let first = format!("identity={ID}&attempt=9007199254740993&limit=256");
        assert!(Asked::parse(&first).is_ok());
        for suffix in [
            "&offset=1",
            "&tier=1",
            "&rank=1&direction=short",
            "&identity=aa",
            "&unknown=1",
            "&offset=01",
            "&limit=0",
            "&limit=257",
        ] {
            assert!(
                Asked::parse(&format!("{first}{suffix}")).is_err(),
                "{suffix}"
            );
        }
        let next = format!("{first}&digest={ID}&tier=999&offset=999999");
        assert!(
            Asked::parse(&next).is_ok(),
            "large direct offsets allocate no unbounded result"
        );
        assert!(Asked::parse(&"x".repeat(crate::detail::MAX_QUERY_BYTES + 1)).is_err());
    }
    #[test]
    fn missing_capture_is_not_a_completed_empty_run_or_automatic_replacement() -> Result<(), String>
    {
        let root =
            std::env::temp_dir().join(format!("candidate-api-missing-{}", std::process::id()));
        let asked = Asked::parse(&format!("identity={ID}&attempt=1"))?;
        let response = render(&root, &asked)?;
        assert!(response.contains("\"status\":\"missing\""));
        assert!(!response.contains("\"capture\":\"sealed"));
        let pinned = Asked::parse(&format!("identity={ID}&attempt=1&digest={ID}"))?;
        assert!(render(&root, &pinned).is_err());
        Ok(())
    }
    #[test]
    fn page_ranges_and_large_exact_integer_rows_are_not_rounded() {
        assert!(validate_window(257, 256, 1).is_ok());
        assert!(validate_window(257, 257, 0).is_err());
        assert!(validate_window(0, 1, 0).is_err());
        assert!(validate_window(u64::MAX, u64::MAX - 1, 2).is_err());
        let row = cli::trades::Row {
            identity: [7; 32],
            seq: 1,
            direction: Direction::Short,
            signal_bar: 9_007_199_254_740_993,
            entry_bar: 9_007_199_254_740_994,
            exit_bar: 9_007_199_254_740_995,
            best: i64::MAX,
            worst: i64::MIN,
            entry_micros: 1,
            exit_micros: 2,
            adverse_ppm: 3,
            adverse_paisa: 4,
            favourable_ppm: 5,
            favourable_paisa: 6,
        };
        let value = trade_json(row);
        assert_eq!(value.get("signal_bar"), Some(&json!("9007199254740993")));
        assert_eq!(value.get("worst"), Some(&json!(i64::MIN.to_string())));
    }

    #[test]
    fn expression_namespace_returns_its_complete_predicate_without_and_fallback()
    -> Result<(), String> {
        let root =
            std::env::temp_dir().join(format!("candidate-api-expression-{}", std::process::id()));
        let attempt = cli::sweep_evidence::begin(
            &root,
            [42; 32],
            cli::sweep_evidence::Operation::Expression,
        )?;
        let expression =
            vocab::expression::Expression::parse("0 | !1").map_err(|why| format!("{why:?}"))?;
        #[expect(
            clippy::default_trait_access,
            reason = "the public capture API infers Column without adding an api-to-indicators graph edge"
        )]
        let column = Default::default();
        let capture = candidate_trades::Capture::begin_expression(
            &root,
            &attempt,
            &[],
            &column,
            &expression,
        )?;
        capture.finish()?;
        let base = format!(
            "identity={}&attempt={}",
            crate::server::hex32([42; 32]),
            attempt.token()
        );
        let and = render(&root, &Asked::parse(&base)?)?;
        assert!(and.contains("\"status\":\"missing\""));
        let saved: Value = serde_json::from_str(&render(
            &root,
            &Asked::parse(&format!("{base}&model=expression"))?,
        )?)
        .map_err(|why| why.to_string())?;
        assert_eq!(saved.get("model"), Some(&json!("expression")));
        assert_eq!(saved.get("status"), Some(&json!("saved")));
        assert_eq!(
            saved
                .get("expression")
                .and_then(|value| value.get("source")),
            Some(&json!(expression.to_string()))
        );
        assert_eq!(
            saved
                .get("expression")
                .and_then(|value| value.get("encoded_hex"))
                .and_then(Value::as_str)
                .map(str::len),
            Some(vocab::expression::ENCODED_LEN * 2)
        );
        assert!(Asked::parse(&format!("{base}&model=and-or")).is_err());
        Ok(())
    }

    #[test]
    fn saved_public_capture_pages_pin_all_sides_and_refuse_lost_children() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!("candidate-api-saved-{}", std::process::id()));
        let attempt =
            cli::sweep_evidence::begin(&root, [42; 32], cli::sweep_evidence::Operation::Audit)?;
        #[expect(
            clippy::default_trait_access,
            reason = "the public capture API infers Column without adding an api-to-indicators graph edge"
        )]
        let column = Default::default();
        let capture = candidate_trades::Capture::begin(&root, &attempt, &[], &column)?;
        let tier = capture.tier(no_cell_tier())?;
        #[expect(
            clippy::default_trait_access,
            reason = "the public capture API infers Grid without adding an api-to-runner graph edge"
        )]
        let grid = Default::default();
        let mask = vocab::ConditionMask::default();
        for rank in 1..=129 {
            for direction in [Direction::Long, Direction::Short] {
                capture.record(
                    &tier,
                    &candidate_trades::Evaluated {
                        rank,
                        mask: &mask,
                        direction,
                        grid: &grid,
                        selected: None,
                    },
                )?;
            }
        }
        let summary = capture.finish()?;
        let base = format!(
            "identity={}&attempt={}",
            crate::server::hex32([42; 32]),
            attempt.token()
        );
        let asked = Asked::parse(&base)?;
        let first: Value =
            serde_json::from_str(&render(&root, &asked)?).map_err(|why| why.to_string())?;
        assert_eq!(first.get("next_offset"), Some(&json!("256")));
        assert_eq!(
            first.get("audit_completion"),
            Some(&json!("running")),
            "sealed pricing is not whole-audit completion"
        );
        assert_eq!(
            first.get("rows").and_then(Value::as_array).map(Vec::len),
            Some(256)
        );
        let query = format!(
            "{base}&digest={}&offset=256",
            crate::server::hex32(summary.digest)
        );
        let asked = Asked::parse(&query)?;
        let second: Value =
            serde_json::from_str(&render(&root, &asked)?).map_err(|why| why.to_string())?;
        assert_eq!(
            second.get("rows").and_then(Value::as_array).map(Vec::len),
            Some(2)
        );
        assert_eq!(second.get("next_offset"), Some(&Value::Null));
        let query = format!(
            "{base}&digest={}&rank=129&direction=short",
            crate::server::hex32(summary.digest)
        );
        let exact = Asked::parse(&query)?;
        let empty: Value =
            serde_json::from_str(&render(&root, &exact)?).map_err(|why| why.to_string())?;
        assert_eq!(
            empty.get("selected").and_then(|value| value.get("cell")),
            Some(&Value::Null)
        );
        assert_eq!(empty.get("total_count"), Some(&json!("0")));
        let child = root
            .join("results/candidate-trades-v1")
            .join(crate::server::hex32([42; 32]))
            .join(attempt.token().to_string())
            .join("0-129-1-candidate.bin");
        std::fs::remove_file(child).map_err(|why| why.to_string())?;
        assert!(
            render(&root, &asked).is_err(),
            "no shortened replacement page"
        );
        Ok(())
    }
    fn no_cell_tier() -> Tier {
        Tier {
            index: 0,
            eligible: 129,
            evaluated: 129,
            horizon: 1,
            rungs: 1,
            step_ppm: None,
            forced_ppm: None,
            ratios: false,
            stops_ppm: Vec::new(),
            rules: cli::Rules {
                max_mae_ppm: 0,
                min_rr_bp: 0,
                min_win_rate_bp: 0,
                min_trades: 0,
                min_assurance_bp: 0,
                min_weakest_bp: 0,
                min_ret_over_dd_bp: 0,
                require_protective_exits: false,
                min_fill_headroom_bp: 0,
                min_avg_rr_bp: 0,
                top: 1,
            },
        }
    }
}

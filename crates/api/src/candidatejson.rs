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
/// The one catalog summary held across requests, and what it was read for.
struct HeldSummary {
    model: Model,
    root: PathBuf,
    identity: [u8; 32],
    attempt: u64,
    summary: std::sync::Arc<Summary>,
}

/// The process's held summary. One slot, like [`trade_page`]'s reader.
static SUMMARY: Mutex<Option<HeldSummary>> = Mutex::new(None);

#[cfg(test)]
thread_local! {
    /// Cold catalog reads [`summary_for`] made on this thread.
    static COLD_SUMMARIES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The sealed catalog's summary for `asked`, read cold only when the held one
/// is for another capture or no longer names the file on disk.
///
/// # Cost, and what invalidates the held summary
///
/// Warm: [`candidate_trades::require_unchanged`], one shared-lock open, a
/// generation comparison and the start-descriptor check, O(1) in the catalog.
/// Cold: [`candidate_trades::read_model`], which reads and hashes the whole
/// catalog, O(catalog bytes) bounded by `MAX_SCAN_BYTES`. The held summary is
/// dropped and re-read when the key (model, root, identity, attempt) differs,
/// or when the generation check refuses, which any rewrite, replacement or
/// truncation of `catalog.bin` causes (D-0991). An absent catalog is never
/// held. One slot: two clients alternating captures make every request cold.
/// D-2283 (W1-api2-2).
fn summary_for(
    slot: &Mutex<Option<HeldSummary>>,
    root: &Path,
    asked: &Asked,
) -> Result<Option<std::sync::Arc<Summary>>, String> {
    let mut held = slot
        .lock()
        .map_err(|_| "candidate summary cache poisoned")?;
    if let Some(found) = held.as_ref().filter(|found| {
        found.model == asked.model
            && found.root == root
            && found.identity == asked.identity
            && found.attempt == asked.attempt
    }) && candidate_trades::require_unchanged(
        root,
        &found.summary,
        crate::detail::MAX_SCAN_BYTES,
    )
    .is_ok()
    {
        return Ok(Some(std::sync::Arc::clone(&found.summary)));
    }
    #[cfg(test)]
    COLD_SUMMARIES.with(|count| count.set(count.get() + 1));
    let Some(summary) = candidate_trades::read_model(
        root,
        asked.identity,
        asked.attempt,
        asked.model,
        crate::detail::MAX_SCAN_BYTES,
    )?
    else {
        return Ok(None);
    };
    let summary = std::sync::Arc::new(summary);
    *held = Some(HeldSummary {
        model: asked.model,
        root: root.to_path_buf(),
        identity: asked.identity,
        attempt: asked.attempt,
        summary: std::sync::Arc::clone(&summary),
    });
    Ok(Some(summary))
}

fn render(root: &Path, asked: &Asked) -> Result<String, String> {
    render_with(&SUMMARY, root, asked)
}

fn render_with(
    slot: &Mutex<Option<HeldSummary>>,
    root: &Path,
    asked: &Asked,
) -> Result<String, String> {
    let Some(summary) = summary_for(slot, root, asked)? else {
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
    // A STOCK AUDIT'S PRICED CANDIDATES SAY WHAT THEY ARE MADE OF. Each page
    // serves per-trade best, worst, adverse and favourable figures, and a
    // RELIANCE audit's were served with neither the gross label nor the
    // corporate-action sentence. An AND-mask capture is keyed by the audit's
    // run identity, the one the results ledger records, so its row names the
    // instrument, as it does for `/sweep-evidence.json`. An expression
    // capture comes from the expression search, keyed by `candidate_identity`,
    // which no ledger row carries; it is not looked up. D-0694, AF-19.
    let note = match summary.model {
        Model::And => crate::detail::recorded_underlying(root, &summary.identity)?
            .map_or_else(String::new, |underlying| cli::equity_note_for(&underlying)),
        Model::Expression => String::new(),
    };
    if summary.tiers == 0 {
        return empty_catalog(&summary, asked, note);
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
    // THE CLOSING CHECK IS THE GENERATION, NOT A SECOND WHOLE READ. This re-read
    // and re-hashed the whole catalog to compare digests; the generation check
    // is the one `tier` and `candidates_page` already trust between their own
    // reads, and any rewrite of `catalog.bin` moves it. D-2283 (W1-api2-2).
    candidate_trades::require_unchanged(root, &summary, crate::detail::MAX_SCAN_BYTES).map_err(
        |why| format!("candidate catalog changed during read; no mixed page exposed: {why}"),
    )?;
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
    let mut body = json!({"schema_version":1,"status":"saved","model":summary.model.as_str(),"expression":expression_json(summary.expression()),"identity":crate::server::hex32(summary.identity),"attempt":summary.attempt.to_string(),"catalog_digest":crate::server::hex32(summary.digest),"execution_digest":crate::server::hex32(summary.execution_digest),"capture":"sealed-pricing-evidence","audit_completion":lifecycle.map(|value|value.completion.as_str()),"kind":kind,"tier_count":summary.tiers.to_string(),"candidate_side_count":summary.candidates.to_string(),"tier":tier_json(&tier),"offset":asked.offset.to_string(),"limit":asked.limit,"total_count":total.to_string(),"next_offset":next,"page_complete":true,"selected":selected,"rows":rows,"refusal":null});
    crate::detail::put_equity_note(&mut body, note)?;
    Ok(body.to_string())
}
fn empty_catalog(summary: &Summary, asked: &Asked, note: String) -> Result<String, String> {
    if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
        return Err("empty candidate catalog has only its initial metadata page".to_owned());
    }
    let mut body = json!({"schema_version":1,"status":"saved","model":summary.model.as_str(),"expression":expression_json(summary.expression()),"identity":crate::server::hex32(summary.identity),"attempt":summary.attempt.to_string(),"catalog_digest":crate::server::hex32(summary.digest),"execution_digest":crate::server::hex32(summary.execution_digest),"capture":"sealed-pricing-evidence","audit_completion":null,"kind":"candidates","tier_count":"0","candidate_side_count":"0","tier":null,"offset":"0","limit":asked.limit,"total_count":"0","next_offset":null,"page_complete":true,"selected":null,"rows":[],"refusal":null});
    crate::detail::put_equity_note(&mut body, note)?;
    Ok(body.to_string())
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
/// How many candidate trade readers the page keeps open at once.
///
/// One slot meant that a reader switching between two candidates re-read and
/// re-verified every trade of each on every switch (W1-api2-3). Eight lets an
/// operator compare a tier's leading candidates, both sides of one rank, or
/// the same rank across a handful of captures without a cold open. A kept
/// reader holds its trade file's open handle and that candidate's record, not
/// its rows. The ninth distinct candidate evicts the least recently paged
/// one. D-4434.
pub(crate) const TRADE_READERS_KEPT: usize = 8;

struct Cached {
    model: Model,
    root: PathBuf,
    identity: [u8; 32],
    attempt: u64,
    digest: [u8; 32],
    key: Key,
    reader: TradeReader,
}
impl Cached {
    /// Whether this reader is the one `summary`'s candidate `key` names.
    fn serves(&self, root: &Path, summary: &Summary, key: Key) -> bool {
        self.model == summary.model
            && self.root == root
            && self.identity == summary.identity
            && self.attempt == summary.attempt
            && self.digest == summary.digest
            && self.key == key
    }
    /// Whether `other` is a reader for this one's candidate. D-4655.
    fn same_as(&self, other: &Self) -> bool {
        self.model == other.model
            && self.root == other.root
            && self.identity == other.identity
            && self.attempt == other.attempt
            && self.digest == other.digest
            && self.key == other.key
    }
}
#[cfg(test)]
thread_local! {
    /// Cold trade-reader opens [`page_through`] made on this thread.
    static COLD_TRADE_READERS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
fn trade_page(
    root: &Path,
    summary: &Summary,
    key: Key,
    offset: u64,
    limit: usize,
) -> Result<(Candidate, Vec<cli::trades::Row>), String> {
    static CACHE: OnceLock<Mutex<Vec<Cached>>> = OnceLock::new();
    // THE READER IS TAKEN OUT, AND THE LOCK RELEASED, BEFORE ANY FILE IS READ
    // (cand-2, D-2576). The guard used to be held across the page, whose cold
    // `TradeReader::open` reads and verifies every trade of a candidate up to
    // `MAX_SCAN_BYTES`. Every other trade-page request then blocked on this
    // mutex INSIDE `detail::run`, each holding one of the four detail permits,
    // so one cold open answered 429 to every other detail route.
    //
    // EIGHT READERS ARE KEPT, NOT ONE (W1-api2-3, D-4434), and the two fixes
    // hold together (D-4655): the mutex is taken once to take the serving
    // reader out and once to put it back, each a scan of at most
    // `TRADE_READERS_KEPT` keys, and never across an open or a page read. A
    // second request for a reader already out opens its own; whichever
    // finishes last is the one kept (`keep`). A reader that refused stays out
    // (D-2763). A poisoned cache holds plain readers and no invariant spans
    // the panic, and the shipped binary aborts on panic regardless, so it is
    // used as found, as `detail::Checkout` does.
    let slot = CACHE.get_or_init(|| Mutex::new(Vec::with_capacity(TRADE_READERS_KEPT)));
    let lock = || {
        slot.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    let held = take_serving(&mut lock(), root, summary, key);
    #[cfg(test)]
    crate::detail::note_slot_free(slot);
    let (held, page) = serve(held, root, summary, key, offset, limit)?;
    keep(&mut lock(), held);
    Ok(page)
}
/// One page through the kept readers, opening one when none serves `key`:
/// [`trade_page`]'s three steps over one `Vec` with no lock between them, for
/// the tests that drive a cache of their own (D-4655).
///
/// The readers are held least recently paged first, so a hit moves to the
/// end and a miss with every slot full evicts the front. Finding the reader
/// compares at most [`TRADE_READERS_KEPT`] keys, a constant; the cold open of
/// a reader no slot holds is `O(trades of that candidate)` and is what the
/// slots exist to avoid repeating. D-4434.
///
/// A page that fails EVICTS its reader. The slot is keyed on content only, and
/// the reader also pins its trade file's filesystem generation, so a file
/// relinked, restored or merely `chmod`ed under the same content left a
/// reader that refused every request for that candidate until a restart, while
/// its own refusal said to reopen. The refusal still answers this request; the
/// next one cold-opens and re-verifies every row and the seal, as every sibling
/// cache does. D-2763, apicache-1.
#[cfg(test)]
fn page_through(
    kept: &mut Vec<Cached>,
    root: &Path,
    summary: &Summary,
    key: Key,
    offset: u64,
    limit: usize,
) -> Result<(Candidate, Vec<cli::trades::Row>), String> {
    let held = take_serving(kept, root, summary, key);
    let (held, page) = serve(held, root, summary, key, offset, limit)?;
    keep(kept, held);
    Ok(page)
}
/// Takes the reader serving `key` out of `kept`, if one is there. At most
/// [`TRADE_READERS_KEPT`] comparisons. D-4434, D-4655.
fn take_serving(
    kept: &mut Vec<Cached>,
    root: &Path,
    summary: &Summary,
    key: Key,
) -> Option<Cached> {
    let at = slot_of(kept, |held| held.serves(root, summary, key))?;
    Some(kept.remove(at))
}
/// The index of the first kept reader `wanted` accepts. A plain loop over a
/// vector [`TRADE_READERS_KEPT`] bounds, as the cache's search was before
/// D-4655 split it, so gate 11's rule 6 reads no data-bounded search here
/// (D-4662).
fn slot_of(kept: &[Cached], wanted: impl Fn(&Cached) -> bool) -> Option<usize> {
    for (at, held) in kept.iter().enumerate() {
        if wanted(held) {
            return Some(at);
        }
    }
    None
}
/// Pages `held`, cold-opening a reader first when there is none. Reads files;
/// no cache lock is held here. A failed page drops the reader rather than
/// returning it to be kept (D-2763).
fn serve(
    held: Option<Cached>,
    root: &Path,
    summary: &Summary,
    key: Key,
    offset: u64,
    limit: usize,
) -> Result<(Cached, (Candidate, Vec<cli::trades::Row>)), String> {
    let mut held = if let Some(held) = held {
        held
    } else {
        #[cfg(test)]
        COLD_TRADE_READERS.with(|count| count.set(count.get() + 1));
        let reader = TradeReader::open(root, summary, key, crate::detail::MAX_SCAN_BYTES)?;
        Cached {
            model: summary.model,
            root: root.to_path_buf(),
            identity: summary.identity,
            attempt: summary.attempt,
            digest: summary.digest,
            key,
            reader,
        }
    };
    let rows = held.reader.page(offset, limit)?;
    let candidate = held.reader.candidate().clone();
    Ok((held, (candidate, rows)))
}
/// Puts `held` back as the most recently paged. A reader for the same
/// candidate already back, which a concurrent request that opened its own put
/// there first, is replaced, so no candidate is ever kept twice; otherwise a
/// full cache evicts its least recently paged. At most
/// [`TRADE_READERS_KEPT`] comparisons. D-4434, D-4655.
fn keep(kept: &mut Vec<Cached>, held: Cached) {
    if let Some(at) = slot_of(kept, |other| other.same_as(&held)) {
        kept.remove(at);
    } else if kept.len() >= TRADE_READERS_KEPT {
        kept.remove(0);
    }
    kept.push(held);
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
    /// **A trade page whose reader went stale is refused once, not until a
    /// restart.** D-2763, apicache-1.
    ///
    /// Every trade file is replaced by a byte-identical copy under a new
    /// inode, which is what a restore, an rsync or a rerun of a deterministic
    /// capture does. The content key is unchanged, so the slot keeps its
    /// reader, and that reader's pinned generation no longer matches. The
    /// request that meets it is refused; the one after it must cold-open and
    /// answer. Before D-2763 the stale reader stayed in the slot and every
    /// later request was refused the same way.
    #[test]
    fn a_trade_reader_whose_file_generation_moved_is_evicted_by_its_refusal() -> Result<(), String>
    {
        let root = crate::scratch::path("candidate-api-stale-reader");
        let _ = std::fs::remove_dir_all(&root);
        let query = and_capture(&root, [77; 32], true)?;
        let asked = Asked::parse(&query)?;
        let summary = candidate_trades::read_model(
            &root,
            asked.identity,
            asked.attempt,
            asked.model,
            crate::detail::MAX_SCAN_BYTES,
        )?
        .ok_or("the capture is sealed")?;
        let key = Key {
            tier: 0,
            rank: 1,
            direction: Direction::Long,
        };
        let short = Key {
            direction: Direction::Short,
            ..key
        };
        let mut slot = Vec::new();
        page_through(&mut slot, &root, &summary, short, 0, 16)?;
        page_through(&mut slot, &root, &summary, key, 0, 16)?;
        assert_eq!(slot.len(), 2, "each first page caches its reader");

        let directory = root
            .join("results/candidate-trades-v1")
            .join(crate::server::hex32([77; 32]))
            .join(asked.attempt.to_string());
        let mut replaced = 0;
        for entry in std::fs::read_dir(&directory).map_err(|why| why.to_string())? {
            let path = entry.map_err(|why| why.to_string())?.path();
            if !path.to_string_lossy().ends_with("-trades.bin") {
                continue;
            }
            let copy = path.with_extension("copy");
            std::fs::copy(&path, &copy).map_err(|why| why.to_string())?;
            std::fs::rename(&copy, &path).map_err(|why| why.to_string())?;
            replaced += 1;
        }
        assert!(replaced > 0, "the fixture has trade files to replace");

        assert!(
            page_through(&mut slot, &root, &summary, key, 0, 16).is_err(),
            "the stale reader refuses the request that meets it"
        );
        assert_eq!(slot.len(), 1, "and that refusal evicts it, and only it");
        assert!(
            slot.iter().all(|held| held.key == short),
            "the other side's reader is not the one evicted"
        );
        page_through(&mut slot, &root, &summary, key, 0, 16)
            .map_err(|why| format!("the next request must cold-open and answer: {why}"))?;
        assert_eq!(slot.len(), 2, "the fresh reader is cached");
        std::fs::remove_dir_all(root).map_err(|why| why.to_string())?;
        Ok(())
    }

    /// **A stock audit's AND-mask capture says what its figures are made of;
    /// an index's, an unrecorded one's and an expression capture's are the
    /// bytes they were.** D-0694, AF-19.
    ///
    /// An AND-mask capture is keyed by the audit's own run identity, which
    /// is the identity the results ledger records, so the ledger row names
    /// the instrument exactly as it does for `/sweep-evidence.json`. Every
    /// capture is read before its ledger row exists and again after, so the
    /// comparison is over the same bytes: an index page does not move, and a
    /// stock page gains `equity_note` and nothing else, on the saved page
    /// with rows and on the empty catalog alike. An expression capture is
    /// keyed by `candidate_identity`, which no ledger row carries, so even a
    /// row naming RELIANCE under that identity adds nothing to it.
    #[test]
    fn an_and_mask_capture_of_a_stock_carries_the_equity_note_and_others_do_not()
    -> Result<(), String> {
        let root = crate::scratch::path("candidate-api-equity-note");
        let _ = std::fs::remove_dir_all(&root);
        // (identity, instrument its ledger row names, a tier with rows)
        let captures = [
            ([0xb1_u8; 32], "NIFTY", true),
            ([0xb2; 32], "RELIANCE", true),
            ([0xb3; 32], "BANKNIFTY", false),
            ([0xb4; 32], "RELIANCE", false),
        ];
        let pages = captures
            .iter()
            .map(|&(identity, underlying, rows)| {
                and_capture(&root, identity, rows).map(|query| (query, underlying, rows))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let expression_id = [0xb5_u8; 32];
        let expression_query = expression_capture(&root, expression_id)?;
        let read = |query: &str| -> Result<String, String> { render(&root, &Asked::parse(query)?) };
        let before = pages
            .iter()
            .map(|(query, _, _)| read(query))
            .collect::<Result<Vec<_>, _>>()?;
        let expression_before = read(&expression_query)?;
        for body in before.iter().chain([&expression_before]) {
            assert!(body.contains(r#""status":"saved""#), "premise: {body}");
            assert!(
                !body.contains("equity_note"),
                "no ledger row names this capture's instrument yet: {body}"
            );
        }
        for (identity, underlying, _) in captures {
            crate::sweepevidence::tests::ledger_row(&root, identity, underlying);
        }
        crate::sweepevidence::tests::ledger_row(&root, expression_id, "RELIANCE");
        let note = cli::equity_note_for("RELIANCE");
        for part in ["GROSS OF EVERY CHARGE", "CORPORATE ACTIONS ARE UNCHECKED"] {
            assert!(note.contains(part), "premise, {part}: {note}");
        }
        for ((query, underlying, rows), before) in pages.iter().zip(&before) {
            let after = read(query)?;
            let what = format!("{underlying}, rows={rows}");
            if *underlying == "RELIANCE" {
                let mut value: Value =
                    serde_json::from_str(&after).map_err(|why| why.to_string())?;
                let said = value
                    .as_object_mut()
                    .and_then(|body| body.remove("equity_note"));
                assert_eq!(
                    said,
                    Some(json!(note)),
                    "{what}: the stored banner's own note"
                );
                assert_eq!(
                    value,
                    serde_json::from_str::<Value>(before).map_err(|why| why.to_string())?,
                    "{what}: the note is the only thing a stock's capture adds"
                );
            } else {
                assert_eq!(&after, before, "{what}: an index capture's page");
            }
            if *rows {
                // The exact trade page of one candidate is the same body.
                let digest = serde_json::from_str::<Value>(before)
                    .map_err(|why| why.to_string())?
                    .get("catalog_digest")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or("premise: the catalog digest")?;
                let trades = read(&format!("{query}&digest={digest}&rank=1&direction=long"))?;
                let trades: Value = serde_json::from_str(&trades).map_err(|why| why.to_string())?;
                assert_eq!(trades.get("kind"), Some(&json!("trades")), "premise");
                assert_eq!(
                    trades.get("equity_note"),
                    (*underlying == "RELIANCE").then(|| json!(note)).as_ref(),
                    "{what}: the trade page"
                );
            }
        }
        assert_eq!(
            read(&expression_query)?,
            expression_before,
            "an expression capture is keyed by an identity no ledger row carries"
        );
        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }

    /// A sealed AND-mask capture for an audit attempt of `identity`, with one
    /// tier holding one candidate on both sides when `rows`, and no tier
    /// otherwise; answers the page-zero query for it.
    fn and_capture(root: &Path, identity: [u8; 32], rows: bool) -> Result<String, String> {
        #[expect(
            clippy::default_trait_access,
            reason = "the public capture API infers Column without adding an api-to-indicators graph edge"
        )]
        let column = Default::default();
        #[expect(
            clippy::default_trait_access,
            reason = "the public capture API infers Grid without adding an api-to-runner graph edge"
        )]
        let grid = Default::default();
        let attempt =
            cli::sweep_evidence::begin(root, identity, cli::sweep_evidence::Operation::Audit)?;
        let capture = candidate_trades::Capture::begin(root, &attempt, &[], &column)?;
        if rows {
            let tier = capture.tier(Tier {
                eligible: 1,
                evaluated: 1,
                ..no_cell_tier()
            })?;
            for direction in [Direction::Long, Direction::Short] {
                capture.record(
                    &tier,
                    &candidate_trades::Evaluated {
                        rank: 1,
                        mask: &vocab::ConditionMask::default(),
                        direction,
                        grid: &grid,
                        selected: None,
                    },
                )?;
            }
        }
        capture.finish()?;
        Ok(format!(
            "identity={}&attempt={}",
            crate::server::hex32(identity),
            attempt.token()
        ))
    }

    /// A sealed expression-model capture under `identity`; answers its query.
    fn expression_capture(root: &Path, identity: [u8; 32]) -> Result<String, String> {
        #[expect(
            clippy::default_trait_access,
            reason = "the public capture API infers Column without adding an api-to-indicators graph edge"
        )]
        let column = Default::default();
        let attempt =
            cli::sweep_evidence::begin(root, identity, cli::sweep_evidence::Operation::Expression)?;
        let expression =
            vocab::expression::Expression::parse("0 | !1").map_err(|why| format!("{why:?}"))?;
        candidate_trades::Capture::begin_expression(root, &attempt, &[], &column, &expression)?
            .finish()?;
        Ok(format!(
            "identity={}&attempt={}&model=expression",
            crate::server::hex32(identity),
            attempt.token()
        ))
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

    /// The body of `name` in `source`, up to its closing brace at column 0.
    fn body<'a>(source: &'a str, name: &str) -> &'a str {
        let found = source
            .split_once(&format!("\nfn {name}("))
            .or_else(|| source.split_once(&format!("\npub fn {name}(")));
        assert!(found.is_some(), "{name} exists");
        let rest = found.unwrap_or_default().1;
        &rest[..rest.find("\n}\n").unwrap_or(rest.len())]
    }

    /// **A candidate page reads the whole sealed catalog at most once, and only
    /// when the held summary is cold; that is stated with the calls the source
    /// makes.** W1-api2-2, D-1444, D-2283.
    ///
    /// `render` reaches the catalog through `summary_for` alone, whose only
    /// whole read is its cold `read_model`; the closing check is the
    /// generation (`require_unchanged`), and `tier` and `candidates_page` go
    /// through `pinned`, which is warm for every summary `read_model` returns.
    #[test]
    fn a_candidate_pages_catalog_reads_are_counted_and_stated() {
        let api = include_str!("candidatejson.rs");
        let cli = include_str!("../../cli/src/candidate_trades.rs");
        assert_eq!(body(api, "render_with").matches("read_model(").count(), 0);
        assert_eq!(
            body(api, "render_with")
                .matches("candidate_trades::require_unchanged(")
                .count(),
            1
        );
        assert_eq!(
            body(api, "render_with")
                .matches("summary_for(slot, root, asked)")
                .count(),
            1
        );
        assert_eq!(
            body(api, "summary_for")
                .matches("candidate_trades::read_model(")
                .count(),
            1
        );
        assert_eq!(
            body(api, "render_with")
                .matches("candidate_trades::tier(")
                .count(),
            1
        );
        assert_eq!(
            body(api, "render_with")
                .matches("candidate_trades::candidates_page(")
                .count(),
            1
        );
        assert_eq!(body(cli, "pinned").matches("read_model(").count(), 1);
        assert!(
            body(cli, "read_model")
                .contains("let _first = summary.catalog_generation.set(generation);")
        );
        assert_eq!(body(cli, "require_unchanged").matches("pinned(").count(), 1);
        assert_eq!(body(cli, "tier").matches("pinned(").count(), 1);
        assert_eq!(body(cli, "candidates_page").matches("pinned(").count(), 2);
        let bullet = crate::booleanjson::tests::d0951_bullet("W1-api2-2");
        for word in [
            "candidatejson::render",
            "candidate_trades::read_model",
            "Since D-2283",
            "`summary_for`",
            "`require_unchanged`",
            "catalog.bin",
            "32 bytes per candidate side plus 40 per tier",
            "MAX_SCAN_BYTES",
            "at most 256 rows",
        ] {
            assert!(bullet.contains(word), "the bullet names {word}: {bullet}");
        }
    }

    /// cand-2, D-2576: a trade page's cold open runs with the cache's mutex
    /// FREE, so no other detail request parks on it holding a permit. The
    /// first page is a cold open (the probe records the slot as it found it);
    /// the second is warm and answers the same rows. On the old `trade_page`
    /// the guard was held across `TradeReader::open`, so the slot could never
    /// be locked from the open point and the probe recorded `false`.
    #[test]
    fn a_cold_trade_reader_does_not_park_detail_permits() -> Result<(), String> {
        let root = crate::scratch::path("candidate-api-cold-unlocked");
        let _ = std::fs::remove_dir_all(&root);
        let query = and_capture(&root, [78; 32], true)?;
        let asked = Asked::parse(&query)?;
        let summary = candidate_trades::read_model(
            &root,
            asked.identity,
            asked.attempt,
            asked.model,
            crate::detail::MAX_SCAN_BYTES,
        )?
        .ok_or("the capture is sealed")?;
        let key = Key {
            tier: 0,
            rank: 1,
            direction: Direction::Long,
        };
        crate::detail::SLOT_FREE_AT_OPEN.with(|cell| cell.set(None));
        let (_, cold) = trade_page(&root, &summary, key, 0, 16)?;
        assert_eq!(
            crate::detail::SLOT_FREE_AT_OPEN.with(std::cell::Cell::get),
            Some(true),
            "the reader was opened with the cache locked"
        );
        let (_, warm) = trade_page(&root, &summary, key, 0, 16)?;
        // The fixture's first candidate may page no trades; what this proves
        // is the free slot above, and that the warm page answers what the
        // cold one did.
        assert_eq!(cold.len(), warm.len());
        std::fs::remove_dir_all(root).map_err(|why| why.to_string())?;
        Ok(())
    }

    /// **The held summary serves a later page without a cold read, and any
    /// other capture or a rewritten catalog makes the next one cold.**
    /// D-2283 (W1-api2-2). Counted on this thread against a slot of its own,
    /// so no other test's requests can evict it.
    #[test]
    fn a_held_summary_serves_again_warm_and_is_dropped_by_a_key_or_a_rewrite() -> Result<(), String>
    {
        let root = crate::scratch::path("candidate-api-held-summary");
        let _ = std::fs::remove_dir_all(&root);
        let first_id = [0xc1_u8; 32];
        let query = and_capture(&root, first_id, true)?;
        let other = and_capture(&root, [0xc2; 32], true)?;
        let slot = Mutex::new(None);
        let cold = || COLD_SUMMARIES.with(std::cell::Cell::get);
        let start = cold();
        let page = |query: &str| render_with(&slot, &root, &Asked::parse(query)?);
        let first = page(&query)?;
        assert_eq!(cold() - start, 1, "the first request reads the catalog");
        assert_eq!(page(&query)?, first, "the same page, warm");
        assert_eq!(cold() - start, 1, "and warm means no second read");
        let digest = serde_json::from_str::<Value>(&first)
            .map_err(|why| why.to_string())?
            .get("catalog_digest")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or("premise: the catalog digest")?;
        let trades = page(&format!("{query}&digest={digest}&rank=1&direction=long"))?;
        assert!(trades.contains(r#""kind":"trades""#), "{trades}");
        assert_eq!(
            cold() - start,
            1,
            "a trade page of the held capture is warm too"
        );

        let theirs = page(&other)?;
        assert!(
            theirs.contains(&crate::server::hex32([0xc2; 32])),
            "another capture is answered from its own catalog: {theirs}"
        );
        assert!(
            !theirs.contains(&crate::server::hex32(first_id)),
            "{theirs}"
        );
        assert_eq!(cold() - start, 2, "another identity is a cold read");
        assert_eq!(page(&query)?, first);
        assert_eq!(cold() - start, 3, "and evicts the first");

        // The same bytes under a new inode: the generation moves, so the held
        // summary is not trusted, and the page read cold is the same page.
        let attempt = query
            .split("attempt=")
            .nth(1)
            .ok_or("premise: an attempt")?;
        let catalog = root
            .join("results/candidate-trades-v1")
            .join(crate::server::hex32(first_id))
            .join(attempt)
            .join("catalog.bin");
        let bytes = std::fs::read(&catalog).map_err(|why| why.to_string())?;
        let fresh = catalog.with_extension("fresh");
        std::fs::write(&fresh, &bytes).map_err(|why| why.to_string())?;
        std::fs::rename(&fresh, &catalog).map_err(|why| why.to_string())?;
        assert_eq!(page(&query)?, first, "the same bytes are the same page");
        assert_eq!(cold() - start, 4, "a moved generation is a cold read");

        // Gone: the held summary does not answer for a missing catalog.
        std::fs::remove_file(&catalog).map_err(|why| why.to_string())?;
        let missing = page(&query)?;
        assert!(missing.contains(r#""status":"missing""#), "{missing}");
        assert_eq!(cold() - start, 5);
        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }

    /// **The trade page keeps eight readers warm: switching among up to
    /// eight candidates opens each once, the ninth evicts the least recently
    /// paged, and the kept pages are the cold pages.** W1-api2-3, D-4434.
    #[test]
    fn trade_readers_keep_eight_candidates_warm_and_evict_the_least_recent() -> Result<(), String> {
        let root = crate::scratch::path("candidate-api-kept-readers");
        let _ = std::fs::remove_dir_all(&root);
        // Five captures, two sides each: ten distinct candidates.
        let mut wanted = Vec::with_capacity(10);
        for n in 0..5_u8 {
            let query = and_capture(&root, [0x90 + n; 32], true)?;
            let asked = Asked::parse(&query)?;
            let summary = candidate_trades::read_model(
                &root,
                asked.identity,
                asked.attempt,
                asked.model,
                crate::detail::MAX_SCAN_BYTES,
            )?
            .ok_or("the capture is sealed")?;
            for direction in [Direction::Long, Direction::Short] {
                let key = Key {
                    tier: 0,
                    rank: 1,
                    direction,
                };
                wanted.push((summary.clone(), key));
            }
        }
        let cold = || COLD_TRADE_READERS.with(std::cell::Cell::get);
        let mut kept = Vec::new();
        let mut page = |at: usize| -> Result<_, String> {
            let (summary, key) = wanted.get(at).ok_or("premise: ten candidates")?;
            let got = page_through(&mut kept, &root, summary, *key, 0, 16)?;
            let fresh = TradeReader::open(&root, summary, *key, crate::detail::MAX_SCAN_BYTES)?
                .candidate()
                .clone();
            assert_eq!(got.0, fresh, "a kept reader answers what a cold one does");
            Ok(kept.len())
        };

        // Alternating between two candidates: two cold opens, not a hundred.
        let start = cold();
        for turn in 0..100 {
            page(turn % 2)?;
        }
        assert_eq!(cold() - start, 2, "two candidates, two opens");

        // Eight distinct candidates fit; revisiting them in any order is warm.
        for at in 2..8 {
            page(at)?;
        }
        assert_eq!(cold() - start, 8);
        for at in [7, 0, 3, 5, 1, 6, 2, 4] {
            assert_eq!(page(at)?, TRADE_READERS_KEPT, "never more than the cap");
        }
        assert_eq!(cold() - start, 8, "every revisit is warm");

        // The ninth evicts the least recently paged (7, from the revisit).
        assert_eq!(page(8)?, TRADE_READERS_KEPT);
        assert_eq!(cold() - start, 9);
        page(4)?;
        assert_eq!(cold() - start, 9, "the most recent survived");
        page(7)?;
        assert_eq!(cold() - start, 10, "the least recent was evicted");
        assert_eq!(page(9)?, TRADE_READERS_KEPT);
        std::fs::remove_dir_all(&root).map_err(|why| why.to_string())?;
        Ok(())
    }

    /// **Two requests that cold-open one candidate at once leave one reader
    /// kept, not two.** cand-2 D-2576 releases the lock across each open and
    /// W1-api2-3 D-4434 keeps eight readers; D-4655 holds them together. Both
    /// requests may open, and whichever puts back last replaces the other's,
    /// so the eight slots never spend two on one candidate.
    #[test]
    fn a_reader_put_back_twice_for_one_candidate_is_kept_once() -> Result<(), String> {
        let root = crate::scratch::path("candidate-api-kept-once");
        let _ = std::fs::remove_dir_all(&root);
        let query = and_capture(&root, [79; 32], true)?;
        let asked = Asked::parse(&query)?;
        let summary = candidate_trades::read_model(
            &root,
            asked.identity,
            asked.attempt,
            asked.model,
            crate::detail::MAX_SCAN_BYTES,
        )?
        .ok_or("the capture is sealed")?;
        let key = Key {
            tier: 0,
            rank: 1,
            direction: Direction::Long,
        };
        let short = Key {
            direction: Direction::Short,
            ..key
        };
        let open = |key: Key| -> Result<Cached, String> {
            Ok(Cached {
                model: summary.model,
                root: root.clone(),
                identity: summary.identity,
                attempt: summary.attempt,
                digest: summary.digest,
                key,
                reader: TradeReader::open(&root, &summary, key, crate::detail::MAX_SCAN_BYTES)?,
            })
        };
        let mut kept = Vec::new();
        keep(&mut kept, open(short)?);
        keep(&mut kept, open(key)?);
        keep(&mut kept, open(key)?);
        assert_eq!(kept.len(), 2, "the second put-back replaces the first");
        assert_eq!(kept.iter().filter(|held| held.key == key).count(), 1);
        assert_eq!(
            kept.last().map(|held| held.key),
            Some(key),
            "and is the most recently paged"
        );
        assert!(
            take_serving(&mut kept, &root, &summary, short).is_some(),
            "the other candidate's reader was not the one replaced"
        );
        std::fs::remove_dir_all(root).map_err(|why| why.to_string())?;
        Ok(())
    }

    /// What a kept trade reader saves: a warm page among eight kept readers
    /// against a cold open of the same candidate. A measurement, run on
    /// purpose; the numbers are in `docs/06-limits.md`. W1-api2-3, D-4434.
    #[test]
    #[ignore = "a latency measurement, run on purpose: see crate::latency"]
    fn latency_trade_reader_warm_page_and_cold_open() -> Result<(), String> {
        let root = crate::scratch::path("candidate-api-reader-latency");
        let _ = std::fs::remove_dir_all(&root);
        let mut wanted = Vec::with_capacity(TRADE_READERS_KEPT);
        for n in 0..4_u8 {
            let query = and_capture(&root, [0xb0 + n; 32], true)?;
            let asked = Asked::parse(&query)?;
            let summary = candidate_trades::read_model(
                &root,
                asked.identity,
                asked.attempt,
                asked.model,
                crate::detail::MAX_SCAN_BYTES,
            )?
            .ok_or("the capture is sealed")?;
            for direction in [Direction::Long, Direction::Short] {
                let key = Key {
                    tier: 0,
                    rank: 1,
                    direction,
                };
                wanted.push((summary.clone(), key));
            }
        }
        let mut kept = Vec::new();
        let mut turn = 0_usize;
        let warm = crate::latency::Timed::run(4_000, || {
            let (summary, key) = wanted
                .get(turn % wanted.len())
                .ok_or("premise: eight candidates")?;
            turn += 1;
            page_through(&mut kept, &root, summary, *key, 0, 16).map(drop)
        })?;
        let (summary, key) = wanted.first().ok_or("premise: a candidate")?;
        let cold = crate::latency::Timed::run(1_000, || {
            TradeReader::open(&root, summary, *key, crate::detail::MAX_SCAN_BYTES).map(drop)
        })?;
        println!(
            "{}",
            warm.line("trade page, 8 readers kept, cycling all 8 (warm)")
        );
        println!(
            "{}",
            cold.line("TradeReader::open of a no-cell candidate (cold, fixed part)")
        );
        std::fs::remove_dir_all(&root).map_err(|why| why.to_string())?;
        Ok(())
    }
}

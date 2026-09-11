//! Bounded, restart-safe recovery through the existing spot acquisition path.
//!
//! A scan checkpoint is one exact symbol/source-timeframe/month intersection.
//! Network work is a separate, exact single-day child, admitted only when that
//! symbol has an observed source bar on that day. Absent history is UNVERIFIED,
//! not automatically a broker omission. No missing candle is synthesized.
//!
//! Auditing is O(rows + days); indexed state costs expected O(1) per update and
//! O(work units) space. Neither the total pull nor device/network latency is O(1).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use axum::{extract::State, http::StatusCode};
use brutex_core::vendor::Vendor;
use pull::session::{Day, Window};
use pull::vendor::Granularity;

use crate::ingest::{self, CashIdentity, SpotRequest};
use crate::pullrun::{FeedReport, Leg, Progress, Route};
use crate::recovery_journal::{Journal, Record, Status};
use crate::server::{self, Loaded, Site};

/// Operational safety limits, never research/selection thresholds. Reservation
/// is durable BEFORE a request; interrupted attempts consume the same budget.
const ATTEMPT_LIMIT: u32 = 3;
const MAX_UNITS: usize = 100_000;
const CONTROL: [u8; 32] = [0; 32];
const POLICY: &str = "spot-recovery-v1-observed-day-3-attempts-global";
const CONTROL_BODY: &str = "plan-seeded-v1";
const SUCCESSOR_POLICY: &str = "spot-recovery-successor-v1";
type Lifecycle = Option<pull::cash_session_cache::VerifiedLifecycleMaster>;

#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err consumes owned errors at the I/O and task boundaries"
)]
fn failure(why: impl ToString) -> String {
    why.to_string()
}

fn encode(raw: &str) -> String {
    use core::fmt::Write as _;
    let mut out = String::new();
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn key(body: &str) -> [u8; 32] {
    brutex_core::blake3::hash(format!("{POLICY}\0{body}").as_bytes())
}

fn hex(key: [u8; 32]) -> String {
    use core::fmt::Write as _;
    key.iter().fold(String::with_capacity(64), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn unhex(text: &str) -> Result<[u8; 32], String> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid recovery plan identity".to_owned());
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(text.as_bytes().chunks_exact(2)) {
        *slot =
            u8::from_str_radix(std::str::from_utf8(pair).map_err(failure)?, 16).map_err(failure)?;
    }
    Ok(out)
}

fn record(body: String) -> Record {
    Record {
        key: key(&body),
        body,
        status: Status::Queued,
        attempts: 0,
        unchanged: 0,
        committed: 0,
        diagnostics: 0,
        missing: 0,
        unverified: 0,
        http_status: 0,
    }
}

fn reserve(item: &mut Record) -> bool {
    if matches!(
        item.status,
        Status::Verified | Status::Exhausted | Status::Blocked
    ) {
        return false;
    }
    if item.attempts >= ATTEMPT_LIMIT {
        item.status = Status::Exhausted;
        return false;
    }
    item.attempts += 1;
    item.status = Status::InFlight;
    true
}

fn receipt_state(attempts: u32, balances: bool, source: Status, still_owed: bool) -> Status {
    if balances && source == Status::Verified {
        Status::Verified
    } else if !still_owed {
        Status::Unverified
    } else if attempts >= ATTEMPT_LIMIT {
        Status::Exhausted
    } else {
        Status::Queued
    }
}

fn refresh_owed(item: &mut Record) {
    if item.status == Status::Verified {
        // Called only after the parent's new source audit finds this day owed.
        // Reuse the remaining budget instead of trusting a stale green receipt.
        item.status = Status::Queued;
    }
}

fn canonical(symbol: &str, dir: &str, window: Window, kind: &str) -> String {
    format!(
        "target=fno&vendor=zerodha&cash_identity=zerodha_cross_checked&member={}&granularity={dir}&from={}&to={}&recovery_kind={kind}",
        encode(symbol),
        window.from(),
        window.to()
    )
}

fn checked(body: &str, today: Day) -> Result<SpotRequest, String> {
    let asked = ingest::parse_spot(body, today).map_err(failure)?;
    if asked.feed != pull::vendor::Feed::Zerodha
        || asked.target != ingest::SpotTarget::Fno
        || asked.cash_identity != CashIdentity::ZerodhaCrossChecked
        || !matches!(asked.granularity, Granularity::Day1 | Granularity::Minute1)
        || asked.members.is_empty()
        || asked
            .members
            .iter()
            .any(|name| !brutex_core::universe::FNO_INDEX.contains(name.as_str()))
        || asked.members.iter().any(|name| {
            matches!(
                name.as_str(),
                "FINNIFTY" | "MIDCPNIFTY" | "NIFTYNXT50" | "NIFTYNEXT50"
            )
        })
    {
        return Err("recovery requires explicit F&O spot members, Zerodha, cross-checked identity and 1day/1min; no basket or policy is substituted".to_owned());
    }
    Ok(asked)
}

/// Normalize selected members and month boundaries without changing either
/// inclusive endpoint. The sorted body set also makes duplicate submissions
/// and member order share a budget, rather than purchase fresh retries.
fn plan(legs: Vec<Leg>, today: Day) -> Result<Vec<Record>, String> {
    let mut bodies = std::collections::BTreeSet::new();
    for leg in legs {
        let asked = checked(&leg.body, today)?;
        if leg.route != Route::Spot || leg.vendor != "zerodha" || leg.dir != asked.granularity.dir()
        {
            return Err("recovery leg envelope disagrees with its payload".to_owned());
        }
        for member in &asked.members {
            let mut from = asked.window.from();
            loop {
                let to = from.end_of_month().min(asked.window.to());
                let window = Window::new(from, to).map_err(failure)?;
                bodies.insert(canonical(member.as_str(), leg.dir.as_str(), window, "scan"));
                if bodies.len() > MAX_UNITS {
                    return Err("recovery plan exceeds its reported 100,000-window bound; nothing was truncated".to_owned());
                }
                if to == asked.window.to() {
                    break;
                }
                from = Day::from_days(to.days_from_epoch() + 1).map_err(failure)?;
            }
        }
    }
    let mut out: Vec<_> = bodies.into_iter().map(record).collect();
    out.sort_by_key(|item| {
        (
            server::param(&item.body, "granularity") != "1day",
            item.body.clone(),
        )
    });
    Ok(out)
}

fn root(site: &Site) -> PathBuf {
    site.store_root.join("audit/recovery-v1")
}

fn plan_path(site: &Site, id: [u8; 32]) -> PathBuf {
    root(site).join(format!("{}.bin", hex(id)))
}

fn active_path(site: &Site) -> PathBuf {
    root(site).join("active.bin")
}

fn scope_identity(units: &[Record]) -> [u8; 32] {
    key(&units
        .iter()
        .map(|row| row.body.as_str())
        .collect::<Vec<_>>()
        .join("\n"))
}

fn successor_identity(body: &str) -> Result<([u8; 32], [u8; 32]), String> {
    let parts: Vec<_> = body.split(':').collect();
    let [policy, predecessor, generation] = parts.as_slice() else {
        return Err("recovery plan has an unknown generation descriptor".to_owned());
    };
    if *policy != SUCCESSOR_POLICY {
        return Err("recovery plan has an unknown generation policy".to_owned());
    }
    let predecessor = unhex(predecessor)?;
    unhex(generation)?;
    Ok((brutex_core::blake3::hash(body.as_bytes()), predecessor))
}

fn pointer_identity(body: &str) -> Result<[u8; 32], String> {
    if body.len() == 64 {
        unhex(body)
    } else {
        successor_identity(body).map(|(id, _)| id)
    }
}

fn active_history(site: &Site) -> Result<crate::recovery_journal::Index, String> {
    let path = active_path(site);
    if !path.try_exists().map_err(failure)? {
        return Ok(crate::recovery_journal::Index::default());
    }
    let history = crate::recovery_journal::snapshot(&path).map_err(failure)?;
    if history.latest.is_empty() {
        return Err("active recovery pointer is empty; prior activation is unverified".to_owned());
    }
    for row in history.latest.values() {
        let mut expected = record(row.body.clone());
        expected.key = row.key;
        expected.attempts = row.attempts;
        if row.attempts == 0 || pointer_identity(&row.body)? != row.key || *row != expected {
            return Err(
                "active recovery history has invalid identity or activation state".to_owned(),
            );
        }
    }
    Ok(history)
}

fn missing_plan(id: [u8; 32]) -> String {
    format!(
        "recovery plan {} is missing after a recorded activation; original window states and completion are unavailable. STOP and attempt events do not reconstruct the plan. Preserve that history and prepare an explicit successor generation before new recovery work",
        hex(id)
    )
}

fn validate_seal(
    latest: &std::collections::HashMap<[u8; 32], Record>,
    id: [u8; 32],
) -> Result<(), String> {
    let control = latest
        .get(&CONTROL)
        .ok_or("recovery plan has no durable seal")?;
    let mut expected = record(control.body.clone());
    expected.key = CONTROL;
    expected.status = control.status;
    if *control != expected
        || !matches!(
            control.status,
            Status::Queued | Status::InFlight | Status::Verified | Status::Blocked
        )
    {
        return Err("recovery plan seal has invalid control state or quantities".to_owned());
    }
    let mut scans: Vec<_> = latest
        .values()
        .filter(|row| server::param(&row.body, "recovery_kind") == "scan")
        .cloned()
        .collect();
    if scans.is_empty()
        || scans.len() > MAX_UNITS
        || scans.iter().any(|row| row.key != key(&row.body))
    {
        return Err(
            "recovery plan scan inventory is empty, oversized or has an invalid work identity"
                .to_owned(),
        );
    }
    scans.sort_by_key(|row| {
        (
            server::param(&row.body, "granularity") != "1day",
            row.body.clone(),
        )
    });
    let scope = scope_identity(&scans);
    if control.body == CONTROL_BODY {
        if scope != id {
            return Err(
                "recovery sealed window inventory does not match its original plan identity"
                    .to_owned(),
            );
        }
    } else {
        let (generation_id, predecessor) = successor_identity(&control.body)?;
        if predecessor != scope || generation_id != id {
            return Err(
                "successor seal does not bind its exact scope, predecessor and generation"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

/// Check before clearing STOP or claiming work. A former identity must never be
/// silently recreated from today's form, even when that form has the same scope.
fn preflight_plan(site: &Site, id: [u8; 32]) -> Result<(), String> {
    let history = active_history(site)?;
    let exists = plan_path(site, id).try_exists().map_err(failure)?;
    if history.latest.contains_key(&id) && !exists {
        return Err(missing_plan(id));
    }
    if exists {
        let journal = crate::recovery_journal::snapshot(&plan_path(site, id)).map_err(failure)?;
        if !journal.latest.contains_key(&CONTROL) && history.latest.contains_key(&id) {
            return Err(
                "previously activated recovery plan has no durable seal; history was not reseeded"
                    .to_owned(),
            );
        }
        if journal.latest.contains_key(&CONTROL) {
            validate_seal(&journal.latest, id)?;
        }
    }
    if !history.latest.is_empty() {
        // A missing shared reservation inventory is not permission to refund
        // requests, including for a newly named or overlapping plan.
        crate::recovery_journal::snapshot(&root(site).join("attempts.bin")).map_err(|why| {
            format!(
                "shared recovery attempt history is unavailable; budgets cannot be reset: {why}"
            )
        })?;
    }
    Ok(())
}

#[derive(Debug)]
struct Submission {
    id: [u8; 32],
    units: Vec<Record>,
    successor: Option<([u8; 32], String)>,
    prepare_only: bool,
}

fn one_parameter(body: &str, name: &str) -> Result<Option<String>, String> {
    let values = server::params(body, name);
    match values.as_slice() {
        [] => Ok(None),
        [value] if !value.is_empty() => Ok(Some(value.clone())),
        _ => Err(format!(
            "recovery field {name} must be supplied once and nonempty"
        )),
    }
}

fn submission(body: &str, today: Day) -> Result<Submission, String> {
    let units = plan(
        crate::pullrun::legs_from(body).map_err(|why| why.why())?,
        today,
    )?;
    let scope = scope_identity(&units);
    let action = one_parameter(body, "recovery_action")?;
    let prepare_only = match action.as_deref() {
        None | Some("start") => false,
        Some("prepare") => true,
        Some(_) => return Err("recovery_action must be start or prepare".to_owned()),
    };
    let predecessor = one_parameter(body, "recovery_supersedes")?;
    let generation = one_parameter(body, "recovery_generation")?;
    let (id, successor) = match (predecessor, generation) {
        (None, None) if !prepare_only => (scope, None),
        (Some(predecessor), Some(generation)) => {
            let predecessor = unhex(&predecessor)?;
            unhex(&generation)?;
            if predecessor != scope {
                return Err("successor scope must reproduce the original canonical plan identity; no members, dates or timeframes are substituted".to_owned());
            }
            let seal = format!("{SUCCESSOR_POLICY}:{}:{generation}", hex(predecessor));
            let id = brutex_core::blake3::hash(seal.as_bytes());
            (id, Some((predecessor, seal)))
        }
        _ => return Err("prepare requires both recovery_supersedes and a new canonical 64-hex recovery_generation; neither is inferred from events".to_owned()),
    };
    Ok(Submission {
        id,
        units,
        successor,
        prepare_only,
    })
}

/// Prepare new queued work only. The predecessor remains missing, the active
/// pointer and STOP history do not change, and no source or vendor path runs.
fn prepare_successor(site: &Site, asked: &Submission) -> Result<(), String> {
    let (predecessor, seal) = asked
        .successor
        .as_ref()
        .ok_or("successor identity required")?;
    let history = active_history(site)?;
    if !history.latest.contains_key(predecessor) {
        return Err("successor predecessor was not recorded in active recovery history".to_owned());
    }
    if history.latest.contains_key(&asked.id) {
        return Err("successor was already activated; preparation cannot reset it".to_owned());
    }
    if plan_path(site, *predecessor)
        .try_exists()
        .map_err(failure)?
    {
        return Err(
            "predecessor plan exists; the missing-plan repair cannot replace it".to_owned(),
        );
    }
    crate::recovery_journal::snapshot(&root(site).join("attempts.bin")).map_err(|why| {
        format!("shared recovery attempt history is unavailable; budgets cannot be reset: {why}")
    })?;
    let path = plan_path(site, asked.id);
    let mut journal = if path.try_exists().map_err(failure)? {
        Journal::open_existing(&path).map_err(failure)?
    } else {
        Journal::create_new(&path).map_err(failure)?
    };
    let expected: HashSet<_> = asked.units.iter().map(|row| row.key).collect();
    if journal.latest.contains_key(&CONTROL) {
        validate_seal(&journal.latest, asked.id)?;
    }
    if journal.latest.iter().any(|(key, row)| {
        if *key == CONTROL {
            row.body != *seal || row.status != Status::Queued
        } else {
            !expected.contains(key) || row.status != Status::Queued || row.attempts != 0
        }
    }) {
        return Err(
            "existing successor contains changed scope, activation or work; nothing was reset"
                .to_owned(),
        );
    }
    for row in &asked.units {
        if let Some(existing) = journal.latest.get(&row.key) {
            if existing != row {
                return Err(
                    "existing successor window disagrees with the requested queued scope"
                        .to_owned(),
                );
            }
        } else {
            journal.append(row.clone()).map_err(failure)?;
        }
    }
    let mut control = record(seal.clone());
    control.key = CONTROL;
    if journal.latest.get(&CONTROL) != Some(&control) {
        journal.append(control).map_err(failure)?;
    }
    Ok(())
}

fn preflight_submission(site: &Site, asked: &Submission) -> Result<(), String> {
    preflight_plan(site, asked.id)?;
    if let Some((_, expected_seal)) = &asked.successor {
        // Activation is a separate explicit request against an already sealed
        // generation. A start never prepares it implicitly.
        let journal =
            crate::recovery_journal::snapshot(&plan_path(site, asked.id)).map_err(failure)?;
        if journal
            .latest
            .get(&CONTROL)
            .is_none_or(|row| &row.body != expected_seal)
        {
            return Err("successor must be prepared with its exact predecessor and generation before activation".to_owned());
        }
        if asked.units.iter().any(|row| {
            journal
                .latest
                .get(&row.key)
                .is_none_or(|existing| existing.body != row.body)
        }) {
            return Err("prepared successor scope is incomplete; activation refused".to_owned());
        }
    }
    Ok(())
}

type JsonHeaders = [(axum::http::HeaderName, &'static str); 1];
type RecoveryReply = (StatusCode, JsonHeaders, String);

async fn prepare_reply(site: Loaded, asked: Submission, headers: JsonHeaders) -> RecoveryReply {
    let id = asked.id;
    let count = asked.units.len();
    let predecessor = asked.successor.as_ref().map(|(id, _)| hex(*id));
    // Journal syncs can take time. The blocking worker also survives a browser
    // disconnect; it still cannot claim a run or reach any source/vendor path.
    let prepared = tokio::task::spawn_blocking(move || prepare_successor(&site, &asked))
        .await
        .map_err(failure)
        .and_then(|result| result);
    match prepared {
        Ok(()) => (StatusCode::CREATED, headers, serde_json::json!({
            "started":false,"prepared":true,"plan":hex(id),"windows":count,
            "supersedes":predecessor,
            "predecessor_state":"missing; original completion remains unverified",
            "coverage_certified":false,"requires_explicit_activation":true,
            "active_pointer_changed":false,"shared_attempt_budgets_reset":false
        }).to_string()),
        Err(why) => (StatusCode::SERVICE_UNAVAILABLE, headers, serde_json::json!({"started":false,"prepared":false,"why":why,"coverage_certified":false}).to_string()),
    }
}

fn update(site: &Site, edit: impl FnOnce(&mut Progress)) {
    let mut held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(progress) = held.as_mut() {
        edit(progress);
    }
}

fn claim(site: &Site, recovery: Option<([u8; 32], bool)>) -> Result<(), String> {
    let mut held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if held.as_ref().is_some_and(Progress::running) {
        return Err("a pull already owns the run slot; recovery was not started twice".to_owned());
    }
    if let Some((id, explicit)) = recovery {
        if explicit {
            crate::recovery_control::clear_stop(site, id)?;
        }
        crate::recovery_control::activate(site, id);
    }
    *held = Some(Progress::claimed());
    Ok(())
}

fn stopping(site: &Site) -> bool {
    site.run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .is_some_and(|progress| progress.stopping)
}

/// Explicit entry point. Uses the same encoded legs as /pull/run, but never
/// invokes that route's whole-basket retry loop.
pub(crate) async fn start(State(site): State<Loaded>, body: String) -> RecoveryReply {
    let result = ingest::today_ist()
        .map_err(failure)
        .and_then(|today| submission(&body, today));
    let headers = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];
    let asked = match result {
        Ok(asked) => asked,
        Err(why) => {
            return (
                StatusCode::BAD_REQUEST,
                headers,
                serde_json::json!({"started":false,"why":why}).to_string(),
            );
        }
    };
    let count = asked.units.len();
    let id = asked.id;
    if asked.prepare_only {
        // This branch never claims a run slot, clears STOP, publishes an
        // activation pointer, reads source bars, or enters the worker.
        return prepare_reply(site, asked, headers).await;
    }
    let preflight_site = Loaded::clone(&site);
    let preflight = tokio::task::spawn_blocking(move || {
        preflight_submission(&preflight_site, &asked)?;
        Ok::<_, String>(asked)
    })
    .await
    .map_err(failure)
    .and_then(|result| result);
    let asked = match preflight {
        Ok(asked) => asked,
        Err(why) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                headers,
                serde_json::json!({"started":false,"why":why,"coverage_certified":false})
                    .to_string(),
            );
        }
    };
    if let Err(why) = claim(&site, Some((id, true))) {
        return (
            StatusCode::CONFLICT,
            headers,
            serde_json::json!({"started":false,"why":why}).to_string(),
        );
    }
    // Detach activation from the browser connection: disconnecting while the
    // seed syncs must not leave a claimed run with no worker. The response
    // still waits for the durable activation result, not mere acceptance.
    let units = asked.successor.is_none().then_some(asked.units);
    let prepared = tokio::spawn(activate_durable(site, id, units))
        .await
        .map_err(failure)
        .and_then(|result| result);
    if let Err(why) = prepared {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            headers,
            serde_json::json!({"started":false,"why":why}).to_string(),
        );
    }
    (StatusCode::ACCEPTED, headers, serde_json::json!({"started":true,"windows":count,"plan":hex(id),"status":"/pull/run.json","coverage_certified":false}).to_string())
}

async fn activate_durable(
    site: Loaded,
    id: [u8; 32],
    units: Option<Vec<Record>>,
) -> Result<(), String> {
    let prepare_site = Loaded::clone(&site);
    let prepared = tokio::task::spawn_blocking(move || seeded(&prepare_site, id, units))
        .await
        .map_err(failure)
        .and_then(|result| result);
    let prepared = match prepared {
        Ok(journal) if !stopping(&site) => journal,
        result => {
            let why = result
                .err()
                .unwrap_or_else(|| "stopped during durable activation".to_owned());
            let stop = crate::recovery_control::stop(&site);
            crate::recovery_control::idle(&site);
            let why = stop.err().map_or(why.clone(), |error| {
                format!("{why}; stop persistence: {error}")
            });
            update(&site, |progress| {
                progress.finished = Some(format!("Recovery BLOCKED: {why}"));
            });
            return Err(why);
        }
    };
    let _task = tokio::spawn(drive(site, id, Ok(prepared), true));
    Ok(())
}

/// Only an already-seeded, explicitly activated plan may resume at boot.
/// Closed/blocked/stopped plans never restart themselves or reset budgets.
pub(crate) fn resume(site: Loaded) -> Result<bool, String> {
    if !active_path(&site).try_exists().map_err(failure)? {
        return Ok(false);
    }
    let active = active_history(&site)?;
    let Some(pointer) = active.latest.values().max_by_key(|record| record.attempts) else {
        return Ok(false);
    };
    let id = pointer_identity(&pointer.body)?;
    if pointer.key != id {
        return Err("active recovery identity mismatch".to_owned());
    }
    if crate::recovery_control::is_stopped(&site, id)? {
        return Ok(false);
    }
    let plan = Journal::open_existing(&plan_path(&site, id)).map_err(|why| {
        if why.kind() == std::io::ErrorKind::NotFound {
            missing_plan(id)
        } else {
            failure(why)
        }
    })?;
    validate_seal(&plan.latest, id)?;
    if !plan
        .latest
        .get(&CONTROL)
        .is_some_and(|row| row.status == Status::InFlight)
    {
        return Ok(false);
    }
    drop(plan);
    drop(active);
    claim(&site, Some((id, false)))?;
    let prepared = seeded(&site, id, None);
    let _task = tokio::spawn(drive(site, id, prepared, false));
    Ok(true)
}

fn seeded(site: &Site, id: [u8; 32], units: Option<Vec<Record>>) -> Result<Journal, String> {
    if !site.store_root.is_dir() {
        return Err(
            "configured source store is unavailable; recovery creates no replacement root"
                .to_owned(),
        );
    }
    preflight_plan(site, id)?;
    // Never recursively recreate a disappeared configured store.
    for directory in [site.store_root.join("audit"), root(site)] {
        match std::fs::create_dir(&directory) {
            Ok(()) => {}
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
            Err(why) => return Err(failure(why)),
        }
    }
    let path = plan_path(site, id);
    let mut journal = if path.try_exists().map_err(failure)? || units.is_none() {
        Journal::open_existing(&path).map_err(failure)?
    } else {
        Journal::create_new(&path).map_err(failure)?
    };
    if let Some(units) = units {
        for item in units {
            if !journal.latest.contains_key(&item.key) {
                journal.append(item).map_err(failure)?;
            }
        }
    } else if !journal.latest.contains_key(&CONTROL) {
        return Err("recovery plan was not durably seeded; no vendor work is allowed".to_owned());
    }
    let mut control = journal
        .latest
        .get(&CONTROL)
        .cloned()
        .unwrap_or_else(|| record(CONTROL_BODY.to_owned()));
    control.key = CONTROL;
    control.status = Status::InFlight;
    journal.append(control).map_err(failure)?;
    validate_seal(&journal.latest, id)?;
    // Create the shared budget once, before the first activation. Once any
    // activation exists, absence of this ledger must never buy fresh retries.
    let attempts = root(site).join("attempts.bin");
    if active_path(site).try_exists().map_err(failure)? || attempts.try_exists().map_err(failure)? {
        drop(Journal::open_existing(&attempts).map_err(failure)?);
    } else {
        drop(Journal::create_new(&attempts).map_err(failure)?);
    }
    let mut active = Journal::open(&active_path(site)).map_err(failure)?;
    let seal = journal.latest.get(&CONTROL).ok_or("missing plan seal")?;
    let pointer_body = if seal.body == CONTROL_BODY {
        hex(id)
    } else {
        seal.body.clone()
    };
    let mut pointer = record(pointer_body);
    pointer.key = id;
    pointer.attempts = active
        .latest
        .values()
        .map(|row| row.attempts)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("recovery activation sequence overflow")?;
    active.append(pointer).map_err(failure)?;
    std::fs::File::open(root(site))
        .and_then(|file| file.sync_all())
        .map_err(failure)?;
    std::fs::File::open(site.store_root.join("audit"))
        .and_then(|file| file.sync_all())
        .map_err(failure)?;
    Ok(journal)
}

async fn drive(site: Loaded, id: [u8; 32], prepared: Result<Journal, String>, explicit: bool) {
    // A caught unwind is not a clean finish; durable InFlight reservations are
    // intentionally left for the next process, still charged to their budget.
    let worker_site = Loaded::clone(&site);
    let worker = tokio::spawn(async move {
        let mut journal = prepared?;
        let mut attempts =
            Journal::open_existing(&root(&worker_site).join("attempts.bin")).map_err(failure)?;
        std::fs::File::open(root(&worker_site))
            .and_then(|file| file.sync_all())
            .map_err(failure)?;
        let answer = execute(&worker_site, &mut journal, &mut attempts, explicit).await;
        let mut control = journal
            .latest
            .get(&CONTROL)
            .cloned()
            .ok_or("missing plan seal")?;
        control.status = if answer.is_ok() {
            Status::Verified
        } else {
            Status::Blocked
        };
        journal.append(control).map_err(failure)?;
        answer
    });
    let result = worker.await.map_err(failure).and_then(|result| result);
    let text = result.unwrap_or_else(|why| format!("Recovery BLOCKED: {why}. Existing source data is preserved; this is not complete coverage."));
    crate::recovery_control::idle(&site);
    update(&site, |progress| {
        progress.finished = Some(text.clone());
        for feed in &mut progress.feeds {
            feed.finished = true;
            feed.doing.clear();
        }
    });
    let _ = telemetry::emit(
        &telemetry::Event::info("pull.recovery", "recovery ended")
            .with("plan", telemetry::Value::Str(&hex(id)))
            .with("summary", telemetry::Value::Str(&text)),
    );
}

fn load_lifecycle(
    site: &Site,
    journal: &mut Journal,
    keys: &[[u8; 32]],
) -> Result<Lifecycle, String> {
    let today = ingest::today_ist().map_err(failure)?;
    let master_day = keys
        .iter()
        .filter_map(|id| journal.latest.get(id))
        .map(|row| checked(&row.body, today).map(|asked| asked.window.to()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or("empty recovery plan")?;
    let loaded = pull::cash_session_cache::read_local_lifecycle(
        &site.store_root.join("session-masters"),
        master_day,
    );
    let evidence_body = loaded.as_ref().map_or_else(
        |why| format!("lifecycle UNVERIFIED: {why}"),
        |master| {
            let proof = master.provenance();
            format!(
                "lifecycle snapshot ONLY: {} sha256={} source={}",
                proof.master_day,
                hex(proof.sha256),
                proof.source_url
            )
        },
    );
    let mut evidence = record(evidence_body);
    evidence.status = Status::Unverified;
    journal.append(evidence).map_err(failure)?;
    Ok(loaded.ok())
}

async fn execute(
    site: &Loaded,
    journal: &mut Journal,
    attempts: &mut Journal,
    explicit: bool,
) -> Result<String, String> {
    let starting_rows = crate::pullrun::rows_now(site);
    let keys: Vec<_> = journal
        .order
        .iter()
        .copied()
        .filter(|id| {
            *id != CONTROL
                && journal
                    .latest
                    .get(id)
                    .is_some_and(|row| server::param(&row.body, "recovery_kind") == "scan")
        })
        .collect();
    let lifecycle = load_lifecycle(site, journal, &keys)?;
    reconcile_pending(site, journal, attempts, &keys, &lifecycle, explicit).await?;
    update(site, |progress| {
        progress.rows_at_start = starting_rows;
        progress.rows_now = starting_rows;
        progress.feeds = vec![FeedReport {
            vendor: "zerodha".to_owned(),
            legs: u32::try_from(keys.len()).unwrap_or(u32::MAX),
            ..FeedReport::default()
        }];
    });
    for id in &keys {
        if stopping(site) {
            return Err("stopped at a durable window boundary".to_owned());
        }
        let mut item = journal
            .latest
            .get(id)
            .cloned()
            .ok_or("checkpoint missing")?;
        update(site, |progress| {
            if let Some(feed) = progress.feeds.first_mut() {
                feed.doing = format!(
                    "Audit {} · {} · {}..{}",
                    server::param(&item.body, "member"),
                    server::param(&item.body, "granularity"),
                    server::param(&item.body, "from"),
                    server::param(&item.body, "to")
                );
            }
        });
        // Reconcile each exact window on explicit resume; do not trust an old
        // green checkpoint after independent store changes. Child budgets stay.
        let assessment = assess(site, &item.body, &lifecycle).await;
        match assessment {
            Ok(mut found) => {
                for day in &found.retry_days {
                    retry_day(site, journal, attempts, &item.body, *day, &lifecycle).await?;
                }
                if !found.retry_days.is_empty() {
                    found = assess(site, &item.body, &lifecycle).await?;
                }
                item.status = found.status();
                item.missing = found.missing;
                item.unverified = found.unverified;
                item.diagnostics = found.evidence_issues;
            }
            Err(why) => {
                item.status = Status::Blocked;
                item.diagnostics = 1;
                note(&item.body, &why);
                let asked = checked(&item.body, ingest::today_ist().map_err(failure)?)?;
                site.journal()
                    .append(&crate::audit::Record::member_failure(
                        crate::audit::Scope::Spot,
                        std::time::SystemTime::now(),
                        symbol(&asked)?,
                        asked.window,
                        &why,
                    ))
                    .map_err(failure)?;
            }
        }
        journal.append(item).map_err(failure)?;
        update(site, |progress| {
            if let Some(feed) = progress.feeds.first_mut() {
                feed.legs_done = feed.legs_done.saturating_add(1);
            }
        });
    }
    let mut counts = (0u64, 0u64, 0u64, 0u64);
    for id in keys {
        let row = journal
            .latest
            .get(&id)
            .ok_or("checkpoint missing at summary")?;
        match row.status {
            Status::Verified => counts.0 += 1,
            Status::NotApplicable => counts.1 += 1,
            Status::Unverified => counts.2 += 1,
            _ => counts.3 += 1,
        }
    }
    Ok(format!(
        "Reconciliation finished: {} stored windows verified against measured schedule; {} not applicable; {} unverified; {} missing/blocked. This is NOT complete historical coverage or point-in-time identity proof. Details: /pull/recovery.json and /audit. No source data was replaced.",
        counts.0, counts.1, counts.2, counts.3
    ))
}

fn note(body: &str, why: &str) {
    let _ = telemetry::emit(
        &telemetry::Event::warn("pull.recovery", "window unresolved")
            .with(
                "symbol",
                telemetry::Value::Str(&server::param(body, "member")),
            )
            .with("from", telemetry::Value::Str(&server::param(body, "from")))
            .with("to", telemetry::Value::Str(&server::param(body, "to")))
            .with("why", telemetry::Value::Str(why)),
    );
}

fn append_attempt(plan: &mut Journal, shared: &mut Journal, item: Record) -> Result<(), String> {
    if !shared.latest.contains_key(&item.key) && shared.latest.len() >= MAX_UNITS {
        return Err("shared recovery attempt index reached its 100,000-unit bound".to_owned());
    }
    // The shared reservation is authoritative across overlapping plans. A
    // failed display-journal append cannot refund a request already reserved.
    shared.append(item.clone()).map_err(failure)?;
    plan.append(item).map_err(failure)
}

fn scope_key(asked: &SpotRequest) -> Result<String, String> {
    Ok(format!(
        "{}|{}|{}",
        symbol(asked)?,
        asked.granularity.dir(),
        asked.window.from().year_month().map_err(failure)?
    ))
}

/// Reconcile requests interrupted after storage but before their receipt, even
/// if the parent's gap has disappeared. A lost receipt never invents new-row
/// counts or marks the old request clean. Explicit reactivation may retry a
/// previously blocked unit with its remaining (never refreshed) budget.
async fn reconcile_pending(
    site: &Loaded,
    journal: &mut Journal,
    attempts: &mut Journal,
    keys: &[[u8; 32]],
    lifecycle: &Lifecycle,
    explicit: bool,
) -> Result<(), String> {
    let today = ingest::today_ist().map_err(failure)?;
    let mut scopes: std::collections::HashMap<String, Vec<Window>> =
        std::collections::HashMap::new();
    for key in keys {
        let row = journal.latest.get(key).ok_or("missing scan scope")?;
        let asked = checked(&row.body, today)?;
        scopes
            .entry(scope_key(&asked)?)
            .or_default()
            .push(asked.window);
    }
    let pending: Vec<_> = attempts
        .latest
        .values()
        .filter(|row| {
            matches!(
                row.status,
                Status::Queued | Status::InFlight | Status::Unverified
            ) || (explicit && row.status == Status::Blocked)
        })
        .cloned()
        .collect();
    for mut item in pending {
        if stopping(site) {
            return Err("stopped while reconciling interrupted requests".to_owned());
        }
        let asked = checked(&item.body, today)?;
        if item.key != key(&item.body)
            || asked.window.days() != 1
            || server::param(&item.body, "recovery_kind") != "gap"
        {
            return Err("shared attempt has invalid identity or non-day scope".to_owned());
        }
        if !scopes.get(&scope_key(&asked)?).is_some_and(|windows| {
            windows.iter().any(|window| {
                asked.window.from() >= window.from() && asked.window.to() <= window.to()
            })
        }) {
            continue;
        }
        match assess(site, &item.body, lifecycle).await {
            Ok(found) => {
                item.missing = found.missing;
                item.unverified = found.unverified;
                item.diagnostics = item.diagnostics.saturating_add(found.evidence_issues);
                item.status = if found.retry_days.is_empty() {
                    // Complete source readback does not reconstruct a lost HTTP
                    // receipt, so this state remains explicitly qualified.
                    item.diagnostics = item.diagnostics.saturating_add(1);
                    Status::Unverified
                } else if item.attempts >= ATTEMPT_LIMIT {
                    Status::Exhausted
                } else {
                    Status::Queued
                };
            }
            Err(why) => {
                item.status = Status::Blocked;
                item.diagnostics = item.diagnostics.saturating_add(1);
                note(&item.body, &why);
            }
        }
        append_attempt(journal, attempts, item)?;
    }
    Ok(())
}

#[derive(Default, Debug)]
struct Assessment {
    held: u64,
    missing: u64,
    unverified: u64,
    evidence_issues: u64,
    retry_days: Vec<Day>,
}

impl Assessment {
    fn status(&self) -> Status {
        if self.evidence_issues > 0 {
            Status::Unverified
        } else if self.missing > 0 {
            Status::Exhausted
        } else if self.unverified > 0 {
            Status::Unverified
        } else if self.held == 0 {
            Status::NotApplicable
        } else {
            Status::Verified
        }
    }
}

fn symbol(asked: &SpotRequest) -> Result<&str, String> {
    if asked.members.len() != 1 {
        return Err("recovery unit must name exactly one symbol".to_owned());
    }
    asked
        .members
        .iter()
        .next()
        .map(brutex_core::symbol::Symbol::as_str)
        .ok_or_else(|| "missing symbol".to_owned())
}

fn is_index(symbol: &str) -> bool {
    matches!(symbol, "NIFTY" | "BANKNIFTY")
}

fn source_path(
    root: &Path,
    symbol: &str,
    dir: &str,
    month: store::path::YearMonth,
) -> Result<PathBuf, String> {
    let timeframe = ingest::parse_granularity(dir)
        .and_then(Granularity::store_timeframe)
        .ok_or("invalid source timeframe")?;
    store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: if is_index(symbol) { "INDEX" } else { "CASH" },
        symbol,
        contract: None,
        timeframe,
        month,
        file: store::path::FileKind::Bars,
    })
    .map(|path| path.to_path_buf(root))
    .map_err(failure)
}

fn read_stamps(
    root: &Path,
    symbol: &str,
    dir: &str,
    month: store::path::YearMonth,
) -> Result<Vec<i64>, String> {
    let path = source_path(root, symbol, dir, month)?;
    if !path.try_exists().map_err(failure)? {
        return Ok(Vec::new());
    }
    let timeframe = ingest::parse_granularity(dir)
        .and_then(Granularity::store_timeframe)
        .ok_or("invalid source timeframe")?;
    let file = crate::bars::open(
        root,
        Vendor::Zerodha,
        "NSE",
        if is_index(symbol) { "INDEX" } else { "CASH" },
        symbol,
        timeframe,
        month,
        None,
    )?;
    if file.header().n_valid > 50_000 {
        return Err("source month exceeds the reported recovery read bound".to_owned());
    }
    (0..file.header().n_valid)
        .map(|at| {
            let bar = file.read_record(at).map_err(failure)?;
            if day_of(bar.ts_micros)?.year_month().map_err(failure)? != month {
                return Err(
                    "source timestamp belongs to a different instrument-month window".to_owned(),
                );
            }
            Ok(bar.ts_micros)
        })
        .collect()
}

fn day_of(stamp: i64) -> Result<Day, String> {
    if stamp.rem_euclid(60_000_000) != 0 {
        return Err("off-minute source timestamp".to_owned());
    }
    pull::session::IstMoment::from_epoch_secs(stamp / 1_000_000)
        .map(pull::session::IstMoment::day)
        .map_err(failure)
}

fn days(stamps: &[i64]) -> Result<HashSet<Day>, String> {
    if stamps.windows(2).any(|pair| matches!(pair,[a,b] if a>=b)) {
        return Err("duplicate or unordered source timestamps".to_owned());
    }
    stamps.iter().copied().map(day_of).collect()
}

fn read_sources(
    root: &Path,
    name: &str,
    month: store::path::YearMonth,
    window: Window,
    is_daily: bool,
) -> Result<(Vec<i64>, Vec<i64>), String> {
    let daily = read_stamps(root, name, "1day", month)?;
    let held_days = days(&daily)?;
    if held_days.len() != daily.len() {
        return Err("multiple daily bars name the same date".to_owned());
    }
    let need_minutes = !is_daily
        || (window.from().days_from_epoch()..=window.to().days_from_epoch()).any(|number| {
            Day::from_days(number).is_ok_and(|day| {
                !held_days.contains(&day)
                    && matches!(
                        pull::calendar::kind_of(i64::from(number)),
                        pull::calendar::DayKind::Open(_)
                    )
            })
        });
    let minutes = if need_minutes {
        read_stamps(root, name, "1min", month)?
    } else {
        Vec::new()
    };
    Ok((daily, minutes))
}

async fn assess(site: &Loaded, body: &str, lifecycle: &Lifecycle) -> Result<Assessment, String> {
    let asked = checked(body, ingest::today_ist().map_err(failure)?)?;
    server::recovery_mapping(site, &asked)?;
    let name = symbol(&asked)?.to_owned();
    let month = asked.window.from().year_month().map_err(failure)?;
    if month != asked.window.to().year_month().map_err(failure)? {
        return Err("recovery scan must be contained in one month".to_owned());
    }
    let source = site.store_root.clone();
    let read_name = name.clone();
    let window = asked.window;
    let is_daily = asked.granularity == Granularity::Day1;
    let (daily, minutes) = tokio::task::spawn_blocking(move || {
        read_sources(&source, &read_name, month, window, is_daily)
    })
    .await
    .map_err(failure)??;
    let daily_days = days(&daily)?;
    let minute_days = days(&minutes)?;
    let first = i64::from(asked.window.from().days_from_epoch());
    let last = i64::from(asked.window.to().days_from_epoch());
    if asked.granularity == Granularity::Day1 {
        let mut found = assess_daily(asked.window, &daily_days, &minute_days, &name)?;
        qualify_lifecycle(
            &mut found,
            lifecycle,
            &name,
            asked.window,
            &daily_days,
            &minute_days,
        );
        return Ok(found);
    }
    let schedule = if is_index(&name) {
        None
    } else {
        server::recovery_cash_schedule(site, &name, asked.window).await?
    };
    let bounded: Vec<_> = minutes
        .into_iter()
        .filter(|stamp| {
            day_of(*stamp).is_ok_and(|day| day >= asked.window.from() && day <= asked.window.to())
        })
        .collect();
    let ledger = if is_index(&name) {
        pull::gaps::classify_spot_index_against(&bounded, first, last, None)
    } else {
        pull::gaps::classify_cash(&bounded, first, last, schedule.as_ref())
    };
    if ledger.truncated || ledger.invalid_timestamps != 0 {
        return Err("source gap audit is truncated or has invalid timestamps".to_owned());
    }
    let mut found = Assessment {
        held: bounded.len() as u64,
        evidence_issues: outside_session_count(&bounded, &name, first, last, schedule.as_ref())?,
        ..Assessment::default()
    };
    let mut retry = std::collections::BTreeSet::new();
    for gap in ledger.gaps {
        let day = Day::from_days(u32::try_from(gap.day).map_err(failure)?).map_err(failure)?;
        if inactive(&name, day) {
            if daily_days.contains(&day) || minute_days.contains(&day) {
                found.unverified += u64::from(gap.minutes());
            }
            continue;
        }
        match gap.reason {
            pull::gaps::Reason::VendorHole
                if daily_days.contains(&day) || minute_days.contains(&day) =>
            {
                found.missing += u64::from(gap.minutes());
                retry.insert(day);
            }
            pull::gaps::Reason::VendorHole | pull::gaps::Reason::Unmeasured => {
                found.unverified += u64::from(gap.minutes());
            }
            _ => {}
        }
    }
    found.retry_days = retry.into_iter().collect();
    qualify_lifecycle(
        &mut found,
        lifecycle,
        &name,
        asked.window,
        &daily_days,
        &minute_days,
    );
    Ok(found)
}

/// Empty-input classification is the same independent calendar authority,
/// exposing every expected/closed/unknown interval. A sorted cursor checks
/// actual records against it without inventing a second session timetable.
fn outside_session_count(
    stored: &[i64],
    name: &str,
    first: i64,
    last: i64,
    schedule: Option<&pull::cash_auction::Schedule>,
) -> Result<u64, String> {
    let shape = if is_index(name) {
        pull::gaps::classify_spot_index_against(&[], first, last, None)
    } else {
        pull::gaps::classify_cash(&[], first, last, schedule)
    };
    if shape.truncated || shape.invalid_timestamps > 0 {
        return Err("independent session shape could not be fully checked".to_owned());
    }
    let mut ranges = shape.gaps.iter().peekable();
    let mut conflicts = 0;
    for stamp in stored {
        let at = pull::session::IstMoment::from_epoch_secs(*stamp / 1_000_000).map_err(failure)?;
        let day = i64::from(at.day().days_from_epoch());
        let minute = at.minute_of_day();
        while ranges
            .peek()
            .is_some_and(|gap| gap.day < day || (gap.day == day && u32::from(gap.to) < minute))
        {
            ranges.next();
        }
        match ranges.peek() {
            Some(gap) if gap.day == day && u32::from(gap.from) <= minute => {
                if matches!(
                    gap.reason,
                    pull::gaps::Reason::Closed | pull::gaps::Reason::OutsideWindow
                ) {
                    conflicts += 1;
                }
            }
            _ => return Err("stored minute lies outside the audited session shape".to_owned()),
        }
    }
    Ok(conflicts)
}

fn inactive(symbol: &str, day: Day) -> bool {
    brutex_core::universe::nse_isin(symbol).is_some_and(|isin| {
        pull::cash_auction::independently_inactive(symbol, isin.as_str(), day).is_some()
    })
}

fn qualify_lifecycle(
    found: &mut Assessment,
    lifecycle: &Lifecycle,
    symbol: &str,
    window: Window,
    daily: &HashSet<Day>,
    minutes: &HashSet<Day>,
) {
    if is_index(symbol) {
        return;
    }
    // Sourced interruption overrides neither an actual conflicting source bar
    // nor anything outside its exact inclusive interval.
    for day in daily.union(minutes) {
        if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
            found.evidence_issues += 1;
        }
    }
    let proof = brutex_core::universe::nse_isin(symbol).and_then(|isin| {
        lifecycle
            .as_ref()
            .and_then(|master| master.inspect(symbol, isin.as_str()).ok())
    });
    if let Some(proof) = proof {
        if matches!(
            proof.assessment.quality,
            pull::cash_auction::LifecycleQuality::Contradictory
                | pull::cash_auction::LifecycleQuality::Unverified
        ) {
            found.evidence_issues += 1;
            found.retry_days.clear();
        }
        if let Some(listed) = proof.identity.metadata.listing.date() {
            let conflicts = daily
                .iter()
                .chain(minutes.iter())
                .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
            if conflicts {
                found.evidence_issues += 1;
            }
            // A current listing date never erases an older bar or grants a
            // prelisting exemption. Stop using the current token to guess it.
            found.retry_days.retain(|day| *day >= listed);
        }
        if proof.assessment.quality == pull::cash_auction::LifecycleQuality::Unavailable
            && (found.held > 0 || found.missing > 0)
        {
            found.evidence_issues += 1;
        }
    } else if found.held > 0 || found.missing > 0 {
        found.evidence_issues += 1;
        found.retry_days.clear();
    }
}

fn assess_daily(
    window: Window,
    daily: &HashSet<Day>,
    minutes: &HashSet<Day>,
    symbol: &str,
) -> Result<Assessment, String> {
    let mut found = Assessment::default();
    for number in window.from().days_from_epoch()..=window.to().days_from_epoch() {
        let day = Day::from_days(number).map_err(failure)?;
        if inactive(symbol, day) {
            if daily.contains(&day) || minutes.contains(&day) {
                found.unverified += 1;
            }
            continue;
        }
        match pull::calendar::kind_of(i64::from(number)) {
            pull::calendar::DayKind::Closed => {
                if daily.contains(&day) {
                    return Err(format!("source daily bar on calendar-closed date {day}"));
                }
            }
            pull::calendar::DayKind::Unmeasured | pull::calendar::DayKind::OpenLengthUnmeasured => {
                found.unverified += 1;
            }
            pull::calendar::DayKind::Open(_) => {
                if daily.contains(&day) {
                    found.held += 1;
                } else if minutes.contains(&day) {
                    found.missing += 1;
                    found.retry_days.push(day);
                } else {
                    found.unverified += 1;
                }
            }
        }
    }
    Ok(found)
}

async fn retry_day(
    site: &Loaded,
    journal: &mut Journal,
    attempts: &mut Journal,
    parent: &str,
    day: Day,
    lifecycle: &Lifecycle,
) -> Result<(), String> {
    let asked = checked(parent, ingest::today_ist().map_err(failure)?)?;
    let body = canonical(
        symbol(&asked)?,
        asked.granularity.dir(),
        Window::new(day, day).map_err(failure)?,
        "gap",
    );
    let new = record(body);
    if !journal.latest.contains_key(&new.key) && journal.latest.len() >= MAX_UNITS {
        return Err("recovery reached its 100,000-unit state bound; the remainder was not silently truncated".to_owned());
    }
    let mut item = attempts.latest.get(&new.key).cloned().unwrap_or(new);
    refresh_owed(&mut item);
    if matches!(item.status, Status::Exhausted | Status::Blocked) {
        journal.append(item).map_err(failure)?;
        return Ok(());
    }
    if item.attempts >= ATTEMPT_LIMIT {
        item.status = Status::Exhausted;
        append_attempt(journal, attempts, item)?;
        return Ok(());
    }
    while item.attempts < ATTEMPT_LIMIT {
        if stopping(site) {
            return Err("stopped before the next source request".to_owned());
        }
        if !reserve(&mut item) {
            break;
        }
        append_attempt(journal, attempts, item.clone())?;
        let asked = checked(&item.body, ingest::today_ist().map_err(failure)?)?;
        let run = server::recovery_spot(site, &asked).await?;
        item.committed = item
            .committed
            .saturating_add(run.total.bars_committed as u64);
        item.diagnostics = run.total.failures.len() as u64;
        item.unchanged = if run.total.bars_committed == 0 {
            item.unchanged.saturating_add(1)
        } else {
            0
        };
        item.http_status = run
            .blocked
            .as_ref()
            .map_or(if run.reached > 0 { 200 } else { 502 }, |block| {
                block.code.as_u16()
            });
        if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
            item.status = Status::Blocked;
            append_attempt(journal, attempts, item)?;
            return Err("source request blocked, stopped or credentials unavailable; inspect /audit; budgets preserved".to_owned());
        }
        let current = assess(site, &item.body, lifecycle).await?;
        item.missing = current.missing;
        item.unverified = current.unverified;
        item.diagnostics = item.diagnostics.saturating_add(current.evidence_issues);
        // A clean HTTP receipt is not enough: source readback must also pass.
        item.status = receipt_state(
            item.attempts,
            run.total.balances(),
            current.status(),
            !current.retry_days.is_empty(),
        );
        append_attempt(journal, attempts, item.clone())?;
        let stored_rows = crate::pullrun::rows_now(site);
        update(site, |progress| {
            progress.rows_now = stored_rows;
            progress.retries = progress
                .retries
                .saturating_add(u32::from(item.attempts > 1));
        });
        if item.status != Status::Queued {
            break;
        }
        tokio::time::sleep(crate::pullrun::RETRY_WAIT).await;
    }
    Ok(())
}

#[derive(Debug, Default)]
struct EventView {
    records: Vec<Record>,
    plan: Option<[u8; 32]>,
    predecessor: Option<[u8; 32]>,
}

fn latest_view(site: &Site) -> Result<EventView, String> {
    if !active_path(site).try_exists().map_err(failure)? {
        return Ok(EventView::default());
    }
    let pointer = crate::recovery_journal::tail(&active_path(site), 1)
        .map_err(failure)?
        .into_iter()
        .next()
        .ok_or("active recovery pointer is empty")?;
    let id = pointer_identity(&pointer.body)?;
    if id != pointer.key {
        return Err("active recovery identity mismatch".to_owned());
    }
    let records = crate::recovery_journal::tail(&plan_path(site, id), 100).map_err(|why| {
        if why.kind() == std::io::ErrorKind::NotFound {
            missing_plan(id)
        } else {
            failure(why)
        }
    })?;
    if crate::recovery_journal::tail(&active_path(site), 1)
        .map_err(failure)?
        .first()
        != Some(&pointer)
    {
        return Err("active recovery changed while reading events; retry this snapshot".to_owned());
    }
    let predecessor = if pointer.body.len() == 64 {
        None
    } else {
        Some(successor_identity(&pointer.body)?.1)
    };
    Ok(EventView {
        records,
        plan: Some(id),
        predecessor,
    })
}

/// Bounded recent EVENTS, not a replay or a claim to be the current inventory.
/// Source inspection and vendor requests are never performed by this GET.
pub(crate) async fn recent(
    State(site): State<Loaded>,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let result = latest_view(&site);
    let headers = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];
    match result {
        Ok(view) => (StatusCode::OK,headers,serde_json::json!({
            "view":"recent journal events, newest first; not current inventory; an active write may still be synchronizing",
            "coverage_certified":false,
            "plan":view.plan.map(hex),
            "supersedes":view.predecessor.map(hex),
            "predecessor_completion_reconstructed":false,
            "limit":100,
            "events":view.records.iter().map(|row|serde_json::json!({
                "key":hex(row.key),"request":row.body,"state":format!("{:?}",row.status),
                "attempts":row.attempts,"unchanged":row.unchanged,"new_bars":row.committed,
                "diagnostics":row.diagnostics,"missing":row.missing,"unverified":row.unverified,"http_status":row.http_status
            })).collect::<Vec<_>>()
        }).to_string()),
        Err(why) => (StatusCode::SERVICE_UNAVAILABLE,headers,serde_json::json!({"error":why,"recovery_state":"history_unavailable","coverage_certified":false,"sweep_input_verdict":"not_evaluated"}).to_string()),
    }
}

/// Server-rendered, read-only comparison view. No second frontend runtime or
/// background loop; the existing run slot remains the single owner.
pub(crate) async fn page(State(site): State<Loaded>) -> axum::response::Html<String> {
    let held = site
        .run
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let progress = held.clone().unwrap_or_default();
    drop(held);
    let (records, predecessor) = match latest_view(&site) {
        Ok(view) => (Ok(view.records), view.predecessor),
        Err(why) => (Err(why), None),
    };
    axum::response::Html(page_html(&progress, records, predecessor))
}

fn page_html(
    progress: &Progress,
    records: Result<Vec<Record>, String>,
    predecessor: Option<[u8; 32]>,
) -> String {
    use crate::render::escape;
    use core::fmt::Write as _;
    let refresh = if progress.running() {
        "<meta http-equiv=\"refresh\" content=\"15\">"
    } else {
        ""
    };
    let mut out = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">{refresh}<title>Pull recovery | brutex</title>
<style>body{{margin:0;background:#f3f7fb;color:#173d59;font:16px/1.5 system-ui,sans-serif}}main{{max-width:1200px;margin:auto;padding:24px}}nav{{display:flex;gap:24px;margin-bottom:32px;flex-wrap:wrap}}a{{color:#0b7285}}a:focus-visible{{outline:3px solid #945900;outline-offset:4px}}h1{{font-size:32px;margin-bottom:8px}}p{{max-width:78ch}}.summary{{padding:18px;background:white;border-left:5px solid #0b7285}}.notice{{color:#945900}}.scroll{{overflow:auto;background:white}}table{{border-collapse:collapse;width:100%;font-size:14px}}caption{{text-align:left;padding:16px;font-weight:600}}th,td{{padding:12px;text-align:left;border-bottom:1px solid #dce5ec;white-space:nowrap}}th{{background:#eaf1f7}}td.num{{text-align:right;font-variant-numeric:tabular-nums}}.state{{font-weight:600}}.bad{{color:#a12a2a}}footer{{margin-top:24px;font-size:14px}}@media(max-width:640px){{main{{padding:14px}}h1{{font-size:26px}}}}</style></head><body><main>
<nav aria-label="Application"><strong>brutex</strong><a href="/ingest">Data pull</a><a href="/audit">Audit records</a><a href="/logs">Logs</a><a href="/pull/recovery">Refresh</a></nav>
<h1>Pull recovery</h1><p>Exact windows. Recorded attempts. No invented candles.</p>"#
    );
    if let Some(predecessor) = predecessor {
        let _ = write!(
            out,
            "<p class=\"notice\">This generation was prepared because original plan <code>{}</code> was missing. Original completion was not reconstructed. These are separate assessments with the shared retry budgets preserved.</p>",
            hex(predecessor)
        );
    }
    let summary = progress
        .finished
        .as_deref()
        .unwrap_or(if progress.running() {
            "Recovery is running. This page refreshes every 15 seconds."
        } else if records.is_err() {
            "Recovery history is unavailable. No running recovery or completed reconciliation is claimed."
        } else {
            "No recovery is running in this process. Recorded progress below remains inspectable."
        });
    let _ = write!(out, "<p class=\"summary\">{}</p>", escape(summary));
    for feed in &progress.feeds {
        let _ = write!(
            out,
            "<p>{}: {} of {} scan windows checked. {}</p>",
            escape(&feed.vendor),
            feed.legs_done,
            feed.legs,
            escape(&feed.doing)
        );
    }
    out.push_str("<p class=\"notice\">Verified means the stored window passed the measured schedule checks, not proof of complete historical trading or identity. Missing and unverified count daily bars for 1day, minutes for 1min. Retry limit reached is not success.</p><div class=\"scroll\" tabindex=\"0\" role=\"region\" aria-label=\"Recovery events\"><table><caption>Latest 100 journal events (repeated windows are separate events; active writes may still be synchronizing)</caption><thead><tr><th>Symbol</th><th>Source</th><th>From</th><th>Through</th><th>Work</th><th>State</th><th>Attempts</th><th>New bars</th><th>Missing</th><th>Unverified</th><th>Diagnostics</th></tr></thead><tbody>");
    match records {
        Ok(records) => {
            let mut shown = 0;
            for row in records
                .iter()
                .filter(|row| !server::param(&row.body, "member").is_empty())
            {
                shown += 1;
                out.push_str("<tr>");
                for field in ["member", "granularity", "from", "to", "recovery_kind"] {
                    let _ = write!(out, "<td>{}</td>", escape(&server::param(&row.body, field)));
                }
                let label = match row.status {
                    Status::Queued => "Queued",
                    Status::InFlight => "Attempt reserved",
                    Status::Verified => "Stored window checked",
                    Status::NotApplicable => "Not applicable",
                    Status::Unverified => "Unverified",
                    Status::Exhausted => "Missing / retry limit",
                    Status::Blocked => "Blocked",
                };
                let _ = write!(
                    out,
                    "<td class=\"state\">{label}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
                    row.attempts, row.committed, row.missing, row.unverified, row.diagnostics
                );
            }
            if shown == 0 {
                out.push_str("<tr><td colspan=\"11\">No window events are available yet. No data completeness is claimed.</td></tr>");
            }
        }
        Err(why) => {
            let _ = write!(
                out,
                "<tr><td colspan=\"11\" class=\"bad\">Snapshot unavailable: {}. A missing plan needs an explicit new generation; refreshing cannot recreate original history. This recovery status does not evaluate any selected backtest input.</td></tr>",
                escape(&why)
            );
        }
    }
    out.push_str("</tbody></table></div><footer>Only requested Zerodha spot data is used. Original source files are preserved. No automatic correction promotion or other-feed substitution. <a href=\"/pull/recovery.json\">Read the machine-readable events</a>.</footer></main></body></html>");
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> Day {
        Day::new(year, month, day).unwrap()
    }
    fn leg(symbol: &str, dir: &str, from: Day, to: Day) -> Leg {
        Leg {
            route: Route::Spot,
            vendor: "zerodha".to_owned(),
            dir: dir.to_owned(),
            label: "fixture".to_owned(),
            body: canonical(symbol, dir, Window::new(from, to).unwrap(), "scan"),
        }
    }
    fn today() -> Day {
        date(2026, 9, 6)
    }

    #[test]
    fn exact_scope_order_dedup_and_partial_month_bounds() {
        let d = leg("NIFTY", "1day", date(2019, 12, 1), date(2026, 9, 4));
        let m = leg("M&M", "1min", date(2019, 12, 1), date(2026, 9, 4));
        let a = plan(vec![m.clone(), d.clone(), d.clone()], today()).unwrap();
        let b = plan(vec![d, m], today()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 164);
        assert_eq!(server::param(&a[0].body, "granularity"), "1day");
        assert_eq!(server::param(&a[0].body, "from"), "2019-12-01");
        let last = checked(&a.last().unwrap().body, today()).unwrap();
        assert_eq!(last.window.to(), date(2026, 9, 4));
        assert_eq!(symbol(&last).unwrap(), "M&M");
        assert!(
            a.iter()
                .all(|r| checked(&r.body, today()).unwrap().window.days() <= 31)
        );
    }

    #[test]
    fn bad_feed_empty_basket_excluded_index_and_wrong_envelope_refuse() {
        let good = leg("NIFTY", "1min", date(2026, 8, 1), date(2026, 8, 31));
        for old in ["zerodha", "zerodha_cross_checked", "NIFTY", "1min"] {
            let mut bad = good.clone();
            bad.body = bad.body.replace(old, "invalid");
            assert!(plan(vec![bad], today()).is_err());
        }
        let mut no_member = good.clone();
        no_member.body = no_member.body.replace("&member=NIFTY", "");
        assert!(plan(vec![no_member], today()).is_err());
        for excluded in ["FINNIFTY", "MIDCPNIFTY", "NIFTYNXT50", "NIFTYNEXT50"] {
            let bad = leg(excluded, "1min", date(2026, 8, 1), date(2026, 8, 31));
            assert!(plan(vec![bad], today()).is_err());
        }
        let mut bad = good;
        bad.dir = "1day".to_owned();
        assert!(plan(vec![bad], today()).is_err());
    }

    #[test]
    fn seeding_reuses_exact_budgets_and_syncs_activation_without_vendor_access() {
        let root = crate::scratch::path("recovery-seed");
        std::fs::create_dir_all(&root).unwrap();
        let site = Site::load(&root.join("missing-masters"), &root);
        let units = plan(
            vec![leg("NIFTY", "1min", date(2026, 8, 27), date(2026, 8, 27))],
            today(),
        )
        .unwrap();
        let id = scope_identity(&units);
        let mut journal = seeded(&site, id, Some(units.clone())).unwrap();
        let mut row = units[0].clone();
        assert!(reserve(&mut row));
        journal.append(row.clone()).unwrap();
        assert!(Journal::open(&plan_path(&site, id)).is_err());
        drop(journal);
        let journal = seeded(&site, id, Some(units)).unwrap();
        assert_eq!(journal.latest.get(&row.key), Some(&row));
        assert_eq!(journal.latest[&CONTROL].status, Status::InFlight);
        let active = Journal::open(&active_path(&site)).unwrap();
        assert_eq!(active.latest[&id].attempts, 2);
        assert_eq!(active.latest[&id].body, hex(id));
        drop(active);
        drop(journal);
        assert!(seeded(&site, key("unseeded"), None).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn start_acknowledges_a_saved_plan_and_read_only_fixture_finishes() {
        let root = crate::scratch::path("recovery-start-accepted");
        std::fs::create_dir_all(&root).unwrap();
        let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
        let inner = canonical(
            "NIFTY",
            "1day",
            Window::new(date(2026, 8, 30), date(2026, 8, 30)).unwrap(),
            "scan",
        );
        let body = format!(
            "leg={}",
            encode(&format!(
                "/pull/spot|zerodha|1day|fixture|{}",
                encode(&inner)
            ))
        );
        let (status, _, answer) = start(State(Loaded::clone(&site)), body).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{answer}");
        let reply: serde_json::Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(reply["windows"], 1);
        let id = unhex(reply["plan"].as_str().unwrap()).unwrap();
        assert!(plan_path(&site, id).is_file());
        assert!(active_path(&site).is_file());
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while site.run.lock().unwrap().as_ref().unwrap().running() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let journal = Journal::open(&plan_path(&site, id)).unwrap();
        assert!(
            journal
                .latest
                .values()
                .any(|row| server::param(&row.body, "member") == "NIFTY")
        );
        assert!(matches!(
            journal.latest[&CONTROL].status,
            Status::Verified | Status::Blocked
        ));
        assert!(
            site.run
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .finished
                .as_ref()
                .unwrap()
                .contains("NOT complete historical coverage")
        );
        assert!(
            !root.join("bars").exists(),
            "test constructor cannot reach live vendor"
        );
        drop(journal);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn stop_during_seed_cannot_be_cleared_by_seed_or_boot_resume() {
        let root = crate::scratch::path("recovery-stop-before-seed");
        std::fs::create_dir_all(&root).unwrap();
        let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
        let units = plan(
            vec![leg("NIFTY", "1day", date(2026, 8, 30), date(2026, 8, 30))],
            today(),
        )
        .unwrap();
        let id = scope_identity(&units);
        claim(&site, Some((id, true))).unwrap();
        let (status, _, _) = server::pull_run_stop(State(Loaded::clone(&site))).await;
        assert_eq!(status, StatusCode::OK);
        drop(seeded(&site, id, Some(units)).unwrap());
        assert!(crate::recovery_control::is_stopped(&site, id).unwrap());
        let restarted = Loaded::new(Site::load(&root.join("missing-masters"), &root));
        assert!(!resume(restarted).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn invalid_start_missing_pointer_and_duplicate_claim_do_not_start_work() {
        let root = crate::scratch::path("recovery-no-work");
        let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
        assert!(!resume(Loaded::clone(&site)).unwrap());
        let response = start(State(Loaded::clone(&site)), "invalid=scope".to_owned()).await;
        assert_eq!(response.0, StatusCode::BAD_REQUEST);
        assert!(response.2.contains("\"started\":false"));
        assert!(!root.exists());
        claim(&site, None).unwrap();
        assert!(claim(&site, None).is_err());
        assert!(seeded(&site, key("missing-root"), Some(Vec::new())).is_err());
        assert!(!root.exists());
    }

    #[tokio::test]
    async fn recovery_boundary_events_are_read_back_from_the_installed_sink() {
        let start = crate::emitted::mark();
        let body = canonical(
            "NIFTY",
            "1min",
            Window::new(date(2026, 8, 27), date(2026, 8, 27)).unwrap(),
            "scan",
        );
        note(&body, "recovery fixture: source evidence unresolved");
        let events = crate::emitted::landed(start, "pull.recovery", "window unresolved");
        assert!(
            events
                .iter()
                .any(|event| crate::emitted::says(event, "symbol", "NIFTY")
                    && crate::emitted::says(
                        event,
                        "why",
                        "recovery fixture: source evidence unresolved"
                    ))
        );
        let root = crate::scratch::path("recovery-event-refusal");
        let site = Loaded::new(Site::load(&root.join("absent"), &root));
        claim(&site, None).unwrap();
        let start = crate::emitted::mark();
        let id = key("recovery event fixture");
        drive(
            Loaded::clone(&site),
            id,
            Err("fixture store unavailable".to_owned()),
            true,
        )
        .await;
        let events = crate::emitted::landed(start, "pull.recovery", "recovery ended");
        assert!(
            events
                .iter()
                .any(|event| crate::emitted::says(event, "plan", &hex(id))
                    && crate::emitted::says(event, "summary", "Recovery BLOCKED"))
        );
        assert!(!site.run.lock().unwrap().as_ref().unwrap().running());
        assert!(!root.exists());
    }

    #[test]
    fn budgets_survive_restart_and_interrupted_attempts_are_charged() {
        let dir = crate::scratch::path("recovery-reserve-budget");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.bin");
        let mut item = record(canonical(
            "NIFTY",
            "1min",
            Window::new(date(2026, 8, 27), date(2026, 8, 27)).unwrap(),
            "gap",
        ));
        for attempt in 1..=ATTEMPT_LIMIT {
            let mut journal = Journal::open(&path).unwrap();
            if let Some(previous) = journal.latest.get(&item.key) {
                item = previous.clone();
            }
            assert!(reserve(&mut item));
            assert_eq!(item.attempts, attempt);
            journal.append(item.clone()).unwrap();
            // Simulate shutdown AFTER reservation and before any receipt.
        }
        let journal = Journal::open(&path).unwrap();
        let mut restored = journal.latest[&item.key].clone();
        assert!(!reserve(&mut restored));
        assert_eq!(restored.status, Status::Exhausted);
        assert_eq!(restored.attempts, 3);
        drop(journal);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn failed_receipts_and_unknown_coverage_never_become_verified() {
        for balances in [true, false] {
            for source in [
                Status::Verified,
                Status::Unverified,
                Status::Exhausted,
                Status::NotApplicable,
            ] {
                let result = receipt_state(1, balances, source, true);
                assert_eq!(
                    result == Status::Verified,
                    balances && source == Status::Verified
                );
                if result != Status::Verified {
                    assert_eq!(receipt_state(3, balances, source, true), Status::Exhausted);
                }
            }
        }
        for state in [Status::Verified, Status::Blocked, Status::Exhausted] {
            let mut item = record("fixture".to_owned());
            item.status = state;
            assert!(!reserve(&mut item));
            assert_eq!(item.attempts, 0);
        }
    }

    #[test]
    fn repaired_gaps_stop_without_laundering_lifecycle_uncertainty() {
        assert_eq!(
            receipt_state(1, true, Status::Unverified, false),
            Status::Unverified
        );
        assert_eq!(
            receipt_state(1, false, Status::Verified, false),
            Status::Unverified
        );
        assert_eq!(
            receipt_state(1, true, Status::Verified, false),
            Status::Verified
        );
        let mut previous = record("fixture".to_owned());
        previous.attempts = 1;
        previous.status = Status::Verified;
        refresh_owed(&mut previous);
        assert!(reserve(&mut previous));
        assert_eq!(previous.attempts, 2);
        previous.status = Status::Exhausted;
        refresh_owed(&mut previous);
        assert!(!reserve(&mut previous));
    }

    #[test]
    fn overlapping_plans_cannot_purchase_more_attempts_for_the_same_gap() {
        let root = crate::scratch::path("recovery-shared-budget");
        std::fs::create_dir_all(&root).unwrap();
        let mut first = Journal::open(&root.join("first.bin")).unwrap();
        let mut second = Journal::open(&root.join("second.bin")).unwrap();
        let mut shared = Journal::open(&root.join("attempts.bin")).unwrap();
        let day = date(2026, 8, 27);
        let mut item = record(canonical(
            "NIFTY",
            "1min",
            Window::new(day, day).unwrap(),
            "gap",
        ));
        for attempt in 1..=ATTEMPT_LIMIT {
            assert!(reserve(&mut item));
            assert_eq!(item.attempts, attempt);
            append_attempt(&mut first, &mut shared, item.clone()).unwrap();
            item.status = if attempt == ATTEMPT_LIMIT {
                Status::Exhausted
            } else {
                Status::Queued
            };
            append_attempt(&mut first, &mut shared, item.clone()).unwrap();
        }
        drop(shared);
        let mut shared = Journal::open(&root.join("attempts.bin")).unwrap();
        let mut same = shared.latest[&item.key].clone();
        refresh_owed(&mut same);
        assert!(!reserve(&mut same));
        append_attempt(&mut second, &mut shared, same.clone()).unwrap();
        assert_eq!(second.latest[&item.key].attempts, 3);
        assert_eq!(second.latest[&item.key].body, first.latest[&item.key].body);
        drop(first);
        drop(second);
        drop(shared);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closed_or_outside_session_source_minutes_never_verify() {
        fn at(day: Day, minute: i64) -> i64 {
            (i64::from(day.days_from_epoch()) * 86_400 + minute * 60
                - pull::session::IST_OFFSET_SECS)
                * 1_000_000
        }
        let sunday = date(2026, 8, 30);
        let number = i64::from(sunday.days_from_epoch());
        let bad = outside_session_count(&[at(sunday, 555)], "NIFTY", number, number, None).unwrap();
        assert_eq!(bad, 1);
        let found = Assessment {
            held: 1,
            evidence_issues: bad,
            ..Assessment::default()
        };
        assert_eq!(found.status(), Status::Unverified);
        let monday = date(2026, 8, 31);
        let number = i64::from(monday.days_from_epoch());
        let mut minutes: Vec<_> = (555..930).map(|m| at(monday, m)).collect();
        assert_eq!(
            outside_session_count(&minutes, "NIFTY", number, number, None).unwrap(),
            0
        );
        minutes.push(at(monday, 1000));
        assert_eq!(
            outside_session_count(&minutes, "NIFTY", number, number, None).unwrap(),
            1
        );
    }

    #[test]
    fn absent_stock_history_is_unknown_not_a_retry_or_exemption() {
        let day = date(2026, 8, 27);
        let window = Window::new(day, day).unwrap();
        let empty = HashSet::new();
        let unknown = assess_daily(window, &empty, &empty, "ABB").unwrap();
        assert_eq!(unknown.status(), Status::Unverified);
        assert_eq!(unknown.unverified, 1);
        assert!(unknown.retry_days.is_empty());
        let witnessed = HashSet::from([day]);
        let missing = assess_daily(window, &empty, &witnessed, "ABB").unwrap();
        assert_eq!(missing.missing, 1);
        assert_eq!(missing.retry_days, vec![day]);
        let good = assess_daily(window, &witnessed, &empty, "ABB").unwrap();
        assert_eq!(good.status(), Status::Verified);
    }

    #[test]
    fn sourced_withdrawal_is_exact_and_conflicting_source_is_preserved() {
        let day = date(2024, 1, 3);
        let window = Window::new(day, day).unwrap();
        let empty = HashSet::new();
        let inactive = assess_daily(window, &empty, &empty, "FORCEMOT").unwrap();
        assert_eq!(inactive.status(), Status::NotApplicable);
        let conflict = assess_daily(window, &HashSet::from([day]), &empty, "FORCEMOT").unwrap();
        assert_eq!(conflict.status(), Status::Unverified);
        let other = assess_daily(window, &empty, &empty, "ABB").unwrap();
        assert_eq!(other.status(), Status::Unverified);
    }

    #[test]
    fn session_boundaries_closed_unknown_and_duplicates_are_not_silent() {
        let sunday = date(2026, 8, 30);
        let closed = Window::new(sunday, sunday).unwrap();
        let empty = HashSet::new();
        assert_eq!(
            assess_daily(closed, &empty, &empty, "ABB")
                .unwrap()
                .status(),
            Status::NotApplicable
        );
        assert!(assess_daily(closed, &HashSet::from([sunday]), &empty, "ABB").is_err());
        let muhurat = date(2024, 11, 1);
        assert_eq!(
            assess_daily(
                Window::new(muhurat, muhurat).unwrap(),
                &empty,
                &empty,
                "ABB"
            )
            .unwrap()
            .status(),
            Status::Unverified
        );
        assert!(days(&[60_000_000, 60_000_000]).is_err());
        assert!(days(&[120_000_000, 60_000_000]).is_err());
        assert!(days(&[60_000_001]).is_err());
    }

    #[test]
    fn fingerprints_bind_window_rung_symbol_and_roundtrip_strictly() {
        let window = Window::new(date(2026, 8, 27), date(2026, 8, 27)).unwrap();
        let base = key(&canonical("NIFTY", "1min", window, "gap"));
        assert_ne!(base, key(&canonical("BANKNIFTY", "1min", window, "gap")));
        assert_ne!(base, key(&canonical("NIFTY", "1day", window, "gap")));
        assert_ne!(base, key(&canonical("NIFTY", "1min", window, "scan")));
        assert_eq!(unhex(&hex(base)).unwrap(), base);
        for bad in ["..", "abcd", &"G".repeat(64), &hex(base).to_uppercase()] {
            assert!(unhex(bad).is_err());
        }
    }

    #[test]
    fn status_page_escapes_payload_and_never_calls_an_empty_state_complete() {
        let mut progress = Progress::claimed();
        progress.finished = Some("<script>bad()</script>".to_owned());
        let html = page_html(
            &progress,
            Ok(vec![record(
                "member=%3Cscript%3E&granularity=1min".to_owned(),
            )]),
            None,
        );
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("Latest 100 journal events"));
        assert!(html.contains("not proof of complete historical"));
        let empty = page_html(&Progress::default(), Ok(Vec::new()), None);
        assert!(empty.contains("No window events"));
        assert!(!empty.contains("http-equiv=\"refresh\""));
        let busy = page_html(
            &Progress::claimed(),
            Err("concurrent write".to_owned()),
            None,
        );
        assert!(busy.contains("Snapshot unavailable"));
        assert!(busy.contains("http-equiv=\"refresh\""));
    }

    #[tokio::test]
    async fn missing_active_plan_is_503_and_visible_without_creating_or_mutating_history() {
        use std::collections::BTreeMap;
        use std::os::unix::fs::MetadataExt as _;

        let scratch = crate::scratch::path("recovery-missing-active-plan-observation");
        std::fs::create_dir(&scratch).unwrap();
        let site = Loaded::new(Site::load(&scratch.join("missing-masters"), &scratch));
        let directory = root(&site);
        std::fs::create_dir_all(&directory).unwrap();
        let id = key("generated missing active plan observation fixture");
        let mut pointer = record(hex(id));
        pointer.key = id;
        pointer.attempts = 2;
        let mut active = Journal::open(&active_path(&site)).unwrap();
        active.append(pointer).unwrap();
        drop(active);
        crate::recovery_control::clear_stop(&site, id).unwrap();
        let mut attempts = Journal::open(&directory.join("attempts.bin")).unwrap();
        let mut attempt = record("generated-observation-attempt".to_owned());
        attempt.status = Status::Blocked;
        attempt.attempts = 2;
        attempts.append(attempt).unwrap();
        drop(attempts);
        let snapshot = || {
            std::fs::read_dir(&directory)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    let metadata = entry.metadata().unwrap();
                    (
                        entry.file_name(),
                        (
                            std::fs::read(entry.path()).unwrap(),
                            metadata.ino(),
                            metadata.mtime(),
                            metadata.mtime_nsec(),
                            metadata.ctime(),
                            metadata.ctime_nsec(),
                        ),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        };
        let before = snapshot();
        assert_eq!(before.len(), 3);
        assert!(!plan_path(&site, id).exists());
        assert!(site.run.lock().unwrap().is_none());
        let expected = latest_view(&site).unwrap_err();
        let (status, _, raw) = recent(State(Loaded::clone(&site))).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let response: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["coverage_certified"], false);
        assert_eq!(response["error"], expected);
        assert!(response.get("events").is_none());
        let html = page(State(Loaded::clone(&site))).await.0;
        assert!(html.contains("Snapshot unavailable"));
        assert!(!html.contains("No window events are available yet"));
        assert!(!plan_path(&site, id).exists());
        assert!(site.run.lock().unwrap().is_none());
        assert_eq!(
            snapshot(),
            before,
            "reads must not repair or rewrite history"
        );
        std::fs::remove_dir_all(scratch).unwrap();
    }

    fn encoded_leg(leg: &Leg) -> String {
        format!(
            "leg={}",
            encode(&format!(
                "/pull/spot|zerodha|{}|fixture|{}",
                leg.dir,
                encode(&leg.body)
            ))
        )
    }

    fn missing_fixture(name: &str) -> (PathBuf, Loaded, String, [u8; 32]) {
        let scratch = crate::scratch::path(name);
        std::fs::create_dir(&scratch).unwrap();
        let site = Loaded::new(Site::load(&scratch.join("missing-masters"), &scratch));
        let selected = leg("NIFTY", "1day", date(2026, 8, 30), date(2026, 8, 30));
        let body = encoded_leg(&selected);
        let units = plan(vec![selected], today()).unwrap();
        let id = scope_identity(&units);
        drop(seeded(&site, id, Some(units)).unwrap());
        crate::recovery_control::clear_stop(&site, id).unwrap();
        let mut attempts = Journal::open_existing(&root(&site).join("attempts.bin")).unwrap();
        let mut old = record(canonical(
            "NIFTY",
            "1day",
            Window::new(date(2026, 8, 30), date(2026, 8, 30)).unwrap(),
            "gap",
        ));
        old.status = Status::Exhausted;
        old.attempts = 3;
        attempts.append(old).unwrap();
        drop(attempts);
        std::fs::remove_file(plan_path(&site, id)).unwrap();
        (scratch, site, body, id)
    }

    fn history_bytes(site: &Site) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
        std::fs::read_dir(root(site))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), std::fs::read(entry.path()).unwrap())
            })
            .collect()
    }

    fn successor_form(
        body: &str,
        predecessor: [u8; 32],
        generation: [u8; 32],
        prepare: bool,
    ) -> String {
        format!(
            "{body}&recovery_action={}&recovery_supersedes={}&recovery_generation={}",
            if prepare { "prepare" } else { "start" },
            hex(predecessor),
            hex(generation)
        )
    }

    #[tokio::test]
    async fn missing_activated_plan_cannot_be_recreated_by_boot_or_same_scope_start() {
        let (scratch, site, body, id) = missing_fixture("recovery-lost-plan-refusal");
        for stopped in [false, true] {
            if stopped {
                crate::recovery_control::activate(&site, id);
                crate::recovery_control::stop(&site).unwrap();
                crate::recovery_control::idle(&site);
            }
            let before = history_bytes(&site);
            if stopped {
                assert!(!resume(Loaded::clone(&site)).unwrap());
            } else {
                assert!(
                    resume(Loaded::clone(&site))
                        .unwrap_err()
                        .contains("missing after a recorded activation")
                );
            }
            let (status, _, answer) = start(State(Loaded::clone(&site)), body.clone()).await;
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{answer}");
            assert!(answer.contains("original window states"));
            assert!(!plan_path(&site, id).exists());
            assert!(site.run.lock().unwrap().is_none());
            assert_eq!(history_bytes(&site), before);
        }
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[tokio::test]
    async fn successor_preparation_is_idempotent_separate_from_activation_and_keeps_old_budgets() {
        let (scratch, site, body, old) = missing_fixture("recovery-successor-prepare");
        let request = successor_form(&body, old, [17; 32], true);
        let asked = submission(&request, today()).unwrap();
        let before = history_bytes(&site);
        assert_ne!(asked.id, old);
        let (status, _, response) = start(State(Loaded::clone(&site)), request.clone()).await;
        assert_eq!(status, StatusCode::CREATED, "{response}");
        let response: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["started"], false);
        assert_eq!(response["prepared"], true);
        assert_eq!(response["coverage_certified"], false);
        assert_eq!(response["supersedes"], hex(old));
        assert_eq!(response["active_pointer_changed"], false);
        assert!(site.run.lock().unwrap().is_none());
        assert!(!plan_path(&site, old).exists());
        for (name, bytes) in &before {
            assert_eq!(std::fs::read(root(&site).join(name)).unwrap(), *bytes);
        }
        let prepared = history_bytes(&site);
        assert_eq!(
            start(State(Loaded::clone(&site)), request).await.0,
            StatusCode::CREATED
        );
        assert_eq!(
            history_bytes(&site),
            prepared,
            "repeat preparation appends nothing"
        );
        let saved = crate::recovery_journal::snapshot(&plan_path(&site, asked.id)).unwrap();
        assert_eq!(saved.latest.len(), asked.units.len() + 1);
        assert!(
            saved
                .latest
                .values()
                .all(|row| row.status == Status::Queued && row.attempts == 0)
        );
        validate_seal(&saved.latest, asked.id).unwrap();
        assert_eq!(active_history(&site).unwrap().latest.len(), 1);
        assert!(
            resume(Loaded::clone(&site)).is_err(),
            "prepared generation did not replace missing active history"
        );
        assert!(site.run.lock().unwrap().is_none());

        // Exercise only durable activation, not the vendor-owning worker.
        let mut activated = seeded(&site, asked.id, None).unwrap();
        assert_eq!(activated.latest[&CONTROL].status, Status::InFlight);
        assert_eq!(activated.latest[&CONTROL].body, asked.successor.unwrap().1);
        let mut terminal = activated.latest[&CONTROL].clone();
        terminal.status = Status::Blocked;
        activated.append(terminal).unwrap();
        for _ in 0..110 {
            let mut observed = asked.units[0].clone();
            observed.status = Status::Unverified;
            activated.append(observed).unwrap();
        }
        drop(activated);
        let active = active_history(&site).unwrap();
        assert_eq!(active.latest.len(), 2);
        assert_eq!(active.latest[&asked.id].attempts, 2);
        assert!(
            unhex(&active.latest[&asked.id].body).is_err(),
            "older readers must refuse the new descriptor"
        );
        assert_eq!(
            pointer_identity(&active.latest[&asked.id].body).unwrap(),
            asked.id
        );
        assert!(
            !resume(Loaded::clone(&site)).unwrap(),
            "terminal successor is not resumed at boot"
        );
        let (status, _, raw) = recent(State(Loaded::clone(&site))).await;
        assert_eq!(status, StatusCode::OK);
        let view: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(view["supersedes"], hex(old));
        assert_eq!(view["predecessor_completion_reconstructed"], false);
        assert_eq!(view["events"].as_array().unwrap().len(), 100);
        let html = page(State(Loaded::clone(&site))).await.0;
        assert!(html.contains(&hex(old)));
        assert!(html.contains("Original completion was not reconstructed"));
        assert_eq!(
            std::fs::read(root(&site).join("attempts.bin")).unwrap(),
            before[&std::ffi::OsString::from("attempts.bin")]
        );
        assert!(!scratch.join("bars").exists());
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[tokio::test]
    async fn concurrent_successor_preparation_or_start_refuses_without_claiming_or_clearing_stop() {
        let (scratch, site, body, old) = missing_fixture("recovery-successor-concurrent");
        let request = successor_form(&body, old, [18; 32], true);
        let asked = submission(&request, today()).unwrap();
        prepare_successor(&site, &asked).unwrap();
        let held = Journal::open_existing(&plan_path(&site, asked.id)).unwrap();
        let before = history_bytes(&site);
        for request in [request, successor_form(&body, old, [18; 32], false)] {
            let (status, _, answer) = start(State(Loaded::clone(&site)), request).await;
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{answer}");
            assert!(site.run.lock().unwrap().is_none());
            assert_eq!(history_bytes(&site), before);
        }
        drop(held);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[tokio::test]
    async fn explicit_successor_start_requires_preparation_then_runs_only_the_private_fixture() {
        let (scratch, site, body, old) = missing_fixture("recovery-successor-start-contract");
        let activate = successor_form(&body, old, [23; 32], false);
        let before = history_bytes(&site);
        let (status, _, answer) = start(State(Loaded::clone(&site)), activate.clone()).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{answer}");
        assert!(site.run.lock().unwrap().is_none());
        assert_eq!(history_bytes(&site), before);
        let request = successor_form(&body, old, [23; 32], true);
        assert_eq!(
            start(State(Loaded::clone(&site)), request).await.0,
            StatusCode::CREATED
        );
        let (status, _, answer) = start(State(Loaded::clone(&site)), activate).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{answer}");
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while site.run.lock().unwrap().as_ref().unwrap().running() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let response: serde_json::Value = serde_json::from_str(&answer).unwrap();
        let id = unhex(response["plan"].as_str().unwrap()).unwrap();
        assert_ne!(id, old);
        assert_eq!(active_history(&site).unwrap().latest[&id].attempts, 2);
        assert!(!plan_path(&site, old).exists());
        assert_eq!(
            std::fs::read(root(&site).join("attempts.bin")).unwrap(),
            before[&std::ffi::OsString::from("attempts.bin")]
        );
        assert!(
            !scratch.join("bars").exists(),
            "Sunday fixture has no source request"
        );
        assert!(
            site.run
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .finished
                .as_ref()
                .unwrap()
                .contains("NOT complete historical coverage")
        );
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn partial_successor_seed_resumes_only_queued_scope_and_a_stale_active_pointer_refuses() {
        let (scratch, site, body, old) = missing_fixture("recovery-successor-partial-seed");
        let asked = submission(&successor_form(&body, old, [24; 32], true), today()).unwrap();
        let mut partial = Journal::create_new(&plan_path(&site, asked.id)).unwrap();
        partial.append(asked.units[0].clone()).unwrap();
        drop(partial);
        prepare_successor(&site, &asked).unwrap();
        let inventory = crate::recovery_journal::snapshot(&plan_path(&site, asked.id)).unwrap();
        validate_seal(&inventory.latest, asked.id).unwrap();
        assert_eq!(inventory.latest.len(), 2);
        let mut active = Journal::open_existing(&active_path(&site)).unwrap();
        let mut invalid = record(hex(old));
        invalid.key = old;
        invalid.status = Status::Verified;
        invalid.attempts = 2;
        active.append(invalid).unwrap();
        drop(active);
        let before = history_bytes(&site);
        assert!(
            active_history(&site)
                .unwrap_err()
                .contains("invalid identity or activation state")
        );
        assert!(prepare_successor(&site, &asked).is_err());
        assert_eq!(history_bytes(&site), before);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn successor_identity_rejects_implicit_duplicate_malformed_or_changed_scope_fields() {
        let body = encoded_leg(&leg("NIFTY", "1day", date(2026, 8, 30), date(2026, 8, 30)));
        let original = submission(&body, today()).unwrap();
        let valid = successor_form(&body, original.id, [19; 32], true);
        let asked = submission(&valid, today()).unwrap();
        assert_ne!(
            asked.id,
            submission(&successor_form(&body, original.id, [20; 32], true), today())
                .unwrap()
                .id
        );
        for bad in [
            format!("{body}&recovery_action=prepare"),
            format!("{valid}&recovery_action=start"),
            format!("{valid}&recovery_supersedes={}", hex(original.id)),
            format!("{valid}&recovery_generation={}", hex([19; 32])),
            valid.replace("recovery_action=prepare", "recovery_action="),
            valid.replace("recovery_action=prepare", "recovery_action=unknown"),
            valid.replace(&hex([19; 32]), "not-an-identity"),
            valid.replace(&hex(original.id), &hex([0; 32])),
            valid.replace("NIFTY", "BANKNIFTY"),
        ] {
            assert!(submission(&bad, today()).is_err(), "{bad}");
        }
    }

    #[test]
    fn lost_budget_stale_pointer_and_incomplete_seal_never_become_prepared_work() {
        let (scratch, site, body, old) = missing_fixture("recovery-successor-incomplete");
        let asked = submission(&successor_form(&body, old, [21; 32], true), today()).unwrap();
        let attempts_path = root(&site).join("attempts.bin");
        let original_attempts = std::fs::read(&attempts_path).unwrap();
        std::fs::remove_file(&attempts_path).unwrap();
        assert!(
            prepare_successor(&site, &asked)
                .unwrap_err()
                .contains("budgets cannot be reset")
        );
        assert!(!plan_path(&site, asked.id).exists());
        std::fs::write(&attempts_path, original_attempts).unwrap();
        let mut incomplete = Journal::create_new(&plan_path(&site, asked.id)).unwrap();
        let mut control = record(asked.successor.as_ref().unwrap().1.clone());
        control.key = CONTROL;
        incomplete.append(control).unwrap();
        drop(incomplete);
        let before = history_bytes(&site);
        assert!(prepare_successor(&site, &asked).is_err());
        assert_eq!(history_bytes(&site), before);
        let mut stale = asked;
        stale.successor.as_mut().unwrap().0 = [22; 32];
        assert!(
            prepare_successor(&site, &stale)
                .unwrap_err()
                .contains("not recorded")
        );
        assert_eq!(history_bytes(&site), before);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    #[ignore = "requires explicit authentic frozen request and receipt paths; read-only, no vendor"]
    fn authentic_saved_request_reproduces_its_accepted_scope_identity_without_rebuilding_history() {
        let form = std::env::var("BRUTEX_RECOVERY_AUDIT_FORM").expect("explicit saved form path");
        let receipt =
            std::env::var("BRUTEX_RECOVERY_AUDIT_RECEIPT").expect("explicit saved receipt path");
        let body = std::fs::read_to_string(form).unwrap();
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
        let selected = submission(body.trim_end(), today()).unwrap();
        assert_eq!(hex(selected.id), receipt["plan"].as_str().unwrap());
        assert_eq!(
            selected.units.len() as u64,
            receipt["windows"].as_u64().unwrap()
        );
        assert_eq!(receipt["coverage_certified"], false);
        println!(
            "authentic_scope_id={} windows={} original_window_states_reconstructed=false source_or_vendor_activity=false",
            hex(selected.id),
            selected.units.len()
        );
    }

    #[test]
    fn source_reader_uses_the_real_store_path_and_never_creates_missing_files() {
        use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
        let root = crate::scratch::path("recovery-source-read");
        let month = YearMonth::new(2026, 8).unwrap();
        assert!(
            read_stamps(&root, "NIFTY", "1min", month)
                .unwrap()
                .is_empty()
        );
        assert!(!root.exists());
        let path = StorePath::new(PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        })
        .unwrap();
        assert_eq!(
            source_path(&root, "NIFTY", "1min", month).unwrap(),
            path.to_path_buf(&root)
        );
        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        let id = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
        let mut file = store::file::BarFile::open_or_create(&root, path, id).unwrap();
        let ts = (i64::from(date(2026, 8, 27).days_from_epoch()) * 86_400 + 9 * 3600 + 15 * 60
            - pull::session::IST_OFFSET_SECS)
            * 1_000_000;
        file.append(&[store::format::Bar {
            ts_micros: ts,
            open: 100,
            high: 110,
            low: 90,
            close: 100,
            volume: 1,
            open_interest: store::format::OI_NULL,
        }])
        .unwrap();
        assert!(read_stamps(&root, "NIFTY", "1min", month).is_err());
        drop(file);
        let before = std::fs::read(path.to_path_buf(&root)).unwrap();
        assert_eq!(
            read_stamps(&root, "NIFTY", "1min", month).unwrap(),
            vec![ts]
        );
        assert!(
            read_stamps(&root, "NIFTY", "1day", month)
                .unwrap()
                .is_empty()
        );
        assert_eq!(std::fs::read(path.to_path_buf(&root)).unwrap(), before);
        std::fs::remove_dir_all(root).unwrap();
    }
}

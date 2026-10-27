//! `GET /basis.json` — a share's measured Zerodha/GDFL basis runs and its
//! corporate actions, and the two tables kept for `/dayflags.json` and
//! `/sources/day.json` (R4-26: Z8, COR-06).
//!
//! ```text
//! GET /basis.json?symbol=SPLITCO               every run and every action of the share
//! GET /basis.json?symbol=SPLITCO&day=YYYY-MM-DD the run holding the day, or why none,
//!                                               and the actions with that ex-date
//! ```
//!
//! # Where the answer comes from
//!
//! The store's `reference/v1/basis_runs` and `reference/v1/corporate_actions`
//! generations, read through R3-30's loader
//! ([`pull::reference_data::load_one`]): a table that is absent or refused
//! (a wrong checksum, a torn file, a file of another table) is NAMED in the
//! answer with its kind and detail, and nothing is served from it; no
//! compiled copy and no empty table stands in. Every run is `measured`; an
//! action row carries its tag, and only an `nse` row `excuses` (a measured
//! row labels and never excuses, `CLAUDE.md` §3 rule 1).
//!
//! # The factor
//!
//! A run's `ratio` is its Zerodha/GDFL close ratio, the stored column. The
//! day answer's `factor` is D-3339's direction, GDFL ÷ Zerodha, the word
//! `/sources/day.json`'s `measured_run` uses for the same run, and the run's
//! own ratio rides beside it as `zerodha_over_gdfl` (check 4 of R4-26 found
//! `factor` saying 67/71 here and 71/67 there for one run). Before a 2:1
//! split it is `2/1`, the ratio of the action that ends the run.
//!
//! # Cost
//!
//! The tables are kept per store root, keyed by one `stat` of each table's
//! directory and one of the file served (and one more of what a name
//! names when it is a symbolic link): a request costs those `stat`s,
//! one hash of the symbol and one or two indexes into that share's day
//! table, O(1). A new generation (a new name in the directory) or a
//! served file changed in place is read again, once, O(rows), and told as
//! the loader's event. Without `day` the answer lists the share's runs and
//! actions, at most `MAX_RUNS_PER_SYMBOL` and `MAX_ACTIONS_PER_SYMBOL`.
//! Measured: `api::server::basis_route::tests::a_lookup_costs_the_same_at_1x_10x_and_100x_runs_measured`
//! and `crates/api/benches/ratio/basis.rs`.

use std::collections::HashMap;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use pull::corporate_actions::{Action, BasisRuns, CorporateActions, Ratio, Run};
use pull::reference_data::{self, Receipt, Refusal, Slot, Table};
use pull::session::Day;
use serde_json::{Value, json};

use super::Loaded;

/// The route.
pub(crate) const ROUTE: &str = "/basis.json";

/// The longest symbol the route reads.
const MAX_SYMBOL_BYTES: usize = 64;

/// What one `stat` says changed: a name linked into a directory or a file
/// rewritten (identity, mtime, size), and a mode repaired (ctime: a
/// `chmod` changes nothing else, so a table refused as unreadable would
/// stay refused after the repair; attack E16 of check 4).
type Meta = (u64, u64, i64, i64, u64, i64, i64);

/// The name's own `stat` and, only when the name is a symbolic link, the
/// `stat` of what it names: a generation that lands inside a linked table
/// directory changes the directory the link names and never the link
/// (attack E7 of check 4). One `stat` a name, two for a link.
type Stamp = Option<(Meta, Option<Meta>)>;

fn meta(meta: &std::fs::Metadata) -> Meta {
    (
        meta.dev(),
        meta.ino(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.size(),
        meta.ctime(),
        meta.ctime_nsec(),
    )
}

fn stamp(path: &Path) -> Stamp {
    let name = std::fs::symlink_metadata(path).ok()?;
    let named = if name.file_type().is_symlink() {
        std::fs::metadata(path).ok().map(|target| meta(&target))
    } else {
        None
    };
    Some((meta(&name), named))
}

/// Both tables as one store serves them.
#[derive(Debug, Clone)]
pub(crate) struct Tables {
    /// The corporate actions.
    pub(crate) actions: Arc<Slot<CorporateActions>>,
    /// The basis runs.
    pub(crate) runs: Arc<Slot<BasisRuns>>,
}

#[derive(Debug, Clone)]
struct Kept {
    key: [Stamp; 4],
    tables: Tables,
}

fn kept() -> &'static Mutex<HashMap<PathBuf, Kept>> {
    static KEPT: OnceLock<Mutex<HashMap<PathBuf, Kept>>> = OnceLock::new();
    // One entry per store root a process serves: one for the server.
    KEPT.get_or_init(|| Mutex::new(HashMap::with_capacity(1)))
}

fn key_of(root: &Path, tables: Option<&Tables>) -> [Stamp; 4] {
    let file = |slot: Option<&Path>| slot.and_then(stamp);
    [
        stamp(&Table::CorporateActions.dir(root)),
        stamp(&Table::BasisRuns.dir(root)),
        file(tables.map(|t| t.actions.path())),
        file(tables.map(|t| t.runs.path())),
    ]
}

/// The two tables of the store at `root`: kept, and read again only when a
/// table's directory or served file changed. O(1) when kept: four `stat`s,
/// and one more for each of those names that is a symbolic link.
/// Proved by `api::server::basis_route::tests::the_tables_are_read_again_only_when_a_generation_lands`.
pub(crate) fn tables(root: &Path) -> Tables {
    let mut held = kept().lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(kept) = held.get(root)
        && kept.key == key_of(root, Some(&kept.tables))
    {
        return kept.tables.clone();
    }
    // THE DIRECTORIES ARE STAMPED BEFORE THE READ: a generation that lands
    // during it changes the next request's stamp, which reads again.
    let before = key_of(root, None);
    let tables = Tables {
        actions: Arc::new(reference_data::load_one(
            root,
            Table::CorporateActions,
            CorporateActions::build,
        )),
        runs: Arc::new(reference_data::load_one(
            root,
            Table::BasisRuns,
            BasisRuns::build,
        )),
    };
    let after = key_of(root, Some(&tables));
    let key = [before[0], before[1], after[2], after[3]];
    held.insert(
        root.to_path_buf(),
        Kept {
            key,
            tables: tables.clone(),
        },
    );
    tables
}

fn receipt_json(receipt: &Receipt) -> Value {
    json!({
        "generation": receipt.generation,
        "source": receipt.source,
        "source_sha256": receipt.source_sha256,
        "fetched": receipt.fetched.to_string(),
        "status": receipt.standing.word(),
        "decision": receipt.decision,
        "rows": receipt.rows,
        "sha256": receipt.sha256,
    })
}

fn refusal_json(why: &Refusal) -> Value {
    json!({
        "table": why.table.word(),
        "kind": why.kind.word(),
        "path": why.path.display().to_string(),
        "generation": why.generation,
        "detail": why.detail,
        "said": why.to_string(),
    })
}

/// Where a table came from: `served` with its receipt, or its refusal.
pub(crate) fn slot_json<T>(slot: &Slot<T>) -> Value {
    match slot {
        Slot::Refused(why) => json!({
            "served": false,
            "origin": slot.origin(),
            "refusal": refusal_json(why),
        }),
        Slot::Store { receipt, path, .. }
        | Slot::Compiled {
            receipt,
            looked_in: path,
            ..
        } => {
            json!({
                "served": true,
                "origin": slot.origin(),
                "path": path.display().to_string(),
                "receipt": receipt_json(receipt),
            })
        }
    }
}

/// One run on the wire.
pub(crate) fn run_json(run: &Run) -> Value {
    json!({
        "first": run.first.to_string(),
        "last": run.last.to_string(),
        "ratio": run.ratio.to_string(),
        "low": run.low.to_string(),
        "high": run.high.to_string(),
        "days": run.days,
        "end": run.end.word(),
        "standing": "measured",
    })
}

/// One action on the wire, with what it may do.
pub(crate) fn action_json(action: &Action) -> Value {
    json!({
        "ex_date": action.ex_date.to_string(),
        "kind": action.kind.word(),
        "ratio": action.ratio.to_string(),
        "tag": action.tag.word(),
        "isin": action.isin,
        "after": action.after.map(|day| day.to_string()),
        // FALSE when a session with no basis lies in (after, ex_date]: the
        // row is on the first day the new basis was seen, not on its day.
        "day_known": action.day_known(),
        "evidence": action.evidence,
        "excuses": action.excuses(),
        "decides_class": action.class().is_some(),
    })
}

/// What the law lets a row do, said once per answer.
pub(crate) const LAW: &str = "a measured row labels a day and never excuses a refusal or decides \
     a class; only an nse row from a sourced file does (CLAUDE.md section 3 rule 1, R4-26)";

/// The asked share and day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asked {
    symbol: String,
    day: Option<Day>,
}

fn param<'a>(query: &'a str, name: &str) -> Result<Option<&'a str>, String> {
    let mut found = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == name {
            if found.is_some() {
                return Err(format!("{name} is given twice"));
            }
            found = Some(value);
        } else if !matches!(key, "symbol" | "day") {
            return Err(format!("{key:?} is not a field of {ROUTE}"));
        }
    }
    Ok(found)
}

/// Reads `symbol` and `day` from the query.
///
/// # Errors
///
/// A missing, empty, overlong or malformed symbol, a day that is not
/// `YYYY-MM-DD`, an unknown or repeated field; each by name.
pub(crate) fn asked(query: &str) -> Result<Asked, String> {
    let symbol = param(query, "symbol")?
        .filter(|symbol| !symbol.is_empty())
        .ok_or_else(|| "symbol is required".to_owned())?;
    // `&` arrives as %26 (M&M): the one escape a stored share name needs.
    let symbol = symbol.replace("%26", "&");
    if symbol.len() > MAX_SYMBOL_BYTES
        || !symbol.bytes().all(|b| {
            b.is_ascii_uppercase() || b.is_ascii_digit() || matches!(b, b'&' | b'-' | b'_')
        })
    {
        return Err(format!(
            "symbol {symbol:?} is not a stored share name (A-Z, 0-9, & - _, at most {MAX_SYMBOL_BYTES} bytes)"
        ));
    }
    let day = match param(query, "day")? {
        None => None,
        Some(text) => Some(reference_data::parse_day(text).ok_or_else(|| not_a_day(text))?),
    };
    Ok(Asked { symbol, day })
}

/// Why `text` is no day the route can ask about. A well-formed `YYYY-MM-DD`
/// the calendar does not hold (`1969-12-31`, `2025-02-30`) is said to be
/// that, not to be malformed (check 4's follow-up: `day=1969-12-31` was
/// answered "is not YYYY-MM-DD").
fn not_a_day(text: &str) -> String {
    let shaped = text.len() == 10
        && text.bytes().enumerate().all(|(at, b)| {
            if at == 4 || at == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    if shaped {
        format!(
            "day {text:?} is no calendar day: the calendar holds {}-01-01 to {}-12-31",
            pull::session::MIN_YEAR,
            pull::session::MAX_YEAR
        )
    } else {
        format!("day {text:?} is not YYYY-MM-DD")
    }
}

/// The answer for `asked` from `tables`. O(1) with `day`; O(the share's
/// runs and actions) without. Measured by
/// `api::server::basis_route::tests::a_lookup_costs_the_same_at_1x_10x_and_100x_runs_measured`.
pub(crate) fn answer(tables: &Tables, asked: &Asked) -> Value {
    let symbol = asked.symbol.as_str();
    let runs = tables.runs.table().ok();
    let actions = tables.actions.table().ok();
    let mut body = json!({
        "symbol": symbol,
        "tables": {
            "basis_runs": slot_json(&tables.runs),
            "corporate_actions": slot_json(&tables.actions),
        },
        "law": LAW,
    });
    let Some(object) = body.as_object_mut() else {
        return body;
    };
    if let Some(day) = asked.day {
        let run = runs.map(|runs| runs.run_on(symbol, day));
        let rows = actions.map(|table| table.labels(symbol, None, day));
        object.insert(
            "day".to_owned(),
            json!({
                "day": day.to_string(),
                "run": run.as_ref().and_then(|run| run.as_ref().ok()).map(|run| run_json(run)),
                // D-3339's direction, GDFL ÷ Zerodha: the word
                // `/sources/day.json`'s `measured_run` uses for one run, with
                // the run's own Zerodha/GDFL ratio beside it, named.
                "factor": run.as_ref().and_then(|run| run.as_ref().ok()).and_then(|run| Ratio::ONE.over(run.ratio)).map(|factor| factor.to_string()),
                "zerodha_over_gdfl": run.as_ref().and_then(|run| run.as_ref().ok()).map(|run| run.ratio.to_string()),
                "outside": match &run {
                    Some(Err(why)) => Some(why.to_string()),
                    Some(Ok(_)) => None,
                    None => Some("the basis-run table is not served".to_owned()),
                },
                "actions": rows.map(|rows| rows.iter().map(action_json).collect::<Vec<_>>()),
            }),
        );
    } else {
        object.insert(
            "runs".to_owned(),
            runs.map_or(Value::Null, |runs| {
                Value::Array(runs.of(symbol).iter().map(run_json).collect())
            }),
        );
        object.insert(
            "actions".to_owned(),
            actions.map_or(Value::Null, |table| {
                Value::Array(table.of(symbol).iter().map(action_json).collect())
            }),
        );
    }
    body
}

/// `GET /basis.json`.
pub(super) async fn basis_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 2],
    String,
) {
    let headers = [
        (
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        ),
        (axum::http::header::CACHE_CONTROL, "no-store"),
    ];
    let asked = match asked(uri.query().unwrap_or_default()) {
        Ok(asked) => asked,
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                headers,
                json!({ "error": why }).to_string(),
            );
        }
    };
    let root = site.store_root.clone();
    match crate::detail::run_ops_read(move || answer(&tables(&root), &asked)).await {
        Ok(body) => (axum::http::StatusCode::OK, headers, body.to_string()),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "a basis lookup",
                crate::detail::MAX_OPS_READ_CONCURRENT,
                &why,
            );
            (status, headers, body)
        }
    }
}

#[cfg(test)]
#[path = "basis_route_tests.rs"]
mod tests;

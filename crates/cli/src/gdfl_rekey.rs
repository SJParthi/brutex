//! `cli gdfl-rekey` — moves GDFL option keys the current decoder files under
//! another key, retires the ones it refuses, and says which it cannot do.
//! D-3331, D-3316; WP-12c, findings DB-A17 and DB-A18.
//!
//! # Usage
//!
//! ```text
//! cli gdfl-rekey (--plan | --apply) [--underlying SYMBOL[,SYMBOL...]]
//! ```
//!
//! The mode is required, so a store write is never what a bare verb does.
//! `--plan` writes nothing: every option key under `bars/gdfl/NSE/FNO/` is
//! read off its bar files and judged by the decoder's two rules
//! (`pull::gdfl_rekey`), and the receipt names each key to move, each to
//! retire and each it cannot do. `--apply` carries the plan out: the moved
//! keys' bars are written under the decoded keys as NEW month files, the
//! census counts them, `imports/gdfl.lineage` gains a `superseded` line per
//! moved key and a `retired` line per retired key, and the census's next FNO
//! epoch stops listing both. No file of an old key is touched, and a second
//! `--apply` writes nothing.
//!
//! # Done and pending (AG-02)
//!
//! The old keys' files are kept for ever, so every run judges them again.
//! The receipt tells what the ledger already records (`N moved and M retired
//! already recorded`, each listed `moved`/`retired ... (recorded in the
//! ledger)`) from what is still to do, and prints `PENDING: P`, the one
//! number a gate reads: after a complete apply a second `--plan` reads
//! `PENDING: 0` and a second `--apply` writes 0 month files, appends 0 ledger
//! lines and installs no epoch. A verdict the ledger contradicts, and a move
//! into a month that already holds bars, are `LEFT` by name and counted in
//! it.
//!
//! `--underlying` narrows the walk; a day's strike ladder is one
//! underlying's, so the narrowed plan is the same plan for those
//! underlyings.
//!
//! # Order (WP-80)
//!
//! It needs the version-4 census (`cli census-migrate gdfl`) and is run
//! before the options import resumes on a D-3331 build: once that import has
//! written a later bar into a month a move needs, the move is refused by name
//! (a month is never rewritten in place). It must not run while the import
//! writes the store.
//!
//! # Exit
//!
//! [`OK`] only when every month file was read and every key was either kept,
//! moved or retired (and, with `--apply`, every move landed); [`FAILED`] when
//! a key is left unresolved, a file was unreadable, a move was refused or the
//! apply stopped; [`MISUSED`] for arguments it does not know, before anything
//! is read.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::symbol::Symbol;
use pull::gdfl_lineage::option_key;
use pull::gdfl_rekey::{self, Applied, CensusStep, Plan, Survey, Withdrawn};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str =
    "cli gdfl-rekey (--plan | --apply) [--underlying SYMBOL[,SYMBOL...]]";

/// Lines of a list the receipt prints before it only counts the rest.
const SHOWN: usize = 200;

/// What the arguments ask.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    only: Option<BTreeSet<Symbol>>,
    apply: bool,
}

/// The arguments, or what is wrong with them: exactly one of `--plan` and
/// `--apply`, and `--underlying` at most once.
fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let mut out = Ask {
        only: None,
        apply: false,
    };
    let mut mode: Option<&str> = None;
    let mut rest = arguments.iter();
    while let Some(flag) = rest.next() {
        match *flag {
            "--plan" | "--apply" => {
                if let Some(first) = mode {
                    return Err(format!(
                        "{flag} after {first}: give one of --plan and --apply, once"
                    ));
                }
                mode = Some(*flag);
                out.apply = *flag == "--apply";
            }
            "--underlying" if out.only.is_none() => {
                let Some(list) = rest.next() else {
                    return Err("--underlying needs SYMBOL[,SYMBOL...]".to_owned());
                };
                let mut wanted = BTreeSet::new();
                for word in list.split(',') {
                    let symbol = Symbol::new(word)
                        .ok()
                        .filter(|symbol| symbol.as_str() == word)
                        .ok_or_else(|| {
                            format!("`{}` is not a store symbol", word.escape_debug())
                        })?;
                    let _ = wanted.insert(symbol);
                }
                out.only = Some(wanted);
            }
            "--underlying" => return Err(format!("{flag} is given twice")),
            other => {
                return Err(format!(
                    "`{}` is not a gdfl-rekey argument; the flags are --plan, --apply and \
                     --underlying",
                    other.escape_debug()
                ));
            }
        }
    }
    if mode.is_none() {
        return Err(
            "give --plan (writes nothing) or --apply (writes the new files, the ledger lines \
             and the census epoch)"
                .to_owned(),
        );
    }
    Ok(out)
}

/// The survey's half of the receipt.
///
/// The ledger totals are the plan's underlyings' when `--underlying` narrows
/// the walk, and the whole ledger's otherwise (AG-02, finding rekey-3): a
/// receipt about NIFTY does not count BANKNIFTY's records.
fn render_survey(
    out: &mut String,
    root: &Path,
    found: &Survey,
    ledger: &Withdrawn,
    only: Option<&BTreeSet<Symbol>>,
) {
    let (superseded, retired) = ledger.counts_for(only);
    let _ = writeln!(
        out,
        "GDFL RE-KEY -- option keys under {}/bars/gdfl/NSE/FNO, read from their bar files \
         (no vendor file was read)",
        root.display()
    );
    let _ = writeln!(out, "  keys with bars      {}", found.keys.len());
    let _ = writeln!(out, "  month files read    {}", found.files);
    let _ = writeln!(out, "  unreadable          {}", found.unreadable);
    for (path, why) in &found.unreadable_named {
        let _ = writeln!(out, "    {}: {why}", path.display());
    }
    let _ = writeln!(out, "  not store names     {}", found.strays);
    for path in &found.strays_named {
        let _ = writeln!(out, "    {}", path.display());
    }
    let scope = only.map_or_else(
        || "the whole ledger".to_owned(),
        |only| {
            only.iter()
                .map(Symbol::as_str)
                .collect::<Vec<_>>()
                .join(",")
        },
    );
    let _ = writeln!(
        out,
        "  ledger              {superseded} superseded and {retired} retired key(s) already \
         recorded ({scope})"
    );
}

/// The plan's half of the receipt.
fn render_plan(out: &mut String, plan: &Plan) {
    let _ = writeln!(
        out,
        "  PLAN: {} key(s) judged; {} kept, {} to move, {} to retire, {} unresolved; {} moved \
         and {} retired already recorded",
        plan.keys,
        plan.kept,
        plan.moves.len(),
        plan.retired.len(),
        plan.unresolved.len(),
        plan.recorded_moves.len(),
        plan.recorded_retired.len()
    );
    // FM-273'S STEP 4 (AG-02 repair r1e): whether the census still lists a
    // key the ledger withdraws. A run stopped after its ledger lines leaves
    // every key recorded and this step pending, and the receipt says so.
    let _ = writeln!(out, "  {}", plan.census.describe());
    // THE ONE NUMBER A GATE READS (AG-02, finding rekey-3): what is still to
    // do or to answer. Recorded moves and retirements are done and are not
    // in it, so a store whose re-key has run reads `pending 0` however long
    // its old files are kept; the census step is in it until the epoch that
    // leaves out every withdrawn key is installed.
    let _ = writeln!(
        out,
        "  PENDING: {} ({} to move, {} to retire, {} unresolved, {} census)",
        plan.pending(),
        plan.moves.len(),
        plan.retired.len(),
        plan.unresolved.len(),
        plan.census.pending()
    );
    for one in plan.moves.iter().take(SHOWN) {
        let _ = writeln!(
            out,
            "    move    {} -> {} ({}; {} month(s), {} bar(s))",
            option_key(one.underlying, one.from),
            option_key(one.underlying, one.to),
            one.kind.as_str(),
            one.months.len(),
            one.bars
        );
    }
    for record in plan.retired.iter().take(SHOWN) {
        let _ = writeln!(out, "    retire  {}", record.refusal());
    }
    for one in plan.unresolved.iter().take(SHOWN) {
        let _ = writeln!(out, "    LEFT    {}: {}", one.key, one.why);
    }
    for one in plan.recorded_moves.iter().take(SHOWN) {
        let _ = writeln!(
            out,
            "    moved   {} -> {} (recorded in the ledger; {} month(s), {} bar(s))",
            option_key(one.underlying, one.from),
            option_key(one.underlying, one.to),
            one.months.len(),
            one.bars
        );
    }
    for record in plan.recorded_retired.iter().take(SHOWN) {
        let _ = writeln!(
            out,
            "    retired {} (recorded in the ledger)",
            option_key(record.underlying, record.key)
        );
    }
    let listed: &[String] = match &plan.census {
        CensusStep::Listed(keys) => keys,
        _ => &[],
    };
    for key in listed.iter().take(SHOWN) {
        let _ = writeln!(
            out,
            "    census  {key} (withdrawn by the ledger, still listed)"
        );
    }
    let shown = listed.len().min(SHOWN)
        + plan.moves.len().min(SHOWN)
        + plan.retired.len().min(SHOWN)
        + plan.unresolved.len().min(SHOWN)
        + plan.recorded_moves.len().min(SHOWN)
        + plan.recorded_retired.len().min(SHOWN);
    let all = listed.len()
        + plan.moves.len()
        + plan.retired.len()
        + plan.unresolved.len()
        + plan.recorded_moves.len()
        + plan.recorded_retired.len();
    if shown < all {
        let _ = writeln!(out, "    ... and {} more not printed", all - shown);
    }
}

/// The apply's half of the receipt.
fn render_applied(out: &mut String, done: &Applied) {
    let _ = writeln!(
        out,
        "  APPLIED: {} key(s) moved ({} month file(s) written, {} already held exactly), \
         {} move(s) refused, {} ledger line(s) appended",
        done.moved.len(),
        done.months_written,
        done.months_already,
        done.refused.len(),
        done.lines_written
    );
    for record in done.moved.iter().take(SHOWN) {
        let _ = writeln!(out, "    {}", record.line());
    }
    for one in &done.refused {
        let _ = writeln!(out, "    REFUSED {}: {}", one.key, one.why);
    }
    for cycle in &done.census {
        let _ = writeln!(
            out,
            "    census {}: recorded {}, already exact {}; now {} keys, {} rows",
            cycle.path.display(),
            cycle.recorded,
            cycle.exact,
            cycle.keys,
            cycle.rows
        );
    }
    match &done.withdrawal {
        Some(epoch) => {
            let from = epoch
                .from_epoch
                .map_or_else(|| "none".to_owned(), |at| format!("{at:06}"));
            let _ = writeln!(
                out,
                "    census epoch {from} -> {:06} ({}): {} -> {} entries; epoch {from} is kept \
                 as it was",
                epoch.to_epoch, epoch.why, epoch.from_entries, epoch.to_entries
            );
        }
        None => {
            let _ = writeln!(
                out,
                "    census lists no withdrawn key; no epoch was installed"
            );
        }
    }
}

/// `gdfl-rekey`, with the store root passed in so every arm is driven by a
/// test without the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let asked = match ask(arguments) {
        Ok(asked) => asked,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let read = store().and_then(|root| {
        let ledger = gdfl_rekey::withdrawn(&root)?;
        let found = gdfl_rekey::survey(&root, asked.only.as_ref())?;
        Ok((root, ledger, found))
    });
    let (root, ledger, found) = match read {
        Ok(read) => read,
        Err(why) => {
            let _ = writeln!(out, "GDFL RE-KEY REFUSED: {why}; nothing was written");
            return FAILED;
        }
    };
    render_survey(out, &root, &found, &ledger, asked.only.as_ref());
    let plan = gdfl_rekey::plan(&found, &ledger);
    render_plan(out, &plan);
    // A census that was not read or would not read leaves step 4 unknown: that
    // is a failure to answer, not pending work (AG-02 repair r1e).
    let clean = found.unreadable == 0
        && plan.unresolved.is_empty()
        && !matches!(plan.census, CensusStep::NotRead | CensusStep::Unreadable(_));
    if !asked.apply {
        let _ = writeln!(
            out,
            "  PLAN ONLY: nothing was written; --apply carries the plan out{}",
            if plan.changes() {
                ""
            } else {
                " (and would change nothing)"
            }
        );
        return if clean { OK } else { FAILED };
    }
    match gdfl_rekey::apply(&root, &found, &plan, &ledger) {
        Ok(done) => {
            render_applied(out, &done);
            if clean && done.refused.is_empty() {
                OK
            } else {
                FAILED
            }
        }
        Err(why) => {
            let _ = writeln!(out, "GDFL RE-KEY STOPPED: {why}");
            FAILED
        }
    }
}

/// `cli gdfl-rekey`, against the CLI's store root (`BRUTEX_STORE`).
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // `--apply` BY AN UNSTAMPED BUILD IS REFUSED (DB-B71, WP-81); `--plan`
    // writes nothing and is not.
    let store = crate::store_write::root_for("gdfl-rekey", arguments);
    command_with(arguments, &store, out)
}

#[cfg(test)]
#[path = "gdfl_rekey_tests.rs"]
mod tests;

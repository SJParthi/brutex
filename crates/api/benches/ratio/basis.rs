//! Gate 8 for `/basis.json` (R4-26, Z8): a share's run on a day, and its
//! whole list of runs and actions, cost the same whatever the basis-run and
//! corporate-action tables hold. Each served store gets 1x, 10x and 100x
//! shares (20, 200 and 2,000, eight runs and four actions each); the kept
//! answer is one hash and one or two indexes into the share's day table,
//! the first request of
//! a store reads the tables once (the `cold` figure the family prints).

use pull::corporate_actions::{Action, Kind, Ratio, Run, RunEnd, Tag};
use pull::reference_data::{Draft, Standing, Table, write};
use pull::session::Day;

use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/basis.json"];

/// Shares per store, 1x / 10x / 100x.
const SHARES: [u32; 3] = [20, 200, 2_000];

fn day(days: u32) -> Option<Day> {
    Day::from_days(days).ok()
}

/// Writes invented runs and actions for `shares` shares into `root`.
/// Every symbol and ratio is invented.
fn install(root: &std::path::Path, shares: u32) -> bool {
    let (Some(base), Some(one), Some(half)) = (
        Day::new(2018, 1, 1).ok(),
        Ratio::new(1, 1),
        Ratio::new(1, 2),
    ) else {
        return false;
    };
    let mut runs = Vec::new();
    let mut actions = Vec::new();
    for s in 0..shares {
        let symbol = format!("SHARE{s:05}");
        for k in 0..8_u32 {
            let start = base.days_from_epoch() + k * 200;
            let (Some(first), Some(last)) = (day(start), day(start + 150)) else {
                return false;
            };
            let ratio = if k % 2 == 0 { half } else { one };
            runs.push(
                Run {
                    symbol: symbol.clone(),
                    first,
                    last,
                    ratio,
                    low: ratio,
                    high: ratio,
                    days: 100,
                    end: RunEnd::Drift,
                }
                .fields(),
            );
            if k % 2 == 1 {
                let Some(factor) = Ratio::new(2, 1) else {
                    return false;
                };
                actions.push(
                    Action {
                        symbol: symbol.clone(),
                        isin: None,
                        ex_date: first,
                        kind: Kind::Unclassed,
                        ratio: factor,
                        tag: Tag::Measured,
                        after: None,
                        evidence: "invented for the bench".to_owned(),
                    }
                    .fields(),
                );
            }
        }
    }
    let draft = |table, rows| Draft {
        table,
        source: "invented for gate 8".to_owned(),
        source_sha256: None,
        fetched: base,
        standing: Standing::Measured,
        decision: "D-6892".to_owned(),
        rows,
    };
    write(root, &draft(Table::BasisRuns, runs)).is_ok()
        && write(root, &draft(Table::CorporateActions, actions)).is_ok()
}

/// Every target of `/basis.json` at 1x, 10x and 100x, each p50 ratio gated.
/// Run by `crates/api/benches/ratio.rs`'s `main` over the stores `served`
/// built.
pub(crate) fn lookups_are_flat(stores: &mut [Served]) -> bool {
    for (served, shares) in stores.iter().zip(SHARES) {
        if !install(&served.root, shares) {
            println!("  BREACH basis: the bench's invented tables did not install");
            return false;
        }
    }
    flat("basis", stores, |_served| {
        vec![
            ok_with(
                "a share's run on a day",
                "/basis.json?symbol=SHARE00010&day=2019-06-03".to_owned(),
                "factor",
            ),
            ok_with(
                "a share's runs and actions",
                "/basis.json?symbol=SHARE00010".to_owned(),
                "runs",
            ),
        ]
    })
}

//! Gate 8 for `/settings.json` (R3-05's read, benched by R3-14 repair 5 so
//! gate 29 is green once both land): the settings in force, and a page of
//! their history, cost the same however many generations the operator has
//! saved.
//!
//! `docs/06-limits.md` L-R305-1 claims a GET is one `stat` once the reader
//! is warm: the generation already held plus a fixed-size body, O(1) in the
//! generations on disk; and that `?history=N` reads N + 1 files, N at most
//! 50, so a fixed N is O(1) too. Each served store here is given 2, 20 and
//! 200 generations (1x, 10x, 100x), saved through `store::settings::save`
//! itself, so the bytes are the real format and every save is checked as the
//! route's own saves are. The needle is the newest generation's number, so a
//! request answered before the reader had stepped to the last generation is
//! never timed. THE PROOF OF BOTH CLAIMS IS THE GROWTH BENCH
//! `crates/api/benches/ratio.rs` (gate 8, `cargo bench -p api --bench
//! ratio`), whose served pass runs `settings_are_flat` below and fails when a
//! p50 grows past the ratio ceiling from 1x to 100x.
//!
//! `POST /settings` is a write and is owed in `.github/gates_routes.rs`'s
//! "not a read" group: its cost is one listing of the settings directory per
//! save, L-R305-6.

use store::settings::{self, Change, Key, Reader};

use super::refuse;
use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/settings.json"];

/// Generations saved per 1,000 census series: 2, 20 and 200 at the three
/// sizes.
const GENERATIONS_PER_THOUSAND: usize = 2;

/// The history page timed: a fixed N well inside `MAX_HISTORY`.
const HISTORY: usize = 5;

/// Saves `served.census / 1000 * 2` generations into `served`'s store,
/// alternating `open.order` so each save changes something. Returns the
/// newest generation's number.
fn generations_into(served: &Served) -> u64 {
    let wanted = (served.census / 1_000).max(1) * GENERATIONS_PER_THOUSAND;
    let mut reader = Reader::new(&served.root);
    let mut newest = 0_u64;
    for i in 0..wanted {
        let change = Change {
            expected: reader.highest(),
            set: vec![(
                Key::OpenOrder,
                if i % 2 == 0 { "oldest" } else { "newest" }.to_owned(),
            )],
            unset: Vec::new(),
            changed_by: "bench".to_owned(),
            reason: format!("ratio bench generation {i}"),
            at_micros: 1_700_000_000_000_000_i64
                .saturating_add(i64::try_from(i).unwrap_or(i64::MAX)),
        };
        match settings::save(&mut reader, &change, |_| Ok(())) {
            Ok(saved) => newest = saved.generation.number,
            Err(why) => refuse(&format!(
                "settings generation {i} at {} series would not save: {why}",
                served.census
            )),
        }
    }
    if newest != u64::try_from(wanted).unwrap_or(u64::MAX) {
        refuse(&format!(
            "{wanted} saves ended at generation {newest}, not {wanted}"
        ));
    }
    newest
}

/// Both targets at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `served_routes_are_flat` over the stores
/// `served` built.
pub(crate) fn settings_are_flat(stores: &mut [Served]) -> bool {
    flat("settings", stores, |served| {
        let newest = generations_into(served);
        let needle = format!("\"generation\":{newest},");
        vec![
            ok_with(
                "the settings in force",
                "/settings.json".to_owned(),
                &needle,
            ),
            ok_with(
                "a page of their history",
                format!("/settings.json?history={HISTORY}"),
                &needle,
            ),
        ]
    })
}

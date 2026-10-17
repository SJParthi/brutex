//! Gate 8 for the three reads the `/db` page makes outside `logs::DB_READS`
//! (R3-14 repair 2, RA-064): the feed picker (`/feeds.json`), the alias
//! table its search and option cells read (`/aliases.json`), and the scrub
//! note beside a series (`/scrub.json`).
//!
//! `.github/gates_routes.rs` owed these three a bench as "an operational
//! page ... not a /db read", which a second agent found false (15 Oct): the
//! `/db` page reads all three. Each is now timed at 1x, 10x and 100x through
//! the real router, gated per request:
//!
//! * `/feeds.json` reads one cached census per feed (`feeds_route.rs` claimed
//!   O(1) in the store and named no proof): the census grows 1,000 to
//!   100,000 series here;
//! * `/aliases.json` serves a table compiled into the binary: no store size
//!   reaches it, and the bench proves the claim rather than assuming it;
//! * `/scrub.json` indexes the scrub ledger incrementally: here the ledger
//!   grows with the census, one finding per other series, and NIFTY's own
//!   finding is written LAST, so the needle is found only once the whole
//!   ledger has been indexed and every timed request is the indexed probe.
//!
//! The proof of each bound above is the ratio bench, `crates/api/benches/ratio.rs`,
//! whose `db_page` module this is, run by `cargo bench -p api --bench ratio`
//! (gate 8): it prints p50, p99 and max at 1x, 10x and 100x and fails on a
//! ratio past its ceiling.

use std::fmt::Write as _;

use brutex_core::vendor::Vendor;

use super::refuse;
use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &["/feeds.json", "/aliases.json", "/scrub.json"];

/// The reason NIFTY's own finding carries: the needle the timed answer must
/// hold, so a request answered before the ledger was indexed to its end is
/// never timed.
const NEEDLE: &str = "written last by the bench";

/// Writes a scrub ledger and cursor of one complete pass into `served`'s
/// store: one bad block for each of its census's other series, then NIFTY's.
fn ledger_into(served: &Served) {
    let mut ledger = String::from("run 1 vendor=gdfl commit=bench pass=1 resume=-\n");
    for i in 0..served.census {
        let _ = writeln!(
            ledger,
            "bad bars/gdfl/NSE/CASH/S{i:07}/1min/2024-06.bin block=1 records=73-145 run=1 why=crc"
        );
    }
    let _ = writeln!(
        ledger,
        "bad bars/gdfl/NSE/INDEX/NIFTY/1s/2024-06.bin block=2 records=146-218 run=1 why={NEEDLE}"
    );
    let found = served.census + 1;
    let cursor = format!(
        "done run=1 pass=1 months={found} blocks={found} bad={found} unverifiable=0 \
         misfiled=0 unbound=0 busy=0 missing=0\n"
    );
    for (name, text) in [
        (cli::scrub_ledger_name(Vendor::Gdfl), ledger),
        (cli::scrub_cursor_name(Vendor::Gdfl), cursor),
    ] {
        let path = served.root.join(name);
        if let Some(parent) = path.parent()
            && let Err(why) = std::fs::create_dir_all(parent)
        {
            refuse(&format!("the scrub ledger's folder: {why}"));
        }
        if let Err(why) = std::fs::write(&path, text) {
            refuse(&format!("the scrub ledger {}: {why}", path.display()));
        }
    }
}

/// Every target at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `served_routes_are_flat` over the stores
/// `served` built.
pub(crate) fn page_reads_are_flat(stores: &mut [Served]) -> bool {
    flat("db page", stores, |served| {
        ledger_into(served);
        vec![
            ok_with("the feed picker", "/feeds.json".to_owned(), "gdfl"),
            ok_with("the alias table", "/aliases.json".to_owned(), "NIFTY"),
            ok_with(
                "one alias",
                "/aliases.json?symbol=NIFTY".to_owned(),
                "NIFTY",
            ),
            ok_with(
                "the scrub note of NIFTY",
                "/scrub.json?feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY".to_owned(),
                NEEDLE,
            ),
            ok_with(
                "the scrub's standing alone",
                "/scrub.json?feed=gdfl".to_owned(),
                "\"recorded\":true",
            ),
        ]
    })
}

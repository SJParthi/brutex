//! What one `/backtest/audit.json` page costs, at 10^2 to 10^4 invocations
//! indexed — D-3303.
//!
//! `cli::operation_audit::read` takes two `sync_all` calls per row on a READ
//! path (the index, then the invocation's own file) so that what it reports
//! has reached the disk, and `page` calls `read` once per row, up to
//! `MAX_PAGE`. That is up to 64 `fsync`s per GET, a cost bounded by the page
//! and not by the index, which is what the numbers show; they are recorded in
//! `docs/06-limits.md`. It asserts nothing about time — a shared host's
//! numbers are not a gate — and it is `#[ignore]`d because it creates up to
//! 10,000 synced journals, so it is run on purpose:
//!
//! ```text
//! cargo test -p cli --test audit_page_latency -- --ignored --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    reason = "a measurement that prints what it measured: a test that cannot \
              panic cannot fail, and the numbers are its output"
)]

use std::path::PathBuf;
use std::time::Instant;

use cli::operation_audit::{MAX_PAGE, Origin, begin, page};

#[test]
#[ignore = "a measurement, run on purpose: see the module doc"]
fn a_full_audit_page_costs_the_same_at_every_index_size() {
    for n in [100_u64, 1_000, 10_000] {
        let root: PathBuf = std::env::temp_dir().join(format!(
            "brutex-audit-page-latency-{n}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        for _ in 0..n {
            drop(begin(&root, Origin::Http, "GET /backtest.json").unwrap());
        }
        let mut ns: Vec<u128> = (0..200)
            .map(|_| {
                let start = Instant::now();
                let rows = page(&root, None, MAX_PAGE).unwrap();
                let took = start.elapsed().as_nanos();
                assert_eq!(rows.len(), MAX_PAGE);
                took
            })
            .collect();
        ns.sort_unstable();
        let at = |q: usize| ns[(ns.len() * q / 1_000).min(ns.len() - 1)];
        println!(
            "audit page of {MAX_PAGE}, {n:>6} indexed: p50 {:>9} ns  p99 {:>9} ns  max {:>9} ns",
            at(500),
            at(990),
            ns[ns.len() - 1]
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

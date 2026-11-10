//! Findings of the o1p99 attack on `crates/pull` (cloud lane
//! `cloud/o1p99-attack-20261110`, 10 Nov 2026). `O1-ATTACK-REPORT.md` at the
//! repository root has the measurements, the likely cause and a proposed fix.
//!
//! EVERY TEST HERE IS `#[ignore]`d BECAUSE IT FAILS on the code it was
//! written against. Run by hand, in the optimised test profile:
//!
//! ```text
//! cargo test --locked -p pull --test o1p99_findings -- --ignored --nocapture
//! ```
//!
//! Un-ignore a test when its fix lands; it is then the regression guard.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fmt::Write as _;
use std::path::PathBuf;

#[allow(
    dead_code,
    reason = "the bench harness is shared with the findings and not every helper is called here"
)]
#[path = "../benches/o1p99/harness.rs"]
mod harness;

use harness::{FACTOR_PERMILLE, measure, permille, verdict};
use pull::master_snapshots::{self, Book, Kind};
use pull::session::Day;

/// FINDING O1P99-F-PULL-01 — `alerts::fit_text` walks and copies the whole
/// caller text to keep at most `TEXT_MAX` (75) bytes of it.
///
/// Claim: `CLAUDE.md` section 3 rule 4 (constant per-operation cost) and
/// `docs/07-o1-architecture.md` law 5, "Bound every input at the boundary".
/// `crates/pull/src/alerts.rs`, `fit_text`: `text.chars().map(..).collect()`
/// builds a cleaned copy of the WHOLE text, then cuts it to `TEXT_MAX`.
/// `put_at` (whose own claim is flat in the queue's length) calls it under
/// the queue's exclusive lock. Measured on the attack host: quietest-round
/// p99 29 ns at 1 byte, 35 us at 10^4 bytes, 3.9 ms at 10^6 bytes.
#[test]
#[ignore = "o1p99 finding O1P99-F-PULL-01: fails today, fit_text is linear in a text it cuts to 75 bytes"]
fn finding_pull_alert_text_costs_its_whole_length_not_text_max() {
    let sizes: [u64; 2] = [u64::try_from(pull::alerts::TEXT_MAX).unwrap(), 1_000_000];
    let clock = harness::clock_ps();
    let mut texts: Vec<String> = sizes
        .iter()
        .map(|&n| {
            "an invented alert "
                .chars()
                .cycle()
                .take(usize::try_from(n).unwrap())
                .collect()
        })
        .collect();
    let tails = measure(
        &mut texts,
        5,
        400,
        1,
        clock,
        |_| {},
        |t, _| {
            let (kept, cut) = pull::alerts::fit_text(t.as_str());
            u64::try_from(kept.len()).unwrap() + u64::from(cut)
        },
    );
    let flat = verdict(
        "O1P99-F-PULL-01",
        "alerts::fit_text: TEXT_MAX bytes -> 10^6 bytes",
        &sizes,
        &tails,
        FACTOR_PERMILLE,
    );
    assert!(
        flat,
        "a 10^6-byte alert text costs {} a TEXT_MAX one",
        harness::times(permille(tails[0].quiet_p99_ps, tails[1].quiet_p99_ps))
    );
}

/// A 40-row Kite dump: three futures for each of ten `ZQ` underlyings and
/// ten options (the shape of C-MS-02's dump at a fiftieth of its rows).
fn small_dump() -> String {
    let mut csv = String::from(pull::vendor::KITE_MASTER_HEADER);
    for row in 0..40_u64 {
        let underlying = row % 10;
        let token = 9_100_000 + row;
        if row < 30 {
            let month = row / 10 + 1;
            write!(
                csv,
                "\n{token},{row},ZQ{underlying:02}F{month},\"ZQ{underlying:02}\",0,2030-0{month}-28,0,0.05,75,FUT,NFO-FUT,NFO"
            )
            .unwrap();
        } else {
            write!(
                csv,
                "\n{token},{row},ZQ{underlying:02}C{row},\"ZQ{underlying:02}\",0,2030-01-28,{row},0.05,75,CE,NFO-OPT,NFO"
            )
            .unwrap();
        }
    }
    csv
}

/// FINDING O1P99-F-PULL-02 — `fnowork::futures_plan` grows with the
/// snapshots held.
///
/// Claim: C-MS-02 (`docs/04-invariants.md`, `crates/pull/benches/ratio/
/// master_snapshots.rs`): the plan "reads the index and parses the newest
/// snapshot alone, so its cost must not grow with the snapshots held". C-MS-02
/// holds 1, 10 and 100 WEEKLY snapshots of a 2,000-row dump, so parsing the
/// newest dump is most of every call and the per-snapshot work hides under
/// it. With a 40-row dump and DAILY snapshots: quietest-round p99 110 to
/// 128 us at 1 snapshot, 280 to 390 us at 100, 2.0 to 2.2 ms at 1,000
/// (15.6x to 20.1x over two runs), about 2 us a snapshot. The inventory
/// trace names three O(snapshots) steps: `Book::open` lists and sorts the
/// snapshot directory, `first_listed::list_in` reads the whole
/// `absorbed.idx` (one line per snapshot ever taken), and the plan loops
/// over every stamp up to `through` (`fnowork.rs`).
#[test]
#[ignore = "o1p99 finding O1P99-F-PULL-02: fails today, futures_plan is linear in the snapshots held"]
fn finding_pull_futures_plan_grows_with_the_snapshots_held() {
    let sizes: [u64; 2] = [1, 1_000];
    let first = Day::new(2024, 1, 1).unwrap();
    let window = pull::session::Window::new(first, Day::new(2026, 12, 31).unwrap()).unwrap();
    let today = Day::new(2027, 1, 1).unwrap();
    let body = small_dump();
    let clock = harness::clock_ps();
    let mut roots: Vec<PathBuf> = sizes
        .iter()
        .map(|&count| {
            let root = std::env::temp_dir().join(format!(
                "brutex-o1p99-finding-cms02-{count}-{}",
                std::process::id()
            ));
            let _ignored = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            for d in 0..u32::try_from(count).unwrap() {
                let on = Day::from_days(first.days_from_epoch() + d).unwrap();
                master_snapshots::snapshot(&root, Kind::Kite, on, "bench://o1p99", body.as_bytes())
                    .unwrap();
            }
            root
        })
        .collect();
    let tails = measure(
        &mut roots,
        4,
        100,
        1,
        clock,
        |_| {},
        |root, _| {
            let book = Book::open(root).unwrap();
            let plan = pull::fnowork::futures_plan(
                &book,
                pull::vendor::ZERODHA_CONTINUOUS,
                "ZQ03",
                window,
                today,
                window.to(),
            )
            .unwrap();
            u64::try_from(plan.asks.len()).unwrap()
        },
    );
    let flat = verdict(
        "O1P99-F-PULL-02",
        "Book::open + futures_plan: 1 daily snapshot -> 1,000",
        &sizes,
        &tails,
        FACTOR_PERMILLE,
    );
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    assert!(
        flat,
        "a plan over 1,000 snapshots costs {} a plan over one",
        harness::times(permille(tails[0].quiet_p99_ps, tails[1].quiet_p99_ps))
    );
}

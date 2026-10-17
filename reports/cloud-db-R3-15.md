STATUS: IN PROGRESS

# R3-15 (cloud): Speed page, browser harness, speed run at every deploy, page budgets

Branch `cloud/db-R3-15-20261017`, from `origin/feat/db-fix` at `f9cdb584`
(Merge R3-14).

## Prerequisites

Both merged into `origin/feat/db-fix`:

- `Merge R3-14: Measured everywhere: p50 and p99 per route kept by the app, ...`
- `Merge WP-58: All sources page`

## Done

- Read the cloud rules, CONTEXT.md, R3-15.md and CLAUDE.md.
- Baseline gates on the base (see below).

## Left

Everything in the work package: Speed page, `speed.js`, `page-report.js`,
header-only up check, browser harness, speed sheet, bundle budget, tests.

## Gate results on the base (`f9cdb584`)

- `cargo fmt --all --check`: EXIT 0
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: EXIT 0
- `cargo deny check`: EXIT 1, exactly the base's five (RUSTSEC-2026-0344
  ruzstd; RUSTSEC-2026-0346/0347/0348/0349 rustls). Owned by DENY-1.
- `node --test web/tests/*.test.js` (node 22.22.0 here): 1400 pass, 0 fail,
  5 cancelled before any change (`ask.test.js` x3, `census-worker.test.js`
  x2: "Promise resolution is still pending but the event loop has already
  resolved").
- `cargo test --workspace --locked`: running.

## NEEDS THE MAC

To be filled in as the part lands.

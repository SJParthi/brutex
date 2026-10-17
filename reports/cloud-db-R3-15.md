STATUS: IN PROGRESS

# R3-15 (cloud): Speed page, browser harness, speed run at every deploy, page budgets

Branch `cloud/db-R3-15-20261017`, from `origin/feat/db-fix` at `f9cdb584`
(Merge R3-14).

## Prerequisites

Both merged into `origin/feat/db-fix`:

- `Merge R3-14: Measured everywhere: p50 and p99 per route kept by the app, ...`
- `Merge WP-58: All sources page`

## Done so far

- `web/src/lib/speed.js`: the pure half (targets, `/speed.json` reader, table rows,
  health tile, nearest-rank summary, ignored-field verdict, page budget over the built files).
- `web/src/routes/speed/+page.svelte`: the Speed page.
- `web/src/lib/page-report.js`: one shared reporter (failed request, uncaught error,
  fallback) to `POST /page-report`, installed by one import line in `+layout.svelte`.
- `web/src/lib/api-status.js`: header-only up check `HEAD /inspection.json`.
- `web/vite.config.js`: `/speed.json` and `/page-report` proxied; the page budget plugin.
- `web/ci/speed-sheet.mjs`, `web/ci/browser-harness.mjs`, `web/tests/speed.test.js`.
- 45 tests pass (`speed.test.js` 40 + the pinned `api-status.test.js` 5), with 100% line,
  branch and function coverage on every touched JS module.

## Left

Browser harness at n = 200 against the real `api` binary (running), speed sheet against it,
mutation pass, full web suite, `cargo test` on the branch, final report.

## Gate results on the base (`f9cdb584`)

- `cargo fmt --all --check`: EXIT 0
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: EXIT 0
- `cargo deny check`: EXIT 1, exactly the base's five (RUSTSEC-2026-0344; RUSTSEC-2026-0346,
  0347, 0348, 0349). Owned by DENY-1.
- `cargo test --workspace --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`:
  EXIT 101, 27 failures in this container (runs as root; see the final report).
- web tests (node 22.22.0): 1400 pass, 0 fail, 5 cancelled.
- `svelte-check`: 7 errors on the base (none in an R3-15 file).

## NEEDS THE MAC

To be completed in the final report.

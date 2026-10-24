STATUS: HELD

# R4-05 (cloud): one reasons dictionary with its gate and route, and the kept alert kinds

Branch `cloud/db-R4-05-20261024`, cut from `origin/feat/db-fix` at `e851da1c` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/R4-05.md`.

## Prerequisites

- IA-16: merged (`Merge IA-16: failure kinds and causes ...` on `origin/feat/db-fix`).
- R3-21: FILES_ONLY link (merge order on `crates/pull/src/alerts.rs` KINDS), not merged; does not block the start.
- IA-12a: dropped (D-6900), not a prerequisite.

## Done so far

- `crates/pull/src/reasons.rs` (new): `TABLE` (a `static`), 184 rows `(code, sentence, action, control)`; `Control`
  = `Repair` ("press Repair"), `RetryRefused` ("Retry refused"), `StoreStep(id)` ("store step <id>"),
  `Operator(what)` ("operator: <what>"), `Nothing` ("no action"); `of(code)` one hash probe; `title(row)`
  = `code: sentence; control`; `body(build)` the `/reasons.json` document and its ETag
  `"<build>.<16 hex of blake3(bytes)>"`, one `pull.reasons` / `reasons table built` event per call.
- `crates/pull/src/reasons_tests.rs` (new): the gate. Sources: `ImportRefusal::code` (parsed),
  `cm_cause`/`nfo_cause`/`import_cause` (parsed), `FailKind::code` (parsed; the store's repair refusals
  reach pages through `FailKind::of_repair`), `RunRefusal::CODES` (called, cross-checked against its
  parsed `code()`), `store::verdict::Code::ALL` and `flags::ALL` (called), `RowCause::code` (parsed,
  `crate::listing::` constants resolved from `listing.rs`) and `HistoryEnds::CODE` (called),
  `verdict_words::REASONS` (called). The parser refuses an unreadable arm, a wildcard arm and any string
  literal it did not take, by name.
- `crates/pull/src/alerts.rs`: KINDS 22 `vendor_format_changed`, 23 `vendor_restated`, 24 `scrub_stale`,
  25 `credential_dead`; one key builder per kind (`vendor_format_changed`, `vendor_restated`,
  `scrub_stale`, `credential_dead` -> `Keyed`), each part checked by `key_part` (alphabet
  `[A-Za-z0-9._&-]`, no token shape), refusal `alert_key_part_refused` never repeating the value.
- `crates/pull/src/alerts_tests.rs`: KINDS pin extended to 25; the old unknown-kind probes moved from 22
  to 26 (`kind_word(26)`, `garbage_alert_parameters_are_refused_by_name`); 7 new tests.
- `crates/pull/src/lib.rs`: one `pub mod reasons;` line (shared line).

## Measured so far (in process, this container: 4 cores, `test` profile; n and method per row)

| What | n | p50 | p99 | max | Method |
|---|---|---|---|---|---|
| `GET /reasons.json`, 1 client, sequential | 200 | 5.2 µs | 24.8 µs | 54.6 µs | handler in process, `Instant` per request (first sample 18.9 µs) |
| `GET /reasons.json`, 60 clients, half revalidating (304) | 12,000 | 2.8 µs | 7.2 µs | 925 µs | 60 tokio tasks x 200 requests, 4 worker threads |
| `pull::reasons::body` (the cold build, once per process) | 200 | 1.03 ms | 1.45 ms | 1.70 ms | 184 rows, 68,971 bytes |
| `pull::reasons::of` | 200 | 64 ns | 183 ns | 445 ns | half known, half unknown codes |
| `titleFor` (browser, Node 22) | 200 | 0.0019 ms | 0.0109 ms | 4.1 ms | 100,000-row table, `performance.now` |

Not measured here (NEEDS THE MAC item 7): the socket's share, the shadow server, an import writing.

## Gates so far (the final run comes after the last edit and the final merge)

- `cargo fmt --all --check`: clean.
- `cargo clippy -p pull -p api --all-targets --locked -- -D warnings`: clean (after fixing 4 findings
  of mine: `format_collect` twice, `bool_assert_comparison`, `borrow_as_ptr`). The workspace run comes
  in the final pass.
- `cargo test -p pull --lib --locked -- reasons:: alerts::`: all pass but one root-only test,
  `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`
  (expects `PermissionDenied` from a 0555 directory; this container runs as uid 0, which the mode does
  not stop). Not caused by this change; to be shown red on the unmodified base in the final pass.
- `cargo test -p api --lib --locked -- reasons_route pool_table operation_audit`: all pass.
- Web: `node --test tests/reasons.test.js tests/import-panel.test.js tests/import-panel-archive.test.js`
  -> 19 pass.
- CI shell gates (run from `ci.yml`'s own `run:` blocks by a local extractor, on this branch and on a
  clean worktree of the base `e851da1c`): PASS on both: 0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 7, 13, 15, 16,
  17, 21, 22, 24, 28, 25, 27, 26, 14, 29, 29b; 6c on the two `.github/*.rs` tools I edited (rustfmt and
  clippy-driver, lib and test). RED on both, the base reds CONTEXT §4.10 names (BASE-REDS-1023):
  1c, 1d, 10, 11, 12, 19, 23, 27b, 30. Inside them this change adds nothing: 1c, 27b, 30 identical;
  1d identical once R4-05's words are declared (`R4_05_LITERAL` in `.github/gates_tree.rs`); 12 the
  same 48 refusals once my cost claims name their tests; 23 lists my two measurement prints as declared
  (in `ci.yml`) and measured; 10, 11, 19 differ only in counts. Not run here: 1e (a whole-workspace
  build and test with `web/` moved aside: the workspace test gate covers its test half).

## Process slips, said plainly

- Three shell commands in this session named or ran the Py-language interpreter by mistake: a version
  probe, an empty no-op script, and a throwaway shell variable whose name held the word. None ran any
  code of that language, nothing it printed was used, and nothing of it is in any file.

## Log
- 2026-10-24 (IST morning): GO seen, spec read, branch cut.
- 2026-10-24: table, gate and alert kinds built; tests green but the root-only one. HOLD seen at this
  save point: committed and pushed, waiting for GO without HOLD.
  NEXT: `crates/api/src/reasons_route.rs` (route, server.rs lines, pool_table_tests and
  operation_audit_tests rows, bench), then the web half, docs rows, final gates.
- 2026-10-24 12:37 IST: HOLD lifted on the base (`55dfb044`); README now carries a per-part GO rule
  and "no typed number outside the Winner Gate" (the title cut is therefore served as `title_cut`,
  the table's own longest title). Built `crates/api/src/reasons_route.rs` (OnceLock body, ETag,
  `no-cache`, 304), its `server.rs` mod and route lines, the `pool_table_tests` and
  `operation_audit_tests` rows, `/reasons.json` in `.github/gates_routes.rs`'s compiled-table owed list;
  `web/src/lib/reasons.js`, `import-panel.js` (`outcomeWord`, `reasonOf`, `cells`), `ImportPanel.svelte`
  (titles, control chip, unknown states); `web/tests/reasons.test.js` and two tests in
  `import-panel.test.js`; `docs/02` §38 rows and key rules; `docs/05` D-TBD-R4-05-1..5.
  `node --test tests/reasons.test.js tests/import-panel.test.js tests/import-panel-archive.test.js`
  -> 19 pass, 0 fail.
  NEXT: run the api tests (route, pool table, audit table), docs/04 and docs/06 rows with the measured
  numbers, then the final merge of feat/db-fix and every gate.
- 2026-10-24 ~13:30 IST: api tests green (route, pool table, audit table); docs/04 rows R405-01..12,
  docs/06 rows L-R405-01..06 with the measurements above; workspace clippy found 4 findings of mine
  (fixed); CI shell gates run on branch and base (results above); `.github/gates_tree.rs` gains
  `R4_05_LITERAL` (gate 1d), `ci.yml` gate 23 declares my two measurement prints, cost claims name
  their tests (gate 12). The base was re-HELD at 12:56 IST for parts without `GO-R4-05`: this save
  point is committed and pushed, and I wait. `origin/feat/db-fix` moved to `1c094c90` (Merge IA-4).
  NEXT: merge `origin/feat/db-fix` (`1c094c90` or newer), then the final gate pass: fmt, workspace
  clippy, workspace test, deny (installed: cargo-deny 0.20.2), llvm-cov and mutants on the touched
  modules, the web gates W1 to W5 (web/build rebuilt), the shell gates again; then STATUS: DONE.
- 2026-10-24 17:34 IST: HOLD lifted. Merged `origin/feat/db-fix` at `7b397bbe` (R4-27, IA-4,
  CLOUD-MUTSTORE, CLOUD-MUTCLI; merge `eb1de065`): the three ledgers conflicted textually only and
  were resolved "both, in merge order" (the base's sections, then R4-05's); KINDS at the head still
  ends at 21 and R3-21 is still not merged. After the merge: the reasons gate 20/20 and alerts 49/51
  (one ignored, the root-only failure). `cargo clippy --workspace --all-targets --locked -- -D warnings`
  -> EXIT 0. `cargo deny check` -> advisories FAILED on RUSTSEC-2026-0386 (tokio `AsyncFd`, unsound),
  red the same on the unmodified base `7b397bbe`; this branch touches no manifest and no lock: NEEDS
  THE MAC. Web: W1 passes once `web/build` is rebuilt and committed (it is, here; a second build
  reproduces it); W2 found one failure of mine, `proxy.test.js` (the dev proxy did not name
  `/reasons.json`: fixed in `web/vite.config.js`), and 5 cancelled timer tests in `ask.test.js` and
  `census-worker.test.js` that are cancelled identically on the base under this container's Node 22
  (CI runs Node 24): NEEDS THE MAC; W3 32 svelte-check errors, the base's own 32 (my one extra, in my
  test, fixed); W4 0 unused selectors; W5 clean. HOLD set again at 18:16 IST: this save point is
  pushed; the detached workspace test run (`cargo test --workspace --locked --no-fail-fast`) finishes
  on its own while I wait.
  NEXT: read the workspace test result; coverage (`cargo llvm-cov --branch`) and `cargo mutants
  --in-diff` on the touched modules; the shell gates once more; then STATUS: DONE.

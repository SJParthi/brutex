STATUS: HELD

# R4-05 (cloud): one reasons dictionary with its gate and route, and the kept alert kinds

Branch `cloud/db-R4-05-20261024`, cut from `origin/feat/db-fix` at `e851da1c` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/R4-05.md`.

## Prerequisites

- IA-16: merged (`Merge IA-16: failure kinds and causes ...` on `origin/feat/db-fix`).
- R3-21: FILES_ONLY link (merge order on `crates/pull/src/alerts.rs` KINDS), not merged; does not block the start.
- IA-12a: dropped (D-6900), not a prerequisite.

## Done so far

- `crates/pull/src/reasons.rs` (new): `TABLE`, 187 rows `(code, sentence, action, control)`; `Control`
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

## Measured so far

- `pull::reasons::of`: n=200 sequential lookups (half known, half unknown), `Instant` per call, debug
  build in this container: p50=60 ns, p99=101 ns, max=157 ns.

## Gates so far (partial; the final run comes after the last edit)

- `cargo test -p pull --lib --locked -- reasons:: alerts::` -> 87 pass, 1 FAIL:
  `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`
  expects `PermissionDenied` from a 0555 directory; this container runs as root (uid 0), which the mode
  does not stop. Not caused by this change; to be confirmed red on the unmodified base.

## Log

- 2026-10-24 (IST morning): GO seen, spec read, branch cut.
- 2026-10-24: table, gate and alert kinds built; tests green but the root-only one. HOLD seen at this
  save point: committed and pushed, waiting for GO without HOLD.
  NEXT: `crates/api/src/reasons_route.rs` (route, server.rs lines, pool_table_tests and
  operation_audit_tests rows, bench), then the web half, docs rows, final gates.

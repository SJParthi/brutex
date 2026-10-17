STATUS: IN PROGRESS

# R3-24 (cloud): logs and audit — audit lines never roll away, more search keys, every state change audited

Branch `cloud/db-R3-24-20261017`, cut from `origin/feat/db-fix` at `f9cdb584`
(Merge R3-14). Prerequisites confirmed merged into `origin/feat/db-fix`
(`git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-14|R3-16|WP-62):"`):
R3-14, R3-16, WP-62 — all three present.

Every file changed is one R3-24 owns (`git diff --stat origin/feat/db-fix -- crates`):
`crates/api/src/logs.rs`, `crates/api/src/audit_json.rs`,
`crates/api/benches/ratio/logs_dirs.rs`, `crates/telemetry/src/sink.rs`,
`crates/telemetry/src/run_dirs.rs`, `crates/telemetry/src/state_events.rs` (new),
`crates/telemetry/src/state_events_tests.rs` (new). Plus this report.

## Done (code and tests pushed)

1. State-change table, state stream, reach (goal items 1 and 4).
2. Search keys `month`, `reason`, `kind`, `from`/`to`, target alone, and "the first 2,000 of N" (goal item 2).
3. Contradicted "day imported" lines marked with the journal's outcome (goal item 3, DB-A48).
4. Importer refusals: a fixed slice per request, a background follower, and the waiting-delete counters on every answer (goal item 5).
5. The `/logs` picture from write-time counters, and the live tail from memory (goal items 2 and 6).
6. `/audit.json`: the store's log reach and the newest state changes.
7. Bench row C-LG-03.

Details, gate results and NEEDS THE MAC are filled in at the final push.

## Left

- Gates: workspace tests (running), coverage, mutants, bench run. Final report.

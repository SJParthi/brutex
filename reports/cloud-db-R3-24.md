STATUS: IN PROGRESS

# R3-24 (cloud): logs and audit — audit lines never roll away, more search keys, every state change audited

Branch `cloud/db-R3-24-20261017`, cut from `origin/feat/db-fix` at `f9cdb584`
(Merge R3-14). Prerequisites, confirmed merged into `origin/feat/db-fix` with
`git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-14|R3-16|WP-62):"`:
R3-14, R3-16 and WP-62, all three present.

Every file changed is one R3-24 owns (`git diff --stat origin/feat/db-fix -- crates`):

| file | what changed |
|---|---|
| `crates/telemetry/src/state_events.rs` (new) | the state-change table: 21 rows, 5 classes, each bound to its emit site |
| `crates/telemetry/src/state_events_tests.rs` (new) | `every_state_change_event_is_audited` and four more |
| `crates/telemetry/src/sink.rs` | the never-rolling state stream, per-file time spans, reach, the picture counters, the live copy, `reason`/`kind` index dimensions |
| `crates/telemetry/src/run_dirs.rs` | hosts `state_events` (`#[path]`), `STATE_STREAM_DIR`, and re-exports R3-24's new types (see "Shared lines" below) |
| `crates/api/src/logs.rs` | the new keys, the time-span search, target alone through the index, "the first 2,000 of N", reach and its alert, the contradicted-line mark, the picture page, the live tail from memory, the refusal follower, both streams' health |
| `crates/api/src/audit_json.rs` | `/audit.json` names the store's log reach and its newest state changes |
| `crates/api/benches/ratio/logs_dirs.rs` | bench row C-LG-03 |

The details, gates and NEEDS THE MAC follow at the final push.
